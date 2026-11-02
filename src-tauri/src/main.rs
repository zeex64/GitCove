// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  // Git runs this binary as GIT_ASKPASS for github.com credentials while signed in.
  if std::env::var_os("GITCOVE_ASKPASS").is_some() {
    let prompt = std::env::args().nth(1).unwrap_or_default();
    std::process::exit(gitcove_lib::github::askpass(&prompt));
  }
  gitcove_lib::run();
}
