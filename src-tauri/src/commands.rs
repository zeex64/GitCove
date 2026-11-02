//! Tauri commands. Every command runs its git work on the blocking thread pool; write
//! commands are serialized per repository.

use crate::git::error::{GitError, GitResult};
use crate::git::types::{
    CommitDetails, ConflictSide, FileDiff, Identity, LogResult, OpEntry, OpResult, Operation, Preview, ProgressEvent,
    PullRequestList, Refs, RepoChangedEvent, RepoInfo, Status,
};
use crate::git::{diff, log, operation, oplog, ops, preview, refs, repo as repo_mod, status};
use crate::watch::{self, RepoWatcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

pub const PROGRESS_EVENT: &str = "git-progress";
pub const REPO_CHANGED_EVENT: &str = "repo-changed";

#[derive(Default)]
pub struct AppState {
    locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
    watchers: Mutex<HashMap<PathBuf, RepoWatcher>>,
}

fn key(repo: &str) -> PathBuf {
    let p = PathBuf::from(repo);
    std::fs::canonicalize(&p).unwrap_or(p)
}

impl AppState {
    fn lock_for(&self, repo: &str) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(key(repo)).or_default().clone()
    }
}

async fn blocking<T, F>(f: F) -> GitResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> GitResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| GitError::other(format!("The operation stopped unexpectedly: {e}.")))?
}

/// Run a write under the repository's mutex.
async fn write<T, F>(state: &AppState, repo: &str, f: F) -> GitResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> GitResult<T> + Send + 'static,
{
    let lock = state.lock_for(repo);
    blocking(move || {
        let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        f()
    })
    .await
}

fn emitter(app: &AppHandle) -> Arc<dyn Fn(ProgressEvent) + Send + Sync> {
    let app = app.clone();
    Arc::new(move |ev: ProgressEvent| {
        let _ = app.emit(PROGRESS_EVENT, ev);
    })
}

// ---------------------------------------------------------------------------------------
// Repositories
// ---------------------------------------------------------------------------------------

#[tauri::command]
pub async fn git_version() -> Result<String, GitError> {
    blocking(repo_mod::version_string).await
}

#[tauri::command]
pub async fn open_repo(path: String) -> Result<RepoInfo, GitError> {
    blocking(move || repo_mod::open_repo(&path)).await
}

#[tauri::command]
pub async fn init_repo(state: State<'_, AppState>, path: String) -> Result<RepoInfo, GitError> {
    let p = path.clone();
    write(&state, &path, move || repo_mod::init_repo(&p)).await
}

#[tauri::command]
pub async fn clone_repo(app: AppHandle, url: String, dest: String, op_id: String) -> Result<RepoInfo, GitError> {
    let emit = emitter(&app);
    blocking(move || repo_mod::clone_repo(&url, &dest, &op_id, Some(&*emit))).await
}

#[tauri::command]
pub async fn watch_repo(app: AppHandle, state: State<'_, AppState>, repo: String) -> Result<(), GitError> {
    let k = key(&repo);
    {
        let map = state.watchers.lock().unwrap_or_else(|e| e.into_inner());
        if map.contains_key(&k) {
            return Ok(());
        }
    }
    let app2 = app.clone();
    let sink: watch::ChangeSink = Arc::new(move |ev: RepoChangedEvent| {
        let _ = app2.emit(REPO_CHANGED_EVENT, ev);
    });
    let root = k.clone();
    let repo_key = repo.clone();
    let watcher = blocking(move || watch::start(repo_key, &root, sink)).await?;
    let mut map = state.watchers.lock().unwrap_or_else(|e| e.into_inner());
    map.entry(k).or_insert(watcher);
    Ok(())
}

