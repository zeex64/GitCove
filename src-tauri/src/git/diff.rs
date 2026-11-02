//! Unified diff parsing, working-tree diffs and commit diffs.

use super::error::{GitError, GitResult};
use super::log::load_commit;
use super::runner::{EMPTY_TREE, Git, Opts, split_nul};
use super::types::{ChangeKind, CommitDetails, CommitFile, DiffLine, DiffLineKind, FileDiff, Hunk};
use std::path::Path;

pub const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_DIFF_LINES: usize = 20_000;

fn parse_range(s: &str) -> (u32, u32) {
    let s = s.trim_start_matches(['-', '+']);
    match s.split_once(',') {
        Some((a, b)) => (a.parse().unwrap_or(0), b.parse().unwrap_or(0)),
        None => (s.parse().unwrap_or(0), 1),
    }
}

/// Parse "@@ -a,b +c,d @@ section".
pub fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32)> {
    let rest = line.strip_prefix("@@ ")?;
    let end = rest.find(" @@")?;
    let mut it = rest[..end].split_whitespace();
    let old = it.next()?;
    let new = it.next()?;
    if !old.starts_with('-') || !new.starts_with('+') {
        return None;
    }
    let (os, ol) = parse_range(old);
    let (ns, nl) = parse_range(new);
    Some((os, ol, ns, nl))
}

/// Strip the a/ or b/ prefix from a header path, undoing C-style quoting.
fn header_path(raw: &str) -> Option<String> {
    let raw = raw.trim_end_matches(['\t', '\r']);
    if raw == "/dev/null" {
        return None;
    }
    let unq = if raw.starts_with('"') { unquote_c(raw) } else { raw.to_string() };
    let p = unq.strip_prefix("a/").or_else(|| unq.strip_prefix("b/")).unwrap_or(&unq);
    Some(p.to_string())
}

