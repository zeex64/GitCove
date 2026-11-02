//! `run_operation`: executes the same Operation a preview described.

use super::error::{GitError, GitErrorKind, GitResult};
use super::oplog::{self, MoveStyle, NewEntry, RefMoveData, Undo, current_state};
use super::ops::{plural, quoted_summary, run_network, stash_at};
use super::preview::{discard_path_list, expand_discard_paths, upstream_of};
use super::refs::{branch_config, branch_exists, branch_tip};
use super::runner::{EMPTY_TREE, Emit, Git, git_at_least, nul_paths, short};
use super::snapshot::{Snapshot, align, capture, capture_files, delete_worktree_file, head_snap, write_tree};
use super::status::{has_tracked_changes, status_files, tracked_changed_paths};
use super::types::{ChangeKind, OpResult, Operation, PullMode, RepoState, ResetMode};
use std::path::Path;

fn conflicted_count(git: &Git) -> GitResult<usize> {
    let t = git.read(["diff", "--name-only", "--diff-filter=U", "-z"])?;
    Ok(super::runner::split_nul(&t).len())
}

/// Build a RefMove entry from snapshots around an operation on `branch` (None = detached).
fn ref_move(
    op: &str,
    branch: Option<String>,
    before_tip: &str,
    before: &Snapshot,
    after: &Snapshot,
    git: &Git,
    stoppable: Option<RepoState>,
    stopped: bool,
) -> GitResult<RefMoveData> {
    let after_tip = match &branch {
        Some(b) => branch_tip(git, b)?.unwrap_or_default(),
        None => after.head.commit.clone().unwrap_or_default(),
    };
    Ok(RefMoveData {
        op: op.to_string(),
        branch,
        before: before_tip.to_string(),
        after: after_tip,
        head_before: before.head.clone(),
        head_after: after.head.clone(),
        style: MoveStyle::Keep,
        stoppable,
        stopped,
        ..Default::default()
    })
}

/// Common handling for operations that can stop on conflicts.
#[allow(clippy::too_many_arguments)]
fn run_stoppable(
    git: &Git,
    kind: &'static str,
    args: &[&str],
    branch: Option<String>,
    before_tip: String,
    stoppable: RepoState,
    label: String,
    ok_message: impl FnOnce(&Git, &Snapshot, &Snapshot) -> String,
    noun: &str,
    progress: Option<(&str, Emit)>,
) -> GitResult<OpResult> {
    let before = capture(git)?;
    let out = match progress {
        Some((op_id, emit)) => run_network(git, args, op_id, emit)?,
        None => git.run(args)?,
    };
    let state = current_state(git)?;
    if !out.ok() {
        let conflicts = conflicted_count(git)?;
        if state == stoppable && state != RepoState::Clean {
            let after = capture(git)?;
            let m = ref_move(kind, branch, &before_tip, &before, &after, git, Some(stoppable), true)?;
            let msg = if conflicts > 0 {
                format!("{noun} stopped on conflicts in {}", plural(conflicts, "file", "files"))
            } else {
                format!("{noun} stopped before finishing")
            };
            return oplog::finish(git, NewEntry::new(kind, label, Undo::RefMove(m)).snapshots(before, after), msg);
        }
        return Err(out.error());
    }
    let after = capture(git)?;
    let mut m = ref_move(kind, branch, &before_tip, &before, &after, git, Some(stoppable), false)?;
    if m.before == m.after && before.head.same_place(&after.head) {
        return Ok(OpResult { entry: None, message: "Already up to date".to_string() });
    }
    m.stopped = false;
    let mut msg = ok_message(git, &before, &after);
    let all = out.all_output();
    if all.contains("Applying autostash resulted in conflicts") || all.contains("Your changes are safe in the stash") {
        msg.push_str(". Your uncommitted changes couldn't be re-applied cleanly; they're saved in the stash list");
    }
    oplog::finish(git, NewEntry::new(kind, label, Undo::RefMove(m)).snapshots(before, after), msg)
}

