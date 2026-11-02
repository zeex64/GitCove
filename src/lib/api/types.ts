// The contract between the Svelte UI and the Rust backend.
// Rust mirrors every type here with `#[serde(rename_all = "camelCase")]`;
// enum-like string unions serialize as the exact strings below.
// Times are unix seconds. Commit ids are full 40-char SHAs unless named `shortId`.

export type RepoState =
  | 'clean'
  | 'merging'
  | 'rebasing'
  | 'cherryPicking'
  | 'reverting'
  | 'bisecting'

export interface RepoInfo {
  /** Absolute path of the worktree root (`git rev-parse --show-toplevel`). */
  path: string
  /** Folder name of the worktree root. */
  name: string
  gitDir: string
  /** True when HEAD is unborn (no commits yet). */
  isEmpty: boolean
}

export interface Head {
  /** Short branch name; null when detached or unborn without a name. */
  branch: string | null
  /** Full SHA HEAD points at; null when unborn. */
  commit: string | null
  detached: boolean
  unborn: boolean
}

export interface Branch {
  name: string
  commit: string
  /** e.g. "origin/main"; null when no upstream is configured. */
  upstream: string | null
  ahead: number
  behind: number
  isHead: boolean
  /** Upstream is configured but the remote branch no longer exists. */
  upstreamGone: boolean
}

export interface RemoteBranch {
  remote: string
  /** Branch name without the remote prefix, e.g. "main". */
  name: string
  /** e.g. "origin/main". */
  fullName: string
  commit: string
}

export type HostKind = 'github' | 'gitlab' | 'forgejo' | 'gitea' | 'bitbucket' | 'other'

export interface Remote {
  name: string
  fetchUrl: string
  pushUrl: string
  host: HostKind
  /** Browser URL of the repository, e.g. "https://github.com/owner/repo"; null when unknown. */
  webUrl: string | null
  owner: string | null
  repo: string | null
}

export interface Tag {
  name: string
  /** Peeled commit the tag points at. */
  commit: string
  annotated: boolean
}

export interface Stash {
  index: number
  /** e.g. "stash@{0}". */
  ref: string
  commit: string
  message: string
  time: number
  branch: string | null
}

export interface Refs {
  head: Head
  branches: Branch[]
  remoteBranches: RemoteBranch[]
  remotes: Remote[]
  tags: Tag[]
  stashes: Stash[]
}

export interface Commit {
  id: string
  shortId: string
  parents: string[]
  authorName: string
  authorEmail: string
  authorTime: number
  committerName: string
  committerTime: number
  summary: string
  body: string
}

export interface LogResult {
  /** Newest first, `--date-order`, over local branches, remote branches, tags and HEAD. */
  commits: Commit[]
  /** True when the limit cut the history short. */
  truncated: boolean
}

export type ChangeKind =
  | 'added'
  | 'modified'
  | 'deleted'
  | 'renamed'
  | 'copied'
  | 'typeChanged'
  | 'untracked'
  | 'conflicted'

export type ConflictKind =
  | 'bothModified'
  | 'bothAdded'
  | 'bothDeleted'
  | 'addedByUs'
  | 'addedByThem'
  | 'deletedByUs'
  | 'deletedByThem'

export interface FileChange {
  path: string
  /** Previous path for renames and copies. */
  origPath: string | null
  /** Change recorded in the index relative to HEAD; null when nothing is staged. */
  staged: ChangeKind | null
  /** Change in the worktree relative to the index; 'untracked' for untracked files. */
  unstaged: ChangeKind | null
  conflict: ConflictKind | null
}

export interface OperationProgress {
  kind: RepoState
  /** Branch being rebased / merged into. */
  branch: string | null
  /** Rebase target, or the commit being merged / picked. */
  onto: string | null
  step: number | null
  total: number | null
  currentCommit: string | null
  currentSummary: string | null
}

export interface Status {
  head: Head
  upstream: string | null
  ahead: number
  behind: number
  state: RepoState
  operation: OperationProgress | null
  files: FileChange[]
}

export type DiffLineKind = 'context' | 'add' | 'del' | 'noNewline'

export interface DiffLine {
  kind: DiffLineKind
  oldNo: number | null
  newNo: number | null
  /** Line content without the leading +/-/space marker and without the trailing newline. */
  text: string
}

export interface Hunk {
  /** The full "@@ -a,b +c,d @@ section" line. */
  header: string
  oldStart: number
  oldLines: number
  newStart: number
  newLines: number
  lines: DiffLine[]
}

export interface FileDiff {
  path: string
  oldPath: string | null
  status: ChangeKind
  binary: boolean
  /** True when the diff exceeded the size cap; `hunks` is then empty. */
  tooLarge: boolean
  additions: number
  deletions: number
  hunks: Hunk[]
}

export interface CommitFile {
  path: string
  oldPath: string | null
  status: ChangeKind
  additions: number
  deletions: number
  binary: boolean
}

export interface CommitDetails {
  commit: Commit
  /** Changes relative to the first parent (or the empty tree for a root commit). */
  files: CommitFile[]
}

