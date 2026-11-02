//! File watching: emits debounced `repo-changed` events for a repository.
//!
//! On Linux (inotify) directories are watched one by one so `node_modules`, `target` and
//! ignored build folders never consume watches. Elsewhere the native recursive watcher is used.

use crate::git::error::{GitError, GitResult};
use crate::git::runner::{Git, Opts, split_nul};
use crate::git::types::{RepoChangeKind, RepoChangedEvent};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{BTreeSet, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type ChangeSink = Arc<dyn Fn(RepoChangedEvent) + Send + Sync>;

const DEBOUNCE: Duration = Duration::from_millis(300);
const MAX_DELAY: Duration = Duration::from_millis(1500);
const SKIP_DIRS: &[&str] = &["node_modules", "target"];

pub struct RepoWatcher {
    stop: Arc<AtomicBool>,
}

impl Drop for RepoWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub root: PathBuf,
    /// The repository's git dir and (for linked worktrees) the common dir.
    pub git_dirs: Vec<PathBuf>,
}

/// Classify a changed path, or None when it should be ignored.
pub fn classify_path(layout: &Layout, path: &Path) -> Option<RepoChangeKind> {
    for gd in &layout.git_dirs {
        if let Ok(rel) = path.strip_prefix(gd) {
            let first = rel.components().next().and_then(|c| c.as_os_str().to_str()).unwrap_or("");
            if matches!(first, "objects" | "logs" | "gitcove" | "lfs" | "hooks") {
                return None;
            }
            if path.extension().and_then(|e| e.to_str()) == Some("lock") {
                return None;
            }
            if first == "FETCH_HEAD" || first.ends_with(".tmp") {
                return None;
            }
            return Some(RepoChangeKind::Git);
        }
    }
    let rel = path.strip_prefix(&layout.root).ok()?;
    let mut comps = rel.components();
    if let Some(Component::Normal(first)) = comps.clone().next() {
        if first == ".git" {
            return None;
        }
    }
    if comps.any(|c| matches!(c, Component::Normal(n) if SKIP_DIRS.iter().any(|s| n == *s))) {
        return None;
    }
    Some(RepoChangeKind::Worktree)
}

fn resolve(root: &Path, p: &str) -> PathBuf {
    let pb = PathBuf::from(p.trim());
    let pb = if pb.is_absolute() { pb } else { root.join(pb) };
    std::fs::canonicalize(&pb).unwrap_or(pb)
}

pub fn layout_for(root: &Path) -> GitResult<Layout> {
    let git = Git::new(root);
    let text = git.read(["rev-parse", "--absolute-git-dir", "--git-common-dir"])?;
    let mut lines = text.lines();
    let root_c = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let gd = resolve(&root_c, lines.next().unwrap_or(".git"));
    let common = resolve(&root_c, lines.next().unwrap_or(".git"));
    let mut git_dirs = vec![gd];
    if !git_dirs.contains(&common) {
        git_dirs.push(common);
    }
    Ok(Layout { root: root_c, git_dirs })
}

/// Ignored directories at watch start (so they are never walked).
fn ignored_dirs(root: &Path) -> HashSet<PathBuf> {
    let git = Git::new(root);
    let mut set = HashSet::new();
    if let Ok(out) = git.exec(
        ["ls-files", "--others", "--ignored", "--exclude-standard", "--directory", "-z"],
        Opts { read_only: true, ..Default::default() },
    ) {
        for p in split_nul(&out.text()) {
            if let Some(d) = p.strip_suffix('/') {
                set.insert(root.join(d));
            }
        }
    }
    set
}

fn is_ignored_by_git(root: &Path, dir: &Path) -> bool {
    let Ok(rel) = dir.strip_prefix(root) else { return false };
    let rel = format!("{}/", rel.to_string_lossy());
    Git::new(root)
        .exec(["check-ignore", "-q", "--", &rel], Opts { read_only: true, ..Default::default() })
        .map(|o| o.code == 0)
        .unwrap_or(false)
}

