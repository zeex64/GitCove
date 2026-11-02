//! Integration tests against real temporary repositories created with the git CLI.

use gitcove_lib::git::types::*;
use gitcove_lib::git::{GitErrorKind, diff, log, operation, oplog, ops, preview, refs, repo, status};
use std::path::{Path, PathBuf};
use std::process::Command;

struct Repo {
    _tmp: tempfile::TempDir,
    path: PathBuf,
}

impl Repo {
    fn new() -> Repo {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("repo");
        std::fs::create_dir_all(&path).unwrap();
        let hooks = tmp.path().join("no-hooks");
        std::fs::create_dir_all(&hooks).unwrap();
        let r = Repo { _tmp: tmp, path };
        r.git(&["init", "-q", "-b", "main"]);
        r.git(&["config", "user.name", "Test User"]);
        r.git(&["config", "user.email", "test@example.com"]);
        r.git(&["config", "commit.gpgsign", "false"]);
        r.git(&["config", "tag.gpgsign", "false"]);
        r.git(&["config", "core.hooksPath", hooks.to_str().unwrap()]);
        r.git(&["config", "core.autocrlf", "false"]);
        r.git(&["config", "merge.ff", "true"]);
        r.git(&["config", "pull.rebase", "false"]);
        r.git(&["config", "rebase.autoStash", "false"]);
        r
    }

    fn p(&self) -> &Path {
        &self.path
    }

    fn git_out(&self, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_EDITOR", "true")
            .env("LC_ALL", "C")
            .output()
            .unwrap()
    }

    fn git(&self, args: &[&str]) -> String {
        let out = self.git_out(args);
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    fn write_bytes(&self, rel: &str, content: &[u8]) {
        std::fs::write(self.path.join(rel), content).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.path.join(rel)).unwrap()
    }

    fn exists(&self, rel: &str) -> bool {
        self.path.join(rel).exists()
    }

    fn commit_file(&self, rel: &str, content: &str, msg: &str) -> String {
        self.write(rel, content);
        self.git(&["add", "--", rel]);
        self.git(&["commit", "-q", "-m", msg]);
        self.head()
    }

    fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"])
    }

    fn branch(&self) -> String {
        self.git(&["rev-parse", "--abbrev-ref", "HEAD"])
    }

    fn tip(&self, b: &str) -> String {
        self.git(&["rev-parse", &format!("refs/heads/{b}")])
    }

    fn staged_names(&self) -> Vec<String> {
        self.git(&["diff", "--cached", "--name-only"]).lines().map(str::to_string).collect()
    }

    fn status(&self) -> Status {
        status::get_status(self.p()).unwrap()
    }
}

fn lines(n: std::ops::Range<u32>) -> String {
    n.map(|i| format!("line {i}\n")).collect()
}

// ---------------------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------------------

#[test]
fn log_of_empty_repo_is_empty() {
    let r = Repo::new();
    let log = log::get_log(r.p(), None).unwrap();
    assert!(log.commits.is_empty());
    assert!(!log.truncated);
    let refs = refs::get_refs(r.p()).unwrap();
    assert!(refs.head.unborn);
    assert_eq!(refs.head.branch.as_deref(), Some("main"));
    assert_eq!(refs.head.commit, None);
    let info = repo::open_repo(r.p().to_str().unwrap()).unwrap();
    assert!(info.is_empty);
    assert_eq!(info.name, "repo");
    let st = r.status();
    assert!(st.head.unborn);
    assert_eq!(st.state, RepoState::Clean);
}

#[test]
fn log_parses_merges_and_multiline_bodies() {
    let r = Repo::new();
    let base = r.commit_file("a.txt", "a\n", "Base commit");
    r.git(&["switch", "-q", "-c", "feat"]);
    r.write("b.txt", "b\n");
    r.git(&["add", "b.txt"]);
    r.git(&["commit", "-q", "-m", "Feature work", "-m", "Body line one\nBody line two"]);
    let feat = r.head();
    r.git(&["switch", "-q", "main"]);
    let main2 = r.commit_file("c.txt", "c\n", "Main work");
    r.git(&["merge", "-q", "--no-ff", "--no-edit", "feat"]);
    let merge = r.head();
    r.git(&["tag", "v1"]);

    let log = log::get_log(r.p(), None).unwrap();
    assert_eq!(log.commits.len(), 4);
    let m = &log.commits[0];
    assert_eq!(m.id, merge);
    assert_eq!(m.parents, vec![main2.clone(), feat.clone()]);
    assert_eq!(m.short_id.len(), 7);
    assert_eq!(m.author_name, "Test User");
    assert_eq!(m.author_email, "test@example.com");
    assert!(m.author_time > 1_600_000_000);
    let f = log.commits.iter().find(|c| c.id == feat).unwrap();
    assert_eq!(f.summary, "Feature work");
    assert_eq!(f.body, "Body line one\nBody line two");
    assert_eq!(log.commits.last().unwrap().id, base);
    assert!(log.commits.last().unwrap().parents.is_empty());

    let limited = log::get_log(r.p(), Some(2)).unwrap();
    assert_eq!(limited.commits.len(), 2);
    assert!(limited.truncated);

    let refs = refs::get_refs(r.p()).unwrap();
    assert_eq!(refs.branches.len(), 2);
    assert!(refs.branches.iter().any(|b| b.name == "main" && b.is_head));
    assert_eq!(refs.tags.len(), 1);
    assert_eq!(refs.tags[0].commit, merge);
}

#[test]
fn log_ignores_stash_and_undo_refs() {
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "one");
    r.write("a.txt", "changed\n");
    ops::stash_push(r.p(), Some("wip"), false).unwrap();
    let log = log::get_log(r.p(), None).unwrap();
    assert_eq!(log.commits.len(), 1, "stash commits and refs/gitcove must not appear in the log");
    let refs = refs::get_refs(r.p()).unwrap();
    assert_eq!(refs.stashes.len(), 1);
    assert_eq!(refs.stashes[0].message, "wip");
    assert_eq!(refs.stashes[0].ref_, "stash@{0}");
    assert_eq!(refs.stashes[0].branch.as_deref(), Some("main"));
}

#[test]
fn status_reports_rename_untracked_mixed_and_conflict() {
    let r = Repo::new();
    r.commit_file("old.txt", &lines(0..20), "base");
    r.commit_file("both.txt", "one\n", "both");
    r.git(&["mv", "old.txt", "new.txt"]);
    r.write("both.txt", "two\n");
    r.git(&["add", "both.txt"]);
    r.write("both.txt", "three\n");
    r.write("dir/untracked.txt", "u\n");

    let st = r.status();
    assert_eq!(st.head.branch.as_deref(), Some("main"));
    assert!(!st.head.detached);
    let renamed = st.files.iter().find(|f| f.path == "new.txt").unwrap();
    assert_eq!(renamed.staged, Some(ChangeKind::Renamed));
    assert_eq!(renamed.orig_path.as_deref(), Some("old.txt"));
    assert_eq!(renamed.unstaged, None);
    let both = st.files.iter().find(|f| f.path == "both.txt").unwrap();
    assert_eq!(both.staged, Some(ChangeKind::Modified));
    assert_eq!(both.unstaged, Some(ChangeKind::Modified));
    let u = st.files.iter().find(|f| f.path == "dir/untracked.txt").unwrap();
    assert_eq!(u.unstaged, Some(ChangeKind::Untracked));
    assert_eq!(u.staged, None);

    // Conflict
    let r = Repo::new();
    r.commit_file("f.txt", "base\n", "base");
    r.git(&["switch", "-q", "-c", "other"]);
    r.commit_file("f.txt", "theirs\n", "theirs");
    r.git(&["switch", "-q", "main"]);
    r.commit_file("f.txt", "ours\n", "ours");
    let out = r.git_out(&["merge", "other"]);
    assert!(!out.status.success());
    let st = r.status();
    assert_eq!(st.state, RepoState::Merging);
    let f = st.files.iter().find(|f| f.path == "f.txt").unwrap();
    assert_eq!(f.conflict, Some(ConflictKind::BothModified));
    assert_eq!(f.staged, None);
    assert_eq!(f.unstaged, Some(ChangeKind::Conflicted));
    let op = st.operation.unwrap();
    assert_eq!(op.kind, RepoState::Merging);
    assert_eq!(op.onto.as_deref(), Some("other"));
    assert_eq!(op.current_summary.as_deref(), Some("theirs"));
}

