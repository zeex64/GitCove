//! Snapshots of repository state taken around write operations so they can be undone:
//! ref snapshots (HEAD, branches, index tree) and file snapshots (worktree contents and
//! index entries of specific paths, stored as blobs in the object database).

use super::error::{GitError, GitResult};
use super::refs::local_branches;
use super::runner::{Git, ZERO_OID, split_nul};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HeadSnap {
    /// Short branch name when HEAD is symbolic (possibly unborn).
    pub branch: Option<String>,
    /// Commit HEAD resolves to; None when unborn.
    pub commit: Option<String>,
}

impl HeadSnap {
    pub fn describe(&self) -> String {
        match (&self.branch, &self.commit) {
            (Some(b), _) => b.clone(),
            (None, Some(c)) => super::runner::short(c).to_string(),
            (None, None) => "HEAD".to_string(),
        }
    }
    /// Same position: same branch, or both detached at the same commit.
    pub fn same_place(&self, other: &HeadSnap) -> bool {
        match (&self.branch, &other.branch) {
            (Some(a), Some(b)) => a == b,
            (None, None) => self.commit == other.commit,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub head: HeadSnap,
    pub branches: BTreeMap<String, String>,
    /// `git write-tree` of the index; None when the index has unmerged entries.
    pub index_tree: Option<String>,
}

pub fn head_snap(git: &Git) -> GitResult<HeadSnap> {
    let sym = git.try_read(["symbolic-ref", "-q", "HEAD"])?.map(|s| s.trim().to_string());
    let branch = sym.map(|s| s.strip_prefix("refs/heads/").map(str::to_string).unwrap_or(s));
    Ok(HeadSnap { branch, commit: git.rev_parse("HEAD")? })
}

pub fn write_tree(git: &Git) -> GitResult<Option<String>> {
    let out = git.run(["write-tree"])?;
    Ok(if out.ok() { Some(out.text().trim().to_string()) } else { None })
}

pub fn capture(git: &Git) -> GitResult<Snapshot> {
    Ok(Snapshot {
        head: head_snap(git)?,
        branches: local_branches(git)?.into_iter().collect(),
        index_tree: write_tree(git)?,
    })
}

impl Snapshot {
    /// Objects worth keeping reachable for this snapshot relative to another.
    pub fn changed_objects(&self, other: Option<&Snapshot>) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(c) = &self.head.commit {
            v.push(c.clone());
        }
        if let Some(t) = &self.index_tree {
            v.push(t.clone());
        }
        if let Some(o) = other {
            for (name, sha) in &self.branches {
                if o.branches.get(name) != Some(sha) {
                    v.push(sha.clone());
                }
            }
        }
        v
    }
}

// ---------------------------------------------------------------------------------------
// File snapshots
// ---------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub mode: String,
    pub sha: String,
    pub stage: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WtState {
    Absent,
    File { blob: String, executable: bool },
    Symlink { blob: String },
    /// A directory (nested repository or submodule) at this path: never touched.
    Dir,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PathState {
    pub path: String,
    /// None when the snapshot only covers the index.
    pub wt: Option<WtState>,
    pub index: Vec<IndexEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSet {
    pub worktree: bool,
    pub files: Vec<PathState>,
}

impl FileSet {
    pub fn paths(&self) -> Vec<String> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }

    /// Blob ids referenced by this snapshot.
    pub fn objects(&self) -> Vec<String> {
        let mut v = Vec::new();
        for f in &self.files {
            match &f.wt {
                Some(WtState::File { blob, .. }) | Some(WtState::Symlink { blob }) => v.push(blob.clone()),
                _ => {}
            }
            for e in &f.index {
                if e.mode != "160000" {
                    v.push(e.sha.clone());
                }
            }
        }
        v.sort();
        v.dedup();
        v
    }

    pub fn get(&self, path: &str) -> Option<&PathState> {
        self.files.iter().find(|f| f.path == path)
    }
}

/// Give both snapshots the same set of paths (paths missing on one side are absent there).
pub fn align(a: &mut FileSet, b: &mut FileSet) {
    let all: BTreeSet<String> = a.files.iter().chain(b.files.iter()).map(|f| f.path.clone()).collect();
    for set in [&mut *a, &mut *b] {
        for p in &all {
            if set.get(p).is_none() {
                set.files.push(PathState {
                    path: p.clone(),
                    wt: if set.worktree { Some(WtState::Absent) } else { None },
                    index: vec![],
                });
            }
        }
        set.files.sort_by(|x, y| x.path.cmp(&y.path));
    }
}

/// Index entries (all stages) for the given paths; entries under directory paths are included.
pub fn index_entries(git: &Git, paths: &[String]) -> GitResult<BTreeMap<String, Vec<IndexEntry>>> {
    let text = if paths.len() > 200 {
        git.read(["ls-files", "-s", "-z"])?
    } else {
        let mut args: Vec<&str> = vec!["ls-files", "-s", "-z", "--"];
        args.extend(paths.iter().map(String::as_str));
        git.read(&args)?
    };
    let wanted: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
    let mut out: BTreeMap<String, Vec<IndexEntry>> = BTreeMap::new();
    for rec in split_nul(&text) {
        let Some((meta, path)) = rec.split_once('\t') else { continue };
        if paths.len() > 200 && !wanted.contains(path) && !wanted.iter().any(|w| path.starts_with(&format!("{w}/"))) {
            continue;
        }
        let mut it = meta.split_whitespace();
        let (Some(mode), Some(sha), Some(stage)) = (it.next(), it.next(), it.next()) else { continue };
        out.entry(path.to_string()).or_default().push(IndexEntry {
            mode: mode.to_string(),
            sha: sha.to_string(),
            stage: stage.parse().unwrap_or(0),
        });
    }
    for v in out.values_mut() {
        v.sort_by_key(|e| e.stage);
    }
    Ok(out)
}

#[cfg(unix)]
fn is_executable(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_meta: &fs::Metadata) -> bool {
    false
}

enum Probe {
    Absent,
    File { executable: bool },
    Symlink(Vec<u8>),
    Dir,
}

fn probe(root: &Path, path: &str) -> Probe {
    let full = root.join(path);
    match fs::symlink_metadata(&full) {
        Err(_) => Probe::Absent,
        Ok(m) if m.file_type().is_symlink() => {
            let target = fs::read_link(&full).map(|t| t.to_string_lossy().into_owned()).unwrap_or_default();
            Probe::Symlink(target.into_bytes())
        }
        Ok(m) if m.is_dir() => Probe::Dir,
        Ok(m) => Probe::File { executable: is_executable(&m) },
    }
}

/// Hash worktree files (raw bytes, no filters). With `write`, store the blobs.
fn hash_files(git: &Git, paths: &[&str], write: bool) -> GitResult<Vec<String>> {
    if paths.is_empty() {
        return Ok(vec![]);
    }
    let mut result = vec![String::new(); paths.len()];
    let mut batch: Vec<(usize, &str)> = Vec::new();
    for (i, p) in paths.iter().enumerate() {
        if p.contains('\n') {
            let mut args = vec!["hash-object", "--no-filters"];
            if write {
                args.push("-w");
            }
            args.push("--");
            args.push(p);
            result[i] = git.write(&args)?.trim().to_string();
        } else {
            batch.push((i, p));
        }
    }
    if !batch.is_empty() {
        let mut input = Vec::new();
        for (_, p) in &batch {
            input.extend_from_slice(p.as_bytes());
            input.push(b'\n');
        }
        let mut args = vec!["hash-object", "--no-filters", "--stdin-paths"];
        if write {
            args.push("-w");
        }
        let out = git.write_stdin(&args, input)?;
        let text = out.text();
        let shas: Vec<&str> = text.lines().collect();
        if shas.len() != batch.len() {
            return Err(GitError::other("Couldn't save a copy of the files before changing them."));
        }
        for ((i, _), sha) in batch.iter().zip(shas) {
            result[*i] = sha.trim().to_string();
        }
    }
    Ok(result)
}

fn hash_bytes(git: &Git, data: Vec<u8>, write: bool) -> GitResult<String> {
    let mut args = vec!["hash-object", "--stdin"];
    if write {
        args.push("-w");
    }
    Ok(git.write_stdin(&args, data)?.text().trim().to_string())
}

/// Snapshot the given concrete paths. `worktree` includes file contents; the index is
/// always included. Blobs are written to the object database when `store` is true.
pub fn capture_files(git: &Git, paths: &[String], worktree: bool, store: bool) -> GitResult<FileSet> {
    let mut uniq: Vec<String> = paths.to_vec();
    uniq.sort();
    uniq.dedup();
    let index = index_entries(git, &uniq)?;
    let mut set = FileSet { worktree, files: vec![] };
    let mut all_paths: BTreeSet<String> = uniq.iter().cloned().collect();
    // Index-only snapshots also cover entries found under directory pathspecs.
    if !worktree {
        all_paths.extend(index.keys().cloned());
    }
    let all_paths: Vec<String> = all_paths.into_iter().collect();
    let mut wt_states: Vec<Option<WtState>> = vec![None; all_paths.len()];
    if worktree {
        let root = git.dir().to_path_buf();
        let mut file_idx = Vec::new();
        let mut file_paths: Vec<&str> = Vec::new();
        for (i, p) in all_paths.iter().enumerate() {
            match probe(&root, p) {
                Probe::Absent => wt_states[i] = Some(WtState::Absent),
                Probe::Dir => wt_states[i] = Some(WtState::Dir),
                Probe::Symlink(target) => {
                    wt_states[i] = Some(WtState::Symlink { blob: hash_bytes(git, target, store)? });
                }
                Probe::File { executable } => {
                    wt_states[i] = Some(WtState::File { blob: String::new(), executable });
                    file_idx.push(i);
                    file_paths.push(p.as_str());
                }
            }
        }
        let shas = hash_files(git, &file_paths, store)?;
        for (i, sha) in file_idx.into_iter().zip(shas) {
            if let Some(WtState::File { blob, .. }) = &mut wt_states[i] {
                *blob = sha;
            }
        }
    }
    for (i, p) in all_paths.iter().enumerate() {
        set.files.push(PathState {
            path: p.clone(),
            wt: wt_states[i].take(),
            index: index.get(p).cloned().unwrap_or_default(),
        });
    }
    Ok(set)
}

/// Paths whose current state differs from `expected`.
pub fn changed_since(git: &Git, expected: &FileSet) -> GitResult<Vec<String>> {
    let current = capture_files(git, &expected.paths(), expected.worktree, false)?;
    let mut changed = Vec::new();
    for exp in &expected.files {
        let Some(cur) = current.get(&exp.path) else {
            changed.push(exp.path.clone());
            continue;
        };
        let wt_differs = expected.worktree
            && match (&exp.wt, &cur.wt) {
                (Some(WtState::File { blob: a, .. }), Some(WtState::File { blob: b, .. })) => a != b,
                (a, b) => a != b,
            };
        if wt_differs || exp.index != cur.index {
            changed.push(exp.path.clone());
        }
    }
    Ok(changed)
}

/// Read blob contents with one `cat-file --batch` process.
fn read_blobs(git: &Git, shas: &[String]) -> GitResult<BTreeMap<String, Vec<u8>>> {
    let mut out = BTreeMap::new();
    if shas.is_empty() {
        return Ok(out);
    }
    let mut input = Vec::new();
    for s in shas {
        input.extend_from_slice(s.as_bytes());
        input.push(b'\n');
    }
    let res = git.read_stdin(["cat-file", "--batch"], input)?;
    let data = res.stdout;
    let mut pos = 0;
    while pos < data.len() {
        let Some(nl) = data[pos..].iter().position(|&b| b == b'\n') else { break };
        let header = String::from_utf8_lossy(&data[pos..pos + nl]).into_owned();
        pos += nl + 1;
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() < 3 {
            // "<sha> missing"
            return Err(GitError::other(format!(
                "A saved copy of a file is missing from the repository ({}). It can't be restored.",
                parts.first().unwrap_or(&"?")
            )));
        }
        let size: usize = parts[2].parse().unwrap_or(0);
        let end = (pos + size).min(data.len());
        out.insert(parts[0].to_string(), data[pos..end].to_vec());
        pos = end + 1;
    }
    Ok(out)
}

fn remove_empty_parents(root: &Path, path: &Path) {
    let mut dir = path.parent().map(Path::to_path_buf);
    while let Some(d) = dir {
        if d == root || !d.starts_with(root) {
            break;
        }
        if fs::remove_dir(&d).is_err() {
            break;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
}

pub fn delete_worktree_file(root: &Path, rel: &str) -> GitResult<()> {
    let full = root.join(rel);
    match fs::symlink_metadata(&full) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => Ok(()),
        Ok(_) => {
            fs::remove_file(&full).map_err(|e| GitError::io(&format!("Couldn't delete {rel}"), e))?;
            remove_empty_parents(root, &full);
            Ok(())
        }
        Err(_) => Ok(()),
    }
}

fn write_file(full: &PathBuf, rel: &str, data: &[u8], executable: bool) -> GitResult<()> {
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).map_err(|e| GitError::io(&format!("Couldn't restore {rel}"), e))?;
    }
    if let Ok(m) = fs::symlink_metadata(full) {
        if m.file_type().is_symlink() {
            let _ = fs::remove_file(full);
        }
    }
    fs::write(full, data).map_err(|e| GitError::io(&format!("Couldn't restore {rel}"), e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(m) = fs::metadata(full) {
            let mut perm = m.permissions();
            let mode = perm.mode();
            let new = if executable { mode | ((mode & 0o444) >> 2) } else { mode & !0o111 };
            if new != mode {
                perm.set_mode(new);
                let _ = fs::set_permissions(full, perm);
            }
        }
    }
    #[cfg(not(unix))]
    let _ = executable;
    Ok(())
}

#[cfg(unix)]
fn make_symlink(target: &str, full: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, full)
}

#[cfg(not(unix))]
fn make_symlink(target: &str, full: &Path) -> std::io::Result<()> {
    // Mirrors git with core.symlinks=false: a plain file holding the link target.
    fs::write(full, target.as_bytes())
}

/// Write a snapshot back: worktree contents (when captured) and index entries.
pub fn restore_files(git: &Git, set: &FileSet) -> GitResult<()> {
    let root = git.dir().to_path_buf();
    if set.worktree {
        let blobs: Vec<String> = set
            .files
            .iter()
            .filter_map(|f| match &f.wt {
                Some(WtState::File { blob, .. }) | Some(WtState::Symlink { blob }) => Some(blob.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let contents = read_blobs(git, &blobs)?;
        for f in &set.files {
            let full = root.join(&f.path);
            match &f.wt {
                None | Some(WtState::Dir) => {}
                Some(WtState::Absent) => delete_worktree_file(&root, &f.path)?,
                Some(WtState::File { blob, executable }) => {
                    let data = contents.get(blob).map(Vec::as_slice).unwrap_or(&[]);
                    write_file(&full, &f.path, data, *executable)?;
                }
                Some(WtState::Symlink { blob }) => {
                    let target = String::from_utf8_lossy(contents.get(blob).map(Vec::as_slice).unwrap_or(&[])).into_owned();
                    if let Some(parent) = full.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if fs::symlink_metadata(&full).is_ok() {
                        let _ = fs::remove_file(&full);
                    }
                    make_symlink(&target, &full).map_err(|e| GitError::io(&format!("Couldn't restore {}", f.path), e))?;
                }
            }
        }
    }
    let mut input = Vec::new();
    for f in &set.files {
        input.extend_from_slice(format!("0 {ZERO_OID}\t{}", f.path).as_bytes());
        input.push(0);
        for e in &f.index {
            input.extend_from_slice(format!("{} {} {}\t{}", e.mode, e.sha, e.stage, f.path).as_bytes());
            input.push(0);
        }
    }
    if !input.is_empty() {
        git.write_stdin(["update-index", "-z", "--index-info"], input)?;
    }
    // Refresh stat info so `git status` doesn't report restored files as racy-dirty.
    let _ = git.run(["update-index", "-q", "--refresh"]);
    Ok(())
}

/// Store a tree object that references every given blob so they stay reachable.
pub fn blob_tree(git: &Git, blobs: &[String]) -> GitResult<Option<String>> {
    if blobs.is_empty() {
        return Ok(None);
    }
    let mut input = Vec::new();
    for (i, b) in blobs.iter().enumerate() {
        input.extend_from_slice(format!("100644 blob {b}\t{i}").as_bytes());
        input.push(0);
    }
    let out = git.write_stdin(["mktree", "-z", "--missing"], input)?;
    Ok(Some(out.text().trim().to_string()))
}
