//! Write commands. Each records an op-log entry before returning.

use super::error::{GitError, GitErrorKind, GitResult, git_line};
use super::oplog::{self, MoveStyle, NewEntry, RefMoveData, Undo, current_state, list_paths, stash_pop_or_apply};
use super::refs::{branch_config, branch_exists, branch_tip, list_remotes, list_stashes};
use super::runner::{Emit, Git, Opts, nul_paths, parse_progress, short};
use super::snapshot::{FileSet, HeadSnap, align, capture, capture_files, head_snap, restore_files};
use super::status::repo_state;
use super::types::{ChangeKind, ConflictSide, OpResult, RepoState};
use std::path::Path;

pub fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {many}") }
}

fn files_label(paths: &[String]) -> String {
    if paths.len() == 1 { paths[0].clone() } else { plural(paths.len(), "file", "files") }
}

/// First line of a commit message for labels, with curly quotes.
pub fn quoted_summary(message: &str) -> String {
    let first = message.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    let s: String = if first.chars().count() > 72 {
        let mut t: String = first.chars().take(71).collect();
        t.push('…');
        t
    } else {
        first.to_string()
    };
    format!("“{s}”")
}

fn require_paths(paths: &[String], verb: &str) -> GitResult<()> {
    if paths.is_empty() {
        return Err(GitError::other(format!("Choose at least one file to {verb}.")));
    }
    Ok(())
}

