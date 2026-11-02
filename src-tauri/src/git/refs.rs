//! Branches, remotes, tags and stashes.

use super::error::GitResult;
use super::runner::{Git, split_nul};
use super::types::{Branch, Head, HostKind, Refs, Remote, RemoteBranch, Stash, Tag};
use std::path::Path;

pub fn get_head(git: &Git) -> GitResult<Head> {
    let sym = git.try_read(["symbolic-ref", "-q", "HEAD"])?.map(|s| s.trim().to_string());
    let commit = git.rev_parse("HEAD")?;
    let branch = sym
        .as_deref()
        .map(|s| s.strip_prefix("refs/heads/").unwrap_or(s).to_string())
        .filter(|s| !s.is_empty());
    Ok(Head {
        detached: sym.is_none(),
        unborn: commit.is_none(),
        branch,
        commit,
    })
}

/// Parse "ahead 1, behind 2" / "gone" from `%(upstream:track,nobracket)`.
pub fn parse_track(track: &str) -> (u32, u32, bool) {
    let t = track.trim();
    if t == "gone" {
        return (0, 0, true);
    }
    let mut ahead = 0;
    let mut behind = 0;
    for part in t.split(',') {
        let mut it = part.split_whitespace();
        match (it.next(), it.next()) {
            (Some("ahead"), Some(n)) => ahead = n.parse().unwrap_or(0),
            (Some("behind"), Some(n)) => behind = n.parse().unwrap_or(0),
            _ => {}
        }
    }
    (ahead, behind, false)
}

pub fn list_remotes(git: &Git) -> GitResult<Vec<Remote>> {
    let text = git.read(["remote", "-v"])?;
    let mut remotes: Vec<Remote> = Vec::new();
    for line in text.lines() {
        let Some((name, rest)) = line.split_once('\t') else { continue };
        let (url, kind) = match rest.rsplit_once(' ') {
            Some((u, k)) => (u.trim(), k.trim()),
            None => (rest.trim(), "(fetch)"),
        };
        let idx = match remotes.iter().position(|r| r.name == name) {
            Some(i) => i,
            None => {
                let info = parse_remote_url(url);
                remotes.push(Remote {
                    name: name.to_string(),
                    fetch_url: url.to_string(),
                    push_url: url.to_string(),
                    host: info.host,
                    web_url: info.web_url,
                    owner: info.owner,
                    repo: info.repo,
                });
                remotes.len() - 1
            }
        };
        if kind == "(push)" {
            remotes[idx].push_url = url.to_string();
        } else {
            remotes[idx].fetch_url = url.to_string();
            let info = parse_remote_url(url);
            let r = &mut remotes[idx];
            r.host = info.host;
            r.web_url = info.web_url;
            r.owner = info.owner;
            r.repo = info.repo;
        }
    }
    Ok(remotes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteUrlInfo {
    pub host: HostKind,
    pub hostname: Option<String>,
    pub web_url: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
}

pub fn host_kind(hostname: &str) -> HostKind {
    let h = hostname.to_ascii_lowercase();
    if h == "github.com" || h.ends_with(".github.com") {
        HostKind::Github
    } else if h == "gitlab.com" || h.contains("gitlab") {
        HostKind::Gitlab
    } else if h == "codeberg.org" || h.contains("forgejo") {
        HostKind::Forgejo
    } else if h.contains("gitea") {
        HostKind::Gitea
    } else if h == "bitbucket.org" {
        HostKind::Bitbucket
    } else {
        HostKind::Other
    }
}

/// Derive host kind, owner, repo and browser URL from https / ssh / scp-like URLs.
pub fn parse_remote_url(url: &str) -> RemoteUrlInfo {
    let none = RemoteUrlInfo { host: HostKind::Other, hostname: None, web_url: None, owner: None, repo: None };
    let url = url.trim();
    let (hostname, path) = if let Some(idx) = url.find("://") {
        let scheme = &url[..idx];
        if scheme == "file" {
            return none;
        }
        let rest = &url[idx + 3..];
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i + 1..]),
            None => (rest, ""),
        };
        let hostport = authority.rsplit('@').next().unwrap_or(authority);
        let host = if hostport.starts_with('[') {
            hostport.split(']').next().unwrap_or(hostport).trim_start_matches('[')
        } else {
            hostport.split(':').next().unwrap_or(hostport)
        };
        (host.to_string(), path.to_string())
    } else if let Some(colon) = url.find(':') {
        // scp-like: [user@]host:path (but not a Windows drive letter or local path)
        let before = &url[..colon];
        if before.contains('/') || before.len() <= 1 {
            return none;
        }
        let host = before.rsplit('@').next().unwrap_or(before);
        (host.to_string(), url[colon + 1..].to_string())
    } else {
        return none;
    };
    if hostname.is_empty() {
        return none;
    }
    let hostname = if hostname.eq_ignore_ascii_case("ssh.github.com") { "github.com".to_string() } else { hostname };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let path = path.trim_matches('/');
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let host = host_kind(&hostname);
    if segs.len() < 2 {
        return RemoteUrlInfo { host, hostname: Some(hostname), web_url: None, owner: None, repo: None };
    }
    let repo = segs[segs.len() - 1].to_string();
    let mut owner_segs: Vec<&str> = segs[..segs.len() - 1].to_vec();
    // Strip common path prefixes of self-hosted instances served over SSH ("~user", "scm").
    if host == HostKind::Bitbucket || host == HostKind::Github {
        owner_segs = vec![owner_segs[owner_segs.len() - 1]];
    }
    let owner = owner_segs.join("/");
    let web_url = Some(format!("https://{hostname}/{owner}/{repo}"));
    RemoteUrlInfo { host, hostname: Some(hostname), web_url, owner: Some(owner), repo: Some(repo) }
}

