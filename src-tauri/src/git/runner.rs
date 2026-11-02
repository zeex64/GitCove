//! Running the user's `git` binary: environment, I/O plumbing, progress parsing and
//! error mapping. Everything else in the backend goes through [`Git`].

use super::error::{GitError, GitErrorKind, GitResult, classify};
use super::types::ProgressEvent;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Callback used by long-running network commands to report progress.
pub type Emit<'a> = Option<&'a (dyn Fn(ProgressEvent) + Send + Sync)>;

const LITERAL_PATHSPEC_COMMANDS: &[&str] = &["add", "restore", "rm", "ls-files", "diff", "diff-tree", "update-index"];

pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
pub const ZERO_OID: &str = "0000000000000000000000000000000000000000";

#[derive(Debug)]
pub struct Out {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Out {
    pub fn ok(&self) -> bool {
        self.code == 0
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
    /// Turn a non-zero exit into a classified error.
    pub fn check(self) -> GitResult<Out> {
        if self.ok() { Ok(self) } else { Err(self.error()) }
    }
    pub fn error(&self) -> GitError {
        let mut detail = self.stderr.clone();
        if detail.trim().is_empty() {
            detail = String::from_utf8_lossy(&self.stdout).into_owned();
        }
        classify(&detail)
    }
    /// stderr + stdout, for messages git prints to either stream.
    pub fn all_output(&self) -> String {
        format!("{}\n{}", self.stderr, String::from_utf8_lossy(&self.stdout))
    }
}

#[derive(Default)]
pub struct Opts<'a> {
    pub read_only: bool,
    pub stdin: Option<Vec<u8>>,
    pub env: Vec<(String, String)>,
    pub progress: Option<&'a mut dyn FnMut(&str)>,
}

#[derive(Clone, Debug)]
pub struct Git {
    dir: PathBuf,
}

impl Git {
    pub fn new(dir: impl Into<PathBuf>) -> Git {
        Git { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn command<I, S>(&self, args: I, opts: &Opts) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<S> = args.into_iter().collect();
        let sub = args.first().map(|a| a.as_ref().to_string_lossy().into_owned()).unwrap_or_default();
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.dir);
        cmd.args(["-c", "core.quotepath=false", "-c", "color.ui=false", "-c", "core.pager=cat"]);
        // Neutralize user settings that would change the output we parse.
        cmd.args([
            "-c",
            "log.showSignature=false",
            "-c",
            "diff.noprefix=false",
            "-c",
            "diff.mnemonicPrefix=false",
            "-c",
            "diff.relative=false",
            "-c",
            "color.diff=false",
            "-c",
            "color.status=false",
        ]);
        cmd.args(&args);
        for k in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_COMMON_DIR",
            "GIT_PREFIX",
        ] {
            cmd.env_remove(k);
        }
        cmd.env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("LANGUAGE", "C")
            .env("GIT_EDITOR", "true")
            .env("GIT_SEQUENCE_EDITOR", "true")
            .env("GIT_MERGE_AUTOEDIT", "no");
        if opts.read_only {
            cmd.env("GIT_OPTIONAL_LOCKS", "0");
        }
        // Paths we pass are literal file names. Only set this for plumbing-like commands
        // that run no hooks or sub-commands (stash, for one, relies on ":/" magic).
        if LITERAL_PATHSPEC_COMMANDS.contains(&sub.as_str()) {
            cmd.env("GIT_LITERAL_PATHSPECS", "1");
        }
        for (k, v) in &opts.env {
            cmd.env(k, v);
        }
        cmd.stdin(if opts.stdin.is_some() { Stdio::piped() } else { Stdio::null() });
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        cmd
    }

