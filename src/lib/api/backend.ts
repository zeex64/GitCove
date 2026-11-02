import type {
  CommitDetails,
  DeviceCode,
  GithubRepo,
  GithubStatus,
  LoginPoll,
  FileDiff,
  LogResult,
  OpEntry,
  OpResult,
  Operation,
  Preview,
  ProgressEvent,
  PullRequestList,
  RepoChangedEvent,
  RepoInfo,
  Refs,
  Status,
} from './types'

/**
 * Every backend command. The Tauri implementation calls `invoke(<snake_case name>, args)`
 * with the camelCase argument object shown; Rust receives the same names via Tauri's
 * automatic camelCase → snake_case argument mapping. `repo` is always the worktree root path.
 * Commands taking `opId` emit `git-progress` events (ProgressEvent) tagged with it.
 */
export interface Backend {
  /** `git_version` — e.g. "2.56.0". Rejects with kind 'gitMissing' when git is not on PATH. */
  gitVersion(): Promise<string>

  /** `open_repo` — resolves the worktree root of any path inside a repository. */
  openRepo(path: string): Promise<RepoInfo>
  /** `init_repo` — `git init` at path, then open it. */
  initRepo(path: string): Promise<RepoInfo>
  /** `clone_repo` — clone `url` into `dest` (a new folder), then open it. */
  cloneRepo(url: string, dest: string, opId: string): Promise<RepoInfo>

  /** `watch_repo` — start emitting `repo-changed` events for this repo (debounced ~300 ms). */
  watchRepo(repo: string): Promise<void>
  /** `unwatch_repo` */
  unwatchRepo(repo: string): Promise<void>

  /** `get_refs` */
  getRefs(repo: string): Promise<Refs>
  /** `get_log` — `limit` defaults to 50000 in Rust when omitted. */
  getLog(repo: string, limit?: number): Promise<LogResult>
  /** `get_status` */
  getStatus(repo: string): Promise<Status>
  /** `get_working_diff` — staged: index vs HEAD; unstaged: worktree vs index (untracked files diff against nothing). */
  getWorkingDiff(repo: string, path: string, staged: boolean): Promise<FileDiff>
  /** `get_commit_details` */
  getCommitDetails(repo: string, id: string): Promise<CommitDetails>
  /** `get_commit_diff` — one file of a commit against its first parent. */
  getCommitDiff(repo: string, id: string, path: string): Promise<FileDiff>

  /** `stage` — `git add -A -- paths` (handles deletions and untracked). */
  stage(repo: string, paths: string[]): Promise<OpResult>
  /** `unstage` — `git restore --staged -- paths` (or `git rm --cached` on an unborn HEAD). */
  unstage(repo: string, paths: string[]): Promise<OpResult>
  /**
   * `apply_patch` — partial staging. `git apply --cached` (cached=true) or a worktree apply
   * (cached=false); `reverse` applies it backwards (unstage lines / discard lines).
   * `label` is the op-log label, e.g. "Stage 3 lines in src/app.ts".
   */
  applyPatch(repo: string, patch: string, cached: boolean, reverse: boolean, label: string): Promise<OpResult>
  /** `commit` — rejects 'nothingToCommit' when the index is empty (unless amending). */
  commit(repo: string, message: string, amend: boolean): Promise<OpResult>
  /** `checkout` — switch to a local branch, a remote branch (creates a tracking branch), or a commit (detached). */
  checkout(repo: string, target: string): Promise<OpResult>
  /** `create_branch` — `from` is a ref or SHA. */
  createBranch(repo: string, name: string, from: string, checkout: boolean): Promise<OpResult>
  /** `rename_branch` */
  renameBranch(repo: string, from: string, to: string): Promise<OpResult>
  /** `fetch` — all remotes when `remote` is null; prunes deleted remote branches. */
  fetch(repo: string, remote: string | null, opId: string): Promise<OpResult>
  /** `push` — never forces (force push goes through `runOperation`). */
  push(repo: string, remote: string, branch: string, setUpstream: boolean, opId: string): Promise<OpResult>
  /** `stash_push` */
  stashPush(repo: string, message: string | null, includeUntracked: boolean): Promise<OpResult>
  /** `stash_apply` — `pop` removes the entry after a clean apply. */
  stashApply(repo: string, index: number, pop: boolean): Promise<OpResult>
  /** `continue_operation` — continue the in-progress rebase / merge / cherry-pick / revert. */
  continueOperation(repo: string): Promise<OpResult>
  /** `abort_operation` */
  abortOperation(repo: string): Promise<OpResult>
  /** `skip_operation` — rebase / cherry-pick only. */
  skipOperation(repo: string): Promise<OpResult>
  /** `resolve_conflict` — take ours / theirs and stage, or just stage ('mark'). */
  resolveConflict(repo: string, path: string, side: 'ours' | 'theirs' | 'mark'): Promise<OpResult>
  /** `create_tag` — annotated when `message` is non-empty. */
  createTag(repo: string, name: string, commit: string, message: string | null): Promise<OpResult>
  /** `delete_tag` */
  deleteTag(repo: string, name: string): Promise<OpResult>
  /** `add_remote` */
  addRemote(repo: string, name: string, url: string): Promise<OpResult>

  /** `preview_operation` — simulate without touching refs, index or worktree. */
  previewOperation(repo: string, op: Operation): Promise<Preview>
  /** `run_operation` — execute the same Operation the preview described. */
  runOperation(repo: string, op: Operation, opId: string): Promise<OpResult>

  /** `get_oplog` — newest first, at most 100 entries. */
  getOplog(repo: string): Promise<OpEntry[]>
  /** `undo` — undo the newest undoable entry, or every entry back to and including `entryId`. */
  undo(repo: string, entryId: string | null): Promise<OpResult>
  /** `redo` — re-apply the most recently undone entry. */
  redo(repo: string): Promise<OpResult>

  /** `list_pull_requests` — open PRs/MRs for a remote, unauthenticated (public repos only for now). */
  listPullRequests(repo: string, remote: string): Promise<PullRequestList>
  /** `open_terminal` — open the user's terminal at the repo root. */
  openTerminal(repo: string): Promise<void>
  /** `startup_paths` — folders passed on the command line or in GITCOVE_OPEN. */
  startupPaths(): Promise<string[]>

  /** `github_status` — who is signed in; a revoked token signs out. */
  githubStatus(): Promise<GithubStatus>
  /** `github_set_client_id` — save the user's OAuth App client ID (device flow must be enabled on it). */
  githubSetClientId(clientId: string): Promise<GithubStatus>
  /** `github_start_login` — begin the device flow; show `userCode` and open `verificationUri`. */
  githubStartLogin(): Promise<DeviceCode>
  /** `github_poll_login` — one poll; call every `interval` seconds until it stops being pending. */
  githubPollLogin(deviceCode: string): Promise<LoginPoll>
  /** `github_sign_out` — forget the token. */
  githubSignOut(): Promise<void>
  /** `github_list_repos` — the user's repositories, most recently pushed first. */
  githubListRepos(): Promise<GithubRepo[]>
  /** `default_clone_dir` — ~/Documents when it exists, else home. */
  defaultCloneDir(): Promise<string>

  onProgress(handler: (e: ProgressEvent) => void): Promise<() => void>
  onRepoChanged(handler: (e: RepoChangedEvent) => void): Promise<() => void>
}