#[test]
fn diff_parses_hunks_binary_and_no_newline() {
    let r = Repo::new();
    r.commit_file("t.txt", "a\nb\nc", "base");
    r.write_bytes("bin.dat", &[0u8, 1, 2, 3, 0, 255]);
    r.git(&["add", "bin.dat"]);
    r.git(&["commit", "-q", "-m", "bin"]);
    r.write("t.txt", "a\nB\nc\nd\n");
    r.write_bytes("bin.dat", &[0u8, 9, 9, 9, 0, 255]);

    let d = diff::get_working_diff(r.p(), "t.txt", false).unwrap();
    assert_eq!(d.status, ChangeKind::Modified);
    assert!(!d.binary);
    assert_eq!(d.hunks.len(), 1);
    let h = &d.hunks[0];
    assert_eq!((h.old_start, h.old_lines, h.new_start, h.new_lines), (1, 3, 1, 4));
    assert!(h.header.starts_with("@@ -1,3 +1,4 @@"));
    let kinds: Vec<DiffLineKind> = h.lines.iter().map(|l| l.kind).collect();
    assert!(kinds.contains(&DiffLineKind::NoNewline));
    let b_del = h.lines.iter().find(|l| l.kind == DiffLineKind::Del && l.text == "b").unwrap();
    assert_eq!(b_del.old_no, Some(2));
    assert_eq!(b_del.new_no, None);
    let b_add = h.lines.iter().find(|l| l.kind == DiffLineKind::Add && l.text == "B").unwrap();
    assert_eq!(b_add.new_no, Some(2));
    let ctx = h.lines.iter().find(|l| l.kind == DiffLineKind::Context && l.text == "a").unwrap();
    assert_eq!((ctx.old_no, ctx.new_no), (Some(1), Some(1)));
    assert_eq!(d.additions, 3);
    assert_eq!(d.deletions, 2);

    let b = diff::get_working_diff(r.p(), "bin.dat", false).unwrap();
    assert!(b.binary);
    assert!(b.hunks.is_empty());

    r.write("new.txt", "x\ny\n");
    let u = diff::get_working_diff(r.p(), "new.txt", false).unwrap();
    assert_eq!(u.status, ChangeKind::Untracked);
    assert_eq!(u.additions, 2);
    assert_eq!(u.hunks[0].lines[0].new_no, Some(1));

    // Staged diff and commit details/diff
    r.git(&["add", "t.txt", "bin.dat"]);
    let s = diff::get_working_diff(r.p(), "t.txt", true).unwrap();
    assert_eq!(s.additions, 3);
    let unstaged = diff::get_working_diff(r.p(), "t.txt", false).unwrap();
    assert!(unstaged.hunks.is_empty());
    r.git(&["commit", "-q", "-m", "change"]);
    let head = r.head();
    let details = diff::get_commit_details(r.p(), &head).unwrap();
    assert_eq!(details.commit.summary, "change");
    let bf = details.files.iter().find(|f| f.path == "bin.dat").unwrap();
    assert!(bf.binary);
    assert_eq!((bf.additions, bf.deletions), (0, 0));
    let tf = details.files.iter().find(|f| f.path == "t.txt").unwrap();
    assert_eq!((tf.additions, tf.deletions, tf.status), (3, 2, ChangeKind::Modified));
    let cd = diff::get_commit_diff(r.p(), &head, "t.txt", None).unwrap();
    assert_eq!(cd.hunks.len(), 1);

    // Root commit diff
    let root = r.git(&["rev-list", "--max-parents=0", "HEAD"]);
    let rd = diff::get_commit_details(r.p(), &root).unwrap();
    assert_eq!(rd.files.len(), 1);
    assert_eq!(rd.files[0].status, ChangeKind::Added);
    let rdd = diff::get_commit_diff(r.p(), &root, "t.txt", None).unwrap();
    assert_eq!(rdd.status, ChangeKind::Added);
    assert_eq!(rdd.additions, 3);
}

#[test]
fn diff_parser_handles_rename_and_too_large() {
    let r = Repo::new();
    r.commit_file("a.txt", &lines(0..50), "base");
    r.git(&["mv", "a.txt", "b.txt"]);
    let d = diff::get_working_diff(r.p(), "b.txt", true).unwrap();
    assert_eq!(d.status, ChangeKind::Renamed);
    assert_eq!(d.old_path.as_deref(), Some("a.txt"));
    r.git(&["commit", "-q", "-m", "rename"]);
    let det = diff::get_commit_details(r.p(), &r.head()).unwrap();
    assert_eq!(det.files.len(), 1);
    assert_eq!(det.files[0].status, ChangeKind::Renamed);
    assert_eq!(det.files[0].old_path.as_deref(), Some("a.txt"));

    r.write("big.txt", &lines(0..25_000));
    let big = diff::get_working_diff(r.p(), "big.txt", false).unwrap();
    assert!(big.too_large);
    assert!(big.hunks.is_empty());
    assert_eq!(big.additions, 25_000);
}

// ---------------------------------------------------------------------------------------
// Staging, commit, undo/redo
// ---------------------------------------------------------------------------------------

#[test]
fn apply_patch_stages_one_hunk_and_undoes() {
    let r = Repo::new();
    r.commit_file("f.txt", &lines(0..30), "base");
    let mut changed: Vec<String> = (0..30).map(|i| format!("line {i}")).collect();
    changed[1] = "FIRST CHANGE".into();
    changed[25] = "SECOND CHANGE".into();
    r.write("f.txt", &(changed.join("\n") + "\n"));

    let d = diff::get_working_diff(r.p(), "f.txt", false).unwrap();
    assert_eq!(d.hunks.len(), 2);
    let h = &d.hunks[0];
    let mut patch = String::from("diff --git a/f.txt b/f.txt\n--- a/f.txt\n+++ b/f.txt\n");
    patch.push_str(&h.header);
    patch.push('\n');
    for l in &h.lines {
        let c = match l.kind {
            DiffLineKind::Context => ' ',
            DiffLineKind::Add => '+',
            DiffLineKind::Del => '-',
            DiffLineKind::NoNewline => '\\',
        };
        patch.push(c);
        patch.push_str(&l.text);
        patch.push('\n');
    }
    let res = ops::apply_patch(r.p(), &patch, true, false, "Stage 2 lines in f.txt").unwrap();
    assert_eq!(res.message, "Staged 2 lines in f.txt");
    let entry = res.entry.unwrap();
    assert_eq!(entry.kind, "applyPatch");
    let cached = r.git(&["diff", "--cached"]);
    assert!(cached.contains("FIRST CHANGE"));
    assert!(!cached.contains("SECOND CHANGE"));
    let unstaged = r.git(&["diff"]);
    assert!(unstaged.contains("SECOND CHANGE"));

    oplog::undo(r.p(), None).unwrap();
    assert!(r.staged_names().is_empty());
    assert!(r.read("f.txt").contains("SECOND CHANGE"), "worktree untouched by index undo");
    oplog::redo(r.p()).unwrap();
    assert!(r.git(&["diff", "--cached"]).contains("FIRST CHANGE"));

    // Discard the second hunk from the worktree, then undo restores it.
    let d = diff::get_working_diff(r.p(), "f.txt", false).unwrap();
    assert_eq!(d.hunks.len(), 1);
    let h = &d.hunks[0];
    let mut p2 = String::from("--- a/f.txt\n+++ b/f.txt\n");
    p2.push_str(&h.header);
    p2.push('\n');
    for l in &h.lines {
        let c = match l.kind {
            DiffLineKind::Context => ' ',
            DiffLineKind::Add => '+',
            DiffLineKind::Del => '-',
            DiffLineKind::NoNewline => '\\',
        };
        p2.push(c);
        p2.push_str(&l.text);
        p2.push('\n');
    }
    ops::apply_patch(r.p(), &p2, false, true, "Discard 2 lines in f.txt").unwrap();
    assert!(!r.read("f.txt").contains("SECOND CHANGE"));
    oplog::undo(r.p(), None).unwrap();
    assert!(r.read("f.txt").contains("SECOND CHANGE"));
}

#[test]
fn stage_unstage_and_undo() {
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "base");
    r.write("a.txt", "a2\n");
    r.write("new/b.txt", "b\n");
    let res = ops::stage(r.p(), &["a.txt".into(), "new/b.txt".into()]).unwrap();
    assert_eq!(res.entry.as_ref().unwrap().label, "Stage 2 files");
    assert_eq!(r.staged_names(), vec!["a.txt", "new/b.txt"]);
    let res = ops::unstage(r.p(), &["a.txt".into()]).unwrap();
    assert_eq!(res.message, "Unstaged a.txt");
    assert_eq!(r.staged_names(), vec!["new/b.txt"]);
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.staged_names(), vec!["a.txt", "new/b.txt"]);
    oplog::undo(r.p(), None).unwrap();
    assert!(r.staged_names().is_empty());
    assert!(r.exists("new/b.txt"));
    let log = oplog::get_oplog(r.p()).unwrap();
    assert!(log.iter().all(|e| e.undone));
}

#[test]
fn commit_undo_redo() {
    let r = Repo::new();
    let base = r.commit_file("a.txt", "a\n", "base");
    r.write("a.txt", "b\n");
    r.write("c.txt", "c\n");
    ops::stage(r.p(), &["a.txt".into(), "c.txt".into()]).unwrap();
    let res = ops::commit(r.p(), "Fix lane colors\n\nLonger body", false).unwrap();
    assert_eq!(res.message, "Committed 2 files to main");
    let entry = res.entry.unwrap();
    assert_eq!(entry.label, "Commit “Fix lane colors”");
    assert!(entry.undoable);
    let new = r.head();
    assert_ne!(new, base);
    assert_eq!(r.git(&["log", "-1", "--format=%B"]), "Fix lane colors\n\nLonger body");

    let u = oplog::undo(r.p(), None).unwrap();
    assert!(u.message.starts_with("Undid Commit"));
    assert_eq!(r.head(), base);
    assert_eq!(r.staged_names(), vec!["a.txt", "c.txt"], "changes come back staged");
    let log = oplog::get_oplog(r.p()).unwrap();
    assert!(log[0].undone);

    oplog::redo(r.p()).unwrap();
    assert_eq!(r.head(), new);
    assert!(r.staged_names().is_empty());
    let log = oplog::get_oplog(r.p()).unwrap();
    assert!(!log[0].undone);

    // Redo is refused after a new change.
    oplog::undo(r.p(), None).unwrap(); // undo commit
    ops::commit(r.p(), "Another", false).unwrap();
    let e = oplog::redo(r.p()).unwrap_err();
    assert_eq!(e.message, "Redo isn't available after a new change.");

    // Nothing to commit.
    let e = ops::commit(r.p(), "empty", false).unwrap_err();
    assert_eq!(e.kind, GitErrorKind::NothingToCommit);

    // Amend and undo the amend.
    let before_amend = r.head();
    r.write("d.txt", "d\n");
    ops::stage(r.p(), &["d.txt".into()]).unwrap();
    let res = ops::commit(r.p(), "Another, amended", true).unwrap();
    assert_eq!(res.entry.unwrap().kind, "amend");
    assert_ne!(r.head(), before_amend);
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.head(), before_amend);
    assert_eq!(r.staged_names(), vec!["d.txt"]);
}

#[test]
fn first_commit_on_unborn_branch_undo() {
    let r = Repo::new();
    r.write("a.txt", "a\n");
    ops::stage(r.p(), &["a.txt".into()]).unwrap();
    ops::commit(r.p(), "Initial", false).unwrap();
    assert!(!r.head().is_empty());
    oplog::undo(r.p(), None).unwrap();
    assert!(r.git_out(&["rev-parse", "--verify", "-q", "HEAD"]).status.code() != Some(0));
    assert_eq!(r.staged_names(), vec!["a.txt"]);
    oplog::redo(r.p()).unwrap();
    assert_eq!(r.git(&["log", "-1", "--format=%s"]), "Initial");
}

