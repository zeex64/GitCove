//! Opening, creating and cloning repositories.

use super::error::{GitError, GitErrorKind, GitResult};
use super::oplog::{self, NewEntry};
use super::ops::run_network;
use super::runner::{Emit, Git, Opts, git_version};
use super::types::{Identity, RepoInfo};
use std::path::{Path, PathBuf};

pub fn version_string() -> GitResult<String> {
    Ok(git_version()?.3)
}

fn not_a_repo(detail: impl Into<String>) -> GitError {
    GitError::new(GitErrorKind::NotARepo, "That folder isn't a Git repository.", detail)
}

pub fn open_repo(path: &str) -> GitResult<RepoInfo> {
    let p = PathBuf::from(path.trim());
    if !p.exists() {
        return Err(GitError::not_found(format!("The folder {} doesn't exist.", p.display())));
    }
    let dir = if p.is_dir() { p.clone() } else { p.parent().map(Path::to_path_buf).unwrap_or(p.clone()) };
    let git = Git::new(&dir);
    let ro = || Opts { read_only: true, ..Default::default() };
    let bare = git.exec(["rev-parse", "--is-bare-repository"], ro())?;
    if !bare.ok() {
        let e = bare.error();
        if e.kind == GitErrorKind::GitMissing {
            return Err(e);
        }
        return Err(not_a_repo(e.detail));
    }
    if bare.text().trim() == "true" {
        return Err(GitError::other("Bare repositories aren't supported."));
    }
    let out = git.exec(["rev-parse", "--show-toplevel", "--absolute-git-dir", "--is-inside-work-tree"], ro())?;
    if !out.ok() {
        let e = out.error();
        if out.stderr.contains("must be run in a work tree") {
            return Err(GitError::other("Open the repository's folder rather than its .git folder.").with_kind(GitErrorKind::NotARepo));
        }
        return Err(not_a_repo(e.detail));
    }
    let text = out.text();
    let mut lines = text.lines();
    let top = lines.next().unwrap_or("").trim().to_string();
    let git_dir = lines.next().unwrap_or("").trim().to_string();
    let inside = lines.next().unwrap_or("").trim() == "true";
    if top.is_empty() || !inside {
        return Err(not_a_repo(text));
    }
    let name = Path::new(&top).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| top.clone());
    let top_git = Git::new(&top);
    let is_empty = top_git.rev_parse("HEAD")?.is_none();
    Ok(RepoInfo { path: top, name, git_dir, is_empty })
}

pub fn init_repo(path: &str) -> GitResult<RepoInfo> {
    let p = PathBuf::from(path.trim());
    std::fs::create_dir_all(&p).map_err(|e| GitError::io("Couldn't create the folder", e))?;
    let git = Git::new(&p);
    git.write(["init"])?;
    let info = open_repo(&p.to_string_lossy())?;
    let g = Git::new(&info.path);
    let _ = oplog::record(
        &g,
        NewEntry::final_("init", "Create repository", "Creating a repository can't be undone here."),
    );
    Ok(info)
}

pub fn clone_repo(url: &str, dest: &str, op_id: &str, emit: Emit) -> GitResult<RepoInfo> {
    let url = url.trim();
    if url.is_empty() {
        return Err(GitError::other("Enter the URL of the repository to clone."));
    }
    let dest_path = PathBuf::from(dest.trim());
    if dest_path.exists() && std::fs::read_dir(&dest_path).map(|mut d| d.next().is_some()).unwrap_or(true) {
        return Err(GitError::other(format!(
            "The folder {} already exists and isn't empty. Choose a new folder.",
            dest_path.display()
        )));
    }
    let parent = dest_path.parent().map(Path::to_path_buf).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&parent).map_err(|e| GitError::io("Couldn't create the folder", e))?;
    let git = Git::new(&parent);
    let dest_str = dest_path.to_string_lossy().into_owned();
    let out = run_network(&git, &["clone", "--progress", "--", url, &dest_str], op_id, emit)?;
    if !out.ok() {
        return Err(out.error());
    }
    let info = open_repo(&dest_str)?;
    let g = Git::new(&info.path);
    let _ = oplog::record(
        &g,
        NewEntry::final_("clone", format!("Clone {url}"), "Cloning can't be undone here. Delete the folder to remove the copy."),
    );
    Ok(info)
}

/// The `user.name` / `user.email` Git would use in this repository (local, global or system).
pub fn get_identity(repo: &Path) -> GitResult<Identity> {
    let git = Git::new(repo);
    let get = |key: &str| -> GitResult<Option<String>> { Ok(git.config_get(key)?.filter(|v| !v.is_empty())) };
    Ok(Identity { name: get("user.name")?, email: get("user.email")? })
}

/// Write `user.name` and `user.email` to the global Git config.
pub fn set_identity(name: &str, email: &str) -> GitResult<()> {
    let (name, email) = (name.trim(), email.trim());
    if name.is_empty() || email.is_empty() {
        return Err(GitError::other("Enter both a name and an email address."));
    }
    // Run outside any repository; --global ignores the working directory anyway.
    let git = Git::new(std::env::temp_dir());
    git.write(["config", "--global", "user.name", name])?;
    git.write(["config", "--global", "user.email", email])?;
    Ok(())
}
