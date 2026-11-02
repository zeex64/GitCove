//! Opening the user's terminal at the repository root.

use crate::git::error::{GitError, GitErrorKind, GitResult};
use std::path::Path;
use std::process::{Command, Stdio};

fn not_found(detail: String) -> GitError {
    GitError::new(
        GitErrorKind::NotFound,
        "Couldn't find a terminal app to open. Install one, or set the TERMINAL environment variable to your terminal.",
        detail,
    )
}

/// Spawn detached; returns false when the program is missing or exits with an error right away.
fn try_spawn(mut cmd: Command) -> bool {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0000_0008); // DETACHED_PROCESS
    }
    match cmd.spawn() {
        Ok(mut child) => {
            std::thread::sleep(std::time::Duration::from_millis(150));
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => false,
                Ok(Some(_)) => true,
                _ => {
                    // Still running: reap it in the background so it never becomes a zombie.
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                    true
                }
            }
        }
        Err(_) => false,
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn candidates(repo: &Path) -> Vec<Command> {
    let dir = repo.to_string_lossy().into_owned();
    let mut v = Vec::new();
    let mk = |prog: &str, args: Vec<String>| {
        let mut c = Command::new(prog);
        c.args(args).current_dir(repo);
        c
    };
    if let Ok(t) = std::env::var("TERMINAL") {
        let t = t.trim().to_string();
        if !t.is_empty() {
            let mut parts = t.split_whitespace();
            if let Some(prog) = parts.next() {
                v.push(mk(prog, parts.map(str::to_string).collect()));
            }
        }
    }
    v.push(mk("kitty", vec!["--directory".into(), dir.clone()]));
    v.push(mk("alacritty", vec!["--working-directory".into(), dir.clone()]));
    v.push(mk("foot", vec![format!("--working-directory={dir}")]));
    v.push(mk("wezterm", vec!["start".into(), "--cwd".into(), dir.clone()]));
    v.push(mk("ghostty", vec![format!("--working-directory={dir}")]));
    v.push(mk("konsole", vec!["--workdir".into(), dir.clone()]));
    v.push(mk("gnome-terminal", vec![format!("--working-directory={dir}")]));
    v.push(mk("xfce4-terminal", vec![format!("--working-directory={dir}")]));
    v.push(mk("x-terminal-emulator", vec![]));
    v
}

#[cfg(target_os = "macos")]
fn candidates(repo: &Path) -> Vec<Command> {
    let mut c = Command::new("open");
    c.args(["-a", "Terminal"]).arg(repo);
    vec![c]
}

#[cfg(windows)]
fn candidates(repo: &Path) -> Vec<Command> {
    let mut wt = Command::new("wt");
    wt.arg("-d").arg(repo);
    let mut cmd = Command::new("cmd");
    cmd.args(["/c", "start", "cmd", "/K", "cd", "/d"]).arg(repo);
    vec![wt, cmd]
}

pub fn open_terminal(repo: &Path) -> GitResult<()> {
    if !repo.is_dir() {
        return Err(GitError::not_found(format!("The folder {} doesn't exist.", repo.display())));
    }
    let mut tried = Vec::new();
    for cmd in candidates(repo) {
        tried.push(cmd.get_program().to_string_lossy().into_owned());
        if try_spawn(cmd) {
            return Ok(());
        }
    }
    Err(not_found(format!("Tried: {}", tried.join(", "))))
}
