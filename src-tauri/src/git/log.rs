//! Commit history.

use super::error::GitResult;
use super::runner::{Git, Opts};
use super::types::{Commit, LogResult};
use std::path::Path;

/// `--format` producing NUL-separated fields and a 0x1e record separator.
pub const COMMIT_FORMAT: &str = "--format=%H%x00%P%x00%an%x00%ae%x00%at%x00%cn%x00%ct%x00%s%x00%b%x1e";

pub fn parse_commits(text: &str) -> Vec<Commit> {
    let mut out = Vec::new();
    for rec in text.split('\x1e') {
        let rec = rec.trim_start_matches(['\n', '\r']);
        if rec.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = rec.splitn(9, '\0').collect();
        if f.len() < 9 || f[0].len() < 7 {
            continue;
        }
        let id = f[0].trim().to_string();
        out.push(Commit {
            short_id: id[..7.min(id.len())].to_string(),
            parents: f[1].split_whitespace().map(str::to_string).collect(),
            author_name: f[2].to_string(),
            author_email: f[3].to_string(),
            author_time: f[4].trim().parse().unwrap_or(0),
            committer_name: f[5].to_string(),
            committer_time: f[6].trim().parse().unwrap_or(0),
            summary: f[7].to_string(),
            body: f[8].trim_end().to_string(),
            id,
        });
    }
    out
}

pub fn head_is_unborn(git: &Git) -> GitResult<bool> {
    Ok(git.rev_parse("HEAD")?.is_none())
}

pub fn get_log(repo: &Path, limit: Option<u32>) -> GitResult<LogResult> {
    let git = Git::new(repo);
    let limit = limit.unwrap_or(50_000).max(1);
    let n = limit.to_string();
    let mut args: Vec<&str> = vec![
        "log",
        "--branches",
        "--remotes",
        "--tags",
        "--date-order",
        "--no-color",
        "-n",
        &n,
        COMMIT_FORMAT,
    ];
    let unborn = head_is_unborn(&git)?;
    if !unborn {
        args.push("HEAD");
    }
    args.push("--");
    let out = git.exec(&args, Opts { read_only: true, ..Default::default() })?;
    if !out.ok() {
        // An empty repository (or one with only unborn refs) has no history.
        if unborn {
            return Ok(LogResult { commits: vec![], truncated: false });
        }
        return Err(out.error());
    }
    let commits = parse_commits(&out.text());
    let truncated = commits.len() as u32 == limit;
    Ok(LogResult { commits, truncated })
}

/// Load one commit.
pub fn load_commit(git: &Git, rev: &str) -> GitResult<Commit> {
    let id = git.require_commit(rev)?;
    let text = git.read(["show", "-s", "--no-color", COMMIT_FORMAT, &id])?;
    parse_commits(&text)
        .into_iter()
        .next()
        .ok_or_else(|| super::error::GitError::not_found(format!("Couldn't read commit {id}.")))
}

/// Load several commits by range arguments, e.g. `["b..a"]`.
pub fn load_range(git: &Git, args: &[&str]) -> GitResult<Vec<Commit>> {
    let mut a = vec!["log", "--no-color", COMMIT_FORMAT];
    a.extend_from_slice(args);
    a.push("--");
    Ok(parse_commits(&git.read(&a)?))
}
