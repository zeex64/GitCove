//! Simulating operations without touching refs, the index or the worktree. Simulated
//! commits are written to the object database as dangling objects and never referenced.

use super::error::{GitError, GitResult};
use super::log::load_range;
use super::ops::{plural, stash_at, stash_paths};
use super::refs::{branch_exists, branch_tip};
use super::runner::{Git, Opts, git_at_least, now_secs, short, split_nul};
use super::snapshot::head_snap;
use super::status::{has_tracked_changes, status_files, tracked_changed_paths};
use super::types::{FileChange, Operation, Preview, PreviewConflicts, PullMode, RefMove, ResetMode, SimulatedCommit};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct CommitMeta {
    pub sha: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub author_date: String,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_date: String,
    pub author_time: i64,
    pub summary: String,
    pub message: String,
}

pub fn load_metas(git: &Git, shas: &[String]) -> GitResult<Vec<CommitMeta>> {
    if shas.is_empty() {
        return Ok(vec![]);
    }
    let mut input = Vec::new();
    for s in shas {
        input.extend_from_slice(s.as_bytes());
        input.push(b'\n');
    }
    let out = git.read_stdin(
        [
            "log",
            "--no-walk=unsorted",
            "--stdin",
            "--date=raw",
            "--format=%H%x00%P%x00%an%x00%ae%x00%ad%x00%cn%x00%ce%x00%cd%x00%at%x00%s%x00%B%x1e",
        ],
        input,
    )?;
    let text = out.text();
    let mut metas = Vec::new();
    for rec in text.split('\x1e') {
        let rec = rec.trim_start_matches(['\n', '\r']);
        if rec.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = rec.splitn(11, '\0').collect();
        if f.len() < 11 {
            continue;
        }
        metas.push(CommitMeta {
            sha: f[0].to_string(),
            parents: f[1].split_whitespace().map(str::to_string).collect(),
            author_name: f[2].to_string(),
            author_email: f[3].to_string(),
            author_date: f[4].to_string(),
            committer_name: f[5].to_string(),
            committer_email: f[6].to_string(),
            committer_date: f[7].to_string(),
            author_time: f[8].trim().parse().unwrap_or(0),
            summary: f[9].to_string(),
            message: f[10].to_string(),
        });
    }
    // Keep the requested order.
    let mut ordered = Vec::with_capacity(shas.len());
    for s in shas {
        if let Some(m) = metas.iter().find(|m| &m.sha == s) {
            ordered.push(m.clone());
        }
    }
    Ok(ordered)
}

/// Identity used for brand-new simulated commits (merge, revert).
fn own_identity(git: &Git) -> (String, String) {
    let name = git.config_get("user.name").ok().flatten().filter(|s| !s.is_empty()).unwrap_or_else(|| "You".into());
    let email = git.config_get("user.email").ok().flatten().unwrap_or_else(|| "you@localhost".into());
    (name, email)
}

pub fn fresh_meta(git: &Git, summary: &str, message: &str) -> CommitMeta {
    let (name, email) = own_identity(git);
    let now = now_secs();
    let date = format!("{now} +0000");
    CommitMeta {
        sha: String::new(),
        parents: vec![],
        author_name: name.clone(),
        author_email: email.clone(),
        author_date: date.clone(),
        committer_name: name,
        committer_email: email,
        committer_date: date,
        author_time: now,
        summary: summary.to_string(),
        message: message.to_string(),
    }
}

/// `git merge-tree --write-tree`: (tree, conflicted files).
pub fn merge_tree(git: &Git, base: Option<&str>, ours: &str, theirs: &str) -> GitResult<(String, Vec<String>)> {
    let mb = base.map(|b| format!("--merge-base={b}"));
    let mut args = vec!["merge-tree", "--write-tree", "--name-only", "--no-messages", "-z"];
    if let Some(m) = &mb {
        args.push(m);
    }
    args.push(ours);
    args.push(theirs);
    let out = git.exec(&args, Opts { read_only: true, ..Default::default() })?;
    if out.code != 0 && out.code != 1 {
        return Err(out.error());
    }
    let text = out.text();
    let fields = split_nul(&text);
    let tree = fields.first().map(|s| s.trim().to_string()).unwrap_or_default();
    let mut files: Vec<String> = if out.code == 1 { fields[1..].iter().map(|s| s.to_string()).collect() } else { vec![] };
    files.retain(|f| !f.is_empty());
    files.dedup();
    Ok((tree, files))
}