/** Every potentially destructive operation. Previewed with `previewOperation`, run with `runOperation`. */
export type Operation =
  | { kind: 'rebase'; branch: string; onto: string }
  | { kind: 'merge'; branch: string }
  | { kind: 'reset'; target: string; mode: 'soft' | 'mixed' | 'hard' }
  | { kind: 'forcePush'; remote: string; branch: string }
  | { kind: 'deleteBranch'; name: string; force: boolean }
  | { kind: 'discard'; paths: string[] }
  | { kind: 'cherryPick'; commit: string }
  | { kind: 'revert'; commit: string }
  | { kind: 'pull'; mode: 'ffOnly' | 'rebase' | 'merge' }
  | { kind: 'dropStash'; index: number }

export interface SimulatedCommit {
  /** SHA of the simulated commit object (dangling; never referenced). */
  id: string
  /** The existing commit this one replays; null for brand-new commits such as a merge commit. */
  replaces: string | null
  parents: string[]
  summary: string
  authorName: string
  authorTime: number
  /** This step would stop on a conflict. */
  conflicted: boolean
}

export interface RefMove {
  /** Short name, e.g. "feat/undo-log" or "HEAD". */
  ref: string
  from: string | null
  to: string | null
}

export interface Preview {
  operation: Operation
  /** Plain-language title, e.g. "Rebase feat/undo-log onto main". */
  title: string
  /** Commits that will exist afterwards, newest first. Drawn as ghosts on the graph. */
  created: SimulatedCommit[]
  /** Existing commits that stop being reachable from the moved ref. Drawn struck through. */
  replaced: string[]
  moves: RefMove[]
  conflicts: {
    files: string[]
    /** False when git is too old to simulate (merge-tree --merge-base needs 2.40+). */
    predicted: boolean
  }
  /** Uncommitted changes would be stashed before and restored after. */
  needsStash: boolean
  /** Files whose uncommitted changes would be thrown away (reset --hard, discard). Undo still restores them. */
  discardedFiles: string[]
  /** Force push only: commits on the remote that the push would remove. */
  remoteCommitsLost: Commit[]
  /** True when the app cannot undo it afterwards (force push changes the remote). */
  irreversible: boolean
  /** Extra plain-language lines for the readout. */
  notes: string[]
}

export interface OpEntry {
  id: string
  time: number
  /** Machine kind, e.g. "commit", "checkout", "rebase". */
  kind: string
  /** Plain-language label, e.g. "Commit “Snapshot refs before every write”". */
  label: string
  undoable: boolean
  undone: boolean
  /** Why it cannot be undone, when `undoable` is false. */
  reason: string | null
}

/** Returned by every command that changes the repository. */
export interface OpResult {
  entry: OpEntry | null
  /** Short past-tense confirmation for the toast, e.g. "Committed 3 files to main". */
  message: string
}

export interface ProgressEvent {
  opId: string
  /** e.g. "Receiving objects". */
  phase: string
  percent: number | null
  line: string
}

export interface RepoChangedEvent {
  repo: string
  /** 'worktree' for file edits, 'git' for refs/index/HEAD changes. */
  kind: 'worktree' | 'git'
}

export type GitErrorKind =
  | 'notARepo'
  | 'gitMissing'
  | 'auth'
  | 'network'
  | 'conflict'
  | 'rejected'
  | 'divergent'
  | 'dirtyWorktree'
  | 'nothingToCommit'
  | 'notFound'
  | 'locked'
  | 'other'

/** Every command rejects with this shape. */
export interface GitError {
  kind: GitErrorKind
  /** Plain language: what went wrong and how to recover. */
  message: string
  /** Raw git stderr, for the "Show details" disclosure. */
  detail: string
}

export interface PullRequest {
  number: number
  title: string
  author: string
  state: 'open' | 'draft' | 'merged' | 'closed'
  url: string
  sourceBranch: string
  targetBranch: string
  updatedAt: number
}

export interface PullRequestList {
  remote: string
  host: HostKind
  status: 'ok' | 'private' | 'unsupported' | 'offline' | 'rateLimited' | 'error'
  /** Plain-language explanation when status is not 'ok'. */
  message: string | null
  items: PullRequest[]
}

export interface GithubAccount {
  login: string
  name: string | null
  avatarUrl: string | null
  htmlUrl: string | null
}

export interface GithubStatus {
  /** An OAuth App client ID is available (built in or saved by the user). */
  configured: boolean
  clientIdSource: 'builtIn' | 'user' | null
  account: GithubAccount | null
  /** Where the token is kept: the OS keychain, or a private file when no keychain answered. */
  storage: 'keychain' | 'file' | null
  /** The account shown is cached because github.com couldn't be reached. */
  offline: boolean
}

export interface DeviceCode {
  deviceCode: string
  /** The code the user types on GitHub, e.g. "WDJB-MJHT". */
  userCode: string
  verificationUri: string
  expiresIn: number
  /** Seconds to wait between polls. */
  interval: number
}

export interface LoginPoll {
  status: 'pending' | 'slowDown' | 'expired' | 'denied' | 'ok' | 'error'
  message: string | null
  account: GithubAccount | null
  interval: number | null
}

export interface GithubRepo {
  fullName: string
  owner: string
  name: string
  private: boolean
  fork: boolean
  archived: boolean
  description: string | null
  cloneUrl: string
  sshUrl: string
  htmlUrl: string
  pushedAt: number
  defaultBranch: string | null
}
