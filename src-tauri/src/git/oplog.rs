//! The operation log behind undo/redo. Persisted at `<git-dir>/gitcove/oplog.json`
//! (newest first, at most 100 entries). Every object an entry may restore is kept
//! reachable through refs under `refs/gitcove/undo/<entryId>/<n>`.

use super::error::{GitError, GitErrorKind, GitResult};
use super::refs::{branch_exists, branch_tip, list_stashes};
use super::runner::{Git, now_secs, short};
use super::snapshot::{FileSet, HeadSnap, Snapshot, blob_tree, changed_since, head_snap, restore_files, write_tree};
use super::status::repo_state;
use super::types::{OpEntry, OpResult, RepoState};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

pub const MAX_ENTRIES: usize = 100;
const UNDO_REF_PREFIX: &str = "refs/gitcove/undo";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum MoveStyle {
    /// `git reset --keep` (worktree follows, local changes kept).
    #[default]
    Keep,
    /// reset --soft: index and worktree untouched.
    Soft,
    /// reset --mixed: index restored from the recorded tree.
    Mixed,
    /// reset --hard: worktree files restored from the file snapshot.
    Hard,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RefMoveData {
    pub op: String,
    /// Branch that moved; None when HEAD was detached.
    pub branch: Option<String>,
    pub before: String,
    pub after: String,
    pub head_before: HeadSnap,
    pub head_after: HeadSnap,
    pub style: MoveStyle,
    pub index_before: Option<String>,
    pub index_after: Option<String>,
    pub files_before: Option<FileSet>,
    pub files_after: Option<FileSet>,
    /// In-progress state this operation can leave behind (rebase, merge, …).
    pub stoppable: Option<RepoState>,
    /// The operation stopped on conflicts.
    pub stopped: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Undo {
    #[default]
    None,
    Commit {
        branch: Option<String>,
        before: Option<String>,
        after: String,
    },
    /// Index entries of some paths (stage, unstage, partial staging).
    Index { before: FileSet, after: FileSet },
    /// Worktree contents and index entries of some paths (discard, partial discard, resolve, stash apply).
    Files { before: FileSet, after: FileSet },
    Checkout {
        before: HeadSnap,
        after: HeadSnap,
        target: String,
        created: Option<(String, String)>,
    },
    CreateBranch {
        name: String,
        sha: String,
        checkout: bool,
        prev: HeadSnap,
        config: Vec<(String, String)>,
    },
    RenameBranch { from: String, to: String },
    DeleteBranch {
        name: String,
        sha: String,
        config: Vec<(String, String)>,
    },
    RefMove(RefMoveData),
    StashPush {
        sha: String,
        message: Option<String>,
        include_untracked: bool,
    },
    /// A stash entry was removed (pop / drop); undo stores it again.
    StashRestore { sha: String, message: String },
    Tag { name: String, object: String, created: bool },
    Remote { name: String, url: String },
}

impl Undo {
    fn objects(&self) -> Vec<String> {
        let mut v = Vec::new();
        match self {
            Undo::None | Undo::RenameBranch { .. } | Undo::Remote { .. } => {}
            Undo::Commit { before, after, .. } => {
                v.extend(before.clone());
                v.push(after.clone());
            }
            Undo::Index { before, after } | Undo::Files { before, after } => {
                v.extend(before.objects());
                v.extend(after.objects());
            }
            Undo::Checkout { before, after, created, .. } => {
                v.extend(before.commit.clone());
                v.extend(after.commit.clone());
                if let Some((_, s)) = created {
                    v.push(s.clone());
                }
            }
            Undo::CreateBranch { sha, prev, .. } => {
                v.push(sha.clone());
                v.extend(prev.commit.clone());
            }
            Undo::DeleteBranch { sha, .. } => v.push(sha.clone()),
            Undo::RefMove(m) => {
                v.push(m.before.clone());
                v.push(m.after.clone());
                v.extend(m.head_before.commit.clone());
                v.extend(m.head_after.commit.clone());
                v.extend(m.index_before.clone());
                v.extend(m.index_after.clone());
                if let Some(f) = &m.files_before {
                    v.extend(f.objects());
                }
                if let Some(f) = &m.files_after {
                    v.extend(f.objects());
                }
            }
            Undo::StashPush { sha, .. } | Undo::StashRestore { sha, .. } => v.push(sha.clone()),
            Undo::Tag { object, .. } => v.push(object.clone()),
        }
        v.retain(|s| !s.is_empty());
        v
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StoredEntry {
    #[serde(flatten)]
    pub entry: OpEntry,
    #[serde(default)]
    pub undo: Undo,
    #[serde(default)]
    pub before: Option<Snapshot>,
    #[serde(default)]
    pub after: Option<Snapshot>,
    /// In-progress operation (rebase, merge, …) the entry was recorded during.
    #[serde(default)]
    pub during: Option<RepoState>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpLogFile {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub entries: Vec<StoredEntry>,
    /// Ids of undone entries that can be redone, most recent undo last.
    #[serde(default)]
    pub redo: Vec<String>,
}

fn log_path(git: &Git) -> GitResult<PathBuf> {
    Ok(git.absolute_git_dir()?.join("gitcove").join("oplog.json"))
}

pub fn load(git: &Git) -> GitResult<OpLogFile> {
    let path = log_path(git)?;
    match std::fs::read(&path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(OpLogFile::default()),
        Err(e) => Err(GitError::io("Couldn't read the undo history", e)),
    }
}

pub fn save(git: &Git, log: &OpLogFile) -> GitResult<()> {
    let path = log_path(git)?;
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| GitError::io("Couldn't save the undo history", e))?;
    let tmp = dir.join("oplog.json.tmp");
    let data = serde_json::to_vec(log).map_err(|e| GitError::other(format!("Couldn't save the undo history: {e}.")))?;
    std::fs::write(&tmp, data).map_err(|e| GitError::io("Couldn't save the undo history", e))?;
    std::fs::rename(&tmp, &path).map_err(|e| GitError::io("Couldn't save the undo history", e))?;
    Ok(())
}

pub fn get_oplog(repo: &Path) -> GitResult<Vec<OpEntry>> {
    let git = Git::new(repo);
    Ok(load(&git)?.entries.into_iter().map(|e| e.entry).take(MAX_ENTRIES).collect())
}

fn new_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let millis = now.as_millis() as u64;
    let mix = (now.subsec_nanos() as u64) ^ ((std::process::id() as u64) << 16) ^ (COUNTER.fetch_add(1, Ordering::Relaxed) as u64 * 7919);
    format!("{}-{}", base36(millis), base36(mix % 1_679_616)) // 4 base36 chars
}

fn base36(mut n: u64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".into();
    }
    let mut v = Vec::new();
    while n > 0 {
        v.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    v.reverse();
    String::from_utf8(v).unwrap()
}

/// Keep objects reachable via refs/gitcove/undo/<id>/<n>.
fn protect(git: &Git, id: &str, objects: &[String]) -> GitResult<()> {
    let mut objs: Vec<String> = objects.iter().filter(|s| s.len() >= 40).cloned().collect();
    objs.sort();
    objs.dedup();
    if objs.is_empty() {
        return Ok(());
    }
    // Loose blobs are bundled into one tree so a single ref covers them.
    let prefix = format!("{UNDO_REF_PREFIX}/{id}/");
    let existing = git.read(["for-each-ref", "--format=%(objectname)", &prefix])?;
    let have: Vec<&str> = existing.lines().collect();
    let mut start = have.len();
    let mut blobs = Vec::new();
    let mut others = Vec::new();
    let todo: Vec<String> = objs.into_iter().filter(|o| !have.contains(&o.as_str())).collect();
    if !todo.is_empty() {
        let mut input = Vec::new();
        for o in &todo {
            input.extend_from_slice(o.as_bytes());
            input.push(b'\n');
        }
        let out = git.read_stdin(["cat-file", "--batch-check"], input)?;
        for line in out.text().lines() {
            let mut it = line.split_whitespace();
            match (it.next(), it.next()) {
                (Some(sha), Some("blob")) => blobs.push(sha.to_string()),
                (Some(_), Some("missing")) | (Some(_), None) => {}
                (Some(sha), Some(_)) => others.push(sha.to_string()),
                _ => {}
            }
        }
    }
    if let Some(tree) = blob_tree(git, &blobs)? {
        others.push(tree);
    }
    let mut input = String::new();
    for o in others {
        input.push_str(&format!("update {prefix}{start} {o}\n"));
        start += 1;
    }
    if !input.is_empty() {
        git.write_stdin(["update-ref", "--stdin"], input.into_bytes())?;
    }
    Ok(())
}

fn drop_refs(git: &Git, ids: &[String]) -> GitResult<()> {
    let mut input = String::new();
    for id in ids {
        let prefix = format!("{UNDO_REF_PREFIX}/{id}/");
        let text = git.read(["for-each-ref", "--format=%(refname)", &prefix])?;
        for r in text.lines() {
            input.push_str(&format!("delete {r}\n"));
        }
    }
    if !input.is_empty() {
        git.write_stdin(["update-ref", "--stdin"], input.into_bytes())?;
    }
    Ok(())
}

/// What a write command hands to [`record`].
pub struct NewEntry {
    pub kind: &'static str,
    pub label: String,
    pub undo: Undo,
    pub before: Option<Snapshot>,
    pub after: Option<Snapshot>,
    /// Some(reason) makes the entry not undoable.
    pub not_undoable: Option<String>,
    /// Extra objects to keep reachable (e.g. the old remote tip of a force push).
    pub extra_objects: Vec<String>,
}

impl NewEntry {
    pub fn new(kind: &'static str, label: impl Into<String>, undo: Undo) -> Self {
        NewEntry { kind, label: label.into(), undo, before: None, after: None, not_undoable: None, extra_objects: vec![] }
    }
    pub fn final_(kind: &'static str, label: impl Into<String>, reason: impl Into<String>) -> Self {
        NewEntry {
            kind,
            label: label.into(),
            undo: Undo::None,
            before: None,
            after: None,
            not_undoable: Some(reason.into()),
            extra_objects: vec![],
        }
    }
    pub fn snapshots(mut self, before: Snapshot, after: Snapshot) -> Self {
        self.before = Some(before);
        self.after = Some(after);
        self
    }
}

/// Keep only the branches that changed so the log stays small in repos with many branches.
fn compact(before: Option<Snapshot>, after: Option<Snapshot>) -> (Option<Snapshot>, Option<Snapshot>) {
    match (before, after) {
        (Some(mut b), Some(mut a)) => {
            let same: Vec<String> =
                b.branches.iter().filter(|(k, v)| a.branches.get(*k) == Some(*v)).map(|(k, _)| k.clone()).collect();
            for k in same {
                b.branches.remove(&k);
                a.branches.remove(&k);
            }
            (Some(b), Some(a))
        }
        other => other,
    }
}

/// Append an entry to the log. Clears the redo stack.
pub fn record(git: &Git, new: NewEntry) -> GitResult<OpEntry> {
    let mut log = load(git)?;
    let entry = OpEntry {
        id: new_id(),
        time: now_secs(),
        kind: new.kind.to_string(),
        label: new.label,
        undoable: new.not_undoable.is_none(),
        undone: false,
        reason: new.not_undoable,
    };
    let mut objects = new.undo.objects();
    objects.extend(new.extra_objects);
    if let Some(b) = &new.before {
        objects.extend(b.changed_objects(new.after.as_ref()));
    }
    if let Some(a) = &new.after {
        objects.extend(a.changed_objects(new.before.as_ref()));
    }
    protect(git, &entry.id, &objects)?;
    let (before, after) = compact(new.before, new.after);
    let during = match current_state(git)? {
        RepoState::Clean => None,
        s => Some(s),
    };
    log.entries.insert(0, StoredEntry { entry: entry.clone(), undo: new.undo, before, after, during });
    log.redo.clear();
    if log.entries.len() > MAX_ENTRIES {
        let removed: Vec<String> = log.entries.drain(MAX_ENTRIES..).map(|e| e.entry.id).collect();
        drop_refs(git, &removed)?;
    }
    log.version = 1;
    save(git, &log)?;
    Ok(entry)
}

/// Record and build the OpResult; if saving the log fails the operation still succeeded.
pub fn finish(git: &Git, new: NewEntry, message: impl Into<String>) -> GitResult<OpResult> {
    let message = message.into();
    match record(git, new) {
        Ok(entry) => Ok(OpResult { entry: Some(entry), message }),
        Err(e) => {
            log::warn!("couldn't record op log entry: {e}");
            Ok(OpResult { entry: None, message: format!("{message}. Undo history couldn't be saved: {}", e.message) })
        }
    }
}

// ---------------------------------------------------------------------------------------
// Shared helpers for undo/redo and operations
// ---------------------------------------------------------------------------------------

pub fn current_state(git: &Git) -> GitResult<RepoState> {
    Ok(repo_state(&git.absolute_git_dir()?))
}

fn state_name(s: RepoState) -> &'static str {
    match s {
        RepoState::Clean => "operation",
        RepoState::Merging => "merge",
        RepoState::Rebasing => "rebase",
        RepoState::CherryPicking => "cherry-pick",
        RepoState::Reverting => "revert",
        RepoState::Bisecting => "bisect",
    }
}

fn require_clean_state(git: &Git) -> GitResult<()> {
    let s = current_state(git)?;
    if s != RepoState::Clean {
        let n = state_name(s);
        return Err(GitError::other(format!("A {n} is in progress. Finish or abort the {n} first, then undo.")));
    }
    Ok(())
}

pub fn abort_state(git: &Git, s: RepoState) -> GitResult<()> {
    let args: &[&str] = match s {
        RepoState::Rebasing => &["rebase", "--abort"],
        RepoState::Merging => &["merge", "--abort"],
        RepoState::CherryPicking => &["cherry-pick", "--abort"],
        RepoState::Reverting => &["revert", "--abort"],
        RepoState::Bisecting => &["bisect", "reset"],
        RepoState::Clean => return Ok(()),
    };
    git.write(args)?;
    Ok(())
}

/// Switch HEAD to a recorded position.
pub fn switch_to(git: &Git, head: &HeadSnap) -> GitResult<()> {
    let res = match (&head.branch, &head.commit) {
        (Some(b), _) => {
            if !branch_exists(git, b)? {
                return Err(GitError::other(format!("Can't switch back to {b} because it has no commits yet or no longer exists.")));
            }
            git.run(["switch", "--no-guess", b])?
        }
        (None, Some(c)) => git.run(["switch", "--detach", c])?,
        (None, None) => return Err(GitError::other("There's no previous position to switch back to.")),
    };
    if !res.ok() {
        let e = res.error();
        if e.kind == GitErrorKind::DirtyWorktree {
            return Err(e.with_message(format!(
                "Switching back to {} would overwrite your uncommitted changes. Stash or commit them first, then undo again.",
                head.describe()
            )));
        }
        return Err(e);
    }
    Ok(())
}

fn moved_error(what: &str, now: Option<&str>) -> GitError {
    let now = now.map(|s| format!(" (it's now at {})", short(s))).unwrap_or_else(|| " (it no longer exists)".to_string());
    GitError::other(format!("{what} has moved since then{now}, so undoing could lose newer work. Undo the newer operations first."))
}

fn restore_config(git: &Git, config: &[(String, String)]) -> GitResult<()> {
    for (k, v) in config {
        git.write(["config", "--local", "--add", k, v])?;
    }
    Ok(())
}

fn list_stash_shas(git: &Git) -> GitResult<Vec<String>> {
    Ok(list_stashes(git)?.into_iter().map(|s| s.commit).collect())
}

/// Pop a stash with --index, falling back to a plain pop when the index can't be restored.
pub fn stash_pop_or_apply(git: &Git, verb: &str, rev: &str) -> GitResult<super::runner::Out> {
    let out = git.run(["stash", verb, "--index", rev])?;
    if !out.ok() {
        let all = out.all_output();
        if all.contains("--index") || all.contains("Conflicts in index") {
            return git.run(["stash", verb, rev]);
        }
    }
    Ok(out)
}

fn index_changed_error(paths: &[String]) -> GitError {
    GitError::other(format!(
        "The staged changes in {} have changed since then, so undoing would overwrite newer staging.",
        list_paths(paths)
    ))
}

fn files_changed_error(paths: &[String]) -> GitError {
    GitError::other(format!(
        "{} changed since then, so restoring would overwrite your newer edits. Undo the newer operations first.",
        list_paths(paths)
    ))
}

pub fn list_paths(paths: &[String]) -> String {
    match paths.len() {
        0 => "the files".to_string(),
        1 => paths[0].clone(),
        2 => format!("{} and {}", paths[0], paths[1]),
        n => format!("{}, {} and {} more", paths[0], paths[1], n - 2),
    }
}

/// Tip of the moved ref (branch or detached HEAD).
fn ref_tip(git: &Git, branch: &Option<String>) -> GitResult<Option<String>> {
    match branch {
        Some(b) => branch_tip(git, b),
        None => git.rev_parse("HEAD"),
    }
}

/// Move `branch` (or detached HEAD) from `from` to `to`.
fn move_ref(git: &Git, m: &RefMoveData, from: &str, to: &str, to_index: Option<&String>, style: MoveStyle) -> GitResult<()> {
    let head = head_snap(git)?;
    let on_branch = match &m.branch {
        Some(b) => head.branch.as_deref() == Some(b.as_str()),
        None => head.branch.is_none(),
    };
    if !on_branch {
        let Some(b) = m.branch.as_ref() else {
            return Err(GitError::other("HEAD is on a branch now. Switch back to the detached commit first, then try again."));
        };
        git.write(["update-ref", "-m", "gitcove: undo", &format!("refs/heads/{b}"), to, from])?;
        return Ok(());
    }
    match style {
        MoveStyle::Keep | MoveStyle::Hard => {
            let out = git.run(["reset", "--keep", to])?;
            if !out.ok() {
                let e = out.error();
                return Err(GitError::new(
                    GitErrorKind::DirtyWorktree,
                    "Your uncommitted changes touch files this would change. Stash or commit them first, then try again.",
                    e.detail,
                ));
            }
        }
        MoveStyle::Soft => {
            git.write(["reset", "--soft", to])?;
        }
        MoveStyle::Mixed => {
            git.write(["reset", "--soft", to])?;
            match to_index {
                Some(tree) => {
                    git.write(["read-tree", tree])?;
                    let _ = git.run(["update-index", "-q", "--refresh"]);
                }
                None => {
                    git.write(["reset", "--mixed", "-q", to])?;
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Undo
// ---------------------------------------------------------------------------------------

/// Index/file entries recorded while resolving conflicts lose their meaning once that
/// operation is gone (finished, aborted or undone): undoing them is then a no-op.
fn context_gone(git: &Git, e: &StoredEntry) -> GitResult<bool> {
    let Some(during) = e.during else { return Ok(false) };
    if !matches!(e.undo, Undo::Index { .. } | Undo::Files { .. }) {
        return Ok(false);
    }
    Ok(current_state(git)? != during)
}

fn undo_one(git: &Git, e: &mut StoredEntry) -> GitResult<()> {
    if context_gone(git, e)? {
        return Ok(());
    }
    match &e.undo {
        Undo::None => Err(GitError::other(
            e.entry.reason.clone().unwrap_or_else(|| "This operation can't be undone.".to_string()),
        )),
        Undo::Commit { branch, before, after } => {
            require_clean_state(git)?;
            let head = head_snap(git)?;
            match branch {
                Some(b) => {
                    if head.branch.as_deref() != Some(b.as_str()) {
                        return Err(GitError::other(format!("Switch back to {b} to undo this commit.")));
                    }
                    let tip = branch_tip(git, b)?;
                    if tip.as_deref() != Some(after.as_str()) {
                        return Err(moved_error(b, tip.as_deref()));
                    }
                    match before {
                        None => {
                            git.write(["update-ref", "-d", &format!("refs/heads/{b}"), after])?;
                        }
                        Some(s) => {
                            git.write(["reset", "--soft", s])?;
                        }
                    }
                }
                None => {
                    if head.branch.is_some() || head.commit.as_deref() != Some(after.as_str()) {
                        return Err(moved_error("HEAD", head.commit.as_deref()));
                    }
                    let Some(s) = before else { return Err(GitError::other("This commit can't be undone.")) };
                    git.write(["reset", "--soft", s])?;
                }
            }
            Ok(())
        }
        Undo::Index { before, after } => {
            let changed = changed_since(git, after)?;
            if !changed.is_empty() {
                return Err(index_changed_error(&changed));
            }
            restore_files(git, before)
        }
        Undo::Files { before, after } => {
            let changed = changed_since(git, after)?;
            if !changed.is_empty() {
                return Err(files_changed_error(&changed));
            }
            restore_files(git, before)
        }
        Undo::Checkout { before, after, created, .. } => {
            require_clean_state(git)?;
            let head = head_snap(git)?;
            if !head.same_place(after) {
                return Err(GitError::other(format!(
                    "You've switched to {} since then. Switch back to {} to undo this.",
                    head.describe(),
                    after.describe()
                )));
            }
            switch_to(git, before)?;
            if let Some((name, sha)) = created {
                if branch_tip(git, name)?.as_deref() == Some(sha.as_str()) {
                    git.write(["branch", "-D", name])?;
                }
            }
            Ok(())
        }
        Undo::CreateBranch { name, sha, checkout, prev, .. } => {
            let head = head_snap(git)?;
            if *checkout && head.branch.as_deref() == Some(name.as_str()) {
                require_clean_state(git)?;
                switch_to(git, prev)?;
            } else if head.branch.as_deref() == Some(name.as_str()) {
                return Err(GitError::other(format!("Switch away from {name} first, then undo.")));
            }
            let tip = branch_tip(git, name)?;
            match tip {
                None => Ok(()),
                Some(t) if t == *sha => {
                    git.write(["branch", "-D", name])?;
                    Ok(())
                }
                Some(t) => Err(GitError::other(format!(
                    "{name} has new commits since it was created (it's now at {}), so deleting it could lose them.",
                    short(&t)
                ))),
            }
        }
        Undo::RenameBranch { from, to } => {
            if !branch_exists(git, to)? {
                return Err(GitError::other(format!("{to} no longer exists, so it can't be renamed back.")));
            }
            if branch_exists(git, from)? {
                return Err(GitError::other(format!("A branch named {from} already exists, so {to} can't be renamed back.")));
            }
            git.write(["branch", "-m", to, from])?;
            Ok(())
        }
        Undo::DeleteBranch { name, sha, config } => {
            if branch_exists(git, name)? {
                return Err(GitError::other(format!("A branch named {name} already exists, so it can't be restored.")));
            }
            git.write(["branch", "--no-track", name, sha])?;
            restore_config(git, config)?;
            Ok(())
        }
        Undo::RefMove(m) => undo_ref_move(git, m),
        Undo::StashPush { sha, .. } => {
            let shas = list_stash_shas(git)?;
            let Some(idx) = shas.iter().position(|s| s == sha) else {
                return Err(GitError::other("That stash no longer exists, so there's nothing to bring back."));
            };
            let rev = format!("stash@{{{idx}}}");
            if idx == 0 {
                let out = stash_pop_or_apply(git, "pop", &rev)?;
                if !out.ok() {
                    return Err(stash_error(out.error()));
                }
            } else {
                let out = stash_pop_or_apply(git, "apply", sha)?;
                if !out.ok() {
                    return Err(stash_error(out.error()));
                }
                git.write(["stash", "drop", &rev])?;
            }
            Ok(())
        }
        Undo::StashRestore { sha, message } => {
            if list_stash_shas(git)?.contains(sha) {
                return Ok(());
            }
            git.write(["stash", "store", "-m", message, sha])?;
            Ok(())
        }
        Undo::Tag { name, object, created } => {
            let r = format!("refs/tags/{name}");
            let cur = git.rev_parse(&r)?;
            if *created {
                match cur {
                    None => Ok(()),
                    Some(c) if c == *object => {
                        git.write(["update-ref", "-d", &r, object])?;
                        Ok(())
                    }
                    Some(_) => Err(GitError::other(format!("Tag {name} was changed since then, so it wasn't deleted."))),
                }
            } else {
                if cur.is_some() {
                    return Err(GitError::other(format!("A tag named {name} already exists, so it can't be restored.")));
                }
                git.write(["update-ref", &r, object, ""])?;
                Ok(())
            }
        }
        Undo::Remote { name, .. } => {
            let exists = git.read(["remote"])?.lines().any(|l| l == name);
            if exists {
                git.write(["remote", "remove", name])?;
            }
            Ok(())
        }
    }
}

fn stash_error(e: GitError) -> GitError {
    if e.kind == GitErrorKind::DirtyWorktree {
        e.with_message("Your current changes overlap with the stashed ones. Stash or commit them first, then undo again.")
    } else {
        e
    }
}

fn undo_ref_move(git: &Git, m: &RefMoveData) -> GitResult<()> {
    let state = current_state(git)?;
    if state != RepoState::Clean {
        if m.stoppable == Some(state) {
            abort_state(git, state)?;
        } else {
            let n = state_name(state);
            return Err(GitError::other(format!("A {n} is in progress. Finish or abort the {n} first, then undo.")));
        }
    }
    let label = m.branch.clone().unwrap_or_else(|| "HEAD".to_string());
    let tip = ref_tip(git, &m.branch)?;
    let needs_move = if tip.as_deref() == Some(m.before.as_str()) {
        false
    } else if tip.as_deref() == Some(m.after.as_str()) {
        true
    } else {
        return Err(moved_error(&label, tip.as_deref()));
    };

    if needs_move {
        match m.style {
            MoveStyle::Mixed => {
                if let Some(expected) = &m.index_after {
                    if write_tree(git)?.as_deref() != Some(expected.as_str()) {
                        return Err(GitError::other(
                            "Staged changes have changed since the reset, so undoing it would overwrite them. Unstage them first.",
                        ));
                    }
                }
            }
            MoveStyle::Hard => {
                if let Some(after) = &m.files_after {
                    let changed = changed_since(git, after)?;
                    if !changed.is_empty() {
                        return Err(files_changed_error(&changed));
                    }
                }
            }
            _ => {}
        }
    }

    // If the operation switched HEAD onto the branch, go back to where HEAD was.
    let head = head_snap(git)?;
    let switched_here = m.branch.is_some()
        && head.branch == m.branch
        && !m.head_before.same_place(&head)
        && m.head_before.branch.as_deref() != m.branch.as_deref();
    if switched_here {
        switch_to(git, &m.head_before)?;
    }
    if needs_move {
        move_ref(git, m, &m.after, &m.before, m.index_before.as_ref(), m.style)?;
        if m.style == MoveStyle::Hard {
            if let Some(files) = &m.files_before {
                restore_files(git, files)?;
            }
        }
    }
    Ok(())
}

/// Undo the newest undoable entry, or every not-undone entry back to and including `entry_id`.
pub fn undo(repo: &Path, entry_id: Option<&str>) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let mut log = load(&git)?;
    let targets: Vec<usize> = match entry_id {
        None => {
            let Some(i) = log.entries.iter().position(|e| e.entry.undoable && !e.entry.undone) else {
                return Err(GitError::other("There's nothing to undo."));
            };
            vec![i]
        }
        Some(id) => {
            let Some(pos) = log.entries.iter().position(|e| e.entry.id == id) else {
                return Err(GitError::not_found("That entry is no longer in the undo history."));
            };
            let target = &log.entries[pos].entry;
            if !target.undoable {
                return Err(GitError::other(
                    target.reason.clone().unwrap_or_else(|| "This operation can't be undone.".to_string()),
                ));
            }
            if target.undone {
                return Err(GitError::other("That operation is already undone."));
            }
            (0..=pos).filter(|&i| log.entries[i].entry.undoable && !log.entries[i].entry.undone).collect()
        }
    };
    let total = targets.len();
    let mut done = 0;
    let mut last: Option<OpEntry> = None;
    let mut failure: Option<GitError> = None;
    for i in targets {
        let mut entry = log.entries[i].clone();
        match undo_one(&git, &mut entry) {
            Ok(()) => {
                entry.entry.undone = true;
                log.redo.retain(|r| *r != entry.entry.id);
                log.redo.push(entry.entry.id.clone());
                last = Some(entry.entry.clone());
                log.entries[i] = entry;
                done += 1;
            }
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
    }
    save(&git, &log)?;
    if let Some(e) = failure {
        if done == 0 {
            return Err(e);
        }
        let msg = format!("Undid {done} of {total} operations, then stopped: {}", e.message);
        return Err(e.with_message(msg));
    }
    let message = if done == 1 {
        let l = last.as_ref().map(|e| e.label.clone()).unwrap_or_default();
        let extra = match last.as_ref().map(|e| e.kind.as_str()) {
            Some("stashPop") => ". The changes it applied stay in your files",
            _ => "",
        };
        format!("Undid {l}{extra}")
    } else {
        format!("Undid {done} operations")
    };
    Ok(OpResult { entry: last, message })
}

// ---------------------------------------------------------------------------------------
// Redo
// ---------------------------------------------------------------------------------------

fn redo_ref_move(git: &Git, m: &mut RefMoveData, id: &str) -> GitResult<()> {
    require_clean_state(git)?;
    if m.stopped {
        return Err(GitError::other("This operation stopped on conflicts, so it can't be redone. Run it again instead."));
    }
    let label = m.branch.clone().unwrap_or_else(|| "HEAD".to_string());
    let tip = ref_tip(git, &m.branch)?;
    if tip.as_deref() != Some(m.before.as_str()) && tip.as_deref() != Some(m.after.as_str()) {
        return Err(moved_error(&label, tip.as_deref()));
    }
    // Re-entering the branch the operation ended on.
    let head = head_snap(git)?;
    if m.branch.is_some() && !m.head_after.same_place(&head) && m.head_after.branch == m.branch {
        switch_to(git, &m.head_after)?;
    }
    if tip.as_deref() == Some(m.after.as_str()) {
        return Ok(());
    }
    if m.style == MoveStyle::Hard {
        // Re-run the hard reset with a fresh snapshot of what it will discard.
        let head = head_snap(git)?;
        let on_branch = match &m.branch {
            Some(b) => head.branch.as_deref() == Some(b.as_str()),
            None => head.branch.is_none(),
        };
        if !on_branch {
            return move_ref(git, m, &m.before.clone(), &m.after.clone(), None, MoveStyle::Keep);
        }
        let parsed = super::status::status_files(git)?;
        let paths = super::status::tracked_changed_paths(&parsed.files);
        let mut files_before = super::snapshot::capture_files(git, &paths, true, true)?;
        git.write(["reset", "--hard", "-q", &m.after])?;
        let mut files_after = super::snapshot::capture_files(git, &paths, true, true)?;
        super::snapshot::align(&mut files_before, &mut files_after);
        let mut objs = files_before.objects();
        objs.extend(files_after.objects());
        protect(git, id, &objs)?;
        m.files_before = Some(files_before);
        m.files_after = Some(files_after);
        return Ok(());
    }
    move_ref(git, m, &m.before.clone(), &m.after.clone(), m.index_after.as_ref(), m.style)
}

fn redo_one(git: &Git, e: &mut StoredEntry) -> GitResult<()> {
    if context_gone(git, e)? {
        let what = e.during.map(state_name).unwrap_or("operation");
        return Err(GitError::other(format!("The {what} this belonged to is no longer in progress, so it can't be redone.")));
    }
    let id = e.entry.id.clone();
    match &mut e.undo {
        Undo::None => Err(GitError::other("This operation can't be redone.")),
        Undo::Commit { branch, before, after } => {
            require_clean_state(git)?;
            let head = head_snap(git)?;
            match branch {
                Some(b) => {
                    if head.branch.as_deref() != Some(b.as_str()) {
                        return Err(GitError::other(format!("Switch back to {b} to redo this commit.")));
                    }
                    let tip = branch_tip(git, b)?;
                    if tip != *before {
                        return Err(moved_error(b, tip.as_deref()));
                    }
                    match before {
                        None => git.write(["update-ref", &format!("refs/heads/{b}"), after, ""])?,
                        Some(_) => git.write(["reset", "--soft", after])?,
                    };
                }
                None => {
                    if head.branch.is_some() || head.commit != *before {
                        return Err(moved_error("HEAD", head.commit.as_deref()));
                    }
                    git.write(["reset", "--soft", after])?;
                }
            }
            Ok(())
        }
        Undo::Index { before, after } => {
            let changed = changed_since(git, before)?;
            if !changed.is_empty() {
                return Err(index_changed_error(&changed));
            }
            restore_files(git, after)
        }
        Undo::Files { before, after } => {
            let changed = changed_since(git, before)?;
            if !changed.is_empty() {
                return Err(files_changed_error(&changed));
            }
            restore_files(git, after)
        }
        Undo::Checkout { before, after, target, created } => {
            require_clean_state(git)?;
            let head = head_snap(git)?;
            if !head.same_place(before) {
                return Err(GitError::other(format!("You've switched to {} since then.", head.describe())));
            }
            if let Some((name, sha)) = created {
                if !branch_exists(git, name)? {
                    let out = git.run(["switch", "--track", target.as_str()])?;
                    if !out.ok() {
                        return Err(out.error());
                    }
                    if let Some(t) = branch_tip(git, name)? {
                        *sha = t;
                    }
                    return Ok(());
                }
            }
            switch_to(git, after)
        }
        Undo::CreateBranch { name, sha, checkout, config, .. } => {
            if branch_exists(git, name)? {
                return Err(GitError::other(format!("A branch named {name} already exists.")));
            }
            git.write(["branch", "--no-track", name, sha])?;
            restore_config(git, config)?;
            if *checkout {
                switch_to(git, &HeadSnap { branch: Some(name.clone()), commit: Some(sha.clone()) })?;
            }
            Ok(())
        }
        Undo::RenameBranch { from, to } => {
            if !branch_exists(git, from)? || branch_exists(git, to)? {
                return Err(GitError::other(format!("Can't rename {from} to {to} again: the branches have changed since.")));
            }
            git.write(["branch", "-m", from, to])?;
            Ok(())
        }
        Undo::DeleteBranch { name, sha, .. } => {
            let tip = branch_tip(git, name)?;
            match tip {
                None => Ok(()),
                Some(t) if t == *sha => {
                    git.write(["branch", "-D", name])?;
                    Ok(())
                }
                Some(t) => Err(moved_error(name, Some(&t))),
            }
        }
        Undo::RefMove(m) => redo_ref_move(git, m, &id),
        Undo::StashPush { sha, message, include_untracked } => {
            let mut args: Vec<String> = vec!["stash".into(), "push".into()];
            if *include_untracked {
                args.push("--include-untracked".into());
            }
            if let Some(m) = message.as_ref().filter(|m| !m.is_empty()) {
                args.push("-m".into());
                args.push(m.clone());
            }
            let out = git.run(&args)?;
            if !out.ok() {
                return Err(out.error());
            }
            if out.all_output().contains("No local changes to save") {
                return Err(GitError::new(GitErrorKind::NothingToCommit, "There are no changes to stash again.", ""));
            }
            let new_sha = git.rev_parse("refs/stash")?.unwrap_or_default();
            protect(git, &id, std::slice::from_ref(&new_sha))?;
            *sha = new_sha;
            Ok(())
        }
        Undo::StashRestore { sha, .. } => {
            let shas = list_stash_shas(git)?;
            if let Some(idx) = shas.iter().position(|s| s == sha) {
                git.write(["stash", "drop", &format!("stash@{{{idx}}}")])?;
            }
            Ok(())
        }
        Undo::Tag { name, object, created } => {
            let r = format!("refs/tags/{name}");
            let cur = git.rev_parse(&r)?;
            if *created {
                if cur.is_some() {
                    return Err(GitError::other(format!("A tag named {name} already exists.")));
                }
                git.write(["update-ref", &r, object.as_str(), ""])?;
            } else if let Some(c) = cur {
                if c != *object {
                    return Err(GitError::other(format!("Tag {name} was changed since then.")));
                }
                git.write(["update-ref", "-d", &r, object.as_str()])?;
            }
            Ok(())
        }
        Undo::Remote { name, url } => {
            if git.read(["remote"])?.lines().any(|l| l == name) {
                return Err(GitError::other(format!("A remote named {name} already exists.")));
            }
            git.write(["remote", "add", name.as_str(), url.as_str()])?;
            Ok(())
        }
    }
}

/// Re-apply the most recently undone entry.
pub fn redo(repo: &Path) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let mut log = load(&git)?;
    let Some(id) = log.redo.last().cloned() else {
        if log.entries.iter().any(|e| e.entry.undone) {
            return Err(GitError::other("Redo isn't available after a new change."));
        }
        return Err(GitError::other("There's nothing to redo."));
    };
    let Some(pos) = log.entries.iter().position(|e| e.entry.id == id && e.entry.undone) else {
        log.redo.pop();
        save(&git, &log)?;
        return Err(GitError::other("There's nothing to redo."));
    };
    let mut entry = log.entries[pos].clone();
    redo_one(&git, &mut entry)?;
    entry.entry.undone = false;
    let result = entry.entry.clone();
    log.entries[pos] = entry;
    log.redo.pop();
    save(&git, &log)?;
    Ok(OpResult { message: format!("Redid {}", result.label), entry: Some(result) })
}
