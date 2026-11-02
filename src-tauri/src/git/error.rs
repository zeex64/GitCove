use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GitErrorKind {
    NotARepo,
    GitMissing,
    Auth,
    Network,
    Conflict,
    Rejected,
    Divergent,
    DirtyWorktree,
    NothingToCommit,
    NotFound,
    Locked,
    Other,
}

/// Every command rejects with this shape: `{ kind, message, detail }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitError {
    pub kind: GitErrorKind,
    pub message: String,
    pub detail: String,
}

pub type GitResult<T> = Result<T, GitError>;

impl GitError {
    pub fn new(kind: GitErrorKind, message: impl Into<String>, detail: impl Into<String>) -> Self {
        GitError {
            kind,
            message: message.into(),
            detail: detail.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::new(GitErrorKind::Other, message, "")
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(GitErrorKind::NotFound, message, "")
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn with_kind(mut self, kind: GitErrorKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn git_missing(detail: impl Into<String>) -> Self {
        Self::new(
            GitErrorKind::GitMissing,
            "Git isn't installed or isn't on your PATH. Install Git, then reopen the app.",
            detail,
        )
    }

    pub fn io(context: &str, err: std::io::Error) -> Self {
        Self::new(GitErrorKind::Other, format!("{context}: {err}."), err.to_string())
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)?;
        if !self.detail.is_empty() {
            write!(f, " ({})", self.detail.trim())?;
        }
        Ok(())
    }
}

impl std::error::Error for GitError {}

/// Pull the most meaningful line out of git's stderr ("fatal: …" / "error: …").
pub fn git_line(stderr: &str) -> String {
    let mut best: Option<&str> = None;
    for line in stderr.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with("hint:") {
            continue;
        }
        if let Some(rest) = l.strip_prefix("fatal:").or_else(|| l.strip_prefix("error:")) {
            best = Some(rest.trim());
            break;
        }
        if best.is_none() {
            best = Some(l);
        }
    }
    let line = best.unwrap_or("").trim();
    if line.is_empty() {
        return String::new();
    }
    let mut s = String::with_capacity(line.len() + 1);
    let mut chars = line.chars();
    if let Some(c) = chars.next() {
        s.extend(c.to_uppercase());
    }
    s.push_str(chars.as_str());
    if !s.ends_with('.') && !s.ends_with('!') && !s.ends_with('?') {
        s.push('.');
    }
    s
}

fn has(stderr: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| stderr.contains(n))
}

/// Map raw git stderr to a typed, plain-language error.
pub fn classify(stderr: &str) -> GitError {
    let detail = stderr.trim().to_string();
    let lower = stderr.to_ascii_lowercase();
    let kind_msg: (GitErrorKind, String) = if stderr.contains("index.lock")
        && (lower.contains("exists") || lower.contains("unable to create"))
    {
        (
            GitErrorKind::Locked,
            "Another Git process is using this repository. Wait for it to finish, or delete .git/index.lock if no Git process is running, then try again.".into(),
        )
    } else if lower.contains("not a git repository") {
        (GitErrorKind::NotARepo, "That folder isn't a Git repository.".into())
    } else if has(
        stderr,
        &[
            "Authentication failed",
            "Permission denied (publickey",
            "could not read Username",
            "could not read Password",
            "Invalid username or password",
            "HTTP Basic: Access denied",
            "terminal prompts disabled",
            "Host key verification failed",
        ],
    ) {
        (
            GitErrorKind::Auth,
            "Git couldn't sign in to the remote. Check that your SSH key or credential helper works for this remote, then try again.".into(),
        )
    } else if has(
        stderr,
        &[
            "Could not resolve host",
            "Network is unreachable",
            "Connection timed out",
            "Connection refused",
            "Could not connect to server",
            "Failed to connect to",
            "Operation timed out",
            "Temporary failure in name resolution",
            "early EOF",
            "The remote end hung up unexpectedly",
        ],
    ) {
        (
            GitErrorKind::Network,
            "Couldn't reach the remote. Check your internet connection, then try again.".into(),
        )
    } else if has(
        stderr,
        &[
            "Need to specify how to reconcile divergent branches",
            "Not possible to fast-forward",
            "have diverged",
            "divergent branches",
        ],
    ) {
        (
            GitErrorKind::Divergent,
            "Your branch and its upstream have diverged. Pull with rebase or merge instead of fast-forward only.".into(),
        )
    } else if has(stderr, &["[rejected]", "non-fast-forward", "fetch first", "stale info", "[remote rejected]"]) {
        (
            GitErrorKind::Rejected,
            "The remote has commits you don't have. Pull first, or force push from the preview.".into(),
        )
    } else if has(
        stderr,
        &[
            "Your local changes to the following files would be overwritten",
            "Please commit your changes or stash them",
            "untracked working tree files would be overwritten",
            "You have unstaged changes",
            "Your index contains uncommitted changes",
            "cannot rebase: You have",
            "Entry '",
            "would be overwritten by",
        ],
    ) {
        (
            GitErrorKind::DirtyWorktree,
            "You have uncommitted changes that would be overwritten. Stash or commit them first, then try again.".into(),
        )
    } else if has(
        stderr,
        &[
            "you need to resolve your current index first",
            "Committing is not possible because you have unmerged files",
            "needs merge",
            "unmerged files",
            "You must edit all merge conflicts",
            "Unmerged paths",
            "is unmerged",
        ],
    ) {
        (
            GitErrorKind::Conflict,
            "Some files still have conflicts. Resolve them and stage the files, then try again.".into(),
        )
    } else if has(stderr, &["nothing to commit", "nothing added to commit", "no changes added to commit"]) {
        (GitErrorKind::NothingToCommit, "There's nothing to commit. Stage some changes first.".into())
    } else if has(
        stderr,
        &[
            "did not match any",
            "unknown revision",
            "not a valid object name",
            "invalid reference",
            "bad revision",
            "No such remote",
            "not found",
            "does not exist",
            "couldn't find remote ref",
            "Needed a single revision",
            "no such branch",
            "is not a commit",
        ],
    ) {
        (GitErrorKind::NotFound, fallback_line(stderr))
    } else {
        (GitErrorKind::Other, fallback_line(stderr))
    };
    GitError::new(kind_msg.0, kind_msg.1, detail)
}

fn fallback_line(stderr: &str) -> String {
    let l = git_line(stderr);
    if l.is_empty() {
        "Git couldn't complete the operation.".to_string()
    } else {
        l
    }
}
