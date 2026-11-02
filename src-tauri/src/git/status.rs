//! Working tree status and in-progress operation detection.

use super::error::GitResult;
use super::runner::{Git, short};
use super::types::{ChangeKind, ConflictKind, FileChange, Head, OperationProgress, RepoState, Status};
use std::fs;
use std::path::Path;

fn kind_of(c: u8) -> Option<ChangeKind> {
    match c {
        b'M' => Some(ChangeKind::Modified),
        b'T' => Some(ChangeKind::TypeChanged),
        b'A' => Some(ChangeKind::Added),
        b'D' => Some(ChangeKind::Deleted),
        b'R' => Some(ChangeKind::Renamed),
        b'C' => Some(ChangeKind::Copied),
        b'U' => Some(ChangeKind::Conflicted),
        _ => None,
    }
}

fn conflict_of(xy: &str) -> ConflictKind {
    match xy {
        "UU" => ConflictKind::BothModified,
        "AA" => ConflictKind::BothAdded,
        "DD" => ConflictKind::BothDeleted,
        "AU" => ConflictKind::AddedByUs,
        "UA" => ConflictKind::AddedByThem,
        "DU" => ConflictKind::DeletedByUs,
        "UD" => ConflictKind::DeletedByThem,
        _ => ConflictKind::BothModified,
    }
}

pub struct ParsedStatus {
    pub head: Head,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
}

/// Parse `git status --porcelain=v2 -z --branch` output.
pub fn parse_porcelain_v2(text: &str) -> ParsedStatus {
    let mut head = Head { branch: None, commit: None, detached: false, unborn: false };
    let mut upstream = None;
    let (mut ahead, mut behind) = (0, 0);
    let mut files = Vec::new();
    let mut fields = text.split('\0').peekable();
    while let Some(rec) = fields.next() {
        if rec.is_empty() {
            continue;
        }
        if let Some(h) = rec.strip_prefix("# ") {
            if let Some(v) = h.strip_prefix("branch.oid ") {
                if v == "(initial)" {
                    head.unborn = true;
                } else {
                    head.commit = Some(v.to_string());
                }
            } else if let Some(v) = h.strip_prefix("branch.head ") {
                if v == "(detached)" {
                    head.detached = true;
                } else {
                    head.branch = Some(v.to_string());
                }
            } else if let Some(v) = h.strip_prefix("branch.upstream ") {
                upstream = Some(v.to_string());
            } else if let Some(v) = h.strip_prefix("branch.ab ") {
                for p in v.split_whitespace() {
                    if let Some(n) = p.strip_prefix('+') {
                        ahead = n.parse().unwrap_or(0);
                    } else if let Some(n) = p.strip_prefix('-') {
                        behind = n.parse().unwrap_or(0);
                    }
                }
            }
            continue;
        }
        let bytes = rec.as_bytes();
        match bytes[0] {
            b'1' => {
                // 1 XY sub mH mI mW hH hI path
                let parts: Vec<&str> = rec.splitn(9, ' ').collect();
                if parts.len() < 9 {
                    continue;
                }
                let xy = parts[1].as_bytes();
                files.push(FileChange {
                    path: parts[8].to_string(),
                    orig_path: None,
                    staged: kind_of(xy[0]),
                    unstaged: kind_of(xy[1]),
                    conflict: None,
                });
            }
            b'2' => {
                // 2 XY sub mH mI mW hH hI Xscore path \0 origPath
                let parts: Vec<&str> = rec.splitn(10, ' ').collect();
                let orig = fields.next().unwrap_or("").to_string();
                if parts.len() < 10 {
                    continue;
                }
                let xy = parts[1].as_bytes();
                files.push(FileChange {
                    path: parts[9].to_string(),
                    orig_path: Some(orig).filter(|s| !s.is_empty()),
                    staged: kind_of(xy[0]),
                    unstaged: kind_of(xy[1]),
                    conflict: None,
                });
            }
            b'u' => {
                // u XY sub m1 m2 m3 mW h1 h2 h3 path
                let parts: Vec<&str> = rec.splitn(11, ' ').collect();
                if parts.len() < 11 {
                    continue;
                }
                files.push(FileChange {
                    path: parts[10].to_string(),
                    orig_path: None,
                    staged: None,
                    unstaged: Some(ChangeKind::Conflicted),
                    conflict: Some(conflict_of(parts[1])),
                });
            }
            b'?' => {
                files.push(FileChange {
                    path: rec[2..].to_string(),
                    orig_path: None,
                    staged: None,
                    unstaged: Some(ChangeKind::Untracked),
                    conflict: None,
                });
            }
            _ => {}
        }
    }
    if head.detached {
        head.branch = None;
    }
    ParsedStatus { head, upstream, ahead, behind, files }
}

pub fn status_files(git: &Git) -> GitResult<ParsedStatus> {
    let text = git.read(["status", "--porcelain=v2", "-z", "--branch", "--untracked-files=all"])?;
    Ok(parse_porcelain_v2(&text))
}