pub fn commit_tree(git: &Git, tree: &str, parents: &[&str], meta: &CommitMeta) -> GitResult<String> {
    let mut args = vec!["commit-tree", "--no-gpg-sign", tree];
    for p in parents {
        args.push("-p");
        args.push(p);
    }
    let env = vec![
        ("GIT_AUTHOR_NAME".to_string(), meta.author_name.clone()),
        ("GIT_AUTHOR_EMAIL".to_string(), meta.author_email.clone()),
        ("GIT_AUTHOR_DATE".to_string(), meta.author_date.clone()),
        ("GIT_COMMITTER_NAME".to_string(), meta.committer_name.clone()),
        ("GIT_COMMITTER_EMAIL".to_string(), meta.committer_email.clone()),
        ("GIT_COMMITTER_DATE".to_string(), meta.committer_date.clone()),
    ];
    let out = git.exec(&args, Opts { stdin: Some(meta.message.as_bytes().to_vec()), env, ..Default::default() })?.check()?;
    Ok(out.text().trim().to_string())
}

fn blank(op: Operation, title: String) -> Preview {
    Preview {
        operation: op,
        title,
        created: vec![],
        replaced: vec![],
        moves: vec![],
        conflicts: PreviewConflicts { files: vec![], predicted: true },
        needs_stash: false,
        discarded_files: vec![],
        remote_commits_lost: vec![],
        irreversible: false,
        notes: vec![],
    }
}