pub fn run_operation(repo: &Path, op: Operation, op_id: &str, emit: Emit) -> GitResult<OpResult> {
    let git = Git::new(repo);
    match op {
        Operation::Rebase { branch, onto } => {
            if !branch_exists(&git, &branch)? {
                return Err(GitError::not_found(format!("There's no local branch named {branch}. Only local branches can be rebased.")));
            }
            let onto_sha = git.require_commit(&onto)?;
            let before_tip = branch_tip(&git, &branch)?.unwrap_or_default();
            let parsed = status_files(&git)?;
            let mut args = vec!["rebase"];
            if git_at_least(2, 38) {
                // Only the previewed branch moves, even with rebase.updateRefs configured.
                args.push("--no-update-refs");
            }
            if has_tracked_changes(&parsed.files) {
                args.push("--autostash");
            }
            args.push(&onto_sha);
            args.push(&branch);
            let (b2, o2) = (branch.clone(), onto.clone());
            run_stoppable(
                &git,
                "rebase",
                &args,
                Some(branch.clone()),
                before_tip,
                RepoState::Rebasing,
                format!("Rebase {branch} onto {onto}"),
                move |_, _, _| format!("Rebased {b2} onto {o2}"),
                "Rebase",
                None,
            )
        }
        Operation::Merge { branch } => {
            let head = head_snap(&git)?;
            let Some(head_sha) = head.commit.clone() else {
                return Err(GitError::other("There are no commits on this branch yet, so there's nothing to merge into."));
            };
            let theirs = git.require_commit(&branch)?;
            let target = head.branch.clone().unwrap_or_else(|| "HEAD".to_string());
            let parsed = status_files(&git)?;
            let mut args = vec!["merge", "--no-edit"];
            if has_tracked_changes(&parsed.files) && git_at_least(2, 27) {
                args.push("--autostash");
            }
            args.push(&branch);
            let (b2, t2, th2) = (branch.clone(), target.clone(), theirs.clone());
            run_stoppable(
                &git,
                "merge",
                &args,
                head.branch.clone(),
                head_sha,
                RepoState::Merging,
                format!("Merge {branch} into {target}"),
                move |_, _, after| {
                    if after.head.commit.as_deref() == Some(th2.as_str()) {
                        format!("Fast-forwarded {t2} to {b2}")
                    } else {
                        format!("Merged {b2} into {t2}")
                    }
                },
                "Merge",
                None,
            )
        }
        Operation::Reset { target, mode } => {
            let target_sha = git.require_commit(&target)?;
            let before = capture(&git)?;
            let Some(head_sha) = before.head.commit.clone() else {
                return Err(GitError::other("There are no commits on this branch yet, so there's nothing to reset."));
            };
            let mut m = RefMoveData {
                op: "reset".into(),
                branch: before.head.branch.clone(),
                before: head_sha.clone(),
                head_before: before.head.clone(),
                ..Default::default()
            };
            let mut paths = Vec::new();
            match mode {
                ResetMode::Hard => {
                    m.style = MoveStyle::Hard;
                    let parsed = status_files(&git)?;
                    paths = tracked_changed_paths(&parsed.files);
                    m.files_before = Some(capture_files(&git, &paths, true, true)?);
                }
                ResetMode::Mixed => {
                    m.style = MoveStyle::Mixed;
                    m.index_before = before.index_tree.clone();
                }
                ResetMode::Soft => m.style = MoveStyle::Soft,
            }
            let flag = format!("--{}", mode.as_str());
            let out = git.run(["reset", "-q", &flag, &target_sha])?;
            if !out.ok() {
                return Err(out.error());
            }
            let after = capture(&git)?;
            m.after = after.head.commit.clone().unwrap_or_default();
            m.head_after = after.head.clone();
            match mode {
                ResetMode::Hard => {
                    let mut fa = capture_files(&git, &paths, true, true)?;
                    if let Some(fb) = m.files_before.as_mut() {
                        align(fb, &mut fa);
                    }
                    m.files_after = Some(fa);
                }
                ResetMode::Mixed => m.index_after = write_tree(&git)?,
                ResetMode::Soft => {}
            }
            let name = before.head.branch.clone().unwrap_or_else(|| "HEAD".to_string());
            let label = format!("Reset {name} to {} ({})", short(&target_sha), mode.as_str());
            let mut msg = format!("Reset {name} to {}", short(&target_sha));
            if mode == ResetMode::Hard && !paths.is_empty() {
                msg.push_str(&format!(" and discarded changes in {}", plural(paths.len(), "file", "files")));
            }
            oplog::finish(&git, NewEntry::new("reset", label, Undo::RefMove(m)).snapshots(before, after), msg)
        }
        Operation::ForcePush { remote, branch } => {
            if !branch_exists(&git, &branch)? {
                return Err(GitError::not_found(format!("There's no local branch named {branch}.")));
            }
            let old_remote = git.rev_parse(&format!("refs/remotes/{remote}/{branch}"))?;
            let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
            let out = run_network(&git, &["push", "--force-with-lease", "--progress", &remote, &refspec], op_id, emit)?;
            if !out.ok() {
                let e = out.error();
                if e.kind == GitErrorKind::Rejected {
                    return Err(e.with_message(format!(
                        "{remote} changed since your last fetch, so the force push was stopped to protect those commits. Fetch, review, then try again."
                    )));
                }
                return Err(e);
            }
            let reason = match &old_remote {
                Some(o) => format!(
                    "Force pushes change the remote and can't be undone here. {remote}/{branch} was at {} before.",
                    short(o)
                ),
                None => "Force pushes change the remote and can't be undone here.".to_string(),
            };
            let mut entry = NewEntry::final_("forcePush", format!("Force push {branch} to {remote}"), reason);
            entry.extra_objects = old_remote.into_iter().collect();
            oplog::finish(&git, entry, format!("Force pushed {branch} to {remote}"))
        }
        Operation::DeleteBranch { name, force } => {
            let Some(sha) = branch_tip(&git, &name)? else {
                return Err(GitError::not_found(format!("There's no branch named {name}. Refresh and try again.")));
            };
            let head = head_snap(&git)?;
            if head.branch.as_deref() == Some(name.as_str()) {
                return Err(GitError::other(format!("You can't delete {name} while you're on it. Switch to another branch first.")));
            }
            let config = branch_config(&git, &name)?;
            let before = capture(&git)?;
            let out = git.run(["branch", if force { "-D" } else { "-d" }, &name])?;
            if !out.ok() {
                let all = out.all_output();
                if all.contains("not fully merged") {
                    return Err(GitError::new(
                        GitErrorKind::Other,
                        format!("{name} has commits that aren't merged yet. Delete it with force from the preview if you're sure; you can undo it."),
                        all.trim(),
                    ));
                }
                return Err(out.error());
            }
            let after = capture(&git)?;
            let undo = Undo::DeleteBranch { name: name.clone(), sha, config };
            oplog::finish(
                &git,
                NewEntry::new("deleteBranch", format!("Delete branch {name}"), undo).snapshots(before, after),
                format!("Deleted {name}"),
            )
        }
        Operation::Discard { paths } => discard(&git, &paths),
        Operation::CherryPick { commit } => {
            let sha = git.require_commit(&commit)?;
            let head = head_snap(&git)?;
            let Some(head_sha) = head.commit.clone() else {
                return Err(GitError::other("There are no commits on this branch yet."));
            };
            let is_merge = git.rev_parse(&format!("{sha}^2"))?.is_some();
            let mut args = vec!["cherry-pick"];
            if is_merge {
                args.extend(["-m", "1"]);
            }
            args.push(&sha);
            let target = head.branch.clone().unwrap_or_else(|| "HEAD".to_string());
            let s = short(&sha).to_string();
            let t2 = target.clone();
            let res = run_stoppable(
                &git,
                "cherryPick",
                &args,
                head.branch.clone(),
                head_sha,
                RepoState::CherryPicking,
                format!("Cherry-pick {s} onto {target}"),
                move |_, _, _| format!("Cherry-picked {s} onto {t2}"),
                "Cherry-pick",
                None,
            );
            empty_pick_error(res, &target, false)
        }
        Operation::Revert { commit } => {
            let sha = git.require_commit(&commit)?;
            let head = head_snap(&git)?;
            let Some(head_sha) = head.commit.clone() else {
                return Err(GitError::other("There are no commits on this branch yet."));
            };
            let is_merge = git.rev_parse(&format!("{sha}^2"))?.is_some();
            let mut args = vec!["revert", "--no-edit"];
            if is_merge {
                args.extend(["-m", "1"]);
            }
            args.push(&sha);
            let target = head.branch.clone().unwrap_or_else(|| "HEAD".to_string());
            let s = short(&sha).to_string();
            let s2 = s.clone();
            let res = run_stoppable(
                &git,
                "revert",
                &args,
                head.branch.clone(),
                head_sha,
                RepoState::Reverting,
                format!("Revert {s}"),
                move |_, _, _| format!("Reverted {s2}"),
                "Revert",
                None,
            );
            empty_pick_error(res, &target, true)
        }
        Operation::Pull { mode } => {
            let head = head_snap(&git)?;
            let Some(branch) = head.branch.clone() else {
                return Err(GitError::other("You're not on a branch. Switch to a branch before pulling."));
            };
            let Some((up, _)) = upstream_of(&git)? else {
                return Err(GitError::other(format!("{branch} has no upstream branch. Push it first to set one up.")));
            };
            let before_tip = head.commit.clone().unwrap_or_default();
            let parsed = status_files(&git)?;
            let dirty = has_tracked_changes(&parsed.files);
            let mut args = vec!["pull", "--progress"];
            let stoppable = match mode {
                PullMode::FfOnly => {
                    args.push("--ff-only");
                    RepoState::Merging
                }
                PullMode::Rebase => {
                    args.push("--rebase");
                    RepoState::Rebasing
                }
                PullMode::Merge => {
                    args.extend(["--no-rebase", "--no-edit"]);
                    RepoState::Merging
                }
            };
            if dirty && (mode == PullMode::Rebase || git_at_least(2, 27)) {
                args.push("--autostash");
            }
            let before = capture(&git)?;
            let out = run_network(&git, &args, op_id, emit)?;
            let state = current_state(&git)?;
            if !out.ok() {
                if state == stoppable && state != RepoState::Clean {
                    let conflicts = conflicted_count(&git)?;
                    let after = capture(&git)?;
                    let m = ref_move("pull", Some(branch.clone()), &before_tip, &before, &after, &git, Some(stoppable), true)?;
                    return oplog::finish(
                        &git,
                        NewEntry::new("pull", format!("Pull {up} into {branch}"), Undo::RefMove(m)).snapshots(before, after),
                        format!("Pull stopped on conflicts in {}", plural(conflicts, "file", "files")),
                    );
                }
                return Err(out.error());
            }
            let after = capture(&git)?;
            let m = ref_move("pull", Some(branch.clone()), &before_tip, &before, &after, &git, Some(stoppable), false)?;
            if m.before == m.after {
                return oplog::finish(
                    &git,
                    NewEntry::final_(
                        "pull",
                        format!("Pull {up} into {branch}"),
                        "Nothing changed locally; pulling only updated remote branches.",
                    ),
                    format!("{branch} is already up to date"),
                );
            }
            oplog::finish(
                &git,
                NewEntry::new("pull", format!("Pull {up} into {branch}"), Undo::RefMove(m)).snapshots(before, after),
                format!("Pulled {up} into {branch}"),
            )
        }
        Operation::DropStash { index } => {
            let (sha, subject, message) = stash_at(&git, index)?;
            git.write(["stash", "drop", &format!("stash@{{{index}}}")])?;
            let undo = Undo::StashRestore { sha, message: subject };
            oplog::finish(
                &git,
                NewEntry::new("dropStash", format!("Delete stash {}", quoted_summary(&message)), undo),
                "Deleted the stash. You can undo this",
            )
        }
    }
}

