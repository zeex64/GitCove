// An in-memory repository for design previews in a plain browser.
// It implements the same Backend contract as the Rust side with synthetic data,
// so every screen and state can be inspected without the desktop shell.
// Scenarios: ?scenario=conflict | empty | detached | clean | welcome | offline

import type { Backend } from './backend'
import type {
  Branch,
  Commit,
  CommitDetails,
  GithubAccount,
  GithubRepo,
  GithubStatus,
  FileChange,
  FileDiff,
  GitError,
  Hunk,
  OpEntry,
  OpResult,
  Operation,
  Preview,
  ProgressEvent,
  Remote,
  RemoteBranch,
  RepoChangedEvent,
  RepoInfo,
  Refs,
  SimulatedCommit,
  Stash,
  Status,
  Tag,
} from './types'

export const DEMO_ROOT = '/home/you/Documents'
const scenario = new URLSearchParams(location.search).get('scenario') ?? ''
// GitHub account previews: ?gh=setup (no client ID yet) | signedin | (default) signed out.
const ghScenario = new URLSearchParams(location.search).get('gh') ?? ''

interface RepoData {
  info: RepoInfo
  commits: Commit[]
  branches: { name: string; commit: string; upstream: string | null }[]
  remotes: RemoteBranch[]
  tags: Tag[]
  stashes: Stash[]
  head: { branch: string | null; commit: string | null }
  files: FileChange[]
  state: Status['state']
  oplog: OpEntry[]
  snapshots: Map<string, { before: string; after: string | null }>
}

const now = () => Math.floor(Date.now() / 1000)
const H = 3600
let idSeq = 0

function sha(seed: string): string {
  let h1 = 0x811c9dc5
  let h2 = 0x01000193
  for (let i = 0; i < seed.length; i++) {
    h1 = Math.imul(h1 ^ seed.charCodeAt(i), 16777619)
    h2 = Math.imul(h2 ^ seed.charCodeAt(i), 2246822519)
  }
  const hex = (n: number) => (n >>> 0).toString(16).padStart(8, '0')
  return (hex(h1) + hex(h2) + hex(h1 ^ h2) + hex(h1 * 31 + h2) + hex(h2 * 17 - h1)).slice(0, 40)
}

function mk(key: string, summary: string, hoursAgo: number, parents: string[], author = 'Alex Kim', body = ''): Commit {
  const id = sha(key)
  const t = now() - Math.round(hoursAgo * H)
  return {
    id,
    shortId: id.slice(0, 7),
    parents,
    authorName: author,
    authorEmail: author === 'Alex Kim' ? 'alex@example.com' : 'bot@renovateapp.com',
    authorTime: t,
    committerName: author,
    committerTime: t,
    summary,
    body,
  }
}

function buildMainRepo(): RepoData {
  const A1 = mk('A1', 'Initial commit', 360, [])
  const A2 = mk('A2', 'Tauri 2 app scaffold', 216, [A1.id])
  const A3 = mk('A3', 'Persist window size and position', 192, [A2.id])
  const A4 = mk('A4', 'Open a repository from a path', 168, [A3.id])
  const A5 = mk('A5', 'Read commits through the git backend', 144, [A4.id])
  const A6 = mk('A6', 'Set up Svelte 5 frontend with Vite', 120, [A5.id])
  const A7 = mk('A7', 'Split diff view into hunks', 96, [A6.id])
  const U1 = mk('U1', 'Add operation log table and migration', 72, [A6.id])
  const G1 = mk('G1', 'First pass at lane allocation', 70, [A6.id])
  const W1 = mk('W1', 'Read scale factor from the active monitor', 48, [A7.id])
  const U2 = mk('U2', 'Snapshot refs before every write', 45, [U1.id])
  const A8 = mk('A8', 'Bump tauri to 2.1.1', 30, [A7.id], 'renovate[bot]')
  const U3 = mk('U3', 'Bind Ctrl+Z to the operation log', 26, [U2.id])
  const U4 = mk(
    'U4',
    'Record undo entries for stash push and pop',
    22,
    [U3.id],
    'Alex Kim',
    'Stash entries are stored by SHA so undo can put them back\neven after the stash list has changed.',
  )
  const G2 = mk('G2', 'Draw commit lanes', 5, [G1.id])
  const W2 = mk('W2', 'Respect GDK_SCALE and per-monitor scale on Wayland', 3, [W1.id])
  const M1 = mk('M1', "Merge branch 'fix/wayland-scaling'", 2, [A8.id, W2.id])
  const commits = [M1, W2, G2, U4, U3, A8, U2, W1, G1, U1, A7, A6, A5, A4, A3, A2, A1]

  const files: FileChange[] = [
    { path: 'src/lib/graph/lanes.ts', origPath: null, staged: null, unstaged: 'modified', conflict: null },
    { path: 'src/routes/+page.svelte', origPath: null, staged: null, unstaged: 'modified', conflict: null },
    { path: 'src-tauri/src/git/oplog.rs', origPath: null, staged: 'added', unstaged: null, conflict: null },
  ]

  const data: RepoData = {
    info: { path: `${DEMO_ROOT}/GitCove`, name: 'GitCove', gitDir: '.git', isEmpty: false },
    commits,
    branches: [
      { name: 'main', commit: M1.id, upstream: 'origin/main' },
      { name: 'feat/undo-log', commit: U4.id, upstream: 'origin/feat/undo-log' },
      { name: 'feat/commit-graph', commit: G2.id, upstream: null },
      { name: 'fix/wayland-scaling', commit: W2.id, upstream: 'origin/fix/wayland-scaling' },
    ],
    remotes: [
      { remote: 'origin', name: 'main', fullName: 'origin/main', commit: M1.id },
      { remote: 'origin', name: 'feat/undo-log', fullName: 'origin/feat/undo-log', commit: U3.id },
      { remote: 'origin', name: 'fix/wayland-scaling', fullName: 'origin/fix/wayland-scaling', commit: W2.id },
    ],
    tags: [{ name: 'v0.1.0', commit: A6.id, annotated: true }],
    stashes: [
      { index: 0, ref: 'stash@{0}', commit: sha('S0'), message: 'WIP on feat/commit-graph: try canvas renderer', time: now() - 50 * H, branch: 'feat/commit-graph' },
    ],
    head: { branch: 'feat/undo-log', commit: U4.id },
    files,
    state: 'clean',
    oplog: [
      { id: 'e3', time: now() - 4 * 60, kind: 'commit', label: 'Commit “Record undo entries for stash push and pop”', undoable: true, undone: false, reason: null },
      { id: 'e2', time: now() - 12 * 60, kind: 'checkout', label: 'Switch to feat/undo-log', undoable: true, undone: false, reason: null },
      { id: 'e1', time: now() - 18 * 60, kind: 'fetch', label: 'Fetch origin', undoable: false, undone: false, reason: "Fetching only updates remote branches; there's nothing to undo." },
      { id: 'e0', time: now() - 2 * H, kind: 'merge', label: 'Merge fix/wayland-scaling into main', undoable: true, undone: false, reason: null },
    ],
    snapshots: new Map(),
  }

  if (scenario === 'clean') data.files = []
  if (scenario === 'detached') data.head = { branch: null, commit: A7.id }
  if (scenario === 'conflict') {
    data.state = 'rebasing'
    data.files = [
      { path: 'src/lib/graph/lanes.ts', origPath: null, staged: null, unstaged: 'conflicted', conflict: 'bothModified' },
      { path: 'src-tauri/src/git/oplog.rs', origPath: null, staged: null, unstaged: 'conflicted', conflict: 'bothModified' },
      { path: 'src/routes/+page.svelte', origPath: null, staged: 'modified', unstaged: null, conflict: null },
    ]
    data.head = { branch: null, commit: M1.id }
  }
  if (scenario === 'empty') {
    data.commits = []
    data.branches = []
    data.remotes = []
    data.tags = []
    data.stashes = []
    data.head = { branch: 'main', commit: null }
    data.info.isEmpty = true
    data.oplog = []
    data.files = [
      { path: 'README.md', origPath: null, staged: null, unstaged: 'untracked', conflict: null },
      { path: 'package.json', origPath: null, staged: null, unstaged: 'untracked', conflict: null },
    ]
  }
  return data
}

