import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Backend } from './backend'
import type { GitError, ProgressEvent, RepoChangedEvent } from './types'

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args).catch((err: unknown) => {
    throw toGitError(err)
  })
}

export function toGitError(err: unknown): GitError {
  if (err && typeof err === 'object' && 'kind' in err && 'message' in err) {
    return err as GitError
  }
  const text = err instanceof Error ? err.message : String(err)
  return { kind: 'other', message: text, detail: text }
}

export const tauriBackend: Backend = {
  gitVersion: () => call('git_version'),
  openRepo: (path) => call('open_repo', { path }),
  initRepo: (path) => call('init_repo', { path }),
  cloneRepo: (url, dest, opId) => call('clone_repo', { url, dest, opId }),
  watchRepo: (repo) => call('watch_repo', { repo }),
  unwatchRepo: (repo) => call('unwatch_repo', { repo }),
  getRefs: (repo) => call('get_refs', { repo }),
  getLog: (repo, limit) => call('get_log', { repo, limit: limit ?? null }),
  getStatus: (repo) => call('get_status', { repo }),
  getWorkingDiff: (repo, path, staged) => call('get_working_diff', { repo, path, staged }),
  getCommitDetails: (repo, id) => call('get_commit_details', { repo, id }),
  getChangeDiff: (repo, path, context) => call('get_change_diff', { repo, path, context: context ?? null }),
  getCommitDiff: (repo, id, path, context) => call('get_commit_diff', { repo, id, path, context: context ?? null }),
  setFileLines: (repo, path, patch, label) => call('set_file_lines', { repo, path, patch, label }),
  stage: (repo, paths) => call('stage', { repo, paths }),
  unstage: (repo, paths) => call('unstage', { repo, paths }),
  applyPatch: (repo, patch, cached, reverse, label) =>
    call('apply_patch', { repo, patch, cached, reverse, label }),
  commit: (repo, message, amend) => call('commit', { repo, message, amend }),
  checkout: (repo, target) => call('checkout', { repo, target }),
  createBranch: (repo, name, from, checkout) => call('create_branch', { repo, name, from, checkout }),
  renameBranch: (repo, from, to) => call('rename_branch', { repo, from, to }),
  fetch: (repo, remote, opId) => call('fetch', { repo, remote, opId }),
  push: (repo, remote, branch, setUpstream, opId) =>
    call('push', { repo, remote, branch, setUpstream, opId }),
  stashPush: (repo, message, includeUntracked) =>
    call('stash_push', { repo, message, includeUntracked }),
  stashApply: (repo, index, pop) => call('stash_apply', { repo, index, pop }),
  continueOperation: (repo) => call('continue_operation', { repo }),
  abortOperation: (repo) => call('abort_operation', { repo }),
  skipOperation: (repo) => call('skip_operation', { repo }),
  resolveConflict: (repo, path, side) => call('resolve_conflict', { repo, path, side }),
  createTag: (repo, name, commit, message) => call('create_tag', { repo, name, commit, message }),
  deleteTag: (repo, name) => call('delete_tag', { repo, name }),
  addRemote: (repo, name, url) => call('add_remote', { repo, name, url }),
  previewOperation: (repo, op) => call('preview_operation', { repo, op }),
  runOperation: (repo, op, opId) => call('run_operation', { repo, op, opId }),
  getOplog: (repo) => call('get_oplog', { repo }),
  undo: (repo, entryId) => call('undo', { repo, entryId }),
  redo: (repo) => call('redo', { repo }),
  listPullRequests: (repo, remote) => call('list_pull_requests', { repo, remote }),
  openTerminal: (repo) => call('open_terminal', { repo }),
  getIdentity: (repo) => call('get_identity', { repo }),
  setIdentity: (name, email) => call('set_identity', { name, email }),
  startupPaths: () => call('startup_paths'),
  githubStatus: () => call('github_status'),
  githubSetClientId: (clientId) => call('github_set_client_id', { clientId }),
  githubStartLogin: () => call('github_start_login'),
  githubPollLogin: (deviceCode) => call('github_poll_login', { deviceCode }),
  githubSignOut: () => call('github_sign_out'),
  githubListRepos: () => call('github_list_repos'),
  defaultCloneDir: () => call('default_clone_dir'),
  onProgress: (handler) => listen<ProgressEvent>('git-progress', (e) => handler(e.payload)),
  onRepoChanged: (handler) => listen<RepoChangedEvent>('repo-changed', (e) => handler(e.payload)),
}