pub fn list_stashes(git: &Git) -> GitResult<Vec<Stash>> {
    let Some(text) = git.try_read(["stash", "list", "--format=%gd%x00%H%x00%gs%x00%ct%x1e"])? else {
        return Ok(vec![]);
    };
    let mut out = Vec::new();
    for rec in text.split('\x1e') {
        let rec = rec.trim_start_matches(['\n', '\r']);
        if rec.is_empty() {
            continue;
        }
        let f: Vec<&str> = rec.split('\0').collect();
        if f.len() < 4 {
            continue;
        }
        let refname = f[0].trim().to_string();
        let index = refname
            .strip_prefix("stash@{")
            .and_then(|s| s.strip_suffix('}'))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(out.len() as u32);
        let (branch, message) = parse_stash_subject(f[2]);
        out.push(Stash {
            index,
            ref_: format!("stash@{{{index}}}"),
            commit: f[1].trim().to_string(),
            message,
            time: f[3].trim().parse().unwrap_or(0),
            branch,
        });
    }
    Ok(out)
}

/// "On main: msg" → (main, msg); "WIP on main: abc1234 subj" → (main, "WIP on main: abc1234 subj").
pub fn parse_stash_subject(subject: &str) -> (Option<String>, String) {
    let s = subject.trim();
    let named = |rest: &str| -> Option<(String, String)> {
        let (b, m) = rest.split_once(": ")?;
        Some((b.to_string(), m.to_string()))
    };
    if let Some(rest) = s.strip_prefix("On ") {
        if let Some((b, m)) = named(rest) {
            let branch = if b == "(no branch)" { None } else { Some(b) };
            return (branch, m);
        }
    }
    if let Some(rest) = s.strip_prefix("WIP on ") {
        if let Some((b, _)) = named(rest) {
            let branch = if b == "(no branch)" { None } else { Some(b) };
            return (branch, s.to_string());
        }
    }
    (None, s.to_string())
}

pub fn get_refs(repo: &Path) -> GitResult<Refs> {
    let git = Git::new(repo);
    let head = get_head(&git)?;
    let remotes = list_remotes(&git)?;
    let text = git.read([
        "for-each-ref",
        "--format=%(refname)%00%(objectname)%00%(*objectname)%00%(upstream:short)%00%(upstream:track,nobracket)%00%(HEAD)%00%(objecttype)%00%(symref)%1e",
        "refs/heads",
        "refs/remotes",
        "refs/tags",
    ])?;
    let mut branches = Vec::new();
    let mut remote_branches = Vec::new();
    let mut tags = Vec::new();
    let mut remote_names: Vec<&str> = remotes.iter().map(|r| r.name.as_str()).collect();
    remote_names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    for rec in text.split('\x1e') {
        let rec = rec.trim_start_matches(['\n', '\r']);
        if rec.is_empty() {
            continue;
        }
        let f: Vec<&str> = rec.split('\0').collect();
        if f.len() < 8 {
            continue;
        }
        let refname = f[0];
        let oid = f[1].to_string();
        if let Some(name) = refname.strip_prefix("refs/heads/") {
            let upstream = Some(f[3].to_string()).filter(|s| !s.is_empty());
            let (ahead, behind, gone) = parse_track(f[4]);
            branches.push(Branch {
                name: name.to_string(),
                commit: oid,
                upstream_gone: upstream.is_some() && gone,
                upstream,
                ahead,
                behind,
                is_head: f[5].trim() == "*",
            });
        } else if let Some(full) = refname.strip_prefix("refs/remotes/") {
            if !f[7].is_empty() || full.ends_with("/HEAD") {
                continue; // symbolic refs such as origin/HEAD
            }
            let remote = remote_names
                .iter()
                .find(|r| full.starts_with(&format!("{r}/")))
                .map(|s| s.to_string())
                .unwrap_or_else(|| full.split('/').next().unwrap_or("").to_string());
            let name = full[remote.len()..].trim_start_matches('/').to_string();
            remote_branches.push(RemoteBranch { remote, name, full_name: full.to_string(), commit: oid });
        } else if let Some(name) = refname.strip_prefix("refs/tags/") {
            let annotated = f[6] == "tag";
            let commit = if annotated && !f[2].is_empty() { f[2].to_string() } else { oid };
            tags.push(Tag { name: name.to_string(), commit, annotated });
        }
    }
    let stashes = list_stashes(&git)?;
    Ok(Refs { head, branches, remote_branches, remotes, tags, stashes })
}