    /// Run git. Only spawn failures are errors; check `Out::code` yourself.
    pub fn exec<I, S>(&self, args: I, mut opts: Opts) -> GitResult<Out>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        if !self.dir.is_dir() {
            return Err(GitError::new(
                GitErrorKind::NotFound,
                format!("The folder {} doesn't exist. It may have been moved or deleted.", self.dir.display()),
                "",
            ));
        }
        let mut cmd = self.command(args, &opts);
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(GitError::git_missing(e.to_string()));
            }
            Err(e) => return Err(GitError::io("Couldn't start Git", e)),
        };

        let stdin_thread = match (child.stdin.take(), opts.stdin.take()) {
            (Some(mut pipe), Some(data)) => Some(std::thread::spawn(move || {
                let _ = pipe.write_all(&data);
                drop(pipe);
            })),
            _ => None,
        };

        let mut stdout_pipe = child.stdout.take().expect("stdout piped");
        let stdout_thread = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout_pipe.read_to_end(&mut buf);
            buf
        });

        let mut stderr_pipe = child.stderr.take().expect("stderr piped");
        let stderr_bytes = if let Some(cb) = opts.progress.as_mut() {
            let mut all = Vec::new();
            let mut pending = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                match stderr_pipe.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        all.extend_from_slice(&chunk[..n]);
                        for &b in &chunk[..n] {
                            if b == b'\r' || b == b'\n' {
                                if !pending.is_empty() {
                                    let line = String::from_utf8_lossy(&pending).into_owned();
                                    cb(&line);
                                    pending.clear();
                                }
                            } else {
                                pending.push(b);
                            }
                        }
                    }
                }
            }
            if !pending.is_empty() {
                cb(&String::from_utf8_lossy(&pending));
            }
            all
        } else {
            let mut buf = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buf);
            buf
        };

        let stdout = stdout_thread.join().unwrap_or_default();
        if let Some(t) = stdin_thread {
            let _ = t.join();
        }
        let status = child.wait().map_err(|e| GitError::io("Git stopped unexpectedly", e))?;
        Ok(Out {
            code: status.code().unwrap_or(-1),
            stdout,
            stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
        })
    }

    /// Read-only command that must succeed; returns stdout as text.
    pub fn read<I, S>(&self, args: I) -> GitResult<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Ok(self.exec(args, Opts { read_only: true, ..Default::default() })?.check()?.text())
    }

    /// Read-only command; `None` when git exits non-zero.
    pub fn try_read<I, S>(&self, args: I) -> GitResult<Option<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let out = self.exec(args, Opts { read_only: true, ..Default::default() })?;
        Ok(if out.ok() { Some(out.text()) } else { None })
    }

    /// Read-only command with stdin input that must succeed.
    pub fn read_stdin<I, S>(&self, args: I, input: Vec<u8>) -> GitResult<Out>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.exec(args, Opts { read_only: true, stdin: Some(input), ..Default::default() })?.check()
    }

    /// Writing command that must succeed; returns stdout as text.
    pub fn write<I, S>(&self, args: I) -> GitResult<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Ok(self.exec(args, Opts::default())?.check()?.text())
    }

    /// Writing command with stdin input that must succeed.
    pub fn write_stdin<I, S>(&self, args: I, input: Vec<u8>) -> GitResult<Out>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.exec(args, Opts { stdin: Some(input), ..Default::default() })?.check()
    }

    /// Writing command whose exit code the caller inspects.
    pub fn run<I, S>(&self, args: I) -> GitResult<Out>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.exec(args, Opts::default())
    }

    /// Resolve a revision to a full object id, or `None`.
    pub fn rev_parse(&self, rev: &str) -> GitResult<Option<String>> {
        if rev.starts_with('-') || rev.is_empty() {
            return Ok(None);
        }
        Ok(self
            .try_read(["rev-parse", "-q", "--verify", rev])?
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()))
    }

    /// Resolve a revision to a commit id, or `None`.
    pub fn commit_id(&self, rev: &str) -> GitResult<Option<String>> {
        self.rev_parse(&format!("{rev}^{{commit}}"))
    }

    /// Resolve a revision to a commit id or fail with a plain notFound error.
    pub fn require_commit(&self, rev: &str) -> GitResult<String> {
        self.commit_id(rev)?
            .ok_or_else(|| GitError::not_found(format!("Couldn't find “{rev}” in this repository. Refresh and try again.")))
    }

    pub fn absolute_git_dir(&self) -> GitResult<PathBuf> {
        let s = self.read(["rev-parse", "--absolute-git-dir"])?;
        Ok(PathBuf::from(s.trim()))
    }

    pub fn is_ancestor(&self, a: &str, b: &str) -> GitResult<bool> {
        let out = self.exec(["merge-base", "--is-ancestor", a, b], Opts { read_only: true, ..Default::default() })?;
        Ok(out.code == 0)
    }

    pub fn config_get(&self, key: &str) -> GitResult<Option<String>> {
        Ok(self.try_read(["config", "--get", key])?.map(|s| s.trim().to_string()))
    }
}