#[tauri::command]
pub async fn unwatch_repo(state: State<'_, AppState>, repo: String) -> Result<(), GitError> {
    let mut map = state.watchers.lock().unwrap_or_else(|e| e.into_inner());
    map.remove(&key(&repo));
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------------------

#[tauri::command]
pub async fn get_refs(repo: String) -> Result<Refs, GitError> {
    blocking(move || refs::get_refs(Path::new(&repo))).await
}

#[tauri::command]
pub async fn get_log(repo: String, limit: Option<u32>) -> Result<LogResult, GitError> {
    blocking(move || log::get_log(Path::new(&repo), limit)).await
}

#[tauri::command]
pub async fn get_status(repo: String) -> Result<Status, GitError> {
    blocking(move || status::get_status(Path::new(&repo))).await
}

#[tauri::command]
pub async fn get_working_diff(repo: String, path: String, staged: bool) -> Result<FileDiff, GitError> {
    blocking(move || diff::get_working_diff(Path::new(&repo), &path, staged)).await
}

#[tauri::command]
pub async fn get_commit_details(repo: String, id: String) -> Result<CommitDetails, GitError> {
    blocking(move || diff::get_commit_details(Path::new(&repo), &id)).await
}

#[tauri::command]
pub async fn get_commit_diff(repo: String, id: String, path: String, context: Option<u32>) -> Result<FileDiff, GitError> {
    blocking(move || diff::get_commit_diff(Path::new(&repo), &id, &path, context)).await
}

/// One file's change from HEAD to the working tree, each +/- line marked `staged`.
#[tauri::command]
pub async fn get_change_diff(repo: String, path: String, context: Option<u32>) -> Result<FileDiff, GitError> {
    blocking(move || diff::get_change_diff(Path::new(&repo), &path, context)).await
}

#[tauri::command]
pub async fn get_identity(repo: String) -> Result<Identity, GitError> {
    blocking(move || repo_mod::get_identity(Path::new(&repo))).await
}

#[tauri::command]
pub async fn set_identity(name: String, email: String) -> Result<(), GitError> {
    blocking(move || repo_mod::set_identity(&name, &email)).await
}

// ---------------------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------------------

#[tauri::command]
pub async fn stage(state: State<'_, AppState>, repo: String, paths: Vec<String>) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::stage(Path::new(&r), &paths)).await
}

#[tauri::command]
pub async fn unstage(state: State<'_, AppState>, repo: String, paths: Vec<String>) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::unstage(Path::new(&r), &paths)).await
}

#[tauri::command]
pub async fn apply_patch(
    state: State<'_, AppState>,
    repo: String,
    patch: String,
    cached: bool,
    reverse: bool,
    label: String,
) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::apply_patch(Path::new(&r), &patch, cached, reverse, &label)).await
}

/// Make `patch` (against HEAD, or /dev/null for a new file) the staged state of `path`;
/// `None` unstages the whole file.
#[tauri::command]
pub async fn set_file_lines(
    state: State<'_, AppState>,
    repo: String,
    path: String,
    patch: Option<String>,
    label: String,
) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::set_file_lines(Path::new(&r), &path, patch.as_deref(), &label)).await
}

#[tauri::command]
pub async fn commit(state: State<'_, AppState>, repo: String, message: String, amend: bool) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::commit(Path::new(&r), &message, amend)).await
}

#[tauri::command]
pub async fn checkout(state: State<'_, AppState>, repo: String, target: String) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::checkout(Path::new(&r), &target)).await
}

#[tauri::command]
pub async fn create_branch(
    state: State<'_, AppState>,
    repo: String,
    name: String,
    from: String,
    checkout: bool,
) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::create_branch(Path::new(&r), &name, &from, checkout)).await
}

#[tauri::command]
pub async fn rename_branch(state: State<'_, AppState>, repo: String, from: String, to: String) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::rename_branch(Path::new(&r), &from, &to)).await
}

#[tauri::command]
pub async fn fetch(
    app: AppHandle,
    state: State<'_, AppState>,
    repo: String,
    remote: Option<String>,
    op_id: String,
) -> Result<OpResult, GitError> {
    let r = repo.clone();
    let emit = emitter(&app);
    write(&state, &repo, move || ops::fetch(Path::new(&r), remote.as_deref(), &op_id, Some(&*emit))).await
}

#[tauri::command]
pub async fn push(
    app: AppHandle,
    state: State<'_, AppState>,
    repo: String,
    remote: String,
    branch: String,
    set_upstream: bool,
    op_id: String,
) -> Result<OpResult, GitError> {
    let r = repo.clone();
    let emit = emitter(&app);
    write(&state, &repo, move || ops::push(Path::new(&r), &remote, &branch, set_upstream, &op_id, Some(&*emit))).await
}

#[tauri::command]
pub async fn stash_push(
    state: State<'_, AppState>,
    repo: String,
    message: Option<String>,
    include_untracked: bool,
) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::stash_push(Path::new(&r), message.as_deref(), include_untracked)).await
}

#[tauri::command]
pub async fn stash_apply(state: State<'_, AppState>, repo: String, index: u32, pop: bool) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::stash_apply(Path::new(&r), index, pop)).await
}

#[tauri::command]
pub async fn continue_operation(state: State<'_, AppState>, repo: String) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::continue_operation(Path::new(&r))).await
}

#[tauri::command]
pub async fn abort_operation(state: State<'_, AppState>, repo: String) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::abort_operation(Path::new(&r))).await
}

#[tauri::command]
pub async fn skip_operation(state: State<'_, AppState>, repo: String) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::skip_operation(Path::new(&r))).await
}