/// Local branch name → sha.
pub fn local_branches(git: &Git) -> GitResult<Vec<(String, String)>> {
    let text = git.read(["for-each-ref", "--format=%(refname:strip=2)%00%(objectname)", "refs/heads"])?;
    Ok(text
        .lines()
        .filter_map(|l| l.split_once('\0'))
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect())
}

pub fn branch_exists(git: &Git, name: &str) -> GitResult<bool> {
    Ok(git.rev_parse(&format!("refs/heads/{name}"))?.is_some())
}

pub fn branch_tip(git: &Git, name: &str) -> GitResult<Option<String>> {
    git.rev_parse(&format!("refs/heads/{name}"))
}

/// `branch.<name>.*` config entries.
pub fn branch_config(git: &Git, name: &str) -> GitResult<Vec<(String, String)>> {
    let prefix = format!("branch.{name}.");
    let text = git.try_read(["config", "--local", "-z", "--get-regexp", &format!("^{}", regex_escape(&prefix))])?;
    let mut out = Vec::new();
    if let Some(text) = text {
        for rec in split_nul(&text) {
            let (k, v) = rec.split_once('\n').unwrap_or((rec, ""));
            out.push((k.to_string(), v.to_string()));
        }
    }
    Ok(out)
}

pub fn regex_escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        if "\\^$.|?*+()[]{}".contains(c) {
            o.push('\\');
        }
        o.push(c);
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_urls() {
        let i = parse_remote_url("git@github.com:owner/repo.git");
        assert_eq!(i.host, HostKind::Github);
        assert_eq!(i.web_url.as_deref(), Some("https://github.com/owner/repo"));
        let i = parse_remote_url("https://gitlab.com/group/sub/proj.git");
        assert_eq!(i.host, HostKind::Gitlab);
        assert_eq!(i.owner.as_deref(), Some("group/sub"));
        assert_eq!(i.repo.as_deref(), Some("proj"));
        let i = parse_remote_url("ssh://git@codeberg.org:2222/me/thing");
        assert_eq!(i.host, HostKind::Forgejo);
        assert_eq!(i.web_url.as_deref(), Some("https://codeberg.org/me/thing"));
        let i = parse_remote_url("https://git.example.com/a/b");
        assert_eq!(i.host, HostKind::Other);
        let i = parse_remote_url("https://gitea.example.com/a/b.git");
        assert_eq!(i.host, HostKind::Gitea);
        let i = parse_remote_url("/local/path/repo");
        assert_eq!(i.web_url, None);
        let i = parse_remote_url("C:\\repos\\x");
        assert_eq!(i.web_url, None);
    }

    #[test]
    fn track() {
        assert_eq!(parse_track("ahead 1, behind 2"), (1, 2, false));
        assert_eq!(parse_track("gone"), (0, 0, true));
        assert_eq!(parse_track(""), (0, 0, false));
    }

    #[test]
    fn stash_subjects() {
        assert_eq!(parse_stash_subject("On main: fix"), (Some("main".into()), "fix".into()));
        let (b, m) = parse_stash_subject("WIP on feat/x: abc1234 subject");
        assert_eq!(b.as_deref(), Some("feat/x"));
        assert_eq!(m, "WIP on feat/x: abc1234 subject");
    }
}