#[test]
fn checkout_and_undo() {
    let r = Repo::new();
    let base = r.commit_file("a.txt", "a\n", "base");
    r.git(&["branch", "feat/undo-log"]);
    let res = ops::checkout(r.p(), "feat/undo-log").unwrap();
    assert_eq!(res.message, "Switched to feat/undo-log");
    assert_eq!(res.entry.unwrap().label, "Switch to feat/undo-log");
    assert_eq!(r.branch(), "feat/undo-log");
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.branch(), "main");
    oplog::redo(r.p()).unwrap();
    assert_eq!(r.branch(), "feat/undo-log");

    // Detached checkout and undo.
    ops::checkout(r.p(), &base[..10]).unwrap();
    assert_eq!(r.branch(), "HEAD");
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.branch(), "feat/undo-log");

    // Dirty worktree conflict maps to dirtyWorktree.
    r.git(&["switch", "-q", "main"]);
    r.commit_file("a.txt", "main change\n", "main change");
    r.write("a.txt", "local edit\n");
    let e = ops::checkout(r.p(), "feat/undo-log").unwrap_err();
    assert_eq!(e.kind, GitErrorKind::DirtyWorktree);
}

#[test]
fn checkout_remote_branch_creates_tracking_branch() {
    let upstream = Repo::new();
    upstream.commit_file("a.txt", "a\n", "base");
    upstream.git(&["branch", "feature"]);
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("clone");
    let info = repo::clone_repo(upstream.p().to_str().unwrap(), dest.to_str().unwrap(), "op1", None).unwrap();
    let clone = Path::new(&info.path);
    let res = ops::checkout(clone, "origin/feature").unwrap();
    assert_eq!(res.message, "Switched to feature");
    let refs = refs::get_refs(clone).unwrap();
    let b = refs.branches.iter().find(|b| b.name == "feature").unwrap();
    assert_eq!(b.upstream.as_deref(), Some("origin/feature"));
    assert!(refs.remote_branches.iter().any(|rb| rb.full_name == "origin/feature" && rb.remote == "origin" && rb.name == "feature"));
    assert!(refs.remotes.iter().any(|r| r.name == "origin" && r.host == HostKind::Other));
    oplog::undo(clone, None).unwrap();
    let refs = refs::get_refs(clone).unwrap();
    assert!(!refs.branches.iter().any(|b| b.name == "feature"), "undo removes the created tracking branch");
    let oplog = oplog::get_oplog(clone).unwrap();
    assert!(oplog.iter().any(|e| e.kind == "clone" && !e.undoable && e.reason.is_some()));
}

#[test]
fn create_rename_and_delete_branch_undo() {
    let r = Repo::new();
    let base = r.commit_file("a.txt", "a\n", "base");
    ops::create_branch(r.p(), "topic", "HEAD", true).unwrap();
    assert_eq!(r.branch(), "topic");
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.branch(), "main");
    assert!(r.git_out(&["rev-parse", "--verify", "-q", "refs/heads/topic"]).status.code() != Some(0));
    oplog::redo(r.p()).unwrap();
    assert_eq!(r.branch(), "topic");
    r.git(&["switch", "-q", "main"]);

    ops::rename_branch(r.p(), "topic", "topic2").unwrap();
    assert_eq!(r.tip("topic2"), base);
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.tip("topic"), base);
}

#[test]
fn delete_branch_and_undo() {
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "base");
    r.git(&["switch", "-q", "-c", "feat/old"]);
    let tip = r.commit_file("b.txt", "b\n", "unmerged work");
    r.git(&["config", "branch.feat/old.remote", "origin"]);
    r.git(&["config", "branch.feat/old.merge", "refs/heads/feat/old"]);
    r.git(&["switch", "-q", "main"]);

    let p = preview::preview_operation(r.p(), Operation::DeleteBranch { name: "feat/old".into(), force: false }).unwrap();
    assert_eq!(p.title, "Delete branch feat/old");
    assert_eq!(p.replaced, vec![tip.clone()]);
    assert_eq!(p.moves[0].to, None);
    assert!(p.notes.iter().any(|n| n.contains("needs force")));

    let e = operation::run_operation(r.p(), Operation::DeleteBranch { name: "feat/old".into(), force: false }, "x", None)
        .unwrap_err();
    assert!(e.message.contains("aren't merged"));
    let res = operation::run_operation(r.p(), Operation::DeleteBranch { name: "feat/old".into(), force: true }, "x", None)
        .unwrap();
    assert_eq!(res.message, "Deleted feat/old");
    assert!(r.git_out(&["rev-parse", "--verify", "-q", "refs/heads/feat/old"]).status.code() != Some(0));
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.tip("feat/old"), tip);
    assert_eq!(r.git(&["config", "branch.feat/old.remote"]), "origin");
    oplog::redo(r.p()).unwrap();
    assert!(r.git_out(&["rev-parse", "--verify", "-q", "refs/heads/feat/old"]).status.code() != Some(0));
}

// ---------------------------------------------------------------------------------------
// Rebase / reset / discard / stash
// ---------------------------------------------------------------------------------------

/// main: base → m1 (edits shared.txt); feat: base → f1 (new file) → f2 (edits shared.txt) → f3 (new file)
fn rebase_fixture(conflict: bool) -> (Repo, Vec<String>) {
    let r = Repo::new();
    r.commit_file("shared.txt", "one\ntwo\nthree\n", "base");
    r.git(&["switch", "-q", "-c", "feat/undo-log"]);
    let f1 = r.commit_file("f1.txt", "f1\n", "Feature one");
    let f2 = if conflict {
        r.commit_file("shared.txt", "one\nFEATURE\nthree\n", "Feature edits shared")
    } else {
        r.commit_file("f2.txt", "f2\n", "Feature two")
    };
    let f3 = r.commit_file("f3.txt", "f3\n", "Feature three");
    r.git(&["switch", "-q", "main"]);
    r.commit_file("shared.txt", "one\nMAIN\nthree\n", "Main edits shared");
    (r, vec![f1, f2, f3])
}

#[test]
fn rebase_preview_predicts_conflict_then_run_stops_and_undo_restores() {
    let (r, feat) = rebase_fixture(true);
    let main_before = r.tip("main");
    let op = Operation::Rebase { branch: "feat/undo-log".into(), onto: "main".into() };
    let p = preview::preview_operation(r.p(), op.clone()).unwrap();
    assert_eq!(p.title, "Rebase feat/undo-log onto main");
    assert_eq!(p.created.len(), 3);
    assert_eq!(p.replaced.len(), 3);
    for c in &feat {
        assert!(p.replaced.contains(c));
    }
    assert!(p.conflicts.predicted);
    assert_eq!(p.conflicts.files, vec!["shared.txt".to_string()]);
    // Newest first, each replaces an original, chained parents.
    assert_eq!(p.created[0].replaces.as_deref(), Some(feat[2].as_str()));
    assert_eq!(p.created[2].replaces.as_deref(), Some(feat[0].as_str()));
    assert_eq!(p.created[2].parents, vec![main_before.clone()]);
    assert_eq!(p.created[1].parents, vec![p.created[2].id.clone()]);
    assert!(p.created[1].conflicted);
    assert!(!p.created[2].conflicted);
    assert_eq!(p.moves[0].ref_, "feat/undo-log");
    assert_eq!(p.moves[0].to.as_deref(), Some(p.created[0].id.as_str()));
    // Preview changed nothing.
    assert_eq!(r.tip("feat/undo-log"), feat[2]);
    assert_eq!(r.branch(), "main");
    assert!(r.status().files.is_empty());

    let res = operation::run_operation(r.p(), op, "op", None).unwrap();
    assert_eq!(res.message, "Rebase stopped on conflicts in 1 file");
    let st = r.status();
    assert_eq!(st.state, RepoState::Rebasing);
    let prog = st.operation.unwrap();
    assert_eq!(prog.branch.as_deref(), Some("feat/undo-log"));
    assert_eq!(prog.onto.as_deref(), Some("main"));
    assert_eq!(prog.step, Some(2));
    assert_eq!(prog.total, Some(3));
    assert_eq!(prog.current_summary.as_deref(), Some("Feature edits shared"));

    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.status().state, RepoState::Clean);
    assert_eq!(r.tip("feat/undo-log"), feat[2]);
    assert_eq!(r.branch(), "main", "undo returns HEAD to where it was");
    assert_eq!(r.tip("main"), main_before);
}

#[test]
fn rebase_run_continue_and_undo() {
    let (r, feat) = rebase_fixture(true);
    let op = Operation::Rebase { branch: "feat/undo-log".into(), onto: "main".into() };
    operation::run_operation(r.p(), op, "op", None).unwrap();
    // Resolve with ours (main's side during a rebase) and continue.
    // During a rebase git's "theirs" is the user's own commit, so keeping "mine" means theirs.
    let kept = ops::resolve_conflict(r.p(), "shared.txt", ConflictSide::Theirs).unwrap();
    assert_eq!(r.read("shared.txt"), "one\nFEATURE\nthree\n");
    assert_eq!(kept.entry.unwrap().label, "Keep your commit’s version of shared.txt");
    let res = ops::continue_operation(r.p()).unwrap();
    assert_eq!(res.message, "Finished the rebase of feat/undo-log");
    assert_eq!(r.status().state, RepoState::Clean);
    assert_ne!(r.tip("feat/undo-log"), feat[2]);
    assert_eq!(r.git(&["rev-list", "--count", "main..feat/undo-log"]), "3");
    // Undo everything back to the rebase entry.
    let log = oplog::get_oplog(r.p()).unwrap();
    let rebase_entry = log.iter().find(|e| e.kind == "rebase").unwrap().id.clone();
    oplog::undo(r.p(), Some(&rebase_entry)).unwrap();
    assert_eq!(r.tip("feat/undo-log"), feat[2]);
    assert_eq!(r.branch(), "main");
}

