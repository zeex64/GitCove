//! Open pull/merge requests from GitHub, GitLab and Gitea/Forgejo, unauthenticated.

use crate::git::error::{GitError, GitResult};
use crate::git::refs::{list_remotes, parse_remote_url};
use crate::git::runner::Git;
use crate::git::types::{HostKind, PullRequest, PullRequestList, PullRequestListStatus, PullRequestState};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

fn private_msg(host: HostKind) -> String {
    if host == HostKind::Github {
        "This repository is private, or your GitHub account can't see it. Sign in from the account button in the title bar to list its pull requests.".into()
    } else {
        "This repository is private or not public. Signing in to this host isn't available yet.".into()
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(6)))
        .http_status_as_error(false)
        .user_agent(concat!("gitcove/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn encode_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Parse RFC 3339 timestamps ("2024-01-02T03:04:05Z", "…05.123+02:00") to unix seconds.
pub fn parse_rfc3339(s: &str) -> i64 {
    let s = s.trim();
    if s.len() < 19 {
        return 0;
    }
    let num = |a: usize, b: usize| s.get(a..b).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
    let (y, mo, d, h, mi, se) = (num(0, 4), num(5, 7), num(8, 10), num(11, 13), num(14, 16), num(17, 19));
    // Days from civil (Howard Hinnant's algorithm).
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let mut t = days * 86_400 + h * 3600 + mi * 60 + se;
    // Offset
    let rest = &s[19..];
    let rest = rest.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    if let Some(sign) = rest.chars().next() {
        if (sign == '+' || sign == '-') && rest.len() >= 6 {
            let oh: i64 = rest[1..3].parse().unwrap_or(0);
            let om: i64 = rest[4..6].parse().unwrap_or(0);
            let off = oh * 3600 + om * 60;
            t += if sign == '+' { -off } else { off };
        }
    }
    t
}

fn str_at<'a>(v: &'a Value, path: &[&str]) -> &'a str {
    let mut cur = v;
    for p in path {
        cur = &cur[*p];
    }
    cur.as_str().unwrap_or("")
}

fn looks_draft(title: &str) -> bool {
    let t = title.trim_start().to_ascii_lowercase();
    t.starts_with("wip:") || t.starts_with("[wip]") || t.starts_with("draft:") || t.starts_with("[draft]")
}