fn rev_list(git: &Git, args: &[&str]) -> GitResult<Vec<String>> {
    let mut a = vec!["rev-list"];
    a.extend_from_slice(args);
    a.push("--");
    Ok(git.read(&a)?.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

/// Result of simulating a sequence of picks.
struct RebaseSim {
    created: Vec<SimulatedCommit>,
    conflict_files: Vec<String>,
    new_tip: String,
    predicted: bool,
    notes: Vec<String>,
    replaced: Vec<String>,
}

/// Simulate `git rebase <onto> <branch>` (branch given as a commit).
fn simulate_rebase(git: &Git, onto_sha: &str, branch_sha: &str, onto_label: &str) -> GitResult<RebaseSim> {
    let mut sim = RebaseSim {
        created: vec![],
        conflict_files: vec![],
        new_tip: branch_sha.to_string(),
        predicted: true,
        notes: vec![],
        replaced: vec![],
    };
    let range = format!("{onto_sha}..{branch_sha}");
    sim.replaced = rev_list(git, &[&range])?;
    if sim.replaced.is_empty() {
        sim.notes.push(format!("Already up to date with {onto_label}: nothing to rebase."));
        return Ok(sim);
    }
    if git.is_ancestor(branch_sha, onto_sha)? {
        // Unreachable in practice (range non-empty), kept for clarity.
        sim.new_tip = onto_sha.to_string();
        return Ok(sim);
    }
    let commits = rev_list(git, &["--reverse", "--no-merges", &range])?;
    let cherry = git.read(["cherry", onto_sha, branch_sha])?;
    let skipped: BTreeSet<String> =
        cherry.lines().filter_map(|l| l.strip_prefix("- ")).map(|s| s.trim().to_string()).collect();
    let to_replay: Vec<String> = commits.into_iter().filter(|c| !skipped.contains(c)).collect();
    if !skipped.is_empty() {
        sim.notes.push(format!(
            "{} already on {onto_label} and will be skipped.",
            if skipped.len() == 1 { "1 commit is".to_string() } else { format!("{} commits are", skipped.len()) }
        ));
    }
    let merges = rev_list(git, &["--merges", &range])?;
    if !merges.is_empty() {
        sim.notes.push(format!(
            "{} will be dropped; their changes are replayed as plain commits.",
            plural(merges.len(), "merge commit", "merge commits")
        ));
    }
    let metas = load_metas(git, &to_replay)?;
    let can_predict = git_at_least(2, 40);
    sim.predicted = can_predict;
    let mut tip = onto_sha.to_string();
    let mut first_conflict = true;
    let mut seen = BTreeSet::new();
    for (n, meta) in metas.iter().enumerate() {
        if !can_predict {
            let id = format!("sim-{}-{}", n + 1, meta.sha);
            sim.created.push(SimulatedCommit {
                id: id.clone(),
                replaces: Some(meta.sha.clone()),
                parents: vec![tip.clone()],
                summary: meta.summary.clone(),
                author_name: meta.author_name.clone(),
                author_time: meta.author_time,
                conflicted: false,
            });
            tip = id;
            continue;
        }
        let base = meta.parents.first().cloned().unwrap_or_else(|| super::runner::EMPTY_TREE.to_string());
        let (tree, files) = merge_tree(git, Some(&base), &tip, &meta.sha)?;
        let conflicted = !files.is_empty();
        if conflicted {
            if first_conflict {
                first_conflict = false;
                if n + 1 < metas.len() {
                    sim.notes.push(
                        "Steps after the first conflict assume it is resolved as-is, so their predictions are less certain."
                            .to_string(),
                    );
                }
            }
            for f in files {
                if seen.insert(f.clone()) {
                    sim.conflict_files.push(f);
                }
            }
        }
        let new = commit_tree(git, &tree, &[&tip], meta)?;
        sim.created.push(SimulatedCommit {
            id: new.clone(),
            replaces: Some(meta.sha.clone()),
            parents: vec![tip.clone()],
            summary: meta.summary.clone(),
            author_name: meta.author_name.clone(),
            author_time: meta.author_time,
            conflicted,
        });
        tip = new;
    }
    if !can_predict {
        sim.notes.push("Your Git version is too old to predict conflicts (merge-tree needs Git 2.40 or newer).".to_string());
        // Synthesized ids are not real objects; the ref would move to an unknown commit.
        sim.new_tip = tip;
    } else {
        sim.new_tip = if metas.is_empty() { onto_sha.to_string() } else { tip };
    }
    sim.created.reverse();
    Ok(sim)
}

fn changed_paths_between(git: &Git, a: &str, b: &str) -> GitResult<Vec<String>> {
    let t = git.read(["diff", "--name-only", "-z", "--no-renames", a, b])?;
    Ok(split_nul(&t).into_iter().map(str::to_string).collect())
}

fn overlaps(dirty: &[String], changed: &[String]) -> bool {
    let set: BTreeSet<&String> = changed.iter().collect();
    dirty.iter().any(|d| set.contains(d))
}

struct MergeSim {
    fast_forward: bool,
    up_to_date: bool,
    created: Vec<SimulatedCommit>,
    conflict_files: Vec<String>,
    new_tip: String,
    predicted: bool,
    result_tree_or_tip: String,
}

fn simulate_merge(git: &Git, head_sha: &str, theirs_sha: &str, theirs_label: &str, message: &str) -> GitResult<MergeSim> {
    let mut sim = MergeSim {
        fast_forward: false,
        up_to_date: false,
        created: vec![],
        conflict_files: vec![],
        new_tip: head_sha.to_string(),
        predicted: true,
        result_tree_or_tip: head_sha.to_string(),
    };
    if git.is_ancestor(theirs_sha, head_sha)? {
        sim.up_to_date = true;
        return Ok(sim);
    }
    let no_ff = git.config_get("merge.ff")?.as_deref() == Some("false");
    if !no_ff && git.is_ancestor(head_sha, theirs_sha)? {
        sim.fast_forward = true;
        sim.new_tip = theirs_sha.to_string();
        sim.result_tree_or_tip = theirs_sha.to_string();
        return Ok(sim);
    }
    let summary = message.lines().next().unwrap_or(message).to_string();
    if !git_at_least(2, 38) {
        sim.predicted = false;
        let id = format!("sim-merge-{}", short(theirs_sha));
        let meta = fresh_meta(git, &summary, message);
        sim.created.push(SimulatedCommit {
            id: id.clone(),
            replaces: None,
            parents: vec![head_sha.to_string(), theirs_sha.to_string()],
            summary,
            author_name: meta.author_name,
            author_time: meta.author_time,
            conflicted: false,
        });
        sim.new_tip = id;
        let _ = theirs_label;
        return Ok(sim);
    }
    let (tree, files) = merge_tree(git, None, head_sha, theirs_sha)?;
    let meta = fresh_meta(git, &summary, &format!("{message}\n"));
    let new = commit_tree(git, &tree, &[head_sha, theirs_sha], &meta)?;
    sim.created.push(SimulatedCommit {
        id: new.clone(),
        replaces: None,
        parents: vec![head_sha.to_string(), theirs_sha.to_string()],
        summary,
        author_name: meta.author_name,
        author_time: meta.author_time,
        conflicted: !files.is_empty(),
    });
    sim.conflict_files = files;
    sim.result_tree_or_tip = new.clone();
    sim.new_tip = new;
    Ok(sim)
}

fn head_label(head: &super::snapshot::HeadSnap) -> String {
    head.branch.clone().unwrap_or_else(|| "HEAD".to_string())
}

fn dirty_paths(files: &[FileChange]) -> Vec<String> {
    tracked_changed_paths(files)
}

pub fn preview_rebase(git: &Git, op: Operation, branch: &str, onto: &str, title: String) -> GitResult<Preview> {
    if !branch_exists(git, branch)? {
        return Err(GitError::not_found(format!("There's no local branch named {branch}. Only local branches can be rebased.")));
    }
    let branch_sha = branch_tip(git, branch)?.unwrap_or_default();
    let onto_sha = git.require_commit(onto)?;
    let mut p = blank(op, title);
    let sim = simulate_rebase(git, &onto_sha, &branch_sha, onto)?;
    let parsed = status_files(git)?;
    let head = head_snap(git)?;
    p.needs_stash = has_tracked_changes(&parsed.files) && !sim.replaced.is_empty();
    if head.branch.as_deref() != Some(branch) && !sim.replaced.is_empty() {
        p.notes.push(format!("You'll be switched to {branch}."));
    }
    if !sim.replaced.is_empty() && sim.new_tip != branch_sha {
        p.moves.push(RefMove { ref_: branch.to_string(), from: Some(branch_sha), to: Some(sim.new_tip.clone()) });
    }
    if p.needs_stash {
        p.notes.push("Your uncommitted changes will be stashed first and restored afterwards.".to_string());
    }
    p.created = sim.created;
    p.replaced = sim.replaced;
    p.conflicts = PreviewConflicts { files: sim.conflict_files, predicted: sim.predicted };
    p.notes.extend(sim.notes);
    if !p.conflicts.files.is_empty() {
        p.notes.push(format!(
            "The rebase will stop on conflicts in {}. You can resolve them or abort.",
            plural(p.conflicts.files.len(), "file", "files")
        ));
    }
    if !p.replaced.is_empty() {
        p.notes.push("You can undo this.".to_string());
    }
    Ok(p)
}

fn preview_merge_into_head(git: &Git, op: Operation, theirs: &str, title: String, message: &str) -> GitResult<Preview> {
    let head = head_snap(git)?;
    let Some(head_sha) = head.commit.clone() else {
        return Err(GitError::other("There are no commits on this branch yet, so there's nothing to merge into."));
    };
    let theirs_sha = git.require_commit(theirs)?;
    let mut p = blank(op, title);
    let sim = simulate_merge(git, &head_sha, &theirs_sha, theirs, message)?;
    let target = head_label(&head);
    if sim.up_to_date {
        p.notes.push(format!("{target} already contains {theirs}: nothing to merge."));
        return Ok(p);
    }
    p.moves.push(RefMove { ref_: target.clone(), from: Some(head_sha.clone()), to: Some(sim.new_tip.clone()) });
    if sim.fast_forward {
        p.notes.push("Fast-forward: no merge commit".to_string());
    }
    let parsed = status_files(git)?;
    let dirty = dirty_paths(&parsed.files);
    if !dirty.is_empty() && sim.predicted {
        let changed = changed_paths_between(git, &head_sha, &sim.result_tree_or_tip)?;
        p.needs_stash = overlaps(&dirty, &changed);
        if p.needs_stash {
            p.notes.push("Your uncommitted changes touch the same files, so they'll be stashed first and restored afterwards.".to_string());
        }
    }
    p.created = sim.created;
    p.conflicts = PreviewConflicts { files: sim.conflict_files, predicted: sim.predicted };
    if !sim.predicted {
        p.notes.push("Your Git version is too old to predict conflicts (needs Git 2.38 or newer).".to_string());
    }
    if !p.conflicts.files.is_empty() {
        p.notes.push(format!(
            "The merge will stop on conflicts in {}. You can resolve them or abort.",
            plural(p.conflicts.files.len(), "file", "files")
        ));
    }
    p.notes.push("You can undo this.".to_string());
    Ok(p)
}

fn preview_pick(git: &Git, op: Operation, commit: &str, revert: bool, title: String) -> GitResult<Preview> {
    let head = head_snap(git)?;
    let Some(head_sha) = head.commit.clone() else {
        return Err(GitError::other("There are no commits on this branch yet."));
    };
    let sha = git.require_commit(commit)?;
    let metas = load_metas(git, std::slice::from_ref(&sha))?;
    let Some(meta) = metas.into_iter().next() else {
        return Err(GitError::not_found(format!("Couldn't read commit {}.", short(&sha))));
    };
    let mut p = blank(op, title);
    let parent = meta.parents.first().cloned().unwrap_or_else(|| super::runner::EMPTY_TREE.to_string());
    if meta.parents.len() > 1 {
        p.notes.push("This is a merge commit; its changes relative to the first parent are used.".to_string());
    }
    let target = head_label(&head);
    if !git_at_least(2, 40) {
        p.conflicts.predicted = false;
        p.notes.push("Your Git version is too old to predict conflicts (merge-tree needs Git 2.40 or newer).".to_string());
        let id = format!("sim-1-{sha}");
        p.created.push(SimulatedCommit {
            id: id.clone(),
            replaces: if revert { None } else { Some(sha.clone()) },
            parents: vec![head_sha.clone()],
            summary: if revert { format!("Revert \"{}\"", meta.summary) } else { meta.summary.clone() },
            author_name: meta.author_name.clone(),
            author_time: meta.author_time,
            conflicted: false,
        });
        p.moves.push(RefMove { ref_: target, from: Some(head_sha), to: Some(id) });
        return Ok(p);
    }
    let (tree, files) = if revert {
        merge_tree(git, Some(&sha), &head_sha, &parent)?
    } else {
        merge_tree(git, Some(&parent), &head_sha, &sha)?
    };
    let new_meta = if revert {
        let summary = format!("Revert \"{}\"", meta.summary);
        fresh_meta(git, &summary, &format!("{summary}\n\nThis reverts commit {sha}.\n"))
    } else {
        let mut m = meta.clone();
        let (name, email) = own_identity(git);
        m.committer_name = name;
        m.committer_email = email;
        m.committer_date = format!("{} +0000", now_secs());
        m
    };
    let head_tree = git.read(["rev-parse", &format!("{head_sha}^{{tree}}")])?.trim().to_string();
    if files.is_empty() && tree == head_tree {
        p.notes.push(if revert {
            format!("{target} doesn't contain these changes anymore, so there's nothing to revert.")
        } else {
            format!("{target} already has these changes, so the commit would be empty.")
        });
        return Ok(p);
    }
    let new = commit_tree(git, &tree, &[&head_sha], &new_meta)?;
    p.created.push(SimulatedCommit {
        id: new.clone(),
        replaces: if revert { None } else { Some(sha.clone()) },
        parents: vec![head_sha.clone()],
        summary: new_meta.summary.clone(),
        author_name: new_meta.author_name.clone(),
        author_time: new_meta.author_time,
        conflicted: !files.is_empty(),
    });
    p.moves.push(RefMove { ref_: target.clone(), from: Some(head_sha.clone()), to: Some(new.clone()) });
    let parsed = status_files(git)?;
    let dirty = dirty_paths(&parsed.files);
    if !dirty.is_empty() {
        let changed = changed_paths_between(git, &head_sha, &new)?;
        if overlaps(&dirty, &changed) {
            p.notes.push(
                "You have uncommitted changes in files this touches. Stash or commit them first, or Git will refuse."
                    .to_string(),
            );
        }
    }
    if !files.is_empty() {
        p.notes.push(format!("This will stop on conflicts in {}.", plural(files.len(), "file", "files")));
    }
    p.conflicts.files = files;
    p.notes.push("You can undo this.".to_string());
    Ok(p)
}

pub fn title_for(git: &Git, op: &Operation) -> String {
    match op {
        Operation::Rebase { branch, onto } => format!("Rebase {branch} onto {onto}"),
        Operation::Merge { branch } => {
            let head = head_snap(git).map(|h| head_label(&h)).unwrap_or_else(|_| "HEAD".into());
            format!("Merge {branch} into {head}")
        }
        Operation::Reset { target, mode } => {
            let head = head_snap(git).map(|h| head_label(&h)).unwrap_or_else(|_| "HEAD".into());
            let t = git.commit_id(target).ok().flatten().map(|s| short(&s).to_string()).unwrap_or_else(|| target.clone());
            format!("Reset {head} to {t} ({})", mode.as_str())
        }
        Operation::ForcePush { remote, branch } => format!("Force push {branch} to {remote}"),
        Operation::DeleteBranch { name, .. } => format!("Delete branch {name}"),
        Operation::Discard { paths } => {
            if paths.len() == 1 {
                format!("Discard changes in {}", paths[0])
            } else {
                format!("Discard changes in {}", plural(paths.len(), "file", "files"))
            }
        }
        Operation::CherryPick { commit } => {
            let head = head_snap(git).map(|h| head_label(&h)).unwrap_or_else(|_| "HEAD".into());
            let c = git.commit_id(commit).ok().flatten().map(|s| short(&s).to_string()).unwrap_or_else(|| commit.clone());
            format!("Cherry-pick {c} onto {head}")
        }
        Operation::Revert { commit } => {
            let c = git.commit_id(commit).ok().flatten().map(|s| short(&s).to_string()).unwrap_or_else(|| commit.clone());
            format!("Revert {c}")
        }
        Operation::Pull { mode } => {
            let head = head_snap(git).map(|h| head_label(&h)).unwrap_or_else(|_| "HEAD".into());
            let up = upstream_of(git).ok().flatten().map(|(u, _)| u).unwrap_or_else(|| "its upstream".into());
            let how = match mode {
                PullMode::FfOnly => "fast-forward only",
                PullMode::Rebase => "rebase",
                PullMode::Merge => "merge",
            };
            format!("Pull {up} into {head} ({how})")
        }
        Operation::DropStash { index } => {
            let msg = stash_at(git, *index).map(|(_, _, m)| super::ops::quoted_summary(&m)).unwrap_or_default();
            if msg.is_empty() { format!("Delete stash@{{{index}}}") } else { format!("Delete stash {msg}") }
        }
    }
}

/// (upstream short name, upstream sha) of the current branch.
pub fn upstream_of(git: &Git) -> GitResult<Option<(String, String)>> {
    let Some(name) = git.try_read(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"])? else {
        return Ok(None);
    };
    let name = name.trim().to_string();
    let Some(sha) = git.commit_id("@{upstream}")? else { return Ok(None) };
    Ok(Some((name, sha)))
}

/// Paths matching the requested discard paths (exact files or directory prefixes), with
/// rename sources included.
pub fn expand_discard_paths(files: &[FileChange], paths: &[String]) -> Vec<FileChange> {
    let mut out = Vec::new();
    for f in files {
        let hit = paths.iter().any(|p| {
            let p = p.trim_end_matches('/');
            f.path == p || f.path.starts_with(&format!("{p}/")) || f.orig_path.as_deref() == Some(p)
        });
        if hit {
            out.push(f.clone());
        }
    }
    out
}

pub fn discard_path_list(selected: &[FileChange]) -> Vec<String> {
    let mut v = Vec::new();
    for f in selected {
        v.push(f.path.clone());
        if let (Some(o), Some(super::types::ChangeKind::Renamed)) = (&f.orig_path, f.staged) {
            v.push(o.clone());
        }
    }
    v.sort();
    v.dedup();
    v
}

pub fn preview_operation(repo: &Path, op: Operation) -> GitResult<Preview> {
    let git = Git::new(repo);
    let title = title_for(&git, &op);
    match op.clone() {
        Operation::Rebase { branch, onto } => preview_rebase(&git, op, &branch, &onto, title),
        Operation::Merge { branch } => {
            let msg = format!("Merge branch '{branch}'");
            preview_merge_into_head(&git, op, &branch, title, &msg)
        }
        Operation::Reset { target, mode } => {
            let head = head_snap(&git)?;
            let Some(head_sha) = head.commit.clone() else {
                return Err(GitError::other("There are no commits on this branch yet, so there's nothing to reset."));
            };
            let target_sha = git.require_commit(&target)?;
            let mut p = blank(op, title);
            p.replaced = rev_list(&git, &[&format!("{target_sha}..{head_sha}")])?;
            if target_sha != head_sha {
                p.moves.push(RefMove { ref_: head_label(&head), from: Some(head_sha.clone()), to: Some(target_sha.clone()) });
            }
            let parsed = status_files(&git)?;
            match mode {
                ResetMode::Hard => {
                    p.discarded_files = tracked_changed_paths(&parsed.files);
                    if !p.discarded_files.is_empty() {
                        p.notes.push(format!(
                            "Uncommitted changes in {} will be thrown away. Untracked files are left alone.",
                            plural(p.discarded_files.len(), "file", "files")
                        ));
                    }
                }
                ResetMode::Mixed => p.notes.push("Changes from the removed commits stay in your files, unstaged.".to_string()),
                ResetMode::Soft => p.notes.push("Changes from the removed commits stay in your files, staged.".to_string()),
            }
            if !p.replaced.is_empty() && !git.is_ancestor(&target_sha, &head_sha)? {
                p.notes.push(format!("{} isn't an ancestor of {}; the branch will jump to it.", short(&target_sha), head_label(&head)));
            }
            p.notes.push("You can undo this.".to_string());
            Ok(p)
        }
        Operation::ForcePush { remote, branch } => {
            let mut p = blank(op, title);
            p.irreversible = true;
            let local = branch_tip(&git, &branch)?
                .ok_or_else(|| GitError::not_found(format!("There's no local branch named {branch}.")))?;
            let remote_ref = format!("refs/remotes/{remote}/{branch}");
            match git.rev_parse(&remote_ref)? {
                Some(remote_sha) => {
                    p.remote_commits_lost = load_range(&git, &[&format!("{local}..{remote_sha}")])?;
                    p.moves.push(RefMove { ref_: format!("{remote}/{branch}"), from: Some(remote_sha), to: Some(local) });
                    p.notes.push(format!("Based on what was last fetched from {remote}."));
                    if !p.remote_commits_lost.is_empty() {
                        p.notes.push(format!(
                            "{} on {remote} will be removed from {branch}. Copies stay in this repository for a while.",
                            plural(p.remote_commits_lost.len(), "commit", "commits")
                        ));
                    }
                }
                None => {
                    p.moves.push(RefMove { ref_: format!("{remote}/{branch}"), from: None, to: Some(local) });
                    p.notes.push(format!("{remote} doesn't have {branch} yet (as of the last fetch); this works like a normal push."));
                }
            }
            p.notes.push("Uses --force-with-lease, so the push stops if the remote changed since your last fetch.".to_string());
            p.notes.push("Force pushes change the remote and can't be undone here.".to_string());
            Ok(p)
        }
        Operation::DeleteBranch { name, force } => {
            let sha = branch_tip(&git, &name)?
                .ok_or_else(|| GitError::not_found(format!("There's no branch named {name}. Refresh and try again.")))?;
            let mut p = blank(op, title);
            let head = head_snap(&git)?;
            if head.branch.as_deref() == Some(name.as_str()) {
                p.notes.push(format!("You can't delete {name} while you're on it. Switch to another branch first."));
            }
            let exclude = format!("--exclude={name}");
            let mut args: Vec<&str> = vec![&sha, "--not", &exclude, "--branches", "--remotes", "--tags"];
            if head.branch.is_none() {
                args.push("HEAD");
            }
            p.replaced = rev_list(&git, &args)?;
            p.moves.push(RefMove { ref_: name.clone(), from: Some(sha), to: None });
            if !p.replaced.is_empty() {
                p.notes.push(format!(
                    "{name} has {} that aren't on any other branch.",
                    plural(p.replaced.len(), "commit", "commits")
                ));
                if !force {
                    p.notes.push("It isn't fully merged, so deleting it needs force.".to_string());
                }
            }
            p.notes.push("You can undo this.".to_string());
            Ok(p)
        }
        Operation::Discard { paths } => {
            let mut p = blank(op, title);
            let parsed = status_files(&git)?;
            let selected = expand_discard_paths(&parsed.files, &paths);
            p.discarded_files = discard_path_list(&selected);
            if p.discarded_files.is_empty() {
                p.notes.push("Those files have no changes to discard.".to_string());
            } else {
                p.notes.push("You can undo this.".to_string());
            }
            Ok(p)
        }
        Operation::CherryPick { commit } => preview_pick(&git, op, &commit, false, title),
        Operation::Revert { commit } => preview_pick(&git, op, &commit, true, title),
        Operation::Pull { mode } => {
            let head = head_snap(&git)?;
            let Some(branch) = head.branch.clone() else {
                return Err(GitError::other("You're not on a branch. Switch to a branch before pulling."));
            };
            let Some((up, up_sha)) = upstream_of(&git)? else {
                return Err(GitError::other(format!("{branch} has no upstream branch. Push it first to set one up.")));
            };
            let note = format!("Based on what was last fetched; pulling fetches {up} first, so the result may include newer commits.");
            let mut p = match mode {
                PullMode::Rebase => {
                    let mut p = preview_rebase(&git, op.clone(), &branch, &up_sha, title)?;
                    // Show the upstream's name rather than its sha.
                    for n in p.notes.iter_mut() {
                        *n = n.replace(&up_sha, &up);
                    }
                    p
                }
                PullMode::Merge => {
                    let msg = format!("Merge branch '{}' of {}", up.split_once('/').map(|x| x.1).unwrap_or(&up), up.split('/').next().unwrap_or(""));
                    preview_merge_into_head(&git, op.clone(), &up_sha, title, &msg)?
                }
                PullMode::FfOnly => {
                    let mut p = blank(op.clone(), title);
                    let head_sha = head.commit.clone().unwrap_or_default();
                    if head.commit.is_none() || git.is_ancestor(&head_sha, &up_sha)? {
                        if head_sha != up_sha {
                            p.moves.push(RefMove { ref_: branch.clone(), from: head.commit.clone(), to: Some(up_sha.clone()) });
                            p.notes.push("Fast-forward: no merge commit".to_string());
                        } else {
                            p.notes.push(format!("{branch} is already up to date with {up}."));
                        }
                    } else if git.is_ancestor(&up_sha, &head_sha)? {
                        p.notes.push(format!("{branch} is already up to date with {up}."));
                    } else {
                        p.notes.push(format!(
                            "{branch} and {up} have diverged, so a fast-forward isn't possible. Pull with rebase or merge instead."
                        ));
                    }
                    p
                }
            };
            p.notes.insert(0, note);
            Ok(p)
        }
        Operation::DropStash { index } => {
            let (sha, _, _) = stash_at(&git, index)?;
            let mut p = blank(op, title);
            p.discarded_files = stash_paths(&git, &sha)?;
            p.notes.push("You can undo this.".to_string());
            Ok(p)
        }
    }
}