#[test]
fn clean_rebase_run_and_undo() {
    let (r, feat) = rebase_fixture(false);
    r.git(&["switch", "-q", "feat/undo-log"]);
    let op = Operation::Rebase { branch: "feat/undo-log".into(), onto: "main".into() };
    let p = preview::preview_operation(r.p(), op.clone()).unwrap();
    assert!(p.conflicts.files.is_empty());
    assert_eq!(p.created.len(), 3);
    let res = operation::run_operation(r.p(), op, "op", None).unwrap();
    assert_eq!(res.message, "Rebased feat/undo-log onto main");
    assert_eq!(res.entry.as_ref().unwrap().label, "Rebase feat/undo-log onto main");
    // The real rebase produces the same trees as the simulation.
    let sim_tree = r.git(&["rev-parse", &format!("{}^{{tree}}", p.created[0].id)]);
    let real_tree = r.git(&["rev-parse", "HEAD^{tree}"]);
    assert_eq!(sim_tree, real_tree);
    let rebased = r.tip("feat/undo-log");
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.tip("feat/undo-log"), feat[2]);
    assert_eq!(r.branch(), "feat/undo-log");
    oplog::redo(r.p()).unwrap();
    assert_eq!(r.tip("feat/undo-log"), rebased);
}

#[test]
fn rebase_preview_skips_commits_already_upstream() {
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "base");
    r.git(&["switch", "-q", "-c", "feat"]);
    let picked = r.commit_file("b.txt", "b\n", "Shared change");
    r.commit_file("c.txt", "c\n", "Feature only");
    r.git(&["switch", "-q", "main"]);
    r.commit_file("m.txt", "m\n", "Main moves on");
    r.git(&["cherry-pick", picked.as_str()]);
    let p = preview::preview_operation(r.p(), Operation::Rebase { branch: "feat".into(), onto: "main".into() }).unwrap();
    assert_eq!(p.created.len(), 1);
    assert_eq!(p.replaced.len(), 2);
    assert!(p.notes.iter().any(|n| n.contains("1 commit is already on main and will be skipped")));
}

#[test]
fn reset_hard_with_dirty_files_then_undo_restores_them() {
    let r = Repo::new();
    let base = r.commit_file("a.txt", "a1\n", "one");
    let second = r.commit_file("a.txt", "a2\n", "two");
    r.write("a.txt", "dirty edit\n");
    r.write("staged-new.txt", "new\n");
    r.git(&["add", "staged-new.txt"]);
    r.write("untracked.txt", "keep me\n");

    let op = Operation::Reset { target: base.clone(), mode: ResetMode::Hard };
    let p = preview::preview_operation(r.p(), op.clone()).unwrap();
    assert_eq!(p.title, format!("Reset main to {} (hard)", &base[..7]));
    assert_eq!(p.replaced, vec![second.clone()]);
    assert_eq!(p.discarded_files, vec!["a.txt".to_string(), "staged-new.txt".to_string()]);

    operation::run_operation(r.p(), op, "op", None).unwrap();
    assert_eq!(r.head(), base);
    assert_eq!(r.read("a.txt"), "a1\n");
    assert!(!r.exists("staged-new.txt"));
    assert_eq!(r.read("untracked.txt"), "keep me\n");

    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.head(), second);
    assert_eq!(r.read("a.txt"), "dirty edit\n");
    assert_eq!(r.read("staged-new.txt"), "new\n");
    assert_eq!(r.staged_names(), vec!["staged-new.txt"]);

    oplog::redo(r.p()).unwrap();
    assert_eq!(r.head(), base);
    assert_eq!(r.read("a.txt"), "a1\n");
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.read("a.txt"), "dirty edit\n");
}

#[test]
fn reset_mixed_and_soft_undo() {
    let r = Repo::new();
    let base = r.commit_file("a.txt", "a1\n", "one");
    let second = r.commit_file("a.txt", "a2\n", "two");
    r.write("b.txt", "b\n");
    r.git(&["add", "b.txt"]);
    operation::run_operation(r.p(), Operation::Reset { target: base.clone(), mode: ResetMode::Mixed }, "x", None).unwrap();
    assert_eq!(r.head(), base);
    assert!(r.staged_names().is_empty());
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.head(), second);
    assert_eq!(r.staged_names(), vec!["b.txt"]);

    operation::run_operation(r.p(), Operation::Reset { target: base.clone(), mode: ResetMode::Soft }, "x", None).unwrap();
    assert_eq!(r.staged_names(), vec!["a.txt", "b.txt"]);
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.head(), second);
    assert_eq!(r.staged_names(), vec!["b.txt"]);
}

#[test]
fn discard_tracked_and_untracked_then_undo_restores_both() {
    let r = Repo::new();
    r.commit_file("tracked.txt", "original\n", "base");
    r.commit_file("staged.txt", "s1\n", "base2");
    r.write("tracked.txt", "my edits\n");
    r.write("staged.txt", "s2\n");
    r.git(&["add", "staged.txt"]);
    r.write("sub/untracked.txt", "precious\n");

    let paths: Vec<String> = vec!["tracked.txt".into(), "staged.txt".into(), "sub/untracked.txt".into()];
    let p = preview::preview_operation(r.p(), Operation::Discard { paths: paths.clone() }).unwrap();
    assert_eq!(p.title, "Discard changes in 3 files");
    assert_eq!(p.discarded_files.len(), 3);
    assert!(p.notes.contains(&"You can undo this.".to_string()));

    let res = operation::run_operation(r.p(), Operation::Discard { paths }, "op", None).unwrap();
    assert_eq!(res.message, "Discarded changes in 3 files");
    assert_eq!(res.entry.as_ref().unwrap().label, "Discard changes in 3 files");
    assert_eq!(r.read("tracked.txt"), "original\n");
    assert_eq!(r.read("staged.txt"), "s1\n");
    assert!(!r.exists("sub/untracked.txt"));
    assert!(!r.exists("sub"), "empty folders are cleaned up");
    assert!(r.status().files.is_empty());

    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.read("tracked.txt"), "my edits\n");
    assert_eq!(r.read("staged.txt"), "s2\n");
    assert_eq!(r.read("sub/untracked.txt"), "precious\n");
    assert_eq!(r.staged_names(), vec!["staged.txt"]);

    oplog::redo(r.p()).unwrap();
    assert_eq!(r.read("tracked.txt"), "original\n");
    assert!(!r.exists("sub/untracked.txt"));
    oplog::undo(r.p(), None).unwrap();
    // Editing a restored file blocks a second undo of an older discard? Not applicable; edit then try redo.
    r.write("tracked.txt", "newer edits\n");
    let e = oplog::redo(r.p()).unwrap_err();
    assert!(e.message.contains("tracked.txt"));
}

#[test]
fn stash_push_and_undo() {
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "base");
    r.write("a.txt", "changed\n");
    r.write("u.txt", "untracked\n");
    let res = ops::stash_push(r.p(), Some("WIP lanes"), true).unwrap();
    assert_eq!(res.entry.as_ref().unwrap().label, "Stash “WIP lanes”");
    assert_eq!(r.read("a.txt"), "a\n");
    assert!(!r.exists("u.txt"));
    assert_eq!(refs::get_refs(r.p()).unwrap().stashes.len(), 1);
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.read("a.txt"), "changed\n");
    assert_eq!(r.read("u.txt"), "untracked\n");
    assert!(refs::get_refs(r.p()).unwrap().stashes.is_empty());
    oplog::redo(r.p()).unwrap();
    assert_eq!(refs::get_refs(r.p()).unwrap().stashes.len(), 1);
    assert_eq!(r.read("a.txt"), "a\n");

    let e = ops::stash_push(r.p(), None, false).unwrap_err();
    assert_eq!(e.kind, GitErrorKind::NothingToCommit);

    // Pop, then undo brings the stash entry back (changes stay).
    let res = ops::stash_apply(r.p(), 0, true).unwrap();
    assert!(res.message.contains("Applied and removed"));
    assert!(refs::get_refs(r.p()).unwrap().stashes.is_empty());
    let u = oplog::undo(r.p(), None).unwrap();
    assert!(u.message.contains("stay in your files"));
    assert_eq!(refs::get_refs(r.p()).unwrap().stashes.len(), 1);
    assert_eq!(r.read("a.txt"), "changed\n");

    // Drop + undo.
    r.git(&["checkout", "--", "a.txt"]);
    std::fs::remove_file(r.p().join("u.txt")).unwrap();
    let p = preview::preview_operation(r.p(), Operation::DropStash { index: 0 }).unwrap();
    assert!(p.discarded_files.contains(&"a.txt".to_string()));
    assert!(p.discarded_files.contains(&"u.txt".to_string()));
    operation::run_operation(r.p(), Operation::DropStash { index: 0 }, "x", None).unwrap();
    assert!(refs::get_refs(r.p()).unwrap().stashes.is_empty());
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(refs::get_refs(r.p()).unwrap().stashes.len(), 1);

    // Apply (no pop) + undo reverts the files.
    ops::stash_apply(r.p(), 0, false).unwrap();
    assert_eq!(r.read("a.txt"), "changed\n");
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.read("a.txt"), "a\n");
    assert!(!r.exists("u.txt"));
}

#[test]
fn merge_preview_and_run_with_undo() {
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "base");
    r.git(&["switch", "-q", "-c", "feat"]);
    let f = r.commit_file("b.txt", "b\n", "feat");
    r.git(&["switch", "-q", "main"]);
    // Fast-forward preview
    let p = preview::preview_operation(r.p(), Operation::Merge { branch: "feat".into() }).unwrap();
    assert!(p.created.is_empty());
    assert!(p.notes.contains(&"Fast-forward: no merge commit".to_string()));
    assert_eq!(p.moves[0].to.as_deref(), Some(f.as_str()));
    // True merge
    let m1 = r.commit_file("c.txt", "c\n", "main work");
    let p = preview::preview_operation(r.p(), Operation::Merge { branch: "feat".into() }).unwrap();
    assert_eq!(p.created.len(), 1);
    assert_eq!(p.created[0].parents, vec![m1.clone(), f.clone()]);
    assert_eq!(p.created[0].summary, "Merge branch 'feat'");
    assert!(p.replaced.is_empty());
    let res = operation::run_operation(r.p(), Operation::Merge { branch: "feat".into() }, "x", None).unwrap();
    assert_eq!(res.message, "Merged feat into main");
    assert_eq!(r.git(&["rev-parse", "HEAD^{tree}"]), r.git(&["rev-parse", &format!("{}^{{tree}}", p.created[0].id)]));
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.head(), m1);
}

