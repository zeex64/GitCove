//! Rust mirror of `src/lib/api/types.ts`. Field names serialize as camelCase and
//! string unions as the exact TypeScript strings.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RepoState {
    Clean,
    Merging,
    Rebasing,
    CherryPicking,
    Reverting,
    Bisecting,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub path: String,
    pub name: String,
    pub git_dir: String,
    pub is_empty: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Head {
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub detached: bool,
    pub unborn: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub name: String,
    pub commit: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub is_head: bool,
    pub upstream_gone: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBranch {
    pub remote: String,
    pub name: String,
    pub full_name: String,
    pub commit: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HostKind {
    Github,
    Gitlab,
    Forgejo,
    Gitea,
    Bitbucket,
    Other,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
    pub host: HostKind,
    pub web_url: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub name: String,
    pub commit: String,
    pub annotated: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Stash {
    pub index: u32,
    #[serde(rename = "ref")]
    pub ref_: String,
    pub commit: String,
    pub message: String,
    pub time: i64,
    pub branch: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Refs {
    pub head: Head,
    pub branches: Vec<Branch>,
    pub remote_branches: Vec<RemoteBranch>,
    pub remotes: Vec<Remote>,
    pub tags: Vec<Tag>,
    pub stashes: Vec<Stash>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub id: String,
    pub short_id: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub author_time: i64,
    pub committer_name: String,
    pub committer_time: i64,
    pub summary: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogResult {
    pub commits: Vec<Commit>,
    pub truncated: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Untracked,
    Conflicted,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConflictKind {
    BothModified,
    BothAdded,
    BothDeleted,
    AddedByUs,
    AddedByThem,
    DeletedByUs,
    DeletedByThem,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub orig_path: Option<String>,
    pub staged: Option<ChangeKind>,
    pub unstaged: Option<ChangeKind>,
    pub conflict: Option<ConflictKind>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OperationProgress {
    pub kind: RepoState,
    pub branch: Option<String>,
    pub onto: Option<String>,
    pub step: Option<u32>,
    pub total: Option<u32>,
    pub current_commit: Option<String>,
    pub current_summary: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub head: Head,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub state: RepoState,
    pub operation: Option<OperationProgress>,
    pub files: Vec<FileChange>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiffLineKind {
    Context,
    Add,
    Del,
    NoNewline,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: ChangeKind,
    pub binary: bool,
    pub too_large: bool,
    pub additions: u32,
    pub deletions: u32,
    pub hunks: Vec<Hunk>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: ChangeKind,
    pub additions: u32,
    pub deletions: u32,
    pub binary: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetails {
    pub commit: Commit,
    pub files: Vec<CommitFile>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

impl ResetMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ResetMode::Soft => "soft",
            ResetMode::Mixed => "mixed",
            ResetMode::Hard => "hard",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PullMode {
    FfOnly,
    Rebase,
    Merge,
}

/// Every potentially destructive operation.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Operation {
    Rebase { branch: String, onto: String },
    Merge { branch: String },
    Reset { target: String, mode: ResetMode },
    ForcePush { remote: String, branch: String },
    DeleteBranch { name: String, force: bool },
    Discard { paths: Vec<String> },
    CherryPick { commit: String },
    Revert { commit: String },
    Pull { mode: PullMode },
    DropStash { index: u32 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SimulatedCommit {
    pub id: String,
    pub replaces: Option<String>,
    pub parents: Vec<String>,
    pub summary: String,
    pub author_name: String,
    pub author_time: i64,
    pub conflicted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RefMove {
    #[serde(rename = "ref")]
    pub ref_: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewConflicts {
    pub files: Vec<String>,
    pub predicted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub operation: Operation,
    pub title: String,
    pub created: Vec<SimulatedCommit>,
    pub replaced: Vec<String>,
    pub moves: Vec<RefMove>,
    pub conflicts: PreviewConflicts,
    pub needs_stash: bool,
    pub discarded_files: Vec<String>,
    pub remote_commits_lost: Vec<Commit>,
    pub irreversible: bool,
    pub notes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpEntry {
    pub id: String,
    pub time: i64,
    pub kind: String,
    pub label: String,
    pub undoable: bool,
    pub undone: bool,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub entry: Option<OpEntry>,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub op_id: String,
    pub phase: String,
    pub percent: Option<f64>,
    pub line: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum RepoChangeKind {
    Worktree,
    Git,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RepoChangedEvent {
    pub repo: String,
    pub kind: RepoChangeKind,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PullRequestState {
    Open,
    Draft,
    Merged,
    Closed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub state: PullRequestState,
    pub url: String,
    pub source_branch: String,
    pub target_branch: String,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PullRequestListStatus {
    Ok,
    Private,
    Unsupported,
    Offline,
    RateLimited,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestList {
    pub remote: String,
    pub host: HostKind,
    pub status: PullRequestListStatus,
    pub message: Option<String>,
    pub items: Vec<PullRequest>,
}

/// `resolve_conflict` side argument.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConflictSide {
    Ours,
    Theirs,
    Mark,
}