fn parse_github_like(items: &Value) -> Vec<PullRequest> {
    items
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|pr| {
                    let title = str_at(pr, &["title"]).to_string();
                    let draft = pr["draft"].as_bool().unwrap_or(false) || looks_draft(&title);
                    PullRequest {
                        number: pr["number"].as_u64().unwrap_or(0),
                        author: str_at(pr, &["user", "login"]).to_string(),
                        state: if draft { PullRequestState::Draft } else { PullRequestState::Open },
                        url: str_at(pr, &["html_url"]).to_string(),
                        source_branch: str_at(pr, &["head", "ref"]).to_string(),
                        target_branch: str_at(pr, &["base", "ref"]).to_string(),
                        updated_at: parse_rfc3339(str_at(pr, &["updated_at"])),
                        title,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_gitlab(items: &Value) -> Vec<PullRequest> {
    items
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|mr| {
                    let title = str_at(mr, &["title"]).to_string();
                    let draft = mr["draft"].as_bool().unwrap_or(false)
                        || mr["work_in_progress"].as_bool().unwrap_or(false)
                        || looks_draft(&title);
                    PullRequest {
                        number: mr["iid"].as_u64().unwrap_or(0),
                        author: str_at(mr, &["author", "username"]).to_string(),
                        state: if draft { PullRequestState::Draft } else { PullRequestState::Open },
                        url: str_at(mr, &["web_url"]).to_string(),
                        source_branch: str_at(mr, &["source_branch"]).to_string(),
                        target_branch: str_at(mr, &["target_branch"]).to_string(),
                        updated_at: parse_rfc3339(str_at(mr, &["updated_at"])),
                        title,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn list(remote: &str, host: HostKind, status: PullRequestListStatus, message: Option<String>, items: Vec<PullRequest>) -> PullRequestList {
    PullRequestList { remote: remote.to_string(), host, status, message, items }
}

pub fn list_pull_requests(repo: &Path, remote_name: &str) -> GitResult<PullRequestList> {
    let git = Git::new(repo);
    let remotes = list_remotes(&git)?;
    let remote = remotes
        .iter()
        .find(|r| r.name == remote_name)
        .ok_or_else(|| GitError::not_found(format!("There's no remote named {remote_name}. Refresh and try again.")))?;
    let info = parse_remote_url(&remote.fetch_url);
    let host = info.host;
    let (Some(hostname), Some(owner), Some(name)) = (info.hostname.clone(), info.owner.clone(), info.repo.clone()) else {
        return Ok(list(
            remote_name,
            host,
            PullRequestListStatus::Unsupported,
            Some("This remote's URL doesn't point to a hosted repository, so there are no pull requests to show.".into()),
            vec![],
        ));
    };
    let url = match host {
        HostKind::Github => format!("https://api.github.com/repos/{owner}/{name}/pulls?state=open&per_page=30"),
        HostKind::Gitlab => format!(
            "https://{hostname}/api/v4/projects/{}/merge_requests?state=opened&per_page=30",
            encode_component(&format!("{owner}/{name}"))
        ),
        HostKind::Forgejo | HostKind::Gitea => {
            format!("https://{hostname}/api/v1/repos/{owner}/{name}/pulls?state=open&limit=30")
        }
        HostKind::Bitbucket | HostKind::Other => {
            return Ok(list(
                remote_name,
                host,
                PullRequestListStatus::Unsupported,
                Some("Pull requests are available for GitHub, GitLab, Gitea and Forgejo remotes.".into()),
                vec![],
            ));
        }
    };
    let mut req = agent().get(&url).header("Accept", "application/json");
    if host == HostKind::Github {
        req = req.header("Accept", "application/vnd.github+json").header("X-GitHub-Api-Version", "2022-11-28");
        // Signed in: private repositories list their pull requests too.
        if hostname == "github.com" {
            if let Some(token) = crate::github::current_token() {
                req = req.header("Authorization", &format!("Bearer {token}"));
            }
        }
    }
    let mut resp = match req.call() {
        Ok(r) => r,
        Err(_) => {
            return Ok(list(
                remote_name,
                host,
                PullRequestListStatus::Offline,
                Some(format!("Couldn't reach {hostname}. Check your connection, then try again.")),
                vec![],
            ));
        }
    };
    let status = resp.status().as_u16();
    let header = |name: &str| resp.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
    if status == 404 || status == 401 {
        return Ok(list(remote_name, host, PullRequestListStatus::Private, Some(private_msg(host)), vec![]));
    }
    if status == 429 || (status == 403 && (header("x-ratelimit-remaining").as_deref() == Some("0") || header("retry-after").is_some())) {
        return Ok(list(
            remote_name,
            host,
            PullRequestListStatus::RateLimited,
            Some(format!("{hostname} is limiting requests from this network right now. Try again in a few minutes.")),
            vec![],
        ));
    }
    if status == 403 {
        return Ok(list(remote_name, host, PullRequestListStatus::Private, Some(private_msg(host)), vec![]));
    }
    if !(200..300).contains(&status) {
        return Ok(list(
            remote_name,
            host,
            PullRequestListStatus::Error,
            Some(format!("{hostname} returned an error (HTTP {status}). Try again later.")),
            vec![],
        ));
    }
    let body: Value = match resp.body_mut().with_config().limit(10 * 1024 * 1024).read_json() {
        Ok(v) => v,
        Err(_) => {
            return Ok(list(
                remote_name,
                host,
                PullRequestListStatus::Error,
                Some(format!("{hostname} sent a response that couldn't be read.")),
                vec![],
            ));
        }
    };
    let items = match host {
        HostKind::Gitlab => parse_gitlab(&body),
        _ => parse_github_like(&body),
    };
    Ok(list(remote_name, host, PullRequestListStatus::Ok, None, items))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), 0);
        assert_eq!(parse_rfc3339("2024-01-02T03:04:05Z"), 1_704_164_645);
        assert_eq!(parse_rfc3339("2024-01-02T05:04:05.123+02:00"), 1_704_164_645);
    }

    #[test]
    fn parses_github_and_gitlab_json() {
        let gh = serde_json::json!([{
            "number": 12, "title": "Add lanes", "draft": true, "html_url": "https://github.com/o/r/pull/12",
            "user": {"login": "alice"}, "head": {"ref": "feat"}, "base": {"ref": "main"},
            "updated_at": "2024-01-02T03:04:05Z"
        }]);
        let items = parse_github_like(&gh);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].number, 12);
        assert_eq!(items[0].state, PullRequestState::Draft);
        assert_eq!(items[0].author, "alice");
        assert_eq!(items[0].source_branch, "feat");
        assert_eq!(items[0].updated_at, 1_704_164_645);
        let gl = serde_json::json!([{
            "iid": 3, "title": "Fix", "work_in_progress": false, "draft": false, "web_url": "https://gitlab.com/g/p/-/merge_requests/3",
            "author": {"username": "bob"}, "source_branch": "fix", "target_branch": "main",
            "updated_at": "2024-01-02T03:04:05.000Z"
        }]);
        let items = parse_gitlab(&gl);
        assert_eq!(items[0].number, 3);
        assert_eq!(items[0].state, PullRequestState::Open);
        assert_eq!(items[0].author, "bob");
        assert_eq!(items[0].target_branch, "main");
    }

    #[test]
    fn encoding() {
        assert_eq!(encode_component("group/sub/proj"), "group%2Fsub%2Fproj");
    }
}
