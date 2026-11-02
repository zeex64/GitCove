pub mod commands;
pub mod git;
pub mod github;
pub mod prs;
pub mod terminal;
pub mod watch;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .manage(commands::AppState::default())
    .setup(|app| {
      use tauri::Manager;
      if let Ok(dir) = app.path().app_config_dir() {
        github::init(dir);
      }
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::git_version,
      commands::open_repo,
      commands::init_repo,
      commands::clone_repo,
      commands::watch_repo,
      commands::unwatch_repo,
      commands::get_refs,
      commands::get_log,
      commands::get_status,
      commands::get_working_diff,
      commands::get_commit_details,
      commands::get_commit_diff,
      commands::stage,
      commands::unstage,
      commands::apply_patch,
      commands::commit,
      commands::checkout,
      commands::create_branch,
      commands::rename_branch,
      commands::fetch,
      commands::push,
      commands::stash_push,
      commands::stash_apply,
      commands::continue_operation,
      commands::abort_operation,
      commands::skip_operation,
      commands::resolve_conflict,
      commands::create_tag,
      commands::delete_tag,
      commands::add_remote,
      commands::preview_operation,
      commands::run_operation,
      commands::get_oplog,
      commands::undo,
      commands::redo,
      commands::list_pull_requests,
      commands::open_terminal,
      commands::startup_paths,
      commands::github_status,
      commands::github_set_client_id,
      commands::github_start_login,
      commands::github_poll_login,
      commands::github_sign_out,
      commands::github_list_repos,
      commands::default_clone_dir,
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