/// Add the original path of staged renames so both sides move together.
fn with_rename_sources(git: &Git, paths: &[String]) -> GitResult<Vec<String>> {
    let mut out: Vec<String> = paths.to_vec();
    if git.rev_parse("HEAD")?.is_some() {
        let text = git.read(["diff", "--cached", "--name-status", "-M", "-z", "--no-ext-diff"])?;
        for (kind, path, old) in super::diff::parse_name_status(&text) {
            if kind == ChangeKind::Renamed && paths.iter().any(|p| *p == path) {
                if let Some(o) = old {
                    out.push(o);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Run a writing command with paths fed through `--pathspec-from-file`.
fn run_with_paths(git: &Git, args: &[&str], paths: &[String]) -> GitResult<()> {
    let mut a: Vec<&str> = args.to_vec();
    a.push("--pathspec-from-file=-");
    a.push("--pathspec-file-nul");
    git.write_stdin(&a, nul_paths(paths))?;
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Staging
// ---------------------------------------------------------------------------------------

fn index_op(git: &Git, paths: &[String], run: impl FnOnce() -> GitResult<()>) -> GitResult<Option<(FileSet, FileSet)>> {
    let mut before = capture_files(git, paths, false, true)?;
    run()?;
    let mut after = capture_files(git, paths, false, true)?;
    align(&mut before, &mut after);
    if before == after {
        return Ok(None);
    }
    Ok(Some((before, after)))
}

pub fn stage(repo: &Path, paths: &[String]) -> GitResult<OpResult> {
    require_paths(paths, "stage")?;
    let git = Git::new(repo);
    let paths = with_rename_sources(&git, paths)?;
    let snap = capture(&git)?;
    let res = index_op(&git, &paths, || run_with_paths(&git, &["add", "-A"], &paths))?;
    let what = files_label(&paths);
    match res {
        None => Ok(OpResult { entry: None, message: format!("{what} had nothing new to stage") }),
        Some((before, after)) => oplog::finish(
            &git,
            NewEntry::new("stage", format!("Stage {what}"), Undo::Index { before, after }).snapshots(snap, capture(&git)?),
            format!("Staged {what}"),
        ),
    }
}

pub fn unstage(repo: &Path, paths: &[String]) -> GitResult<OpResult> {
    require_paths(paths, "unstage")?;
    let git = Git::new(repo);
    let paths = with_rename_sources(&git, paths)?;
    let unborn = git.rev_parse("HEAD")?.is_none();
    let snap = capture(&git)?;
    let res = index_op(&git, &paths, || {
        if unborn {
            run_with_paths(&git, &["rm", "--cached", "-r", "-q", "-f", "--ignore-unmatch"], &paths)
        } else {
            run_with_paths(&git, &["restore", "--staged"], &paths)
        }
    })?;
    let what = files_label(&paths);
    match res {
        None => Ok(OpResult { entry: None, message: format!("{what} had nothing staged") }),
        Some((before, after)) => oplog::finish(
            &git,
            NewEntry::new("unstage", format!("Unstage {what}"), Undo::Index { before, after }).snapshots(snap, capture(&git)?),
            format!("Unstaged {what}"),
        ),
    }
}

/// Header lines of a patch (everything outside hunks, minus "diff" lines).
///
/// Inside a hunk, "--- x" is a removed line "-- x", not a header. Like `git apply --recount`,
/// a hunk runs until a line that isn't a hunk line, or the next "@@" / "diff" line.
fn patch_header_lines(patch: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut in_hunk = false;
    for line in patch.lines() {
        if line.starts_with("diff ") {
            in_hunk = false;
        } else if line.starts_with("@@ ") {
            in_hunk = true;
        } else if !(in_hunk && (line.is_empty() || line.starts_with([' ', '-', '+', '\\']))) {
            in_hunk = false;
            out.push(line);
        }
    }
    out
}

/// Paths touched by a unified diff patch.
pub fn patch_paths(patch: &str) -> Vec<String> {
    let mut out = Vec::new();
    let clean = |raw: &str| -> Option<String> {
        let raw = raw.trim_end_matches(['\t', '\r']);
        let raw = raw.split('\t').next().unwrap_or(raw);
        if raw == "/dev/null" {
            return None;
        }
        let unq = if raw.starts_with('"') { super::diff::unquote_c(raw) } else { raw.to_string() };
        Some(unq.strip_prefix("a/").or_else(|| unq.strip_prefix("b/")).unwrap_or(&unq).to_string())
    };
    for line in patch_header_lines(patch) {
        if let Some(p) = line.strip_prefix("--- ").or_else(|| line.strip_prefix("+++ ")) {
            if let Some(p) = clean(p) {
                out.push(p);
            }
        } else if let Some(p) = line.strip_prefix("rename from ").or_else(|| line.strip_prefix("rename to ")) {
            out.push(p.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn past_tense_label(label: &str, cached: bool, reverse: bool) -> String {
    for (verb, past) in [("Stage ", "Staged "), ("Unstage ", "Unstaged "), ("Discard ", "Discarded "), ("Apply ", "Applied ")] {
        if let Some(rest) = label.strip_prefix(verb) {
            return format!("{past}{rest}");
        }
    }
    match (cached, reverse) {
        (true, false) => "Staged the selected lines".into(),
        (true, true) => "Unstaged the selected lines".into(),
        (false, true) => "Discarded the selected lines".into(),
        (false, false) => "Applied the change".into(),
    }
}

pub fn apply_patch(repo: &Path, patch: &str, cached: bool, reverse: bool, label: &str) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let paths = patch_paths(patch);
    if paths.is_empty() {
        return Err(GitError::other("The change to apply is empty. Select some lines first."));
    }
    let mut data = patch.as_bytes().to_vec();
    if !data.ends_with(b"\n") {
        data.push(b'\n');
    }
    let mut args = vec!["apply", "--whitespace=nowarn", "--recount"];
    if cached {
        args.push("--cached");
    }
    if reverse {
        args.push("--reverse");
    }
    args.push("-");
    let snap = capture(&git)?;
    let mut before = capture_files(&git, &paths, !cached, true)?;
    let out = git.exec(&args, Opts { stdin: Some(data), ..Default::default() })?;
    if !out.ok() {
        let e = out.error();
        if e.kind == GitErrorKind::Other || e.kind == GitErrorKind::NotFound {
            return Err(e.with_kind(GitErrorKind::Other).with_message(
                "Couldn't apply the selected lines because the file changed. Refresh and try again.",
            ));
        }
        return Err(e);
    }
    let mut after = capture_files(&git, &paths, !cached, true)?;
    align(&mut before, &mut after);
    let label = if label.trim().is_empty() {
        match (cached, reverse) {
            (true, false) => format!("Stage lines in {}", list_paths(&paths)),
            (true, true) => format!("Unstage lines in {}", list_paths(&paths)),
            (false, true) => format!("Discard lines in {}", list_paths(&paths)),
            (false, false) => format!("Apply a change to {}", list_paths(&paths)),
        }
    } else {
        label.trim().to_string()
    };
    let message = past_tense_label(&label, cached, reverse);
    let undo = if cached { Undo::Index { before, after } } else { Undo::Files { before, after } };
    oplog::finish(&git, NewEntry::new("applyPatch", label, undo).snapshots(snap, capture(&git)?), message)
}

const STALE_LINES: &str = "Couldn't stage those lines because the file changed. Refresh and try again.";

/// Put the index entries of `paths` back to HEAD (or remove them when `clear`).
fn reset_index_paths(git: &Git, paths: &[String], before: &FileSet, clear: bool) -> GitResult<()> {
    if clear {
        // -f: drop the entry even when it differs from both HEAD and the worktree (the
        // previous entry is in the undo snapshot; the worktree isn't touched).
        return run_with_paths(git, &["rm", "--cached", "-f", "-q", "--ignore-unmatch"], paths);
    }
    // `restore --staged` fails on a path that is in neither HEAD nor the index (untracked).
    let mut known = Vec::new();
    for p in paths {
        let in_index = before.get(p).is_some_and(|f| !f.index.is_empty());
        if in_index || git.rev_parse(&format!("HEAD:{p}"))?.is_some() {
            known.push(p.clone());
        }
    }
    if known.is_empty() {
        return Ok(());
    }
    run_with_paths(git, &["restore", "--staged"], &known)
}

/// Make exactly the changes in `patch` the staged state of `path`, as one undoable entry.
///
/// The patch's preimage is HEAD's version of the file (or nothing, for a new-file patch with
/// `--- /dev/null`). `None` unstages everything for the path. If the patch no longer applies,
/// the index is put back exactly as it was.
pub fn set_file_lines(repo: &Path, path: &str, patch: Option<&str>, label: &str) -> GitResult<OpResult> {
    if path.is_empty() {
        return Err(GitError::other("Choose a file first."));
    }
    let git = Git::new(repo);
    let patch = patch.filter(|p| !p.trim().is_empty());
    let paths = with_rename_sources(&git, &[path.to_string()])?;
    if let Some(p) = patch {
        let touched = patch_paths(p);
        if touched.is_empty() {
            return Err(GitError::other("The change to apply is empty. Select some lines first."));
        }
        if let Some(other) = touched.iter().find(|t| !paths.contains(t)) {
            return Err(GitError::other(format!("The selected lines are in {other}, not {path}. Refresh and try again.")));
        }
    }
    let unborn = git.rev_parse("HEAD")?.is_none();
    let mut before = capture_files(&git, &paths, false, true)?;
    if before.files.iter().any(|f| f.index.iter().any(|e| e.stage > 0)) {
        return Err(GitError::new(
            GitErrorKind::Conflict,
            format!("{path} has conflicts. Resolve them before choosing lines to commit."),
            "",
        ));
    }
    let snap = capture(&git)?;
    // A new-file patch applies to an index without the path; anything else applies to HEAD's version.
    let new_file = patch.is_some_and(|p| patch_header_lines(p).iter().any(|l| l.trim_end() == "--- /dev/null"));
    let applied = (|| -> GitResult<()> {
        reset_index_paths(&git, &paths, &before, unborn || new_file)?;
        if let Some(p) = patch {
            let mut data = p.as_bytes().to_vec();
            if !data.ends_with(b"\n") {
                data.push(b'\n');
            }
            let args = ["apply", "--cached", "--recount", "--whitespace=nowarn", "-"];
            let out = git.exec(args, Opts { stdin: Some(data), ..Default::default() })?;
            if !out.ok() {
                let e = out.error();
                if e.kind == GitErrorKind::Other || e.kind == GitErrorKind::NotFound {
                    return Err(e.with_kind(GitErrorKind::Other).with_message(STALE_LINES));
                }
                return Err(e);
            }
        }
        Ok(())
    })();
    if let Err(e) = applied {
        if let Err(re) = restore_files(&git, &before) {
            let msg = format!("Couldn't stage those lines, and the staged changes in {path} couldn't be put back: {}", re.message);
            return Err(re.with_message(msg));
        }
        return Err(e);
    }
    let mut after = capture_files(&git, &paths, false, true)?;
    align(&mut before, &mut after);
    let unstaging = patch.is_none();
    if before == after {
        let message =
            if unstaging { format!("{path} had nothing staged") } else { format!("Those lines in {path} were already staged") };
        return Ok(OpResult { entry: None, message });
    }
    let label = if !label.trim().is_empty() {
        label.trim().to_string()
    } else if unstaging {
        format!("Unstage {path}")
    } else {
        format!("Stage lines in {path}")
    };
    let message = past_tense_label(&label, true, unstaging);
    oplog::finish(&git, NewEntry::new("stageLines", label, Undo::Index { before, after }).snapshots(snap, capture(&git)?), message)
}

// ---------------------------------------------------------------------------------------
// Commit
// ---------------------------------------------------------------------------------------

pub fn commit(repo: &Path, message: &str, amend: bool) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let git_dir = git.absolute_git_dir()?;
    let merging = git_dir.join("MERGE_HEAD").exists();
    let unborn = git.rev_parse("HEAD")?.is_none();
    if amend && unborn {
        return Err(GitError::other("There's no commit to amend yet."));
    }
    let staged_count = {
        let mut args = vec!["diff", "--cached", "--name-only", "-z", "--no-renames"];
        if unborn {
            args.push(super::runner::EMPTY_TREE);
        }
        super::runner::split_nul(&git.read(&args)?).len()
    };
    if !amend && !merging && staged_count == 0 {
        return Err(GitError::new(
            GitErrorKind::NothingToCommit,
            "There's nothing to commit. Tick the files or lines to include first.",
            "",
        ));
    }
    let message_trimmed = message.trim();
    if message_trimmed.is_empty() && !amend {
        return Err(GitError::other("Write a commit message first."));
    }
    let before = capture(&git)?;
    let mut args: Vec<&str> = vec!["commit", "--cleanup=whitespace"];
    if amend {
        args.push("--amend");
    }
    let out = if message_trimmed.is_empty() {
        args.push("--no-edit");
        git.run(&args)?
    } else {
        args.extend(["-F", "-"]);
        git.exec(&args, Opts { stdin: Some(message.as_bytes().to_vec()), ..Default::default() })?
    };
    if !out.ok() {
        let all = out.all_output();
        if all.contains("Please tell me who you are") || all.contains("unable to auto-detect email address") {
            return Err(GitError::other(
                "Git doesn't know who you are yet. Set user.name and user.email in your Git config, then commit again.",
            )
            .with_kind(GitErrorKind::Other)
            .with_detail(all));
        }
        let e = out.error();
        if e.kind == GitErrorKind::Other {
            return Err(e.with_message(format!("Git didn't create the commit: {}", git_line(&all))));
        }
        return Err(e);
    }
    let after = capture(&git)?;
    let new_sha = after.head.commit.clone().unwrap_or_default();
    let summary_msg = if message_trimmed.is_empty() {
        git.read(["log", "-1", "--format=%s", &new_sha])?.trim().to_string()
    } else {
        message_trimmed.to_string()
    };
    let label = if amend {
        format!("Amend {}", quoted_summary(&summary_msg))
    } else {
        format!("Commit {}", quoted_summary(&summary_msg))
    };
    let where_ = before.head.branch.clone().unwrap_or_else(|| "detached HEAD".to_string());
    let msg = if amend {
        format!("Amended the last commit on {where_}")
    } else {
        format!("Committed {} to {where_}", plural(staged_count, "file", "files"))
    };
    let undo = Undo::Commit { branch: before.head.branch.clone(), before: before.head.commit.clone(), after: new_sha };
    let kind = if amend { "amend" } else { "commit" };
    oplog::finish(&git, NewEntry::new(kind, label, undo).snapshots(before, after), msg)
}

trait WithDetail {
    fn with_detail(self, d: String) -> Self;
}
impl WithDetail for GitError {
    fn with_detail(mut self, d: String) -> Self {
        self.detail = d.trim().to_string();
        self
    }
}

// ---------------------------------------------------------------------------------------
// Branches
// ---------------------------------------------------------------------------------------

fn dirty_switch_error(e: GitError, target: &str) -> GitError {
    if e.kind == GitErrorKind::DirtyWorktree {
        e.with_message(format!(
            "Switching to {target} would overwrite your uncommitted changes. Stash or commit them first, then switch."
        ))
    } else {
        e
    }
}

enum CheckoutPlan {
    Local(String),
    Track { remote_ref: String, local: String },
    Detach(String),
}

fn plan_checkout(git: &Git, target: &str) -> GitResult<CheckoutPlan> {
    let target = target.trim();
    if target.is_empty() {
        return Err(GitError::other("Choose a branch or commit to switch to."));
    }
    if branch_exists(git, target)? {
        return Ok(CheckoutPlan::Local(target.to_string()));
    }
    let short_target = target.strip_prefix("refs/remotes/").unwrap_or(target);
    if git.rev_parse(&format!("refs/remotes/{short_target}"))?.is_some() {
        let mut remotes: Vec<String> = list_remotes(git)?.into_iter().map(|r| r.name).collect();
        remotes.sort_by_key(|r| std::cmp::Reverse(r.len()));
        if let Some(r) = remotes.iter().find(|r| short_target.starts_with(&format!("{r}/"))) {
            let local = short_target[r.len() + 1..].to_string();
            if branch_exists(git, &local)? {
                return Ok(CheckoutPlan::Local(local));
            }
            return Ok(CheckoutPlan::Track { remote_ref: short_target.to_string(), local });
        }
    }
    if let Some(stripped) = target.strip_prefix("refs/heads/") {
        if branch_exists(git, stripped)? {
            return Ok(CheckoutPlan::Local(stripped.to_string()));
        }
    }
    match git.commit_id(target)? {
        Some(sha) => Ok(CheckoutPlan::Detach(sha)),
        None => Err(GitError::not_found(format!("Couldn't find a branch or commit named “{target}”. Refresh and try again."))),
    }
}

pub fn checkout(repo: &Path, target: &str) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let plan = plan_checkout(&git, target)?;
    let before = capture(&git)?;
    let (out, created_name, display, track_target) = match &plan {
        CheckoutPlan::Local(b) => {
            if before.head.branch.as_deref() == Some(b.as_str()) {
                return Ok(OpResult { entry: None, message: format!("Already on {b}") });
            }
            (git.run(["switch", "--no-guess", b.as_str()])?, None, b.clone(), String::new())
        }
        CheckoutPlan::Track { remote_ref, local } => (
            git.run(["switch", "--track", remote_ref.as_str()])?,
            Some(local.clone()),
            local.clone(),
            remote_ref.clone(),
        ),
        CheckoutPlan::Detach(sha) => {
            if before.head.branch.is_none() && before.head.commit.as_deref() == Some(sha.as_str()) {
                return Ok(OpResult { entry: None, message: format!("Already at {}", short(sha)) });
            }
            (git.run(["switch", "--detach", sha.as_str()])?, None, format!("{} (detached)", short(sha)), String::new())
        }
    };
    if !out.ok() {
        return Err(dirty_switch_error(out.error(), &display));
    }
    let after = capture(&git)?;
    let created = match &created_name {
        Some(n) => branch_tip(&git, n)?.map(|s| (n.clone(), s)),
        None => None,
    };
    let undo = Undo::Checkout {
        before: before.head.clone(),
        after: after.head.clone(),
        target: if track_target.is_empty() { target.to_string() } else { track_target },
        created,
    };
    let label = format!("Switch to {display}");
    let msg = format!("Switched to {display}");
    oplog::finish(&git, NewEntry::new("checkout", label, undo).snapshots(before, after), msg)
}

pub fn create_branch(repo: &Path, name: &str, from: &str, checkout: bool) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let name = name.trim();
    if name.is_empty() {
        return Err(GitError::other("Enter a name for the new branch."));
    }
    let valid = git.try_read(["check-ref-format", "--branch", name])?;
    if valid.is_none() {
        return Err(GitError::other(format!("“{name}” isn't a valid branch name. Avoid spaces and characters like ~ ^ : ? * [ \\.")));
    }
    if branch_exists(&git, name)? {
        return Err(GitError::other(format!("A branch named {name} already exists. Choose another name.")));
    }
    let from = if from.trim().is_empty() { "HEAD" } else { from.trim() };
    let sha = git.require_commit(from)?;
    let before = capture(&git)?;
    git.write(["branch", name, from])?;
    if checkout {
        let out = git.run(["switch", "--no-guess", name])?;
        if !out.ok() {
            let _ = git.run(["branch", "-D", name]);
            return Err(dirty_switch_error(out.error(), name));
        }
    }
    let after = capture(&git)?;
    let config = branch_config(&git, name)?;
    let undo = Undo::CreateBranch { name: name.to_string(), sha, checkout, prev: before.head.clone(), config };
    let msg = if checkout { format!("Created {name} and switched to it") } else { format!("Created {name}") };
    oplog::finish(&git, NewEntry::new("createBranch", format!("Create branch {name}"), undo).snapshots(before, after), msg)
}

pub fn rename_branch(repo: &Path, from: &str, to: &str) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let to = to.trim();
    if !branch_exists(&git, from)? {
        return Err(GitError::not_found(format!("There's no branch named {from}. Refresh and try again.")));
    }
    if git.try_read(["check-ref-format", "--branch", to])?.is_none() {
        return Err(GitError::other(format!("“{to}” isn't a valid branch name.")));
    }
    if branch_exists(&git, to)? {
        return Err(GitError::other(format!("A branch named {to} already exists. Choose another name.")));
    }
    let before = capture(&git)?;
    git.write(["branch", "-m", from, to])?;
    let after = capture(&git)?;
    let undo = Undo::RenameBranch { from: from.to_string(), to: to.to_string() };
    oplog::finish(
        &git,
        NewEntry::new("renameBranch", format!("Rename branch {from} to {to}"), undo).snapshots(before, after),
        format!("Renamed {from} to {to}"),
    )
}

// ---------------------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------------------

/// Run a network command, forwarding progress lines to `emit`.
pub fn run_network(git: &Git, args: &[&str], op_id: &str, emit: Emit) -> GitResult<super::runner::Out> {
    let mut cb = |line: &str| {
        if let Some(e) = emit {
            e(parse_progress(op_id, line));
        }
    };
    // Signed in to GitHub: git can ask this app for github.com credentials.
    git.exec(args, Opts { progress: Some(&mut cb), env: crate::github::git_env(), ..Default::default() })
}

pub fn fetch(repo: &Path, remote: Option<&str>, op_id: &str, emit: Emit) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let remote = remote.map(str::trim).filter(|r| !r.is_empty());
    let mut args = vec!["fetch", "--prune", "--progress"];
    match remote {
        Some(r) => args.push(r),
        None => args.push("--all"),
    }
    let out = run_network(&git, &args, op_id, emit)?;
    if !out.ok() {
        return Err(out.error());
    }
    let (label, msg) = match remote {
        Some(r) => (format!("Fetch {r}"), format!("Fetched {r}")),
        None => ("Fetch all remotes".to_string(), "Fetched all remotes".to_string()),
    };
    oplog::finish(
        &git,
        NewEntry::final_("fetch", label, "Fetching only updates remote branches; there's nothing to undo."),
        msg,
    )
}

pub fn push(repo: &Path, remote: &str, branch: &str, set_upstream: bool, op_id: &str, emit: Emit) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let mut args = vec!["push", "--progress"];
    if set_upstream {
        args.push("-u");
    }
    args.push(remote);
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    args.push(&refspec);
    let out = run_network(&git, &args, op_id, emit)?;
    if !out.ok() {
        return Err(out.error());
    }
    let msg = if out.all_output().contains("Everything up-to-date") {
        format!("{branch} is already up to date on {remote}")
    } else {
        format!("Pushed {branch} to {remote}")
    };
    oplog::finish(
        &git,
        NewEntry::final_("push", format!("Push {branch} to {remote}"), "Pushes change the remote and can't be undone here."),
        msg,
    )
}

// ---------------------------------------------------------------------------------------
// Stash
// ---------------------------------------------------------------------------------------

pub fn stash_push(repo: &Path, message: Option<&str>, include_untracked: bool) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let message = message.map(str::trim).filter(|m| !m.is_empty());
    let prev = git.rev_parse("refs/stash")?;
    let mut args: Vec<&str> = vec!["stash", "push"];
    if include_untracked {
        args.push("--include-untracked");
    }
    if let Some(m) = message {
        args.push("-m");
        args.push(m);
    }
    let before = capture(&git)?;
    let out = git.run(&args)?;
    if !out.ok() {
        return Err(out.error());
    }
    let sha = git.rev_parse("refs/stash")?;
    if out.all_output().contains("No local changes to save") || sha.is_none() || sha == prev {
        return Err(GitError::new(GitErrorKind::NothingToCommit, "There are no changes to stash.", out.all_output().trim()));
    }
    let after = capture(&git)?;
    let sha = sha.unwrap_or_default();
    let label = match message {
        Some(m) => format!("Stash {}", quoted_summary(m)),
        None => "Stash changes".to_string(),
    };
    let undo = Undo::StashPush { sha, message: message.map(str::to_string), include_untracked };
    oplog::finish(&git, NewEntry::new("stashPush", label, undo).snapshots(before, after), "Stashed your changes")
}

/// (sha, reflog subject) of a stash entry.
pub fn stash_at(git: &Git, index: u32) -> GitResult<(String, String, String)> {
    let stashes = list_stashes(git)?;
    let s = stashes
        .into_iter()
        .find(|s| s.index == index)
        .ok_or_else(|| GitError::not_found(format!("There's no stash@{{{index}}}. Refresh and try again.")))?;
    let subject = git
        .read(["log", "-1", "--format=%s", &s.commit])
        .map(|t| t.trim().to_string())
        .unwrap_or_else(|_| s.message.clone());
    Ok((s.commit, subject, s.message))
}

/// Files a stash would touch (worktree, index and untracked parts).
pub fn stash_paths(git: &Git, sha: &str) -> GitResult<Vec<String>> {
    let mut paths = Vec::new();
    let base = format!("{sha}^1");
    for (a, b) in [(base.clone(), sha.to_string()), (base.clone(), format!("{sha}^2"))] {
        if git.rev_parse(&b)?.is_some() {
            let t = git.read(["diff", "--name-only", "-z", "--no-renames", &a, &b])?;
            paths.extend(super::runner::split_nul(&t).into_iter().map(str::to_string));
        }
    }
    if git.rev_parse(&format!("{sha}^3"))?.is_some() {
        let t = git.read(["ls-tree", "-r", "-z", "--name-only", &format!("{sha}^3")])?;
        paths.extend(super::runner::split_nul(&t).into_iter().map(str::to_string));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn conflicted_count(git: &Git) -> GitResult<usize> {
    let t = git.read(["diff", "--name-only", "--diff-filter=U", "-z"])?;
    Ok(super::runner::split_nul(&t).len())
}

pub fn stash_apply(repo: &Path, index: u32, pop: bool) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let (sha, subject, message) = stash_at(&git, index)?;
    let paths = stash_paths(&git, &sha)?;
    let snap = capture(&git)?;
    let mut before_files = capture_files(&git, &paths, true, true)?;
    let rev = format!("stash@{{{index}}}");
    let out = stash_pop_or_apply(&git, if pop { "pop" } else { "apply" }, &rev)?;
    let conflicts = conflicted_count(&git)?;
    if !out.ok() && conflicts == 0 {
        let e = out.error();
        if e.kind == GitErrorKind::DirtyWorktree {
            return Err(e.with_message(
                "Your current changes overlap with the stashed ones. Commit or stash them first, then apply the stash.",
            ));
        }
        return Err(e);
    }
    let mut after_files = capture_files(&git, &paths, true, true)?;
    align(&mut before_files, &mut after_files);
    let dropped = pop && !list_stashes(&git)?.iter().any(|s| s.commit == sha);
    let quoted = quoted_summary(&message);
    if dropped {
        let undo = Undo::StashRestore { sha, message: subject };
        return oplog::finish(
            &git,
            NewEntry::new("stashPop", format!("Pop stash {quoted}"), undo).snapshots(snap, capture(&git)?),
            "Applied and removed the stash. Undo brings the stash back; the applied changes stay in your files",
        );
    }
    let msg = if conflicts > 0 {
        format!("Applied the stash with conflicts in {}", plural(conflicts, "file", "files"))
    } else {
        "Applied the stash".to_string()
    };
    let undo = Undo::Files { before: before_files, after: after_files };
    oplog::finish(&git, NewEntry::new("stashApply", format!("Apply stash {quoted}"), undo).snapshots(snap, capture(&git)?), msg)
}

// ---------------------------------------------------------------------------------------
// In-progress operations
// ---------------------------------------------------------------------------------------

fn state_noun(s: RepoState) -> &'static str {
    match s {
        RepoState::Rebasing => "rebase",
        RepoState::Merging => "merge",
        RepoState::CherryPicking => "cherry-pick",
        RepoState::Reverting => "revert",
        RepoState::Bisecting => "bisect",
        RepoState::Clean => "operation",
    }
}

/// Branch an in-progress operation belongs to and that branch's tip.
fn op_branch(git: &Git, state: RepoState) -> GitResult<(Option<String>, Option<String>)> {
    let git_dir = git.absolute_git_dir()?;
    if state == RepoState::Rebasing {
        let dir = if git_dir.join("rebase-merge").is_dir() { git_dir.join("rebase-merge") } else { git_dir.join("rebase-apply") };
        let head_name = std::fs::read_to_string(dir.join("head-name")).unwrap_or_default().trim().to_string();
        if let Some(b) = head_name.strip_prefix("refs/heads/") {
            return Ok((Some(b.to_string()), branch_tip(git, b)?));
        }
        let orig = std::fs::read_to_string(dir.join("orig-head")).unwrap_or_default().trim().to_string();
        return Ok((None, Some(orig).filter(|s| !s.is_empty())));
    }
    let head = head_snap(git)?;
    Ok((head.branch, head.commit))
}

fn sequence_step(
    repo: &Path,
    kind: &'static str,
    args_for: impl Fn(RepoState) -> Option<Vec<&'static str>>,
    verb_label: &str,
) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let state = current_state(&git)?;
    if state == RepoState::Clean {
        return Err(GitError::other("There's no rebase, merge, cherry-pick or revert in progress."));
    }
    let Some(args) = args_for(state) else {
        return Err(GitError::other(format!("A {} can't skip commits. Continue or abort it instead.", state_noun(state))));
    };
    if kind == "continue" && conflicted_count(&git)? > 0 {
        return Err(GitError::new(
            GitErrorKind::Conflict,
            "Some files still have conflicts. Resolve them and stage the files, then continue.",
            "",
        ));
    }
    let (branch, before_tip) = op_branch(&git, state)?;
    let before_tip = before_tip.unwrap_or_default();
    let before = capture(&git)?;
    let out = git.run(&args)?;
    let new_state = current_state(&git)?;
    let conflicts = conflicted_count(&git)?;
    let stopped = new_state == state && new_state != RepoState::Clean;
    if !out.ok() && !(stopped && conflicts > 0) {
        let all = out.all_output();
        let e = out.error();
        if all.contains("No changes - did you forget") || all.contains("is now empty") || all.contains("nothing to commit") {
            return Err(e.with_kind(GitErrorKind::Other).with_message(
                "This commit has no changes left after resolving the conflicts. Skip it instead.",
            ));
        }
        return Err(e);
    }
    let after = capture(&git)?;
    let after_tip = match &branch {
        Some(b) => branch_tip(&git, b)?,
        None => after.head.commit.clone(),
    }
    .unwrap_or_default();
    let noun = state_noun(state);
    let where_ = branch.clone().unwrap_or_else(|| "HEAD".to_string());
    let label = format!("{verb_label} {noun} of {where_}");
    let msg = if stopped && conflicts > 0 {
        format!("{} stopped on conflicts in {}", capitalize(noun), plural(conflicts, "file", "files"))
    } else if stopped {
        format!("{} {}", past(verb_label), noun)
    } else {
        format!("Finished the {noun} of {where_}")
    };
    let m = RefMoveData {
        op: kind.to_string(),
        branch: branch.clone(),
        before: before_tip.clone(),
        after: after_tip,
        head_before: HeadSnap { branch: branch.clone(), commit: Some(before_tip) },
        head_after: after.head.clone(),
        style: MoveStyle::Keep,
        stoppable: Some(state),
        stopped,
        ..Default::default()
    };
    oplog::finish(&git, NewEntry::new(kind, label, Undo::RefMove(m)).snapshots(before, after), msg)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn past(verb: &str) -> String {
    match verb {
        "Continue" => "Continued the".into(),
        "Skip commit in" => "Skipped a commit in the".into(),
        v => v.to_string(),
    }
}

pub fn continue_operation(repo: &Path) -> GitResult<OpResult> {
    sequence_step(
        repo,
        "continue",
        |s| match s {
            RepoState::Rebasing => Some(vec!["rebase", "--continue"]),
            RepoState::Merging => Some(vec!["commit", "--no-edit"]),
            RepoState::CherryPicking => Some(vec!["cherry-pick", "--continue"]),
            RepoState::Reverting => Some(vec!["revert", "--continue"]),
            _ => None,
        },
        "Continue",
    )
}

pub fn skip_operation(repo: &Path) -> GitResult<OpResult> {
    sequence_step(
        repo,
        "skip",
        |s| match s {
            RepoState::Rebasing => Some(vec!["rebase", "--skip"]),
            RepoState::CherryPicking => Some(vec!["cherry-pick", "--skip"]),
            _ => None,
        },
        "Skip commit in",
    )
}

pub fn abort_operation(repo: &Path) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let state = repo_state(&git.absolute_git_dir()?);
    if state == RepoState::Clean {
        return Err(GitError::other("There's no operation in progress to abort."));
    }
    let noun = state_noun(state);
    oplog::abort_state(&git, state)?;
    oplog::finish(
        &git,
        NewEntry::final_(
            "abort",
            format!("Abort {noun}"),
            format!("An abort can't be undone. Everything is back where it was before the {noun} started."),
        ),
        format!("Aborted the {noun}"),
    )
}

pub fn resolve_conflict(repo: &Path, path: &str, side: ConflictSide) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let paths = vec![path.to_string()];
    let snap = capture(&git)?;
    let mut before = capture_files(&git, &paths, true, true)?;
    let stage_wanted = match side {
        ConflictSide::Ours => Some(2u8),
        ConflictSide::Theirs => Some(3u8),
        ConflictSide::Mark => None,
    };
    match stage_wanted {
        Some(stage) => {
            let has_stage = before.get(path).map(|p| p.index.iter().any(|e| e.stage == stage)).unwrap_or(false);
            let has_any_conflict = before.get(path).map(|p| p.index.iter().any(|e| e.stage > 0)).unwrap_or(false);
            if !has_any_conflict {
                return Err(GitError::other(format!("{path} doesn't have a conflict to resolve.")));
            }
            if has_stage {
                let flag = if stage == 2 { "--ours" } else { "--theirs" };
                git.write(["checkout", flag, "--", &format!(":(literal){path}")])?;
                git.write(["add", "--", path])?;
            } else {
                // That side deleted the file: resolve as a deletion.
                git.write(["rm", "-q", "-f", "--", path])?;
            }
        }
        None => {
            git.write(["add", "-A", "--", path])?;
        }
    }
    let mut after = capture_files(&git, &paths, true, true)?;
    align(&mut before, &mut after);
    // Git's "ours" is the branch being built on: during a rebase that is the upstream,
    // and the user's own commit is "theirs". Name the side the user actually kept.
    let rebasing = matches!(oplog::current_state(&git)?, crate::git::types::RepoState::Rebasing);
    let (ours_name, theirs_name) = if rebasing { ("the upstream’s", "your commit’s") } else { ("your", "their") };
    let (label, msg) = match side {
        ConflictSide::Ours => (format!("Keep {ours_name} version of {path}"), format!("Resolved {path} with {ours_name} version")),
        ConflictSide::Theirs => (format!("Keep {theirs_name} version of {path}"), format!("Resolved {path} with {theirs_name} version")),
        ConflictSide::Mark => (format!("Mark {path} as resolved"), format!("Marked {path} as resolved")),
    };
    oplog::finish(&git, NewEntry::new("resolveConflict", label, Undo::Files { before, after }).snapshots(snap, capture(&git)?), msg)
}