struct Walker<'a> {
    watcher: &'a Mutex<RecommendedWatcher>,
    layout: &'a Layout,
    ignored: &'a HashSet<PathBuf>,
    stop: &'a AtomicBool,
    warned: bool,
}

impl Walker<'_> {
    fn skip_dir(&self, dir: &Path) -> bool {
        if self.ignored.contains(dir) {
            return true;
        }
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if SKIP_DIRS.contains(&name) {
            return true;
        }
        if dir.starts_with(&self.layout.root) && dir != self.layout.root && name == ".git" {
            return true;
        }
        for gd in &self.layout.git_dirs {
            if let Ok(rel) = dir.strip_prefix(gd) {
                let first = rel.components().next().and_then(|c| c.as_os_str().to_str()).unwrap_or("");
                if matches!(first, "objects" | "logs" | "gitcove" | "lfs" | "hooks" | "modules" | "worktrees") {
                    return true;
                }
            }
        }
        false
    }

    fn watch_one(&mut self, dir: &Path) -> bool {
        let res = self.watcher.lock().map(|mut w| w.watch(dir, RecursiveMode::NonRecursive));
        match res {
            Ok(Ok(())) => true,
            Ok(Err(e)) => {
                if !self.warned {
                    log::warn!("file watching is incomplete for {}: {e}", self.layout.root.display());
                    self.warned = true;
                }
                false
            }
            Err(_) => false,
        }
    }

    /// Watch `dir` and all its non-skipped subdirectories.
    fn add_tree(&mut self, dir: &Path) {
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            if self.stop.load(Ordering::Relaxed) {
                return;
            }
            if self.skip_dir(&d) && d != self.layout.root {
                continue;
            }
            if !self.watch_one(&d) {
                continue;
            }
            if let Ok(rd) = std::fs::read_dir(&d) {
                for entry in rd.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        stack.push(entry.path());
                    }
                }
            }
        }
    }
}

/// Start watching `root`; events are tagged with `repo_key` (the path the frontend passed).
pub fn start(repo_key: String, root: &Path, sink: ChangeSink) -> GitResult<RepoWatcher> {
    let layout = layout_for(root)?;
    let (tx, rx) = channel::<notify::Result<notify::Event>>();
    let watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .map_err(|e| GitError::other(format!("Couldn't watch the repository for changes: {e}.")))?;
    let watcher = Arc::new(Mutex::new(watcher));
    let stop = Arc::new(AtomicBool::new(false));
    let stop_t = stop.clone();
    std::thread::Builder::new()
        .name("repo-watch".into())
        .spawn(move || run_loop(repo_key, layout, watcher, rx, stop_t, sink))
        .map_err(|e| GitError::other(format!("Couldn't watch the repository for changes: {e}.")))?;
    Ok(RepoWatcher { stop })
}