function buildSmallRepo(name: string): RepoData {
  const c3 = mk(name + '3', name === 'dotfiles' ? 'hyprland: bind Super+G to git client' : 'Draft: notes on undo design', 30, [])
  const c2 = mk(name + '2', name === 'dotfiles' ? 'fish: add git abbreviations' : 'Publish first post', 80, [])
  c3.parents = [c2.id]
  const c1 = mk(name + '1', 'Initial commit', 400, [])
  c2.parents = [c1.id]
  return {
    info: { path: `${DEMO_ROOT}/${name}`, name, gitDir: '.git', isEmpty: false },
    commits: [c3, c2, c1],
    branches: [{ name: 'main', commit: c3.id, upstream: null }],
    remotes: [],
    tags: [],
    stashes: [],
    head: { branch: 'main', commit: c3.id },
    files: name === 'blog' ? [{ path: 'posts/undo.md', origPath: null, staged: null, unstaged: 'modified', conflict: null }] : [],
    state: 'clean',
    oplog: [],
    snapshots: new Map(),
  }
}

const LANES_DIFF: Hunk[] = [
  {
    header: '@@ -38,9 +38,14 @@ export interface Lane {',
    oldStart: 38,
    oldLines: 9,
    newStart: 38,
    newLines: 14,
    lines: [
      ['context', 'export function allocate(rows: Row[]): Lane[] {'],
      ['del', '  const lanes = new Array(rows.length)'],
      ['add', '  const lanes: Lane[] = []'],
      ['add', '  // The first parent keeps its lane so long-lived branches stay put.'],
      ['add', '  for (const row of rows) {'],
      ['add', '    const slot = lanes.findIndex((l) => l.next === row.id)'],
      ['add', '    if (slot === -1) lanes.push({ next: row.parents[0], color: nextColor() })'],
      ['add', '    else lanes[slot].next = row.parents[0]'],
      ['context', '  }'],
      ['del', '  return lanes.filter(Boolean)'],
      ['add', '  return lanes'],
      ['context', '}'],
      ['context', ''],
      ['context', 'export function laneX(index: number): number {'],
      ['context', '  return 14 + index * 16'],
    ].map(([kind, text]) => ({ kind: kind as 'context', oldNo: null, newNo: null, text })),
  },
  {
    header: '@@ -71,6 +76,8 @@ export function laneX(index: number): number {',
    oldStart: 71,
    oldLines: 6,
    newStart: 76,
    newLines: 8,
    lines: [
      ['context', 'export function colorFor(lane: Lane, main: Set<string>): number {'],
      ['context', '  if (main.has(lane.next)) return 0'],
      ['add', '  // Colour 0 is reserved for the default branch.'],
      ['add', '  if (lane.color === 0) return 1'],
      ['context', '  return lane.color'],
      ['context', '}'],
    ].map(([kind, text]) => ({ kind: kind as 'context', oldNo: null, newNo: null, text })),
  },
]

function numberLines(hunks: Hunk[]): Hunk[] {
  return hunks.map((h) => {
    let o = h.oldStart
    let n = h.newStart
    return {
      ...h,
      lines: h.lines.map((l) => {
        if (l.kind === 'context') return { ...l, oldNo: o++, newNo: n++ }
        if (l.kind === 'del') return { ...l, oldNo: o++, newNo: null }
        if (l.kind === 'add') return { ...l, oldNo: null, newNo: n++ }
        return l
      }),
    }
  })
}

function genericDiff(path: string, status: FileDiff['status']): FileDiff {
  if (status === 'untracked' || status === 'added') {
    const body =
      path.endsWith('.rs')
        ? ['use serde::{Deserialize, Serialize};', '', '#[derive(Serialize, Deserialize, Clone)]', 'pub struct OpEntry {', '    pub id: String,', '    pub time: i64,', '    pub kind: String,', '    pub label: String,', '}']
        : ['# ' + path, '', 'A desktop Git client built with Tauri and Svelte.']
    const hunk: Hunk = {
      header: `@@ -0,0 +1,${body.length} @@`,
      oldStart: 0,
      oldLines: 0,
      newStart: 1,
      newLines: body.length,
      lines: body.map((text) => ({ kind: 'add', oldNo: null, newNo: null, text })),
    }
    return { path, oldPath: null, status, binary: false, tooLarge: false, additions: body.length, deletions: 0, hunks: numberLines([hunk]) }
  }
  const hunk: Hunk = {
    header: '@@ -12,6 +12,7 @@',
    oldStart: 12,
    oldLines: 6,
    newStart: 12,
    newLines: 7,
    lines: [
      ['context', '<script lang="ts">'],
      ['context', "  import History from '$lib/History.svelte'"],
      ['del', "  import Changes from '$lib/Changes.svelte'"],
      ['add', "  import ChangesPanel from '$lib/ChangesPanel.svelte'"],
      ['add', "  import StatusBar from '$lib/StatusBar.svelte'"],
      ['context', '</script>'],
      ['context', ''],
    ].map(([kind, text]) => ({ kind: kind as 'context', oldNo: null, newNo: null, text })),
  }
  return { path, oldPath: null, status, binary: false, tooLarge: false, additions: 2, deletions: 1, hunks: numberLines([hunk]) }
}