#[test]
fn cherry_pick_and_revert_preview_and_undo() {
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "base");
    r.git(&["switch", "-q", "-c", "feat"]);
    let f = r.commit_file("b.txt", "b\n", "Add b");
    r.git(&["switch", "-q", "main"]);
    let head = r.head();
    let p = preview::preview_operation(r.p(), Operation::CherryPick { commit: f.clone() }).unwrap();
    assert_eq!(p.created.len(), 1);
    assert_eq!(p.created[0].replaces.as_deref(), Some(f.as_str()));
    assert_eq!(p.created[0].summary, "Add b");
    operation::run_operation(r.p(), Operation::CherryPick { commit: f.clone() }, "x", None).unwrap();
    assert!(r.exists("b.txt"));
    let picked = r.head();
    let p = preview::preview_operation(r.p(), Operation::Revert { commit: picked.clone() }).unwrap();
    assert_eq!(p.created.len(), 1);
    assert_eq!(p.created[0].summary, "Revert \"Add b\"");
    operation::run_operation(r.p(), Operation::Revert { commit: picked.clone() }, "x", None).unwrap();
    assert!(!r.exists("b.txt"));
    oplog::undo(r.p(), None).unwrap();
    assert!(r.exists("b.txt"));
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.head(), head);
}

#[test]
fn fetch_pull_push_and_force_push_preview() {
    let upstream = Repo::new();
    upstream.commit_file("a.txt", "a\n", "base");
    upstream.git(&["config", "receive.denyCurrentBranch", "ignore"]);
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("c");
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let ev2 = events.clone();
    let emit = move |e: ProgressEvent| ev2.lock().unwrap().push(e);
    let info = repo::clone_repo(upstream.p().to_str().unwrap(), dest.to_str().unwrap(), "clone-1", Some(&emit)).unwrap();
    let c = Path::new(&info.path);
    Command::new("git").args(["config", "user.name", "C"]).current_dir(c).output().unwrap();
    Command::new("git").args(["config", "user.email", "c@example.com"]).current_dir(c).output().unwrap();
    assert!(events.lock().unwrap().iter().all(|e| e.op_id == "clone-1"));

    let up2 = upstream.commit_file("b.txt", "b\n", "upstream work");
    let res = ops::fetch(c, None, "f1", None).unwrap();
    assert_eq!(res.message, "Fetched all remotes");
    let entry = res.entry.unwrap();
    assert!(!entry.undoable);
    assert_eq!(entry.reason.as_deref(), Some("Fetching only updates remote branches; there's nothing to undo."));
    let st = status::get_status(c).unwrap();
    assert_eq!(st.upstream.as_deref(), Some("origin/main"));
    assert_eq!(st.behind, 1);

    let p = preview::preview_operation(c, Operation::Pull { mode: PullMode::FfOnly }).unwrap();
    assert_eq!(p.moves[0].to.as_deref(), Some(up2.as_str()));
    let before = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(c).output().unwrap();
    let before = String::from_utf8_lossy(&before.stdout).trim().to_string();
    operation::run_operation(c, Operation::Pull { mode: PullMode::FfOnly }, "p1", None).unwrap();
    assert!(c.join("b.txt").exists());
    oplog::undo(c, None).unwrap();
    assert!(!c.join("b.txt").exists());
    let now = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(c).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&now.stdout).trim(), before);

    // Local commit; force push preview shows the remote commit that would be lost.
    std::fs::write(c.join("local.txt"), "l\n").unwrap();
    ops::stage(c, &["local.txt".into()]).unwrap();
    ops::commit(c, "local", false).unwrap();
    let p = preview::preview_operation(c, Operation::ForcePush { remote: "origin".into(), branch: "main".into() }).unwrap();
    assert!(p.irreversible);
    assert_eq!(p.remote_commits_lost.len(), 1);
    assert_eq!(p.remote_commits_lost[0].id, up2);
    assert!(p.notes.iter().any(|n| n.contains("Based on what was last fetched from origin")));

    // Normal push is rejected (non-fast-forward).
    let e = ops::push(c, "origin", "main", false, "push1", None).unwrap_err();
    assert_eq!(e.kind, GitErrorKind::Rejected);
    // Pull with rebase then push.
    operation::run_operation(c, Operation::Pull { mode: PullMode::Rebase }, "p2", None).unwrap();
    let res = ops::push(c, "origin", "main", true, "push2", None).unwrap();
    assert_eq!(res.message, "Pushed main to origin");
    assert_eq!(res.entry.unwrap().reason.as_deref(), Some("Pushes change the remote and can't be undone here."));
}

#[test]
fn tags_and_remotes_undo() {
    let r = Repo::new();
    let base = r.commit_file("a.txt", "a\n", "base");
    ops::create_tag(r.p(), "v1", "HEAD", Some("Release one")).unwrap();
    let refs = refs::get_refs(r.p()).unwrap();
    assert!(refs.tags[0].annotated);
    assert_eq!(refs.tags[0].commit, base);
    oplog::undo(r.p(), None).unwrap();
    assert!(refs::get_refs(r.p()).unwrap().tags.is_empty());
    oplog::redo(r.p()).unwrap();
    ops::delete_tag(r.p(), "v1").unwrap();
    assert!(refs::get_refs(r.p()).unwrap().tags.is_empty());
    oplog::undo(r.p(), None).unwrap();
    let t = &refs::get_refs(r.p()).unwrap().tags[0];
    assert!(t.annotated);

    ops::add_remote(r.p(), "origin", "git@github.com:owner/repo.git").unwrap();
    let remotes = refs::get_refs(r.p()).unwrap().remotes;
    assert_eq!(remotes[0].host, HostKind::Github);
    assert_eq!(remotes[0].web_url.as_deref(), Some("https://github.com/owner/repo"));
    assert_eq!(remotes[0].owner.as_deref(), Some("owner"));
    oplog::undo(r.p(), None).unwrap();
    assert!(refs::get_refs(r.p()).unwrap().remotes.is_empty());
}

#[test]
fn undo_refuses_when_branch_moved_on() {
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "base");
    r.git(&["switch", "-q", "-c", "feat"]);
    r.commit_file("b.txt", "b\n", "feat work");
    r.git(&["switch", "-q", "main"]);
    r.commit_file("c.txt", "c\n", "main work");
    operation::run_operation(r.p(), Operation::Merge { branch: "feat".into() }, "x", None).unwrap();
    // Commit outside the app on top of the merge.
    r.commit_file("d.txt", "d\n", "outside");
    let e = oplog::undo(r.p(), None).unwrap_err();
    assert!(e.message.contains("main has moved since then"), "{}", e.message);
}

#[test]
fn oplog_keeps_objects_reachable_and_caps_entries() {
    let r = Repo::new();
    r.commit_file("a.txt", "0\n", "base");
    for i in 0..3 {
        r.write("a.txt", &format!("{i}x\n"));
        ops::stage(r.p(), &["a.txt".into()]).unwrap();
    }
    let undo_refs = r.git(&["for-each-ref", "--format=%(refname)", "refs/gitcove/undo"]);
    assert!(!undo_refs.is_empty());
    let log = oplog::get_oplog(r.p()).unwrap();
    assert_eq!(log.len(), 3);
    assert!(log[0].id != log[1].id);
    // Undo several at once (back to the oldest).
    let oldest = log[2].id.clone();
    let res = oplog::undo(r.p(), Some(&oldest)).unwrap();
    assert_eq!(res.message, "Undid 3 operations");
    assert!(r.staged_names().is_empty());
}

#[test]
fn errors_for_non_repos() {
    let tmp = tempfile::tempdir().unwrap();
    let e = repo::open_repo(tmp.path().to_str().unwrap()).unwrap_err();
    assert_eq!(e.kind, GitErrorKind::NotARepo);
    assert_eq!(e.message, "That folder isn't a Git repository.");
    let bare = tmp.path().join("bare.git");
    Command::new("git").args(["init", "-q", "--bare", bare.to_str().unwrap()]).output().unwrap();
    let e = repo::open_repo(bare.to_str().unwrap()).unwrap_err();
    assert_eq!(e.message, "Bare repositories aren't supported.");
    let v = repo::version_string().unwrap();
    assert!(v.split('.').count() == 3);
    let init = repo::init_repo(tmp.path().join("fresh").to_str().unwrap()).unwrap();
    assert!(init.is_empty);
}

// ---------------------------------------------------------------------------------------
// JSON contract shapes
// ---------------------------------------------------------------------------------------