fn empty_pick_error(res: GitResult<OpResult>, target: &str, revert: bool) -> GitResult<OpResult> {
    match res {
        Err(e) if e.detail.contains("is now empty") || e.detail.contains("nothing to commit") => {
            Err(e.with_kind(GitErrorKind::Other).with_message(if revert {
                format!("{target} doesn't contain these changes anymore, so there's nothing to revert.")
            } else {
                format!("{target} already has these changes, so there's nothing to cherry-pick.")
            }))
        }
        other => other,
    }
}

fn discard(git: &Git, paths: &[String]) -> GitResult<OpResult> {
    if paths.is_empty() {
        return Err(GitError::other("Choose at least one file to discard."));
    }
    let parsed = status_files(git)?;
    let selected = expand_discard_paths(&parsed.files, paths);
    if selected.is_empty() {
        return Ok(OpResult { entry: None, message: "Nothing to discard".to_string() });
    }
    let all_paths = discard_path_list(&selected);
    let untracked: Vec<String> = selected
        .iter()
        .filter(|f| f.unstaged == Some(ChangeKind::Untracked))
        .map(|f| f.path.clone())
        .collect();
    let tracked: Vec<String> = all_paths.iter().filter(|p| !untracked.contains(p)).cloned().collect();
    let snap = capture(git)?;
    let mut before = capture_files(git, &all_paths, true, true)?;
    if !tracked.is_empty() {
        let source = if git.rev_parse("HEAD")?.is_some() { "--source=HEAD".to_string() } else { format!("--source={EMPTY_TREE}") };
        git.write_stdin(
            ["restore", &source, "--staged", "--worktree", "--pathspec-from-file=-", "--pathspec-file-nul"],
            nul_paths(&tracked),
        )?;
    }
    let root = git.dir().to_path_buf();
    for p in &untracked {
        delete_worktree_file(&root, p)?;
    }
    let mut after = capture_files(git, &all_paths, true, true)?;
    align(&mut before, &mut after);
    let shown: Vec<String> = selected.iter().map(|f| f.path.clone()).collect();
    let what = if shown.len() == 1 { shown[0].clone() } else { plural(shown.len(), "file", "files") };
    oplog::finish(
        git,
        NewEntry::new("discard", format!("Discard changes in {what}"), Undo::Files { before, after }).snapshots(snap, capture(git)?),
        format!("Discarded changes in {what}"),
    )
}