/// Split NUL-separated output, dropping the trailing empty field.
pub fn split_nul(s: &str) -> Vec<&str> {
    let mut v: Vec<&str> = s.split('\0').collect();
    if v.last() == Some(&"") {
        v.pop();
    }
    v
}

pub fn short(sha: &str) -> &str {
    if sha.len() > 7 { &sha[..7] } else { sha }
}

/// Paths as NUL-separated stdin input for `--pathspec-from-file=- --pathspec-file-nul`.
pub fn nul_paths<S: AsRef<str>>(paths: &[S]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in paths {
        v.extend_from_slice(p.as_ref().as_bytes());
        v.push(0);
    }
    v
}

/// Parse a git progress line such as "Receiving objects:  42% (123/456), 1.2 MiB".
pub fn parse_progress(op_id: &str, raw: &str) -> ProgressEvent {
    let line = raw.trim();
    let body = line.strip_prefix("remote:").map(str::trim).unwrap_or(line);
    let mut phase = body.to_string();
    let mut percent = None;
    if let Some(colon) = body.find(':') {
        let (p, rest) = body.split_at(colon);
        let rest = rest[1..].trim_start();
        if let Some(pct) = rest.find('%') {
            if let Ok(n) = rest[..pct].trim().parse::<f64>() {
                percent = Some(n);
                phase = p.trim().to_string();
            }
        } else if !p.contains(' ') || p.split_whitespace().count() <= 3 {
            phase = p.trim().to_string();
        }
    }
    ProgressEvent { op_id: op_id.to_string(), phase, percent, line: line.to_string() }
}

/// Git version as (major, minor, patch, display string). Cached after the first success.
pub fn git_version() -> GitResult<(u32, u32, u32, String)> {
    static CACHE: OnceLock<(u32, u32, u32, String)> = OnceLock::new();
    if let Some(v) = CACHE.get() {
        return Ok(v.clone());
    }
    let cwd = std::env::temp_dir();
    let out = Git::new(cwd).exec(["--version"], Opts { read_only: true, ..Default::default() })?;
    if !out.ok() {
        return Err(GitError::git_missing(out.stderr));
    }
    let text = out.text();
    let ver = text.trim().strip_prefix("git version ").unwrap_or(text.trim()).to_string();
    let mut nums = ver.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).map(|s| s.parse::<u32>().unwrap_or(0));
    let major = nums.next().unwrap_or(0);
    let minor = nums.next().unwrap_or(0);
    let patch = nums.next().unwrap_or(0);
    let display = format!("{major}.{minor}.{patch}");
    let v = (major, minor, patch, display);
    let _ = CACHE.set(v.clone());
    Ok(v)
}

pub fn git_at_least(major: u32, minor: u32) -> bool {
    match git_version() {
        Ok((ma, mi, _, _)) => (ma, mi) >= (major, minor),
        Err(_) => false,
    }
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_lines() {
        let e = parse_progress("x", "Receiving objects:  42% (123/456), 1.2 MiB | 2 MiB/s");
        assert_eq!(e.phase, "Receiving objects");
        assert_eq!(e.percent, Some(42.0));
        let e = parse_progress("x", "remote: Counting objects: 100% (5/5), done.");
        assert_eq!(e.phase, "Counting objects");
        assert_eq!(e.percent, Some(100.0));
        let e = parse_progress("x", "Cloning into 'repo'...");
        assert_eq!(e.percent, None);
    }
}