/// Undo git's C-style path quoting ("a/caf\303\251").
pub fn unquote_c(s: &str) -> String {
    let inner = s.trim_matches('"');
    let bytes = inner.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() {
            let n = bytes[i + 1];
            match n {
                b'n' => out.push(b'\n'),
                b't' => out.push(b'\t'),
                b'r' => out.push(b'\r'),
                b'"' => out.push(b'"'),
                b'\\' => out.push(b'\\'),
                b'a' => out.push(7),
                b'b' => out.push(8),
                b'f' => out.push(12),
                b'v' => out.push(11),
                b'0'..=b'7' if i + 4 <= bytes.len() => {
                    let oct = std::str::from_utf8(&bytes[i + 1..i + 4]).unwrap_or("");
                    if let Ok(v) = u8::from_str_radix(oct, 8) {
                        out.push(v);
                        i += 4;
                        continue;
                    }
                    out.push(n);
                }
                _ => out.push(n),
            }
            i += 2;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn empty_diff(path: &str, status: ChangeKind) -> FileDiff {
    FileDiff {
        path: path.to_string(),
        old_path: None,
        status,
        binary: false,
        too_large: false,
        additions: 0,
        deletions: 0,
        hunks: vec![],
    }
}

/// Parse a unified diff (possibly several files) into `FileDiff`s.
pub fn parse_diff(text: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut cur: Option<FileDiff> = None;
    let mut old_no = 0u32;
    let mut new_no = 0u32;
    let mut in_hunk = false;
    let mut minus_path: Option<String> = None;

    let finish = |cur: &mut Option<FileDiff>, files: &mut Vec<FileDiff>| {
        if let Some(f) = cur.take() {
            files.push(f);
        }
    };

    let lines: Vec<&str> = text.split('\n').collect();
    let n = lines.len();
    for (idx, line) in lines.iter().enumerate() {
        let line = *line;
        // A trailing empty string after the final newline is not a line.
        if idx == n - 1 && line.is_empty() {
            break;
        }
        if line.starts_with("diff --git ") || line.starts_with("diff --cc ") || line.starts_with("diff --combined ") {
            finish(&mut cur, &mut files);
            in_hunk = false;
            minus_path = None;
            let mut f = empty_diff("", ChangeKind::Modified);
            // Best effort: "diff --git a/x b/y"; refined by ---/+++ and rename headers.
            if let Some(rest) = line.strip_prefix("diff --git ") {
                if let Some(pos) = rest.find(" b/") {
                    f.path = header_path(&rest[pos + 1..]).unwrap_or_default();
                    if let Some(a) = header_path(&rest[..pos]) {
                        if a != f.path {
                            f.old_path = Some(a);
                        }
                    }
                } else if rest.starts_with('"') {
                    // Quoted paths: take the last quoted token.
                    if let Some(start) = rest[1..].find("\" ").map(|i| i + 3) {
                        f.path = header_path(&rest[start..]).unwrap_or_default();
                    }
                }
            } else if let Some(rest) = line.strip_prefix("diff --cc ").or_else(|| line.strip_prefix("diff --combined ")) {
                f.path = rest.to_string();
            }
            cur = Some(f);
            continue;
        }
        let Some(f) = cur.as_mut() else {
            // Diff without a "diff --git" header (e.g. plain unified diff).
            if line.starts_with("--- ") {
                cur = Some(empty_diff("", ChangeKind::Modified));
                minus_path = header_path(&line[4..]);
            }
            continue;
        };
        if in_hunk {
            match line.as_bytes().first() {
                Some(b' ') | None => {
                    f.hunks.last_mut().unwrap().lines.push(DiffLine {
                        kind: DiffLineKind::Context,
                        old_no: Some(old_no),
                        new_no: Some(new_no),
                        text: line.get(1..).unwrap_or("").to_string(),
                    });
                    old_no += 1;
                    new_no += 1;
                    continue;
                }
                Some(b'+') => {
                    f.additions += 1;
                    f.hunks.last_mut().unwrap().lines.push(DiffLine {
                        kind: DiffLineKind::Add,
                        old_no: None,
                        new_no: Some(new_no),
                        text: line[1..].to_string(),
                    });
                    new_no += 1;
                    continue;
                }
                Some(b'-') => {
                    f.deletions += 1;
                    f.hunks.last_mut().unwrap().lines.push(DiffLine {
                        kind: DiffLineKind::Del,
                        old_no: Some(old_no),
                        new_no: None,
                        text: line[1..].to_string(),
                    });
                    old_no += 1;
                    continue;
                }
                Some(b'\\') => {
                    f.hunks.last_mut().unwrap().lines.push(DiffLine {
                        kind: DiffLineKind::NoNewline,
                        old_no: None,
                        new_no: None,
                        text: line[1..].trim().to_string(),
                    });
                    continue;
                }
                _ => {
                    in_hunk = false;
                }
            }
        }
        if line.starts_with("@@ ") {
            if let Some((os, ol, ns, nl)) = parse_hunk_header(line) {
                f.hunks.push(Hunk {
                    header: line.to_string(),
                    old_start: os,
                    old_lines: ol,
                    new_start: ns,
                    new_lines: nl,
                    lines: vec![],
                });
                old_no = os;
                new_no = ns;
                in_hunk = true;
            }
        } else if line.starts_with("@@@") {
            // Combined diff (conflicted file): not representable; leave hunks empty.
            in_hunk = false;
        } else if let Some(p) = line.strip_prefix("--- ") {
            minus_path = header_path(p);
            if minus_path.is_none() {
                f.status = ChangeKind::Added;
            }
        } else if let Some(p) = line.strip_prefix("+++ ") {
            match header_path(p) {
                Some(p) => {
                    f.path = p;
                    if let Some(m) = &minus_path {
                        if *m != f.path && f.old_path.is_none() {
                            f.old_path = Some(m.clone());
                        }
                    }
                }
                None => {
                    f.status = ChangeKind::Deleted;
                    if let Some(m) = &minus_path {
                        f.path = m.clone();
                    }
                }
            }
        } else if line.starts_with("new file mode") {
            f.status = ChangeKind::Added;
        } else if line.starts_with("deleted file mode") {
            f.status = ChangeKind::Deleted;
        } else if let Some(p) = line.strip_prefix("rename from ") {
            f.status = ChangeKind::Renamed;
            f.old_path = Some(if p.starts_with('"') { unquote_c(p) } else { p.to_string() });
        } else if let Some(p) = line.strip_prefix("rename to ") {
            f.status = ChangeKind::Renamed;
            f.path = if p.starts_with('"') { unquote_c(p) } else { p.to_string() };
        } else if let Some(p) = line.strip_prefix("copy from ") {
            f.status = ChangeKind::Copied;
            f.old_path = Some(if p.starts_with('"') { unquote_c(p) } else { p.to_string() });
        } else if let Some(p) = line.strip_prefix("copy to ") {
            f.status = ChangeKind::Copied;
            f.path = if p.starts_with('"') { unquote_c(p) } else { p.to_string() };
        } else if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            f.binary = true;
        }
    }
    finish(&mut cur, &mut files);
    for f in &mut files {
        if !matches!(f.status, ChangeKind::Renamed | ChangeKind::Copied) && f.old_path.as_deref() == Some(&f.path) {
            f.old_path = None;
        }
    }
    files
}

/// Count +/- lines of a raw diff without building hunks (for oversized diffs).
fn count_changes(text: &str) -> (u32, u32) {
    let (mut a, mut d) = (0, 0);
    let mut in_hunk = false;
    for line in text.lines() {
        if line.starts_with("@@ ") {
            in_hunk = true;
        } else if line.starts_with("diff --git ") {
            in_hunk = false;
        } else if in_hunk {
            if line.starts_with('+') {
                a += 1;
            } else if line.starts_with('-') {
                d += 1;
            }
        }
    }
    (a, d)
}

/// Turn raw diff output for one file into a `FileDiff`, applying the size cap.
pub fn file_diff_from_raw(raw: &[u8], path: &str, default_status: ChangeKind) -> FileDiff {
    let text = String::from_utf8_lossy(raw);
    let too_large = raw.len() > MAX_DIFF_BYTES || bytecount_lines(raw) > MAX_DIFF_LINES;
    if too_large {
        let mut f = parse_header_only(&text, path, default_status);
        let (a, d) = count_changes(&text);
        f.additions = a;
        f.deletions = d;
        f.too_large = true;
        f.hunks.clear();
        return f;
    }
    let mut diffs = parse_diff(&text);
    let mut f = if diffs.is_empty() {
        empty_diff(path, default_status)
    } else {
        // Prefer the entry for the requested path.
        let pos = diffs.iter().position(|d| d.path == path).unwrap_or(0);
        diffs.swap_remove(pos)
    };
    if f.path.is_empty() {
        f.path = path.to_string();
    }
    if default_status == ChangeKind::Untracked {
        f.status = ChangeKind::Untracked;
        f.old_path = None;
    }
    f
}

fn bytecount_lines(raw: &[u8]) -> usize {
    raw.iter().filter(|&&b| b == b'\n').count()
}

fn parse_header_only(text: &str, path: &str, default_status: ChangeKind) -> FileDiff {
    // Parse only the header portion (up to the first hunk) to get status/paths cheaply.
    let end = text.find("\n@@ ").map(|i| i + 1).unwrap_or(text.len().min(64 * 1024));
    let mut head = parse_diff(&text[..end]).into_iter().next().unwrap_or_else(|| empty_diff(path, default_status));
    if head.path.is_empty() {
        head.path = path.to_string();
    }
    if default_status == ChangeKind::Untracked {
        head.status = ChangeKind::Untracked;
    }
    head
}

fn status_letter(s: &str) -> ChangeKind {
    match s.as_bytes().first() {
        Some(b'A') => ChangeKind::Added,
        Some(b'D') => ChangeKind::Deleted,
        Some(b'R') => ChangeKind::Renamed,
        Some(b'C') => ChangeKind::Copied,
        Some(b'T') => ChangeKind::TypeChanged,
        Some(b'U') => ChangeKind::Conflicted,
        _ => ChangeKind::Modified,
    }
}

/// Parse `--name-status -z` output into (status, path, oldPath).
pub fn parse_name_status(text: &str) -> Vec<(ChangeKind, String, Option<String>)> {
    let f = split_nul(text);
    let mut out = Vec::new();
    let mut i = 0;
    while i < f.len() {
        let st = f[i];
        let kind = status_letter(st);
        if matches!(kind, ChangeKind::Renamed | ChangeKind::Copied) && i + 2 < f.len() {
            out.push((kind, f[i + 2].to_string(), Some(f[i + 1].to_string())));
            i += 3;
        } else if i + 1 < f.len() {
            out.push((kind, f[i + 1].to_string(), None));
            i += 2;
        } else {
            break;
        }
    }
    out
}

/// Parse `--numstat -z` output into (additions, deletions, binary, path).
pub fn parse_numstat(text: &str) -> Vec<(u32, u32, bool, String)> {
    let f = split_nul(text);
    let mut out = Vec::new();
    let mut i = 0;
    while i < f.len() {
        let rec = f[i];
        let mut parts = rec.splitn(3, '\t');
        let a = parts.next().unwrap_or("");
        let d = parts.next().unwrap_or("");
        let p = parts.next().unwrap_or("");
        let binary = a == "-" || d == "-";
        let (a, d) = (a.parse().unwrap_or(0), d.parse().unwrap_or(0));
        if p.is_empty() {
            // Rename: "a\td\t\0old\0new"
            if i + 2 < f.len() {
                out.push((a, d, binary, f[i + 2].to_string()));
            }
            i += 3;
        } else {
            out.push((a, d, binary, p.to_string()));
            i += 1;
        }
    }
    out
}

fn is_untracked(git: &Git, path: &str) -> GitResult<bool> {
    let tracked = git.read(["ls-files", "-z", "--", path])?;
    if !tracked.is_empty() {
        return Ok(false);
    }
    Ok(git.dir().join(path).symlink_metadata().is_ok())
}

fn is_unmerged(git: &Git, path: &str) -> GitResult<bool> {
    let text = git.read(["ls-files", "-u", "-z", "--", path])?;
    Ok(!text.is_empty())
}

/// staged: index vs HEAD; unstaged: worktree vs index (untracked files diff against nothing).
pub fn get_working_diff(repo: &Path, path: &str, staged: bool) -> GitResult<FileDiff> {
    let git = Git::new(repo);
    let ro = || Opts { read_only: true, ..Default::default() };
    if staged {
        let unborn = git.rev_parse("HEAD")?.is_none();
        // Find a rename source so the diff shows it as a rename.
        let mut base: Vec<String> = vec!["diff".into(), "--cached".into(), "-M".into(), "--no-color".into(), "--no-ext-diff".into()];
        if unborn {
            base.push(EMPTY_TREE.into());
        }
        let mut ns_args = base.clone();
        ns_args.extend(["--name-status".to_string(), "-z".to_string()]);
        let ns = git.read(&ns_args)?;
        let entry = parse_name_status(&ns).into_iter().find(|(_, p, _)| p == path);
        let mut args = base;
        args.push("--".into());
        if let Some((_, _, Some(old))) = &entry {
            args.push(old.clone());
        }
        args.push(path.to_string());
        let out = git.exec(&args, ro())?.check()?;
        let status = entry.map(|e| e.0).unwrap_or(ChangeKind::Modified);
        return Ok(file_diff_from_raw(&out.stdout, path, status));
    }
    if is_untracked(&git, path)? {
        // Git maps /dev/null specially on every platform, including Windows.
        let out = git.exec(["diff", "--no-index", "--no-color", "--no-ext-diff", "--", "/dev/null", path], ro())?;
        if out.code > 1 {
            return Err(out.error());
        }
        return Ok(file_diff_from_raw(&out.stdout, path, ChangeKind::Untracked));
    }
    if is_unmerged(&git, path)? {
        // Show the conflicted file (with markers) against HEAD as a normal unified diff.
        let out = git.exec(["diff", "--no-color", "--no-ext-diff", "HEAD", "--", path], ro())?.check()?;
        let mut f = file_diff_from_raw(&out.stdout, path, ChangeKind::Conflicted);
        f.status = ChangeKind::Conflicted;
        return Ok(f);
    }
    let out = git.exec(["diff", "--no-color", "--no-ext-diff", "--", path], ro())?.check()?;
    Ok(file_diff_from_raw(&out.stdout, path, ChangeKind::Modified))
}

fn parent_or_empty(git: &Git, id: &str) -> GitResult<(String, String)> {
    let commit = git.require_commit(id)?;
    let parent = git.rev_parse(&format!("{commit}^1"))?.unwrap_or_else(|| EMPTY_TREE.to_string());
    Ok((commit, parent))
}

pub fn get_commit_details(repo: &Path, id: &str) -> GitResult<CommitDetails> {
    let git = Git::new(repo);
    let commit = load_commit(&git, id)?;
    let parent = commit.parents.first().cloned().unwrap_or_else(|| EMPTY_TREE.to_string());
    let ns = git.read(["diff-tree", "-r", "-M", "--no-commit-id", "--name-status", "-z", &parent, &commit.id])?;
    let num = git.read(["diff-tree", "-r", "-M", "--no-commit-id", "--numstat", "-z", &parent, &commit.id])?;
    let stats = parse_numstat(&num);
    let files = parse_name_status(&ns)
        .into_iter()
        .map(|(status, path, old)| {
            let st = stats.iter().find(|s| s.3 == path);
            CommitFile {
                additions: st.map(|s| s.0).unwrap_or(0),
                deletions: st.map(|s| s.1).unwrap_or(0),
                binary: st.map(|s| s.2).unwrap_or(false),
                path,
                old_path: old,
                status,
            }
        })
        .collect();
    Ok(CommitDetails { commit, files })
}

pub fn get_commit_diff(repo: &Path, id: &str, path: &str) -> GitResult<FileDiff> {
    let git = Git::new(repo);
    let (commit, parent) = parent_or_empty(&git, id)?;
    let ns = git.read(["diff-tree", "-r", "-M", "--no-commit-id", "--name-status", "-z", &parent, &commit])?;
    let entry = parse_name_status(&ns).into_iter().find(|(_, p, _)| p == path);
    let Some((status, _, old)) = entry else {
        return Err(GitError::not_found(format!("{path} didn't change in this commit.")));
    };
    let mut args: Vec<String> =
        vec!["diff".into(), "-M".into(), "--no-color".into(), "--no-ext-diff".into(), parent, commit, "--".into()];
    if let Some(o) = &old {
        args.push(o.clone());
    }
    args.push(path.to_string());
    let out = git.exec(&args, Opts { read_only: true, ..Default::default() })?.check()?;
    let mut f = file_diff_from_raw(&out.stdout, path, status);
    f.status = status;
    if old.is_some() {
        f.old_path = old;
    }
    Ok(f)
}