#[tauri::command]
pub async fn resolve_conflict(
    state: State<'_, AppState>,
    repo: String,
    path: String,
    side: ConflictSide,
) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::resolve_conflict(Path::new(&r), &path, side)).await
}

#[tauri::command]
pub async fn create_tag(
    state: State<'_, AppState>,
    repo: String,
    name: String,
    commit: String,
    message: Option<String>,
) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::create_tag(Path::new(&r), &name, &commit, message.as_deref())).await
}

#[tauri::command]
pub async fn delete_tag(state: State<'_, AppState>, repo: String, name: String) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::delete_tag(Path::new(&r), &name)).await
}

#[tauri::command]
pub async fn add_remote(state: State<'_, AppState>, repo: String, name: String, url: String) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || ops::add_remote(Path::new(&r), &name, &url)).await
}

// ---------------------------------------------------------------------------------------
// Preview / run / undo
// ---------------------------------------------------------------------------------------

#[tauri::command]
pub async fn preview_operation(repo: String, op: Operation) -> Result<Preview, GitError> {
    blocking(move || preview::preview_operation(Path::new(&repo), op)).await
}

#[tauri::command]
pub async fn run_operation(
    app: AppHandle,
    state: State<'_, AppState>,
    repo: String,
    op: Operation,
    op_id: String,
) -> Result<OpResult, GitError> {
    let r = repo.clone();
    let emit = emitter(&app);
    write(&state, &repo, move || operation::run_operation(Path::new(&r), op, &op_id, Some(&*emit))).await
}

#[tauri::command]
pub async fn get_oplog(repo: String) -> Result<Vec<OpEntry>, GitError> {
    blocking(move || oplog::get_oplog(Path::new(&repo))).await
}

#[tauri::command]
pub async fn undo(state: State<'_, AppState>, repo: String, entry_id: Option<String>) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || oplog::undo(Path::new(&r), entry_id.as_deref())).await
}

#[tauri::command]
pub async fn redo(state: State<'_, AppState>, repo: String) -> Result<OpResult, GitError> {
    let r = repo.clone();
    write(&state, &repo, move || oplog::redo(Path::new(&r))).await
}

// ---------------------------------------------------------------------------------------
// Integrations
// ---------------------------------------------------------------------------------------

#[tauri::command]
pub async fn list_pull_requests(repo: String, remote: String) -> Result<PullRequestList, GitError> {
    blocking(move || crate::prs::list_pull_requests(Path::new(&repo), &remote)).await
}

#[tauri::command]
pub async fn open_terminal(repo: String) -> Result<(), GitError> {
    blocking(move || crate::terminal::open_terminal(Path::new(&repo))).await
}

/// Folders named on the command line (`gitcove ~/code/repo`) or in `GITCOVE_OPEN`, to open at launch.
/// Only existing directories are returned, made absolute against the launch directory.
#[tauri::command]
pub async fn startup_paths() -> Result<Vec<String>, GitError> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let from_env = std::env::var("GITCOVE_OPEN").ok().into_iter().flat_map(|v| {
        std::env::split_paths(&v).map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>()
    });
    let paths = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .chain(from_env)
        .map(|a| {
            let p = PathBuf::from(&a);
            if p.is_absolute() { p } else { cwd.join(p) }
        })
        .filter(|p| p.is_dir())
        .map(|p| p.canonicalize().unwrap_or(p).to_string_lossy().into_owned())
        .collect();
    Ok(paths)
}

// ---------------------------------------------------------------------------------------
// GitHub account
// ---------------------------------------------------------------------------------------

use crate::github::{self, DeviceCode, GithubRepo, GithubStatus, LoginPoll};

#[tauri::command]
pub async fn github_status() -> Result<GithubStatus, GitError> {
    blocking(|| Ok(github::status())).await
}

#[tauri::command]
pub async fn github_set_client_id(client_id: String) -> Result<GithubStatus, GitError> {
    blocking(move || github::set_client_id(&client_id)).await
}

#[tauri::command]
pub async fn github_start_login() -> Result<DeviceCode, GitError> {
    blocking(github::start_login).await
}

#[tauri::command]
pub async fn github_poll_login(device_code: String) -> Result<LoginPoll, GitError> {
    blocking(move || github::poll_login(&device_code)).await
}

#[tauri::command]
pub async fn github_sign_out() -> Result<(), GitError> {
    blocking(github::sign_out).await
}

#[tauri::command]
pub async fn github_list_repos() -> Result<Vec<GithubRepo>, GitError> {
    blocking(github::list_repos).await
}

#[tauri::command]
pub async fn default_clone_dir() -> Result<String, GitError> {
    Ok(github::default_clone_dir())
}