// ---------------------------------------------------------------------------------------
// Tags and remotes
// ---------------------------------------------------------------------------------------

pub fn create_tag(repo: &Path, name: &str, commit: &str, message: Option<&str>) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let name = name.trim();
    if name.is_empty() {
        return Err(GitError::other("Enter a name for the tag."));
    }
    if git.try_read(["check-ref-format", &format!("refs/tags/{name}")])?.is_none() {
        return Err(GitError::other(format!("“{name}” isn't a valid tag name.")));
    }
    if git.rev_parse(&format!("refs/tags/{name}"))?.is_some() {
        return Err(GitError::other(format!("A tag named {name} already exists. Choose another name.")));
    }
    let sha = git.require_commit(commit)?;
    let message = message.map(str::trim).filter(|m| !m.is_empty());
    match message {
        Some(m) => {
            git.exec(["tag", "-a", name, &sha, "-F", "-"], Opts { stdin: Some(m.as_bytes().to_vec()), ..Default::default() })?
                .check()?;
        }
        None => {
            git.write(["tag", name, &sha])?;
        }
    }
    let object = git.rev_parse(&format!("refs/tags/{name}"))?.unwrap_or_default();
    let undo = Undo::Tag { name: name.to_string(), object, created: true };
    oplog::finish(
        &git,
        NewEntry::new("createTag", format!("Create tag {name}"), undo),
        format!("Created tag {name} at {}", short(&sha)),
    )
}

pub fn delete_tag(repo: &Path, name: &str) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let object = git
        .rev_parse(&format!("refs/tags/{name}"))?
        .ok_or_else(|| GitError::not_found(format!("There's no tag named {name}. Refresh and try again.")))?;
    git.write(["tag", "-d", name])?;
    let undo = Undo::Tag { name: name.to_string(), object, created: false };
    oplog::finish(&git, NewEntry::new("deleteTag", format!("Delete tag {name}"), undo), format!("Deleted tag {name}"))
}

pub fn add_remote(repo: &Path, name: &str, url: &str) -> GitResult<OpResult> {
    let git = Git::new(repo);
    let (name, url) = (name.trim(), url.trim());
    if name.is_empty() || url.is_empty() {
        return Err(GitError::other("Enter both a name and a URL for the remote."));
    }
    if git.read(["remote"])?.lines().any(|l| l == name) {
        return Err(GitError::other(format!("A remote named {name} already exists. Choose another name.")));
    }
    git.write(["remote", "add", name, url])?;
    let undo = Undo::Remote { name: name.to_string(), url: url.to_string() };
    oplog::finish(&git, NewEntry::new("addRemote", format!("Add remote {name}"), undo), format!("Added remote {name}"))
}