#[test]
fn json_shapes_match_contract() {
    use serde_json::json;
    let op: Operation = serde_json::from_value(json!({"kind": "forcePush", "remote": "origin", "branch": "main"})).unwrap();
    assert!(matches!(op, Operation::ForcePush { .. }));
    let op: Operation = serde_json::from_value(json!({"kind": "pull", "mode": "ffOnly"})).unwrap();
    assert!(matches!(op, Operation::Pull { mode: PullMode::FfOnly }));
    let op: Operation = serde_json::from_value(json!({"kind": "deleteBranch", "name": "x", "force": true})).unwrap();
    assert_eq!(serde_json::to_value(&op).unwrap(), json!({"kind": "deleteBranch", "name": "x", "force": true}));
    let op: Operation = serde_json::from_value(json!({"kind": "dropStash", "index": 2})).unwrap();
    assert!(matches!(op, Operation::DropStash { index: 2 }));

    let e = gitcove_lib::git::GitError::new(GitErrorKind::NotARepo, "m", "d");
    assert_eq!(serde_json::to_value(&e).unwrap(), json!({"kind": "notARepo", "message": "m", "detail": "d"}));
    assert_eq!(serde_json::to_value(GitErrorKind::DirtyWorktree).unwrap(), json!("dirtyWorktree"));
    assert_eq!(serde_json::to_value(RepoState::CherryPicking).unwrap(), json!("cherryPicking"));
    assert_eq!(serde_json::to_value(ChangeKind::TypeChanged).unwrap(), json!("typeChanged"));
    assert_eq!(serde_json::to_value(DiffLineKind::NoNewline).unwrap(), json!("noNewline"));
    assert_eq!(serde_json::to_value(HostKind::Github).unwrap(), json!("github"));
    assert_eq!(serde_json::to_value(PullRequestListStatus::RateLimited).unwrap(), json!("rateLimited"));
    assert_eq!(serde_json::to_value(RepoChangeKind::Worktree).unwrap(), json!("worktree"));

    let s = Stash { index: 0, ref_: "stash@{0}".into(), commit: "c".into(), message: "m".into(), time: 1, branch: None };
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        json!({"index": 0, "ref": "stash@{0}", "commit": "c", "message": "m", "time": 1, "branch": null})
    );
    let m = RefMove { ref_: "main".into(), from: None, to: Some("x".into()) };
    assert_eq!(serde_json::to_value(&m).unwrap(), json!({"ref": "main", "from": null, "to": "x"}));
    let pe = ProgressEvent { op_id: "o".into(), phase: "p".into(), percent: None, line: "l".into() };
    assert_eq!(serde_json::to_value(&pe).unwrap(), json!({"opId": "o", "phase": "p", "percent": null, "line": "l"}));
    let fc = FileChange { path: "a".into(), orig_path: None, staged: None, unstaged: Some(ChangeKind::Untracked), conflict: None };
    assert_eq!(
        serde_json::to_value(&fc).unwrap(),
        json!({"path": "a", "origPath": null, "staged": null, "unstaged": "untracked", "conflict": null})
    );
    let side: ConflictSide = serde_json::from_value(json!("theirs")).unwrap();
    assert_eq!(side, ConflictSide::Theirs);
}

// ---------------------------------------------------------------------------------------
// Watcher
// ---------------------------------------------------------------------------------------

fn wait_for(events: &std::sync::Arc<std::sync::Mutex<Vec<RepoChangedEvent>>>, kind: RepoChangeKind) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(5) {
        if events.lock().unwrap().iter().any(|e| e.kind == kind) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

#[test]
fn watcher_reports_worktree_and_git_changes() {
    use gitcove_lib::watch;
    use std::sync::{Arc, Mutex};
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "base");
    r.write("src/deep/file.txt", "x\n");
    std::fs::create_dir_all(r.p().join("node_modules/pkg")).unwrap();
    let events: Arc<Mutex<Vec<RepoChangedEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let ev = events.clone();
    let sink: watch::ChangeSink = Arc::new(move |e: RepoChangedEvent| ev.lock().unwrap().push(e));
    let w = watch::start("the-key".into(), r.p(), sink).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(400));

    r.write("src/deep/file.txt", "changed\n");
    assert!(wait_for(&events, RepoChangeKind::Worktree), "worktree edit in a nested folder is reported");
    assert!(events.lock().unwrap().iter().all(|e| e.repo == "the-key"));
    std::thread::sleep(std::time::Duration::from_millis(400));
    events.lock().unwrap().clear();

    r.git(&["branch", "new-branch"]);
    assert!(wait_for(&events, RepoChangeKind::Git), "ref changes are reported as git");
    std::thread::sleep(std::time::Duration::from_millis(400));
    events.lock().unwrap().clear();

    r.write("node_modules/pkg/index.js", "x\n");
    std::thread::sleep(std::time::Duration::from_millis(900));
    assert!(events.lock().unwrap().is_empty(), "node_modules is ignored: {:?}", events.lock().unwrap());

    // New directories are picked up.
    r.write("newdir/inner/f.txt", "1\n");
    assert!(wait_for(&events, RepoChangeKind::Worktree));
    std::thread::sleep(std::time::Duration::from_millis(400));
    events.lock().unwrap().clear();
    r.write("newdir/inner/f.txt", "2\n");
    assert!(wait_for(&events, RepoChangeKind::Worktree), "files in newly created folders are watched");
    drop(w);
}

#[test]
fn resolve_conflict_undo_restores_conflict_stages() {
    let r = Repo::new();
    r.commit_file("f.txt", "base\n", "base");
    r.git(&["switch", "-q", "-c", "other"]);
    r.commit_file("f.txt", "theirs\n", "theirs");
    r.git(&["switch", "-q", "main"]);
    let main_tip = r.commit_file("f.txt", "ours\n", "ours");
    let p = preview::preview_operation(r.p(), Operation::Merge { branch: "other".into() }).unwrap();
    assert_eq!(p.conflicts.files, vec!["f.txt".to_string()]);
    assert!(p.created[0].conflicted);
    let res = operation::run_operation(r.p(), Operation::Merge { branch: "other".into() }, "m", None).unwrap();
    assert_eq!(res.message, "Merge stopped on conflicts in 1 file");
    let with_markers = r.read("f.txt");
    assert!(with_markers.contains("<<<<<<<"));

    ops::resolve_conflict(r.p(), "f.txt", ConflictSide::Ours).unwrap();
    assert_eq!(r.read("f.txt"), "ours\n");
    assert!(r.status().files.iter().all(|f| f.conflict.is_none()));
    oplog::undo(r.p(), None).unwrap();
    let st = r.status();
    let f = st.files.iter().find(|f| f.path == "f.txt").unwrap();
    assert_eq!(f.conflict, Some(ConflictKind::BothModified));
    assert_eq!(r.read("f.txt"), with_markers);

    // Mark as resolved via stage, undo, then undo the merge itself (aborts it).
    r.write("f.txt", "merged\n");
    ops::stage(r.p(), &["f.txt".into()]).unwrap();
    assert!(r.status().files.iter().all(|f| f.conflict.is_none()));
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.status().files.iter().find(|f| f.path == "f.txt").unwrap().conflict, Some(ConflictKind::BothModified));
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.status().state, RepoState::Clean);
    assert_eq!(r.head(), main_tip);
    assert_eq!(r.read("f.txt"), "ours\n");
}

#[test]
fn continue_merge_records_undoable_entry() {
    let r = Repo::new();
    r.commit_file("f.txt", "base\n", "base");
    r.git(&["switch", "-q", "-c", "other"]);
    r.commit_file("f.txt", "theirs\n", "theirs");
    r.git(&["switch", "-q", "main"]);
    let main_tip = r.commit_file("f.txt", "ours\n", "ours");
    operation::run_operation(r.p(), Operation::Merge { branch: "other".into() }, "m", None).unwrap();
    let e = ops::continue_operation(r.p()).unwrap_err();
    assert_eq!(e.kind, GitErrorKind::Conflict);
    r.write("f.txt", "merged\n");
    ops::stage(r.p(), &["f.txt".into()]).unwrap();
    let res = ops::continue_operation(r.p()).unwrap();
    assert_eq!(res.message, "Finished the merge of main");
    assert_eq!(r.status().state, RepoState::Clean);
    assert_ne!(r.head(), main_tip);
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.head(), main_tip);
    let log = oplog::get_oplog(r.p()).unwrap();
    assert_eq!(log[0].kind, "continue");
    assert!(log[0].undone);
}

#[test]
fn oplog_is_capped_at_100_entries_and_drops_old_refs() {
    let r = Repo::new();
    r.commit_file("a.txt", "a\n", "base");
    let first = ops::create_tag(r.p(), "t0", "HEAD", None).unwrap().entry.unwrap().id;
    let first_refs = r.git(&["for-each-ref", "--format=%(refname)", &format!("refs/gitcove/undo/{first}/")]);
    assert!(!first_refs.is_empty());
    for i in 1..=100 {
        ops::create_tag(r.p(), &format!("t{i}"), "HEAD", None).unwrap();
    }
    let log = oplog::get_oplog(r.p()).unwrap();
    assert_eq!(log.len(), 100);
    assert!(log.iter().all(|e| e.id != first));
    assert_eq!(log[0].label, "Create tag t100");
    let gone = r.git(&["for-each-ref", "--format=%(refname)", &format!("refs/gitcove/undo/{first}/")]);
    assert!(gone.is_empty(), "refs of entries that rolled off are deleted");
}

// ---------------------------------------------------------------------------------------
// Change diff (HEAD → worktree) and line staging
// ---------------------------------------------------------------------------------------

/// Rust port of the frontend's `buildPatch(diff, selected, false)` (src/lib/util/patch.ts):
/// a patch against the diff's old side holding only the +/- lines `pick` selects.
fn build_patch(d: &FileDiff, pick: impl Fn(&DiffLine) -> bool) -> Option<String> {
    let mut out: Vec<String> = Vec::new();
    let mut delta: i64 = 0;
    for h in &d.hunks {
        let mut body: Vec<String> = Vec::new();
        let (mut old_count, mut new_count) = (0i64, 0i64);
        let mut changed = false;
        let mut last_kept = false;
        for l in &h.lines {
            match l.kind {
                DiffLineKind::Context => {
                    body.push(format!(" {}", l.text));
                    old_count += 1;
                    new_count += 1;
                    last_kept = true;
                }
                DiffLineKind::Add if pick(l) => {
                    body.push(format!("+{}", l.text));
                    new_count += 1;
                    changed = true;
                    last_kept = true;
                }
                DiffLineKind::Add => last_kept = false,
                DiffLineKind::Del if pick(l) => {
                    body.push(format!("-{}", l.text));
                    old_count += 1;
                    changed = true;
                    last_kept = true;
                }
                DiffLineKind::Del => {
                    body.push(format!(" {}", l.text));
                    old_count += 1;
                    new_count += 1;
                    last_kept = true;
                }
                DiffLineKind::NoNewline => {
                    if last_kept {
                        body.push("\\ No newline at end of file".into());
                    }
                }
            }
        }
        if !changed {
            continue;
        }
        let start = h.old_start as i64;
        out.push(format!("@@ -{start},{old_count} +{},{new_count} @@", start + delta));
        delta += new_count - old_count;
        out.extend(body);
    }
    if out.is_empty() {
        return None;
    }
    let old = d.old_path.as_deref().unwrap_or(&d.path);
    let mut patch = vec![format!("diff --git a/{old} b/{}", d.path)];
    if matches!(d.status, ChangeKind::Untracked | ChangeKind::Added) {
        patch.extend(["new file mode 100644".to_string(), "--- /dev/null".to_string(), format!("+++ b/{}", d.path)]);
    } else {
        patch.extend([format!("--- a/{old}"), format!("+++ b/{}", d.path)]);
    }
    patch.extend(out);
    Some(patch.join("\n") + "\n")
}