function err(kind: GitError['kind'], message: string): GitError {
  return { kind, message, detail: message }
}

const delay = (ms = 60) => new Promise((r) => setTimeout(r, ms))

export function createDemoBackend(): Backend {
  const repos = new Map<string, RepoData>()
  for (const r of [buildMainRepo(), buildSmallRepo('dotfiles'), buildSmallRepo('blog')]) repos.set(r.info.path, r)
  const progressHandlers = new Set<(e: ProgressEvent) => void>()
  /** Lines picked into the index per repo+path, as `add:text` / `del:text`. */
  const lineState = new Map<string, Set<string>>()
  const demoAccount: GithubAccount = { login: 'alexkim', name: 'Alex Kim', avatarUrl: null, htmlUrl: 'https://github.com/alexkim' }
  const gh = { configured: ghScenario !== 'setup', account: ghScenario === 'signedin' ? demoAccount : null, polls: 0 }
  const ghStatus = (): GithubStatus => ({
    configured: gh.configured,
    clientIdSource: gh.configured ? 'user' : null,
    account: gh.account,
    storage: gh.account ? 'keychain' : null,
    offline: false,
  })
  const ghRepos: GithubRepo[] = [
    ['alexkim', 'GitCove', 'A cross-platform desktop Git client.', true, 2],
    ['alexkim', 'dotfiles', 'Hyprland, fish and Neovim config.', false, 30],
    ['alexkim', 'blog', 'Notes and posts.', false, 80],
    ['tauri-apps', 'tauri', 'Build smaller, faster, and more secure desktop applications with a web frontend.', false, 5],
    ['alexkim', 'pkgbuilds', 'PKGBUILDs for CachyOS packages I maintain.', false, 200],
    ['acme-studio', 'design-tokens', 'Shared colour and type tokens.', true, 50],
    ['acme-studio', 'api-gateway', null, true, 400],
    ['alexkim', 'advent-of-code', 'Solutions in Rust.', false, 900],
    ['alexkim', 'old-site', 'The previous personal site.', false, 4000],
  ].map(([owner, name, description, isPrivate, hours]) => ({
    fullName: `${owner}/${name}`,
    owner: owner as string,
    name: name as string,
    private: isPrivate as boolean,
    fork: name === 'tauri',
    archived: name === 'old-site',
    description: description as string | null,
    cloneUrl: `https://github.com/${owner}/${name}.git`,
    sshUrl: `git@github.com:${owner}/${name}.git`,
    htmlUrl: `https://github.com/${owner}/${name}`,
    pushedAt: now() - (hours as number) * H,
    defaultBranch: 'main',
  }))
  const changedHandlers = new Set<(e: RepoChangedEvent) => void>()

  const get = (path: string): RepoData => {
    const r = repos.get(path)
    if (!r) throw err('notARepo', "That folder isn't a Git repository.")
    return r
  }

  const serialize = (r: RepoData) =>
    JSON.stringify({ commits: r.commits, branches: r.branches, remotes: r.remotes, tags: r.tags, stashes: r.stashes, head: r.head, files: r.files, state: r.state })

  const restore = (r: RepoData, json: string) => Object.assign(r, JSON.parse(json))

  const record = (r: RepoData, before: string, kind: string, label: string, message: string, undoable = true): OpResult => {
    const entry: OpEntry = { id: `d${++idSeq}`, time: now(), kind, label, undoable, undone: false, reason: undoable ? null : "This can't be undone here." }
    r.oplog = [entry, ...r.oplog.filter((e) => !e.undone)].slice(0, 100)
    r.snapshots.set(entry.id, { before, after: serialize(r) })
    return { entry, message }
  }

  const byId = (r: RepoData) => new Map(r.commits.map((c) => [c.id, c]))

  const ancestors = (r: RepoData, tip: string | null): Set<string> => {
    const map = byId(r)
    const seen = new Set<string>()
    const stack = tip ? [tip] : []
    while (stack.length) {
      const id = stack.pop()!
      if (seen.has(id)) continue
      seen.add(id)
      map.get(id)?.parents.forEach((p) => stack.push(p))
    }
    return seen
  }

  const resolve = (r: RepoData, ref: string): string => {
    const b = r.branches.find((x) => x.name === ref)
    if (b) return b.commit
    const rb = r.remotes.find((x) => x.fullName === ref)
    if (rb) return rb.commit
    const c = r.commits.find((x) => x.id === ref || x.id.startsWith(ref))
    if (c) return c.id
    throw err('notFound', `Couldn't find “${ref}”.`)
  }

  const ordered = (r: RepoData, ids: Set<string>) => r.commits.filter((c) => ids.has(c.id))

  const ab = (r: RepoData, a: string, b: string) => {
    const A = ancestors(r, a)
    const B = ancestors(r, b)
    return { ahead: [...A].filter((x) => !B.has(x)).length, behind: [...B].filter((x) => !A.has(x)).length }
  }

  const sortCommits = (r: RepoData) => r.commits.sort((x, y) => y.committerTime - x.committerTime)

  const replay = (r: RepoData, list: Commit[], onto: string, simulate: boolean): SimulatedCommit[] => {
    const out: SimulatedCommit[] = []
    let parent = onto
    const base = now()
    ;[...list].reverse().forEach((c, i) => {
      const id = sha(`${c.id}:${onto}:${simulate}`)
      out.unshift({ id, replaces: c.id, parents: [parent], summary: c.summary, authorName: c.authorName, authorTime: c.authorTime, conflicted: false })
      if (!simulate) r.commits.push({ ...c, id, shortId: id.slice(0, 7), parents: [parent], committerTime: base + i })
      parent = id
    })
    return out
  }

  const headCommit = (r: RepoData) => r.head.commit

  const moveHeadBranch = (r: RepoData, to: string) => {
    const b = r.branches.find((x) => x.name === r.head.branch)
    if (b) b.commit = to
    r.head.commit = to
  }

  const preview = (r: RepoData, op: Operation): Preview => {
    const base: Preview = { operation: op, title: '', created: [], replaced: [], moves: [], conflicts: { files: [], predicted: true }, needsStash: false, discardedFiles: [], remoteCommitsLost: [], irreversible: false, notes: [] }
    const dirty = r.files.some((f) => f.unstaged !== 'untracked')
    switch (op.kind) {
      case 'rebase': {
        const tip = resolve(r, op.branch)
        const onto = resolve(r, op.onto)
        const base2 = ancestors(r, onto)
        const list = ordered(r, new Set([...ancestors(r, tip)].filter((x) => !base2.has(x)))).filter((c) => c.parents.length < 2)
        const created = replay(r, list, onto, true)
        const ontoName = r.branches.find((b) => b.commit === onto)?.name ?? onto.slice(0, 7)
        return {
          ...base,
          title: `Rebase ${op.branch} onto ${ontoName}`,
          created,
          replaced: list.map((c) => c.id),
          moves: [{ ref: op.branch, from: tip, to: created[0]?.id ?? onto }],
          needsStash: dirty && r.head.branch === op.branch,
        }
      }
      case 'merge': {
        const theirs = resolve(r, op.branch)
        const ours = headCommit(r)!
        if (ancestors(r, theirs).has(ours)) {
          return { ...base, title: `Fast-forward ${r.head.branch} to ${op.branch}`, moves: [{ ref: r.head.branch ?? 'HEAD', from: ours, to: theirs }], notes: ['Fast-forward: no merge commit.'] }
        }
        const id = sha(`merge:${ours}:${theirs}`)
        return {
          ...base,
          title: `Merge ${op.branch} into ${r.head.branch}`,
          created: [{ id, replaces: null, parents: [ours, theirs], summary: `Merge branch '${op.branch}'`, authorName: 'Alex Kim', authorTime: now(), conflicted: false }],
          moves: [{ ref: r.head.branch ?? 'HEAD', from: ours, to: id }],
        }
      }
      case 'reset': {
        const target = resolve(r, op.target)
        const keep = ancestors(r, target)
        const lost = [...ancestors(r, headCommit(r))].filter((x) => !keep.has(x))
        return {
          ...base,
          title: `Reset ${r.head.branch ?? 'HEAD'} to ${target.slice(0, 7)} (${op.mode})`,
          replaced: lost,
          moves: [{ ref: r.head.branch ?? 'HEAD', from: headCommit(r), to: target }],
          discardedFiles: op.mode === 'hard' ? r.files.filter((f) => f.unstaged !== 'untracked').map((f) => f.path) : [],
          notes: op.mode === 'soft' ? ['Changes from the removed commits stay staged.'] : op.mode === 'mixed' ? ['Changes from the removed commits stay in your files, unstaged.'] : [],
        }
      }
      case 'forcePush': {
        const local = resolve(r, op.branch)
        const remote = r.remotes.find((x) => x.fullName === `${op.remote}/${op.branch}`)
        const keep = ancestors(r, local)
        const lost = remote ? ordered(r, new Set([...ancestors(r, remote.commit)].filter((x) => !keep.has(x)))) : []
        return { ...base, title: `Force push ${op.branch} to ${op.remote}`, remoteCommitsLost: lost, irreversible: true, moves: [{ ref: `${op.remote}/${op.branch}`, from: remote?.commit ?? null, to: local }], notes: [`Based on what was last fetched from ${op.remote}.`] }
      }
      case 'deleteBranch': {
        const tip = resolve(r, op.name)
        const others = new Set<string>()
        for (const b of r.branches) if (b.name !== op.name) ancestors(r, b.commit).forEach((x) => others.add(x))
        for (const rb of r.remotes) ancestors(r, rb.commit).forEach((x) => others.add(x))
        const only = [...ancestors(r, tip)].filter((x) => !others.has(x))
        return { ...base, title: `Delete branch ${op.name}`, replaced: only, moves: [{ ref: op.name, from: tip, to: null }], notes: only.length ? [`${only.length} commits exist only on this branch.`] : ['Every commit on this branch is also on another branch.'] }
      }
      case 'discard':
        return { ...base, title: `Discard changes in ${op.paths.length === 1 ? op.paths[0] : `${op.paths.length} files`}`, discardedFiles: op.paths, notes: ['You can undo this.'] }
      case 'cherryPick':
      case 'revert': {
        const c = byId(r).get(resolve(r, op.commit))!
        const id = sha(`${op.kind}:${c.id}:${headCommit(r)}`)
        const summary = op.kind === 'revert' ? `Revert "${c.summary}"` : c.summary
        return { ...base, title: `${op.kind === 'revert' ? 'Revert' : 'Cherry-pick'} “${c.summary}”`, created: [{ id, replaces: null, parents: [headCommit(r)!], summary, authorName: c.authorName, authorTime: now(), conflicted: false }], moves: [{ ref: r.head.branch ?? 'HEAD', from: headCommit(r), to: id }] }
      }
      case 'pull': {
        const b = r.branches.find((x) => x.name === r.head.branch)
        if (!b?.upstream) throw err('notFound', 'This branch has no upstream.')
        return preview(r, op.mode === 'merge' ? { kind: 'merge', branch: b.upstream } : { kind: 'rebase', branch: b.name, onto: b.upstream })
      }
      case 'dropStash':
        return { ...base, title: `Drop stash@{${op.index}}`, notes: ['You can undo this.'] }
    }
  }

  const run = (r: RepoData, op: Operation): OpResult => {
    const before = serialize(r)
    const p = preview(r, op)
    switch (op.kind) {
      case 'rebase': {
        const tip = resolve(r, op.branch)
        const onto = resolve(r, op.onto)
        const base2 = ancestors(r, onto)
        const list = ordered(r, new Set([...ancestors(r, tip)].filter((x) => !base2.has(x)))).filter((c) => c.parents.length < 2)
        const created = replay(r, list, onto, false)
        const b = r.branches.find((x) => x.name === op.branch)!
        b.commit = created[0]?.id ?? onto
        if (r.head.branch === op.branch) r.head.commit = b.commit
        sortCommits(r)
        return record(r, before, 'rebase', p.title, `Rebased ${op.branch} (${created.length} commits)`)
      }
      case 'merge': {
        const created = p.created[0]
        if (created) {
          r.commits.unshift({ ...byId(r).get(headCommit(r)!)!, id: created.id, shortId: created.id.slice(0, 7), parents: created.parents, summary: created.summary, body: '', authorName: 'Alex Kim', authorTime: now(), committerTime: now() })
          moveHeadBranch(r, created.id)
        } else moveHeadBranch(r, p.moves[0].to!)
        return record(r, before, 'merge', p.title, `Merged ${op.branch}`)
      }
      case 'reset':
        moveHeadBranch(r, p.moves[0].to!)
        if (op.mode === 'hard') r.files = r.files.filter((f) => f.unstaged === 'untracked')
        return record(r, before, 'reset', p.title, `Reset to ${p.moves[0].to!.slice(0, 7)}`)
      case 'forcePush': {
        const rb = r.remotes.find((x) => x.fullName === `${op.remote}/${op.branch}`)
        if (rb) rb.commit = resolve(r, op.branch)
        return record(r, before, 'forcePush', p.title, `Force pushed ${op.branch}`, false)
      }
      case 'deleteBranch':
        r.branches = r.branches.filter((b) => b.name !== op.name)
        return record(r, before, 'deleteBranch', p.title, `Deleted ${op.name}`)
      case 'discard':
        r.files = r.files.filter((f) => !op.paths.includes(f.path) || f.staged).map((f) => (op.paths.includes(f.path) ? { ...f, unstaged: null } : f))
        return record(r, before, 'discard', p.title, `Discarded changes in ${op.paths.length === 1 ? op.paths[0] : `${op.paths.length} files`}`)
      case 'cherryPick':
      case 'revert': {
        const c = p.created[0]
        r.commits.unshift({ ...byId(r).get(headCommit(r)!)!, id: c.id, shortId: c.id.slice(0, 7), parents: c.parents, summary: c.summary, body: '', authorTime: now(), committerTime: now() })
        moveHeadBranch(r, c.id)
        return record(r, before, op.kind, p.title, op.kind === 'revert' ? 'Reverted 1 commit' : 'Cherry-picked 1 commit')
      }
      case 'pull': {
        const b = r.branches.find((x) => x.name === r.head.branch)!
        if (op.mode === 'ffOnly') {
          const up = resolve(r, b.upstream!)
          if (!ancestors(r, up).has(b.commit)) throw err('divergent', 'Your branch and its upstream have both changed. Choose rebase or merge.')
          moveHeadBranch(r, up)
          return record(r, before, 'pull', `Pull ${b.upstream}`, `Pulled ${b.upstream}`)
        }
        return run(r, op.mode === 'merge' ? { kind: 'merge', branch: b.upstream! } : { kind: 'rebase', branch: b.name, onto: b.upstream! })
      }
      case 'dropStash':
        r.stashes = r.stashes.filter((s) => s.index !== op.index).map((s, i) => ({ ...s, index: i, ref: `stash@{${i}}` }))
        return record(r, before, 'dropStash', p.title, 'Dropped stash')
    }
  }

  const emitChanged = (repo: string) => setTimeout(() => changedHandlers.forEach((h) => h({ repo, kind: 'git' })), 0)

  const write = async (repo: string, fn: (r: RepoData) => OpResult): Promise<OpResult> => {
    await delay(80)
    const r = get(repo)
    const result = fn(r)
    emitChanged(repo)
    return result
  }

  const progress = async (opId: string, phases: string[]) => {
    for (const phase of phases) {
      for (let p = 0; p <= 100; p += 25) {
        progressHandlers.forEach((h) => h({ opId, phase, percent: p, line: `${phase}: ${p}%` }))
        await delay(70)
      }
    }
  }

  const backend: Backend = {
    async gitVersion() {
      return '2.56.0'
    },
    async openRepo(path) {
      await delay()
      const r = repos.get(path.replace(/\/+$/, ''))
      if (!r) throw err('notARepo', "That folder isn't a Git repository.")
      return r.info
    },
    async initRepo(path) {
      const name = path.split('/').pop() || 'repo'
      const r = buildSmallRepo(name)
      r.commits = []
      r.branches = []
      r.head = { branch: 'main', commit: null }
      r.info = { path, name, gitDir: '.git', isEmpty: true }
      repos.set(path, r)
      return r.info
    },
    async cloneRepo(url, dest, opId) {
      await progress(opId, ['Receiving objects', 'Resolving deltas'])
      const name = dest.split('/').pop() || 'repo'
      const r = buildSmallRepo(name)
      r.info = { path: dest, name, gitDir: '.git', isEmpty: false }
      r.remotes = [{ remote: 'origin', name: 'main', fullName: 'origin/main', commit: r.commits[0].id }]
      r.branches[0].upstream = 'origin/main'
      repos.set(dest, r)
      void url
      return r.info
    },
    async watchRepo() {},
    async unwatchRepo() {},
    async getRefs(repo): Promise<Refs> {
      await delay(20)
      const r = get(repo)
      const branches: Branch[] = r.branches.map((b) => {
        const up = b.upstream ? r.remotes.find((x) => x.fullName === b.upstream) : null
        const counts = up ? ab(r, b.commit, up.commit) : { ahead: 0, behind: 0 }
        return { name: b.name, commit: b.commit, upstream: up ? b.upstream : null, ...counts, isHead: r.head.branch === b.name, upstreamGone: !!b.upstream && !up }
      })
      return {
        head: { branch: r.head.branch, commit: r.head.commit, detached: !r.head.branch && !!r.head.commit, unborn: !r.head.commit },
        branches,
        remoteBranches: r.remotes,
        remotes: (
          r.remotes.length || r.info.name === 'GitCove'
            ? [
                { name: 'origin', fetchUrl: 'git@github.com:you/GitCove.git', pushUrl: 'git@github.com:you/GitCove.git', host: 'github', webUrl: 'https://github.com/you/GitCove', owner: 'you', repo: 'GitCove' },
                { name: 'codeberg', fetchUrl: 'https://codeberg.org/you/GitCove.git', pushUrl: 'https://codeberg.org/you/GitCove.git', host: 'forgejo', webUrl: 'https://codeberg.org/you/GitCove', owner: 'you', repo: 'GitCove' },
              ].slice(0, scenario === 'empty' ? 0 : 2)
            : []
        ) as Remote[],
        tags: r.tags,
        stashes: r.stashes,
      }
    },
    async getLog(repo) {
      await delay(30)
      return { commits: [...get(repo).commits], truncated: false }
    },
    async getStatus(repo): Promise<Status> {
      await delay(15)
      const r = get(repo)
      const b = r.branches.find((x) => x.name === r.head.branch)
      const up = b?.upstream ? r.remotes.find((x) => x.fullName === b.upstream) : null
      const counts = b && up ? ab(r, b.commit, up.commit) : { ahead: 0, behind: 0 }
      const map = byId(r)
      return {
        head: { branch: r.head.branch, commit: r.head.commit, detached: !r.head.branch && !!r.head.commit, unborn: !r.head.commit },
        upstream: up ? b!.upstream : null,
        ...counts,
        state: r.state,
        operation:
          r.state === 'rebasing'
            ? { kind: 'rebasing', branch: 'feat/undo-log', onto: 'main', step: 2, total: 4, currentCommit: r.commits[6]?.id ?? null, currentSummary: map.get(r.commits[6]?.id)?.summary ?? null }
            : null,
        files: r.files,
      }
    },
    async getWorkingDiff(repo, path, staged) {
      await delay(25)
      const f = get(repo).files.find((x) => x.path === path)
      if (!f) throw err('notFound', `${path} has no changes.`)
      if (path === 'src/lib/graph/lanes.ts' && !staged) {
        const hunks = numberLines(LANES_DIFF)
        if (f.conflict) {
          const stopped = get(repo).commits[6]
          // As git shows it mid-rebase: the file with markers against HEAD, so "yours" is context.
          const lines: [string, string][] = [
            ['context', 'export function allocate(rows: Row[]): Lane[] {'],
            ['add', '<<<<<<< HEAD (main)'],
            ['context', '  const lanes = new Array(rows.length)'],
            ['add', '======='],
            ['add', '  const lanes: Lane[] = []'],
            ['add', '  // The first parent keeps its lane so long-lived branches stay put.'],
            ['add', `>>>>>>> ${stopped.shortId} (${stopped.summary})`],
            ['context', '  for (const row of rows) {'],
            ['context', '    const slot = lanes.findIndex((l) => l.next === row.id)'],
          ]
          const conflictHunk: Hunk = {
            header: '@@ -38,4 +38,9 @@ export interface Lane {',
            oldStart: 38,
            oldLines: 4,
            newStart: 38,
            newLines: 9,
            lines: lines.map(([kind, text]) => ({ kind: kind as 'context', oldNo: null, newNo: null, text })),
          }
          return { path, oldPath: null, status: 'conflicted', binary: false, tooLarge: false, additions: 5, deletions: 0, hunks: numberLines([conflictHunk]) }
        }
        return { path, oldPath: null, status: 'modified', binary: false, tooLarge: false, additions: 9, deletions: 2, hunks }
      }
      const kind = (staged ? f.staged : f.unstaged) ?? 'modified'
      return genericDiff(path, kind === 'conflicted' ? 'modified' : kind)
    },
    async getChangeDiff(repo, path) {
      const f = get(repo).files.find((x) => x.path === path)
      if (!f) throw err('notFound', `${path} has no changes.`)
      const d = await this.getWorkingDiff(repo, path, !f.unstaged)
      if (f.conflict) return d
      const picked = lineState.get(repo + '\0' + path)
      let n = 0
      const hunks = d.hunks.map((h) => ({
        ...h,
        lines: h.lines.map((l) => {
          if (l.kind !== 'add' && l.kind !== 'del') return l
          // A partly staged file without recorded picks stages every other line.
          const staged = picked ? picked.has(`${l.kind}:${l.text}`) : !f.staged ? false : !f.unstaged ? true : n++ % 2 === 0
          return { ...l, staged }
        }),
      }))
      return { ...d, hunks, status: f.unstaged === 'untracked' && !f.staged ? 'untracked' : d.status }
    },
    async getCommitDetails(repo, id): Promise<CommitDetails> {
      await delay(40)
      const c = byId(get(repo)).get(id)
      if (!c) throw err('notFound', 'Commit not found.')
      const seed = parseInt(id.slice(0, 4), 16)
      const pool = ['src/lib/graph/lanes.ts', 'src/lib/state/repo.svelte.ts', 'src-tauri/src/git/oplog.rs', 'src-tauri/src/git/runner.rs', 'src/app.css', 'README.md', 'package.json']
      const n = 1 + (seed % 4)
      const files = Array.from({ length: n }, (_, i) => {
        const path = pool[(seed + i * 3) % pool.length]
        return { path, oldPath: null, status: (i === 0 && seed % 5 === 0 ? 'added' : 'modified') as FileDiff['status'], additions: 3 + ((seed >> i) % 60), deletions: (seed >> (i + 2)) % 18, binary: false }
      }).filter((f, i, all) => all.findIndex((x) => x.path === f.path) === i)
      return { commit: c, files }
    },
    async getCommitDiff(_repo, _id, path) {
      await delay(30)
      return path === 'src/lib/graph/lanes.ts' ? { path, oldPath: null, status: 'modified', binary: false, tooLarge: false, additions: 9, deletions: 2, hunks: numberLines(LANES_DIFF) } : genericDiff(path, 'modified')
    },
    stage: (repo, paths) =>
      write(repo, (r) => {
        for (const p of paths) lineState.delete(repo + '\0' + p)
        const before = serialize(r)
        r.files = r.files.map((f) => (paths.includes(f.path) ? { ...f, staged: f.unstaged === 'untracked' ? 'added' : (f.unstaged ?? f.staged), unstaged: null, conflict: null } : f))
        return record(r, before, 'stage', paths.length === 1 ? `Stage ${paths[0]}` : `Stage ${paths.length} files`, 'Staged')
      }),
    unstage: (repo, paths) =>
      write(repo, (r) => {
        for (const p of paths) lineState.delete(repo + '\0' + p)
        const before = serialize(r)
        r.files = r.files.map((f) => (paths.includes(f.path) ? { ...f, unstaged: f.staged === 'added' && f.path !== 'src-tauri/src/git/oplog.rs' ? 'untracked' : (f.staged ?? f.unstaged), staged: null } : f))
        return record(r, before, 'unstage', paths.length === 1 ? `Unstage ${paths[0]}` : `Unstage ${paths.length} files`, 'Unstaged')
      }),
    applyPatch: (repo, patch, cached, reverse, label) =>
      write(repo, (r) => {
        const before = serialize(r)
        const path = /^diff --git a\/\S+ b\/(\S+)/m.exec(patch)?.[1]
        r.files = r.files.map((f) => {
          if (f.path !== path) return f
          if (!cached) return f
          return reverse ? { ...f, unstaged: f.unstaged ?? 'modified' } : { ...f, staged: f.staged ?? 'modified' }
        })
        return record(r, before, cached ? 'stage' : 'discard', label, label.replace(/^(\w+)/, (w) => w + 'd').replace('Staged', 'Staged').replace('Discardd', 'Discarded'))
      }),
    setFileLines: (repo, path, patch, label) =>
      write(repo, (r) => {
        const before = serialize(r)
        const key = repo + '\0' + path
        const lines = (patch ?? '').split('\n').filter((l) => /^[+-]/.test(l) && !/^(\+\+\+|---) /.test(l))
        if (!lines.length) lineState.delete(key)
        else lineState.set(key, new Set(lines.map((l) => `${l[0] === '+' ? 'add' : 'del'}:${l.slice(1)}`)))
        r.files = r.files.map((f) => {
          if (f.path !== path) return f
          const kind = f.unstaged ?? f.staged ?? 'modified'
          if (!lines.length) return { ...f, staged: null, unstaged: kind === 'added' ? 'untracked' : kind }
          return { ...f, staged: f.staged ?? (kind === 'untracked' ? 'added' : kind), unstaged: kind }
        })
        return record(r, before, 'stage', label, label.replace(/^Stage/, 'Staged').replace(/^Unstage/, 'Unstaged'))
      }),
    commit: (repo, message, amend) =>
      write(repo, (r) => {
        const staged = r.files.filter((f) => f.staged)
        if (!staged.length && !amend) throw err('nothingToCommit', 'Nothing is staged. Stage the changes you want to commit first.')
        const before = serialize(r)
        const [summary, ...rest] = message.split('\n')
        const parent = amend ? (byId(r).get(r.head.commit!)?.parents ?? []) : r.head.commit ? [r.head.commit] : []
        const c: Commit = { id: sha(`c${++idSeq}${message}`), shortId: '', parents: parent, authorName: 'Alex Kim', authorEmail: 'alex@example.com', authorTime: now(), committerName: 'Alex Kim', committerTime: now(), summary, body: rest.join('\n').trim() }
        c.shortId = c.id.slice(0, 7)
        if (amend) r.commits = r.commits.filter((x) => x.id !== r.head.commit)
        r.commits.unshift(c)
        if (!r.head.branch) r.head.commit = c.id
        else if (r.branches.find((b) => b.name === r.head.branch)) moveHeadBranch(r, c.id)
        else {
          r.branches.push({ name: r.head.branch, commit: c.id, upstream: null })
          r.head.commit = c.id
        }
        r.info.isEmpty = false
        for (const f of r.files) lineState.delete(repo + '\0' + f.path)
        r.files = r.files.map((f) => ({ ...f, staged: null })).filter((f) => f.unstaged)
        return record(r, before, 'commit', `Commit “${summary}”`, `Committed ${staged.length === 1 ? '1 file' : `${staged.length} files`} to ${r.head.branch ?? 'HEAD'}`)
      }),
    checkout: (repo, target) =>
      write(repo, (r) => {
        const before = serialize(r)
        const local = r.branches.find((b) => b.name === target)
        if (local) r.head = { branch: local.name, commit: local.commit }
        else {
          const rb = r.remotes.find((x) => x.fullName === target)
          if (rb) {
            r.branches.push({ name: rb.name, commit: rb.commit, upstream: rb.fullName })
            r.head = { branch: rb.name, commit: rb.commit }
          } else r.head = { branch: null, commit: resolve(r, target) }
        }
        return record(r, before, 'checkout', `Switch to ${r.head.branch ?? target.slice(0, 7)}`, `Switched to ${r.head.branch ?? target.slice(0, 7)}`)
      }),
    createBranch: (repo, name, from, checkout) =>
      write(repo, (r) => {
        if (r.branches.some((b) => b.name === name)) throw err('other', `A branch named “${name}” already exists.`)
        const before = serialize(r)
        const commit = resolve(r, from)
        r.branches.push({ name, commit, upstream: null })
        if (checkout) r.head = { branch: name, commit }
        return record(r, before, 'createBranch', `Create branch ${name}`, checkout ? `Created and switched to ${name}` : `Created ${name}`)
      }),
    renameBranch: (repo, from, to) =>
      write(repo, (r) => {
        const before = serialize(r)
        const b = r.branches.find((x) => x.name === from)
        if (!b) throw err('notFound', `No branch named ${from}.`)
        b.name = to
        if (r.head.branch === from) r.head.branch = to
        return record(r, before, 'renameBranch', `Rename ${from} to ${to}`, `Renamed ${from} to ${to}`)
      }),
    async fetch(repo, _remote, opId) {
      if (scenario === 'offline') {
        await delay(300)
        throw err('network', "Couldn't reach github.com. Check your connection; you can keep working offline.")
      }
      await progress(opId, ['Receiving objects'])
      const r = get(repo)
      return record(r, serialize(r), 'fetch', 'Fetch all remotes', 'Fetched', false)
    },
    async push(repo, remote, branch, _setUpstream, opId) {
      await progress(opId, ['Writing objects'])
      const r = get(repo)
      const b = r.branches.find((x) => x.name === branch)!
      const full = `${remote}/${branch}`
      const rb = r.remotes.find((x) => x.fullName === full)
      if (rb && !ancestors(r, b.commit).has(rb.commit)) throw err('rejected', "The remote has commits you don't have. Pull first, or force push from the preview.")
      if (rb) rb.commit = b.commit
      else r.remotes.push({ remote, name: branch, fullName: full, commit: b.commit })
      b.upstream = full
      emitChanged(repo)
      return record(r, serialize(r), 'push', `Push ${branch} to ${remote}`, `Pushed ${branch} to ${remote}`, false)
    },
    stashPush: (repo, message) =>
      write(repo, (r) => {
        if (!r.files.length) throw err('other', 'There are no changes to stash.')
        const before = serialize(r)
        r.stashes = [{ index: 0, ref: 'stash@{0}', commit: sha(`s${++idSeq}`), message: message ?? `WIP on ${r.head.branch}`, time: now(), branch: r.head.branch }, ...r.stashes.map((s) => ({ ...s, index: s.index + 1, ref: `stash@{${s.index + 1}}` }))]
        r.files = []
        return record(r, before, 'stash', 'Stash changes', 'Stashed your changes')
      }),
    stashApply: (repo, index, pop) =>
      write(repo, (r) => {
        const before = serialize(r)
        r.files.push({ path: 'src/lib/graph/canvas.ts', origPath: null, staged: null, unstaged: 'untracked', conflict: null })
        if (pop) r.stashes = r.stashes.filter((s) => s.index !== index).map((s, i) => ({ ...s, index: i, ref: `stash@{${i}}` }))
        return record(r, before, 'stashApply', `${pop ? 'Pop' : 'Apply'} stash@{${index}}`, pop ? 'Popped stash' : 'Applied stash')
      }),
    continueOperation: (repo) =>
      write(repo, (r) => {
        if (r.files.some((f) => f.conflict)) throw err('conflict', 'Resolve and stage every conflicted file before continuing.')
        const before = serialize(r)
        r.state = 'clean'
        r.head = { branch: 'feat/undo-log', commit: r.branches.find((b) => b.name === 'feat/undo-log')!.commit }
        return record(r, before, 'continue', 'Continue rebase', 'Rebase finished')
      }),
    abortOperation: (repo) =>
      write(repo, (r) => {
        const before = serialize(r)
        r.state = 'clean'
        r.files = r.files.filter((f) => !f.conflict)
        r.head = { branch: 'feat/undo-log', commit: r.branches.find((b) => b.name === 'feat/undo-log')!.commit }
        return record(r, before, 'abort', 'Abort rebase', 'Rebase aborted; feat/undo-log is back where it was')
      }),
    skipOperation: (repo) => write(repo, (r) => record(r, serialize(r), 'skip', 'Skip commit', 'Skipped this commit')),
    resolveConflict: (repo, path, side) =>
      write(repo, (r) => {
        const before = serialize(r)
        r.files = r.files.map((f) => (f.path === path ? { ...f, conflict: null, unstaged: null, staged: 'modified' } : f))
        return record(r, before, 'resolve', side === 'mark' ? `Mark ${path} resolved` : `Use ${side === 'ours' ? 'mine' : 'theirs'} for ${path}`, 'Resolved')
      }),
    createTag: (repo, name, commit) =>
      write(repo, (r) => {
        const before = serialize(r)
        r.tags.push({ name, commit: resolve(r, commit), annotated: false })
        return record(r, before, 'tag', `Create tag ${name}`, `Tagged ${commit.slice(0, 7)} as ${name}`)
      }),
    deleteTag: (repo, name) =>
      write(repo, (r) => {
        const before = serialize(r)
        r.tags = r.tags.filter((t) => t.name !== name)
        return record(r, before, 'deleteTag', `Delete tag ${name}`, `Deleted tag ${name}`)
      }),
    addRemote: (repo, name) =>
      write(repo, (r) => record(r, serialize(r), 'addRemote', `Add remote ${name}`, `Added remote ${name}`)),
    async previewOperation(repo, op) {
      await delay(110)
      return preview(get(repo), op)
    },
    runOperation: (repo, op, opId) =>
      (async () => {
        if (op.kind === 'forcePush' || op.kind === 'pull') await progress(opId, ['Writing objects'])
        return write(repo, (r) => run(r, op))
      })(),
    async getOplog(repo) {
      return [...get(repo).oplog]
    },
    undo: (repo, entryId) =>
      write(repo, (r) => {
        const targets = r.oplog.filter((e) => e.undoable && !e.undone)
        const stopAt = entryId ? targets.findIndex((e) => e.id === entryId) : 0
        if (stopAt < 0 || !targets.length) throw err('other', 'Nothing to undo.')
        const chosen = targets.slice(0, stopAt + 1)
        for (const e of chosen) {
          const snap = r.snapshots.get(e.id)
          if (snap) restore(r, snap.before)
          e.undone = true
        }
        const last = chosen[chosen.length - 1]
        return { entry: null, message: chosen.length === 1 ? `Undid ${last.label}` : `Undid ${chosen.length} operations` }
      }),
    redo: (repo) =>
      write(repo, (r) => {
        const i = r.oplog.findIndex((e) => !e.undone)
        const e = i === -1 ? r.oplog[r.oplog.length - 1] : r.oplog[i - 1]
        if (!e?.undone) throw err('other', 'Nothing to redo.')
        const snap = r.snapshots.get(e.id)
        if (!snap?.after) throw err('other', "This can't be redone.")
        restore(r, snap.after)
        e.undone = false
        return { entry: null, message: `Redid ${e.label}` }
      }),
    async listPullRequests(repo, remote) {
      await delay(200)
      const r = get(repo)
      if (remote === 'codeberg') return { remote, host: 'forgejo', status: 'ok', message: null, items: [] }
      return {
        remote,
        host: 'github',
        status: 'ok',
        message: null,
        items: r.info.name === 'GitCove'
          ? [
              { number: 12, title: 'Orthogonal lane drawing', author: 'you', state: 'open', url: 'https://github.com/you/GitCove/pull/12', sourceBranch: 'feat/commit-graph', targetBranch: 'main', updatedAt: now() - 5 * H },
              { number: 14, title: 'Undo log for every write', author: 'you', state: 'draft', url: 'https://github.com/you/GitCove/pull/14', sourceBranch: 'feat/undo-log', targetBranch: 'main', updatedAt: now() - 22 * H },
            ]
          : [],
      }
    },
    async openTerminal() {},
    async startupPaths() {
      return []
    },
    async githubStatus() {
      await delay(120)
      return ghStatus()
    },
    async githubSetClientId(clientId) {
      await delay(80)
      if (!/^[A-Za-z0-9]{16,40}$/.test(clientId.trim())) {
        throw err('other', 'That doesn’t look like a GitHub OAuth App client ID. It’s the short code shown under “Client ID” on the app’s settings page.')
      }
      gh.configured = true
      return ghStatus()
    },
    async githubStartLogin() {
      await delay(300)
      gh.polls = 0
      return { deviceCode: 'demo-device-code', userCode: 'WDJB-MJHT', verificationUri: 'https://github.com/login/device', expiresIn: 900, interval: 2 }
    },
    async githubPollLogin() {
      await delay(150)
      gh.polls++
      if (gh.polls < 3) return { status: 'pending', message: null, account: null, interval: null }
      gh.account = demoAccount
      return { status: 'ok', message: null, account: demoAccount, interval: null }
    },
    async githubSignOut() {
      gh.account = null
    },
    async githubListRepos() {
      await delay(400)
      if (!gh.account) throw err('auth', 'Sign in to GitHub to see your repositories.')
      return ghRepos
    },
    async getIdentity() {
      return new URLSearchParams(location.search).get('identity') === 'none' ? { name: null, email: null } : { name: 'Alex Kim', email: 'alex@example.com' }
    },
    async setIdentity() {},
    async defaultCloneDir() {
      return DEMO_ROOT
    },
    async onProgress(handler) {
      progressHandlers.add(handler)
      return () => progressHandlers.delete(handler)
    },
    async onRepoChanged(handler) {
      changedHandlers.add(handler)
      return () => changedHandlers.delete(handler)
    },
  }
  return backend
}