fn read_trim(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn summary_of(git: &Git, sha: &str) -> Option<String> {
    git.try_read(["log", "-1", "--format=%s", sha, "--"]).ok().flatten().map(|s| s.trim().to_string())
}

/// Resolve a commit to a branch name when it is exactly a branch tip, else a short sha.
pub fn friendly_name(git: &Git, sha: &str) -> String {
    if let Ok(Some(name)) = git.try_read([
        "name-rev",
        "--name-only",
        "--no-undefined",
        "--refs=refs/heads/*",
        "--refs=refs/remotes/*",
        sha,
    ]) {
        let n = name.trim();
        if !n.is_empty() && !n.contains('~') && !n.contains('^') {
            let n = n.strip_prefix("remotes/").unwrap_or(n);
            return n.to_string();
        }
    }
    short(sha).to_string()
}

pub fn repo_state(git_dir: &Path) -> RepoState {
    if git_dir.join("rebase-merge").is_dir() || git_dir.join("rebase-apply").is_dir() {
        // `git am` also uses rebase-apply; treat it as rebasing only when it is a rebase.
        if git_dir.join("rebase-apply").is_dir() && git_dir.join("rebase-apply/applying").exists() {
            return RepoState::Clean;
        }
        RepoState::Rebasing
    } else if git_dir.join("MERGE_HEAD").exists() {
        RepoState::Merging
    } else if git_dir.join("CHERRY_PICK_HEAD").exists() {
        RepoState::CherryPicking
    } else if git_dir.join("REVERT_HEAD").exists() {
        RepoState::Reverting
    } else if git_dir.join("BISECT_LOG").exists() {
        RepoState::Bisecting
    } else {
        RepoState::Clean
    }
}

fn operation_progress(git: &Git, git_dir: &Path, state: RepoState, head: &Head) -> Option<OperationProgress> {
    let mut p = OperationProgress {
        kind: state,
        branch: head.branch.clone(),
        onto: None,
        step: None,
        total: None,
        current_commit: None,
        current_summary: None,
    };
    match state {
        RepoState::Clean => return None,
        RepoState::Rebasing => {
            let (dir, step_file, total_file) = if git_dir.join("rebase-merge").is_dir() {
                (git_dir.join("rebase-merge"), "msgnum", "end")
            } else {
                (git_dir.join("rebase-apply"), "next", "last")
            };
            p.branch = read_trim(&dir.join("head-name"))
                .filter(|s| s != "detached HEAD")
                .map(|s| s.strip_prefix("refs/heads/").map(str::to_string).unwrap_or(s));
            p.onto = read_trim(&dir.join("onto")).map(|sha| friendly_name(git, &sha));
            p.step = read_trim(&dir.join(step_file)).and_then(|s| s.parse().ok());
            p.total = read_trim(&dir.join(total_file)).and_then(|s| s.parse().ok());
            let current = read_trim(&git_dir.join("REBASE_HEAD"))
                .or_else(|| read_trim(&dir.join("stopped-sha")))
                .or_else(|| read_trim(&dir.join("original-commit")));
            let current = current.and_then(|c| git.commit_id(&c).ok().flatten());
            if let Some(c) = current {
                p.current_summary = summary_of(git, &c);
                p.current_commit = Some(c);
            }
        }
        RepoState::Merging | RepoState::CherryPicking | RepoState::Reverting => {
            let file = match state {
                RepoState::Merging => "MERGE_HEAD",
                RepoState::CherryPicking => "CHERRY_PICK_HEAD",
                _ => "REVERT_HEAD",
            };
            if let Some(sha) = read_trim(&git_dir.join(file)).and_then(|s| s.lines().next().map(str::to_string)) {
                p.onto = Some(friendly_name(git, &sha));
                p.current_summary = summary_of(git, &sha);
                p.current_commit = Some(sha);
            }
            if state != RepoState::Merging {
                let todo = fs::read_to_string(git_dir.join("sequencer/todo")).unwrap_or_default();
                let done = fs::read_to_string(git_dir.join("sequencer/done")).unwrap_or_default();
                let count = |s: &str| s.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).count() as u32;
                let (t, d) = (count(&todo), count(&done));
                if t + d > 0 {
                    p.total = Some(t + d);
                    p.step = Some(d.max(1));
                }
            }
        }
        RepoState::Bisecting => {
            p.branch = read_trim(&git_dir.join("BISECT_START")).or(p.branch);
        }
    }
    Some(p)
}

pub fn get_status(repo: &Path) -> GitResult<Status> {
    let git = Git::new(repo);
    let parsed = status_files(&git)?;
    let git_dir = git.absolute_git_dir()?;
    let state = repo_state(&git_dir);
    let operation = operation_progress(&git, &git_dir, state, &parsed.head);
    Ok(Status {
        head: parsed.head,
        upstream: parsed.upstream,
        ahead: parsed.ahead,
        behind: parsed.behind,
        state,
        operation,
        files: parsed.files,
    })
}

/// Paths with tracked changes (staged, unstaged or conflicted), including rename sources.
pub fn tracked_changed_paths(files: &[FileChange]) -> Vec<String> {
    let mut out = Vec::new();
    for f in files {
        if f.unstaged == Some(ChangeKind::Untracked) {
            continue;
        }
        out.push(f.path.clone());
        if let (Some(o), Some(ChangeKind::Renamed)) = (&f.orig_path, f.staged) {
            out.push(o.clone());
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn has_tracked_changes(files: &[FileChange]) -> bool {
    files.iter().any(|f| f.unstaged != Some(ChangeKind::Untracked))
}