fn find_line<'a>(d: &'a FileDiff, kind: DiffLineKind, text: &str) -> &'a DiffLine {
    d.hunks
        .iter()
        .flat_map(|h| &h.lines)
        .find(|l| l.kind == kind && l.text == text)
        .unwrap_or_else(|| panic!("no {kind:?} line {text:?} in {d:#?}"))
}

/// (text, staged) of every +/- line, in order.
fn staged_flags(d: &FileDiff) -> Vec<(String, Option<bool>)> {
    d.hunks
        .iter()
        .flat_map(|h| &h.lines)
        .filter(|l| matches!(l.kind, DiffLineKind::Add | DiffLineKind::Del))
        .map(|l| (l.text.clone(), l.staged))
        .collect()
}

fn context_count(d: &FileDiff) -> usize {
    d.hunks.iter().flat_map(|h| &h.lines).filter(|l| l.kind == DiffLineKind::Context).count()
}

/// f.txt committed as `line 0`..`line 29`, then line 1 and line 25 edited in the worktree.
fn two_region_fixture() -> Repo {
    let r = Repo::new();
    r.commit_file("f.txt", &lines(0..30), "base");
    let mut changed: Vec<String> = (0..30).map(|i| format!("line {i}")).collect();
    changed[1] = "FIRST CHANGE".into();
    changed[25] = "SECOND CHANGE".into();
    r.write("f.txt", &(changed.join("\n") + "\n"));
    r
}

fn first_region(l: &DiffLine) -> bool {
    l.text == "line 1" || l.text == "FIRST CHANGE"
}

fn second_region(l: &DiffLine) -> bool {
    l.text == "line 25" || l.text == "SECOND CHANGE"
}

#[test]
fn change_diff_marks_staged_lines() {
    let r = two_region_fixture();
    // Stage only the first region, the way the old two-list UI did (index → worktree patch).
    let wd = diff::get_working_diff(r.p(), "f.txt", false).unwrap();
    let patch = build_patch(&wd, first_region).unwrap();
    ops::apply_patch(r.p(), &patch, true, false, "").unwrap();

    let d = diff::get_change_diff(r.p(), "f.txt", None).unwrap();
    assert_eq!(d.status, ChangeKind::Modified);
    assert_eq!(d.hunks.len(), 2);
    assert_eq!((d.additions, d.deletions), (2, 2));
    assert_eq!(find_line(&d, DiffLineKind::Del, "line 1").staged, Some(true));
    assert_eq!(find_line(&d, DiffLineKind::Add, "FIRST CHANGE").staged, Some(true));
    assert_eq!(find_line(&d, DiffLineKind::Del, "line 25").staged, Some(false));
    assert_eq!(find_line(&d, DiffLineKind::Add, "SECOND CHANGE").staged, Some(false));
    assert!(d.hunks.iter().flat_map(|h| &h.lines).filter(|l| l.kind == DiffLineKind::Context).all(|l| l.staged.is_none()));
    // `staged` is only serialized when set.
    let ctx = d.hunks[0].lines.iter().find(|l| l.kind == DiffLineKind::Context).unwrap();
    assert!(serde_json::to_value(ctx).unwrap().get("staged").is_none());
    let add = find_line(&d, DiffLineKind::Add, "FIRST CHANGE");
    assert_eq!(serde_json::to_value(add).unwrap()["staged"], serde_json::json!(true));
    // The staged diff and worktree diff are untouched by the new reader.
    assert!(r.git(&["diff", "--cached"]).contains("FIRST CHANGE"));

    // Adjacent lines in one hunk can differ: the index holds one of two inserted lines.
    let mut staged: Vec<String> = (0..30).map(|i| format!("line {i}")).collect();
    staged.insert(11, "ADDED STAGED".into());
    r.write("f.txt", &(staged.join("\n") + "\n"));
    r.git(&["add", "f.txt"]);
    let mut work = staged.clone();
    work.insert(12, "ADDED UNSTAGED".into());
    work.retain(|l| l != "line 19"); // only in HEAD and the index
    r.write("f.txt", &(work.join("\n") + "\n"));
    let d = diff::get_change_diff(r.p(), "f.txt", None).unwrap();
    assert_eq!(
        staged_flags(&d),
        vec![
            ("ADDED STAGED".to_string(), Some(true)),
            ("ADDED UNSTAGED".to_string(), Some(false)),
            ("line 19".to_string(), Some(false)),
        ]
    );
    // A deletion that is staged.
    r.git(&["add", "f.txt"]);
    let d = diff::get_change_diff(r.p(), "f.txt", None).unwrap();
    assert_eq!(find_line(&d, DiffLineKind::Del, "line 19").staged, Some(true));
    assert!(staged_flags(&d).iter().all(|(_, s)| *s == Some(true)));
}

#[test]
fn change_diff_for_untracked_unborn_deleted_and_renamed() {
    let r = Repo::new();
    // Unborn HEAD: a staged file edited afterwards, and an untracked file.
    r.write("a.txt", "one\ntwo\n");
    r.git(&["add", "a.txt"]);
    r.write("a.txt", "one\ntwo\nthree\n");
    r.write("u.txt", "x\ny\n");
    let a = diff::get_change_diff(r.p(), "a.txt", None).unwrap();
    assert_eq!(a.status, ChangeKind::Added);
    assert_eq!(
        staged_flags(&a),
        vec![("one".into(), Some(true)), ("two".into(), Some(true)), ("three".into(), Some(false))]
    );
    let u = diff::get_change_diff(r.p(), "u.txt", None).unwrap();
    assert_eq!(u.status, ChangeKind::Untracked);
    assert_eq!(u.additions, 2);
    assert_eq!(staged_flags(&u), vec![("x".into(), Some(false)), ("y".into(), Some(false))]);

    // Line staging on an unborn HEAD: only "three" staged, then nothing, then undo.
    let patch = build_patch(&a, |l| l.text == "three").unwrap();
    ops::set_file_lines(r.p(), "a.txt", Some(&patch), "").unwrap();
    assert_eq!(r.git(&["show", ":a.txt"]), "three");
    ops::set_file_lines(r.p(), "a.txt", None, "").unwrap();
    assert!(r.staged_names().is_empty());
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.git(&["show", ":a.txt"]), "three");

    // With commits: an untracked file, a deleted file and a staged rename.
    let r = Repo::new();
    r.commit_file("gone.txt", "g1\ng2\n", "gone");
    r.commit_file("old.txt", &lines(0..20), "old");
    std::fs::remove_file(r.p().join("gone.txt")).unwrap();
    let g = diff::get_change_diff(r.p(), "gone.txt", None).unwrap();
    assert_eq!(g.status, ChangeKind::Deleted);
    assert_eq!(staged_flags(&g), vec![("g1".into(), Some(false)), ("g2".into(), Some(false))]);
    r.git(&["rm", "-q", "--cached", "gone.txt"]);
    let g = diff::get_change_diff(r.p(), "gone.txt", None).unwrap();
    assert_eq!(staged_flags(&g), vec![("g1".into(), Some(true)), ("g2".into(), Some(true))]);

    r.git(&["mv", "old.txt", "new.txt"]);
    let mut renamed: Vec<String> = (0..20).map(|i| format!("line {i}")).collect();
    renamed[3] = "EDITED AFTER RENAME".into();
    r.write("new.txt", &(renamed.join("\n") + "\n"));
    let n = diff::get_change_diff(r.p(), "new.txt", None).unwrap();
    assert_eq!(n.status, ChangeKind::Renamed);
    assert_eq!(n.old_path.as_deref(), Some("old.txt"));
    assert_eq!((n.additions, n.deletions), (1, 1));
    assert!(staged_flags(&n).iter().all(|(_, s)| s.is_none()), "renames are whole-file only");

    // Binary files keep the existing handling.
    r.write_bytes("bin.dat", &[0u8, 1, 2, 3]);
    let b = diff::get_change_diff(r.p(), "bin.dat", None).unwrap();
    assert!(b.binary);
    assert_eq!(b.status, ChangeKind::Untracked);
}

#[test]
fn change_diff_of_conflicted_file_has_no_flags() {
    let r = Repo::new();
    r.commit_file("f.txt", "base\n", "base");
    r.git(&["switch", "-q", "-c", "other"]);
    r.commit_file("f.txt", "theirs\n", "theirs");
    r.git(&["switch", "-q", "main"]);
    r.commit_file("f.txt", "ours\n", "ours");
    assert!(!r.git_out(&["merge", "other"]).status.success());
    let d = diff::get_change_diff(r.p(), "f.txt", None).unwrap();
    assert_eq!(d.status, ChangeKind::Conflicted);
    assert!(d.additions > 0);
    assert!(staged_flags(&d).iter().all(|(_, s)| s.is_none()));
    let e = ops::set_file_lines(r.p(), "f.txt", None, "").unwrap_err();
    assert_eq!(e.kind, GitErrorKind::Conflict);
    assert!(r.status().files.iter().any(|f| f.conflict.is_some()), "the conflict is left alone");
}