fn run_loop(
    repo_key: String,
    layout: Layout,
    watcher: Arc<Mutex<RecommendedWatcher>>,
    rx: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    stop: Arc<AtomicBool>,
    sink: ChangeSink,
) {
    let manual = cfg!(target_os = "linux");
    let ignored = if manual { ignored_dirs(&layout.root) } else { HashSet::new() };
    {
        let mut walker = Walker { watcher: &watcher, layout: &layout, ignored: &ignored, stop: &stop, warned: false };
        if manual {
            walker.add_tree(&layout.root);
            for gd in layout.git_dirs.clone() {
                walker.watch_one(&gd);
                walker.add_tree(&gd.join("refs"));
            }
        } else {
            let mut paths = vec![layout.root.clone()];
            for gd in &layout.git_dirs {
                if !gd.starts_with(&layout.root) {
                    paths.push(gd.clone());
                }
            }
            for p in paths {
                if let Ok(mut w) = watcher.lock() {
                    if let Err(e) = w.watch(&p, RecursiveMode::Recursive) {
                        log::warn!("couldn't watch {}: {e}", p.display());
                    }
                }
            }
        }
    }

    let mut pending_git = false;
    let mut pending_wt: BTreeSet<PathBuf> = BTreeSet::new();
    let mut first: Option<Instant> = None;
    let mut last = Instant::now();
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                let is_create = matches!(event.kind, EventKind::Create(_) | EventKind::Modify(notify::event::ModifyKind::Name(_)));
                for path in &event.paths {
                    let Some(kind) = classify_path(&layout, path) else { continue };
                    if manual && is_create && path.is_dir() {
                        let skip = kind == RepoChangeKind::Worktree && is_ignored_by_git(&layout.root, path);
                        if !skip {
                            let mut walker =
                                Walker { watcher: &watcher, layout: &layout, ignored: &ignored, stop: &stop, warned: true };
                            walker.add_tree(path);
                        }
                    }
                    match kind {
                        RepoChangeKind::Git => pending_git = true,
                        RepoChangeKind::Worktree => {
                            if pending_wt.len() < 2000 {
                                pending_wt.insert(path.clone());
                            }
                        }
                    }
                    last = Instant::now();
                    first.get_or_insert(last);
                }
            }
            Ok(Err(_)) => {}
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if let Some(f) = first {
            if last.elapsed() >= DEBOUNCE || f.elapsed() >= MAX_DELAY {
                if pending_git {
                    sink(RepoChangedEvent { repo: repo_key.clone(), kind: RepoChangeKind::Git });
                }
                if !pending_wt.is_empty() && !all_ignored(&layout.root, &pending_wt) {
                    sink(RepoChangedEvent { repo: repo_key.clone(), kind: RepoChangeKind::Worktree });
                }
                pending_git = false;
                pending_wt.clear();
                first = None;
            }
        }
    }
    drop(watcher);
}

/// True when git ignores every changed path (e.g. build output in an ignored folder).
fn all_ignored(root: &Path, paths: &BTreeSet<PathBuf>) -> bool {
    if paths.len() > 500 {
        return false;
    }
    let mut input = Vec::new();
    let mut n = 0;
    for p in paths {
        if let Ok(rel) = p.strip_prefix(root) {
            let s = rel.to_string_lossy();
            if s.is_empty() || s == ".gitignore" || s.ends_with("/.gitignore") {
                return false;
            }
            input.extend_from_slice(s.as_bytes());
            input.push(0);
            n += 1;
        }
    }
    if n == 0 {
        return false;
    }
    let out = Git::new(root).exec(
        ["check-ignore", "-z", "--stdin"],
        Opts { read_only: true, stdin: Some(input), ..Default::default() },
    );
    match out {
        Ok(o) if o.code == 0 => split_nul(&o.text()).len() >= n,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify() {
        let layout = Layout { root: PathBuf::from("/r"), git_dirs: vec![PathBuf::from("/r/.git")] };
        assert_eq!(classify_path(&layout, Path::new("/r/src/a.ts")), Some(RepoChangeKind::Worktree));
        assert_eq!(classify_path(&layout, Path::new("/r/.git/HEAD")), Some(RepoChangeKind::Git));
        assert_eq!(classify_path(&layout, Path::new("/r/.git/refs/heads/main")), Some(RepoChangeKind::Git));
        assert_eq!(classify_path(&layout, Path::new("/r/.git/index.lock")), None);
        assert_eq!(classify_path(&layout, Path::new("/r/.git/objects/ab/cdef")), None);
        assert_eq!(classify_path(&layout, Path::new("/r/.git/logs/HEAD")), None);
        assert_eq!(classify_path(&layout, Path::new("/r/.git/gitcove/oplog.json")), None);
        assert_eq!(classify_path(&layout, Path::new("/r/node_modules/x/y.js")), None);
        assert_eq!(classify_path(&layout, Path::new("/r/a/target/debug/x")), None);
        assert_eq!(classify_path(&layout, Path::new("/elsewhere/x")), None);
    }
}