#[test]
fn set_file_lines_stages_exact_selection_and_undoes() {
    let r = two_region_fixture();
    // Start with everything staged: set_file_lines replaces the staged state, it doesn't add to it.
    ops::stage(r.p(), &["f.txt".into()]).unwrap();
    let d = diff::get_change_diff(r.p(), "f.txt", None).unwrap();
    assert!(staged_flags(&d).iter().all(|(_, s)| *s == Some(true)));

    // (a) A subset: exactly those changes are staged.
    let patch = build_patch(&d, first_region).unwrap();
    let res = ops::set_file_lines(r.p(), "f.txt", Some(&patch), "").unwrap();
    assert_eq!(res.message, "Staged lines in f.txt");
    let entry = res.entry.unwrap();
    assert_eq!((entry.kind.as_str(), entry.label.as_str()), ("stageLines", "Stage lines in f.txt"));
    assert!(entry.undoable);
    let cached = r.git(&["diff", "--cached"]);
    assert!(cached.contains("+FIRST CHANGE") && cached.contains("-line 1\n"));
    assert!(!cached.contains("SECOND CHANGE") && !cached.contains("-line 25"));
    assert!(r.read("f.txt").contains("SECOND CHANGE"), "the worktree is untouched");
    let d = diff::get_change_diff(r.p(), "f.txt", None).unwrap();
    assert_eq!(
        staged_flags(&d),
        vec![
            ("line 1".into(), Some(true)),
            ("FIRST CHANGE".into(), Some(true)),
            ("line 25".into(), Some(false)),
            ("SECOND CHANGE".into(), Some(false)),
        ]
    );

    // (b) None unstages everything for the path; a second time is a no-op.
    let res = ops::set_file_lines(r.p(), "f.txt", None, "Unstage f.txt").unwrap();
    assert_eq!(res.message, "Unstaged f.txt");
    assert!(r.staged_names().is_empty());
    let res = ops::set_file_lines(r.p(), "f.txt", None, "").unwrap();
    assert!(res.entry.is_none());
    assert_eq!(res.message, "f.txt had nothing staged");

    // (c) Undo restores the previous index, one entry at a time.
    oplog::undo(r.p(), None).unwrap();
    let cached = r.git(&["diff", "--cached"]);
    assert!(cached.contains("FIRST CHANGE") && !cached.contains("SECOND CHANGE"));
    oplog::undo(r.p(), None).unwrap();
    let cached = r.git(&["diff", "--cached"]);
    assert!(cached.contains("FIRST CHANGE") && cached.contains("SECOND CHANGE"));
    oplog::redo(r.p()).unwrap();
    assert!(!r.git(&["diff", "--cached"]).contains("SECOND CHANGE"));

    // (d) A stale patch fails and leaves the index exactly as it was.
    let d = diff::get_change_diff(r.p(), "f.txt", None).unwrap();
    let stale = build_patch(&d, second_region).unwrap();
    // HEAD moves on outside the app, changing the context around the second region.
    let mut upstream: Vec<String> = (0..30).map(|i| format!("line {i}")).collect();
    upstream[26] = "line 26 changed upstream".into();
    r.write("f.txt", &(upstream.join("\n") + "\n"));
    r.git(&["add", "f.txt"]);
    r.git(&["commit", "-q", "-m", "upstream"]);
    upstream[1] = "FIRST CHANGE".into();
    upstream[25] = "SECOND CHANGE".into();
    r.write("f.txt", &(upstream.join("\n") + "\n"));
    let fresh = diff::get_change_diff(r.p(), "f.txt", None).unwrap();
    ops::set_file_lines(r.p(), "f.txt", Some(&build_patch(&fresh, first_region).unwrap()), "").unwrap();
    let index_before = r.git(&["ls-files", "-s", "--", "f.txt"]);
    let log_before = oplog::get_oplog(r.p()).unwrap().len();
    let e = ops::set_file_lines(r.p(), "f.txt", Some(&stale), "").unwrap_err();
    assert_eq!(e.kind, GitErrorKind::Other);
    assert_eq!(e.message, "Couldn't stage those lines because the file changed. Refresh and try again.");
    assert_eq!(r.git(&["ls-files", "-s", "--", "f.txt"]), index_before);
    assert_eq!(oplog::get_oplog(r.p()).unwrap().len(), log_before);
    assert!(r.git(&["diff", "--cached"]).contains("FIRST CHANGE"));

    // A patch for a different file is refused before anything changes.
    let e = ops::set_file_lines(r.p(), "other.txt", Some(&stale), "").unwrap_err();
    assert!(e.message.contains("f.txt"), "{}", e.message);
    assert_eq!(r.git(&["ls-files", "-s", "--", "f.txt"]), index_before);
}

#[test]
fn set_file_lines_stages_part_of_an_untracked_file() {
    let r = Repo::new();
    r.commit_file("base.txt", "base\n", "base");
    // A removed line that looks like a patch header must not confuse path detection.
    r.write("new.txt", "a\nb\n-- c\nd\ne\n");
    let d = diff::get_change_diff(r.p(), "new.txt", None).unwrap();
    assert_eq!(d.status, ChangeKind::Untracked);
    assert!(staged_flags(&d).iter().all(|(_, s)| *s == Some(false)));
    let patch = build_patch(&d, |l| l.text != "-- c" && l.text != "e").unwrap();
    assert!(patch.contains("new file mode 100644\n--- /dev/null\n+++ b/new.txt\n"));
    let res = ops::set_file_lines(r.p(), "new.txt", Some(&patch), "Stage 3 lines in new.txt").unwrap();
    assert_eq!(res.message, "Staged 3 lines in new.txt");
    assert_eq!(r.git(&["show", ":new.txt"]), "a\nb\nd");
    assert_eq!(r.read("new.txt"), "a\nb\n-- c\nd\ne\n");

    let d = diff::get_change_diff(r.p(), "new.txt", None).unwrap();
    assert_eq!(d.status, ChangeKind::Added);
    assert_eq!(
        staged_flags(&d),
        vec![
            ("a".into(), Some(true)),
            ("b".into(), Some(true)),
            ("-- c".into(), Some(false)),
            ("d".into(), Some(true)),
            ("e".into(), Some(false)),
        ]
    );
    // Re-selecting from the combined diff (now status Added) replaces the staged lines.
    let patch = build_patch(&d, |l| l.text == "-- c").unwrap();
    ops::set_file_lines(r.p(), "new.txt", Some(&patch), "").unwrap();
    assert_eq!(r.git(&["show", ":new.txt"]), "-- c");
    // Unchecking the file makes it untracked again; undo brings the staged lines back.
    ops::set_file_lines(r.p(), "new.txt", None, "").unwrap();
    assert!(r.staged_names().is_empty());
    assert_eq!(diff::get_change_diff(r.p(), "new.txt", None).unwrap().status, ChangeKind::Untracked);
    oplog::undo(r.p(), None).unwrap();
    assert_eq!(r.git(&["show", ":new.txt"]), "-- c");
}

#[test]
fn diff_context_option() {
    let r = Repo::new();
    r.commit_file("f.txt", &lines(0..40), "base");
    let mut changed: Vec<String> = (0..40).map(|i| format!("line {i}")).collect();
    changed[20] = "CHANGED".into();
    let head = r.commit_file("f.txt", &(changed.join("\n") + "\n"), "change");
    let default = diff::get_commit_diff(r.p(), &head, "f.txt", None).unwrap();
    let wide = diff::get_commit_diff(r.p(), &head, "f.txt", Some(10)).unwrap();
    assert_eq!(context_count(&default), 6);
    assert_eq!(context_count(&wide), 20);
    assert_eq!(context_count(&diff::get_commit_diff(r.p(), &head, "f.txt", Some(0)).unwrap()), 0);

    changed[30] = "WORKTREE".into();
    r.write("f.txt", &(changed.join("\n") + "\n"));
    assert_eq!(context_count(&diff::get_change_diff(r.p(), "f.txt", None).unwrap()), 6);
    let whole = diff::get_change_diff(r.p(), "f.txt", Some(100)).unwrap();
    assert_eq!(context_count(&whole), 39);
    assert_eq!(staged_flags(&whole), vec![("line 30".into(), Some(false)), ("WORKTREE".into(), Some(false))]);
}

/// Runs `identity_in_isolated_config` in a child process whose global and system Git config
/// are empty, so the developer's own identity is never read or written.
#[test]
fn identity_reads_and_writes_isolated_config() {
    let tmp = tempfile::tempdir().unwrap();
    let global = tmp.path().join("gitconfig");
    std::fs::write(&global, "").unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "identity_in_isolated_config", "--nocapture", "--test-threads=1"])
        .env("GITCOVE_ISOLATED_IDENTITY", "1")
        .env("GIT_CONFIG_GLOBAL", &global)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("1 passed"), "the isolated test didn't run: {stdout}");
    let written = std::fs::read_to_string(&global).unwrap();
    assert!(written.contains("Ada Lovelace") && written.contains("ada@example.com"), "{written}");
}

/// Body of the test above. Does nothing unless run in that isolated child process.
#[test]
fn identity_in_isolated_config() {
    if std::env::var_os("GITCOVE_ISOLATED_IDENTITY").is_none() {
        return;
    }
    let r = Repo::new();
    r.git(&["config", "--unset", "user.name"]);
    r.git(&["config", "--unset", "user.email"]);
    let id = repo::get_identity(r.p()).unwrap();
    assert_eq!(id, Identity { name: None, email: None });
    assert_eq!(serde_json::to_value(&id).unwrap(), serde_json::json!({"name": null, "email": null}));

    let e = repo::set_identity("  ", "ada@example.com").unwrap_err();
    assert_eq!(e.kind, GitErrorKind::Other);
    assert!(repo::set_identity("Ada", "").is_err());
    assert_eq!(repo::get_identity(r.p()).unwrap(), Identity { name: None, email: None });

    repo::set_identity(" Ada Lovelace ", "ada@example.com\n").unwrap();
    let id = repo::get_identity(r.p()).unwrap();
    assert_eq!(id.name.as_deref(), Some("Ada Lovelace"));
    assert_eq!(id.email.as_deref(), Some("ada@example.com"));
    // Repository config wins over global.
    r.git(&["config", "user.name", "Local Name"]);
    assert_eq!(repo::get_identity(r.p()).unwrap().name.as_deref(), Some("Local Name"));
}
