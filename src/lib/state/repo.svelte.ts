import { SvelteMap } from 'svelte/reactivity'
import { backend } from '../api'
import type {
  Branch,
  Commit,
  CommitDetails,
  FileChange,
  FileDiff,
  GitError,
  Identity,
  OpEntry,
  OpResult,
  Operation,
  Preview,
  ProgressEvent,
  PullRequestList,
  RepoInfo,
  Refs,
  Status,
} from '../api/types'
import { firstParentChain, layoutGraph, type GraphCommit } from '../graph/layout'
import { buildPatch, changedLines, lineKey, type LineKey } from '../util/patch'
import { newOpId, plural } from '../util/format'
import type { AppState } from './app.svelte'

export type Selection = { kind: 'wip' } | { kind: 'commit'; id: string }
export type RepoTab = 'changes' | 'history'
/** A file's checkbox: staged, not staged, or partly staged. */
export type Check = boolean | 'mixed'

const CONTEXT = 3
const CONTEXT_STEP = 25

export interface RefLabel {
  kind: 'branch' | 'remote' | 'tag'
  name: string
  isHead: boolean
  /** Moves here in the preview. */
  ghost?: boolean
  /** Moves away (or disappears) in the preview. */
  struck?: boolean
}

export interface Busy {
  label: string
  opId: string
  percent: number | null
  phase: string
}

export const WIP_ID = '__wip__'
const DEFAULT_BRANCHES = ['main', 'master', 'trunk', 'develop']
const AUTO_FETCH_MS = 10 * 60 * 1000

export class RepoStore {
  info: RepoInfo
  path: string
  name: string
  #app: AppState

  refs = $state<Refs | null>(null)
  log = $state.raw<Commit[]>([])
  truncated = $state(false)
  status = $state<Status | null>(null)
  oplog = $state<OpEntry[]>([])
  loading = $state(true)
  loadError = $state<GitError | null>(null)

  tab = $state<RepoTab>('changes')
  selection = $state<Selection>({ kind: 'wip' })
  /** Path of the changed file whose diff is shown in the Changes tab. */
  fileSel = $state<string | null>(null)
  /** HEAD → worktree diff of `fileSel`; +/- lines carry whether they're staged. */
  diff = $state.raw<FileDiff | null>(null)
  diffError = $state<GitError | null>(null)
  diffContext = $state(CONTEXT)
  identity = $state<Identity | null>(null)

  details = new SvelteMap<string, CommitDetails>()
  commitFile = $state<string | null>(null)
  commitDiff = $state.raw<FileDiff | null>(null)
  commitContext = $state(CONTEXT)

  preview = $state.raw<Preview | null>(null)
  previewOp = $state.raw<Operation | null>(null)
  previewLoading = $state(false)
  previewError = $state<GitError | null>(null)
  /** Live drag of a branch label onto another commit. */
  drag = $state<{ branch: string; target: string | null } | null>(null)

  busy = $state<Record<string, Busy>>({})
  prs = new SvelteMap<string, PullRequestList>()
  lastFetch = $state<number | null>(null)
  offline = $state(false)

  summary = $state('')
  body = $state('')
  amend = $state(false)

  #changeTimer: ReturnType<typeof setTimeout> | null = null
  #pendingKinds = new Set<'worktree' | 'git'>()
  #fetchTimer: ReturnType<typeof setInterval> | null = null
  #dragTimer: ReturnType<typeof setTimeout> | null = null
  #previewSeq = 0

  constructor(info: RepoInfo, app: AppState) {
    this.info = info
    this.path = info.path
    this.name = info.name
    this.#app = app
  }

  // ---------- derived views ----------

  byId = $derived.by(() => {
    const m = new Map<string, Commit>()
    for (const c of this.log) m.set(c.id, c)
    return m
  })

  head = $derived(this.status?.head ?? this.refs?.head ?? null)
  /** Mid-rebase HEAD is detached by design; name the branch being rebased instead of warning. */
  inOperation = $derived(!!this.status && this.status.state !== 'clean')
  branchName = $derived(this.head?.branch ?? (this.inOperation ? (this.status?.operation?.branch ?? null) : null))
  detached = $derived(!!this.head?.detached && !this.inOperation)
  /**
   * Who is who in a conflict. Git's "ours" is the branch being built on: during a rebase that is
   * the upstream, and the user's own commit is "theirs". Labels and "Use mine" follow the user, not git.
   */
  conflictSides = $derived.by(() => {
    const s = this.status
    const op = s?.operation
    const short = op?.currentCommit ? ` ${op.currentCommit.slice(0, 7)}` : ''
    switch (s?.state) {
      case 'rebasing':
        return { ours: `${op?.onto ?? 'Upstream'} (upstream)`, theirs: `Your commit${short}`, other: op?.onto ?? 'upstream', mineIsOurs: false }
      case 'merging':
        return { ours: `Your branch${this.branchName ? ` ${this.branchName}` : ''}`, theirs: op?.onto ?? 'Incoming branch', other: op?.onto ?? 'theirs', mineIsOurs: true }
      case 'cherryPicking':
        return { ours: 'Your branch', theirs: `Picked commit${short}`, other: 'picked', mineIsOurs: true }
      case 'reverting':
        return { ours: 'Your branch', theirs: `Reverted change${short}`, other: 'reverted', mineIsOurs: true }
      default:
        return { ours: 'Yours', theirs: 'Theirs', other: 'theirs', mineIsOurs: true }
    }
  })

  /** Primitive copies stop a status refresh (new objects, same values) from re-laying out the graph. */
  headId = $derived(this.head?.commit ?? null)
  headBranchName = $derived(this.head?.branch ?? null)
  headBranch = $derived<Branch | null>(this.refs?.branches.find((b) => b.isHead) ?? null)
  defaultBranch = $derived(
    DEFAULT_BRANCHES.map((n) => this.refs?.branches.find((b) => b.name === n)).find(Boolean) ?? this.headBranch,
  )
  remotes = $derived(this.refs?.remotes ?? [])
  primaryRemote = $derived(this.remotes.find((r) => r.name === 'origin') ?? this.remotes[0] ?? null)

  files = $derived(this.status?.files ?? [])
  conflicted = $derived(this.files.filter((f) => f.conflict))
  staged = $derived(this.files.filter((f) => f.staged && !f.conflict))
  unstaged = $derived(this.files.filter((f) => f.unstaged && !f.conflict))
  dirty = $derived(this.files.length > 0)
  /** One list for the Changes tab, conflicts first. */
  changeFiles = $derived([...this.conflicted, ...this.files.filter((f) => !f.conflict)])
  /** The list header's checkbox. */
  allCheck = $derived.by<Check>(() => {
    const files = this.files.filter((f) => !f.conflict)
    if (!files.some((f) => f.staged)) return false
    // Unresolved conflicts can't be in the commit yet, so the list is never fully ticked while they remain.
    if (this.conflicted.length) return 'mixed'
    return files.every((f) => f.staged && !f.unstaged) ? true : 'mixed'
  })

  /** Labels per commit, with the preview's ref moves applied. */
  labels = $derived.by(() => {
    const m = new Map<string, RefLabel[]>()
    const add = (id: string, l: RefLabel) => {
      const list = m.get(id)
      if (list) list.push(l)
      else m.set(id, [l])
    }
    const r = this.refs
    if (!r) return m
    const moves = new Map((this.preview?.moves ?? []).map((mv) => [mv.ref, mv]))
    for (const b of r.branches) {
      const mv = moves.get(b.name)
      add(b.commit, { kind: 'branch', name: b.name, isHead: b.isHead, struck: !!mv })
      if (mv?.to) add(mv.to, { kind: 'branch', name: b.name, isHead: b.isHead, ghost: true })
    }
    for (const rb of r.remoteBranches) {
      if (rb.name === 'HEAD') continue
      const mv = moves.get(rb.fullName)
      add(rb.commit, { kind: 'remote', name: rb.fullName, isHead: false, struck: !!mv })
      if (mv?.to) add(mv.to, { kind: 'remote', name: rb.fullName, isHead: false, ghost: true })
    }
    for (const t of r.tags) add(t.commit, { kind: 'tag', name: t.name, isHead: false })
    return m
  })

  mainChain = $derived.by(() => {
    const tip = this.defaultBranch?.commit ?? this.headId
    return firstParentChain(tip, this.byId)
  })

  graphCommits = $derived.by<GraphCommit[]>(() => {
    const out: GraphCommit[] = []
    const p = this.preview
    const headMove = p?.moves.find((m) => m.ref === this.headBranchName || m.ref === 'HEAD')
    const wipParent = headMove?.to ?? this.headId
    out.push({
      id: WIP_ID,
      shortId: '',
      parents: wipParent ? [wipParent] : [],
      authorName: '',
      authorEmail: '',
      authorTime: Date.now() / 1000,
      committerName: '',
      committerTime: Date.now() / 1000,
      summary: '',
      body: '',
      wip: true,
    })
    if (p) {
      for (const s of p.created) {
        out.push({
          id: s.id,
          shortId: s.id.startsWith('sim-') ? '' : s.id.slice(0, 7),
          parents: s.parents,
          authorName: s.authorName,
          authorEmail: '',
          authorTime: s.authorTime,
          committerName: s.authorName,
          committerTime: s.authorTime,
          summary: s.summary,
          body: '',
          ghost: true,
          conflicted: s.conflicted,
        })
      }
    }
    const voided = new Set([...(p?.replaced ?? []), ...(p?.remoteCommitsLost.map((c) => c.id) ?? [])])
    if (voided.size === 0) return out.concat(this.log)
    for (const c of this.log) out.push(voided.has(c.id) ? { ...c, void: true } : c)
    return out
  })

  layout = $derived(layoutGraph(this.graphCommits, this.mainChain))

  selectedDetails = $derived(this.selection.kind === 'commit' ? (this.details.get(this.selection.id) ?? null) : null)

  undoable = $derived(this.oplog.find((e) => e.undoable && !e.undone) ?? null)
  redoable = $derived.by(() => {
    const i = this.oplog.findIndex((e) => !e.undone)
    const candidate = i === -1 ? this.oplog[this.oplog.length - 1] : this.oplog[i - 1]
    return candidate?.undone ? candidate : null
  })

  // ---------- loading ----------

  async load() {
    this.loading = true
    try {
      await this.refreshAll()
      this.loadError = null
    } catch (e) {
      this.loadError = e as GitError
    } finally {
      this.loading = false
    }
    backend.watchRepo(this.path).catch(() => {})
    this.loadIdentity()
    this.loadPullRequests()
    if (this.remotes.length) {
      this.fetch(null, { quiet: true })
      this.#fetchTimer = setInterval(() => this.fetch(null, { quiet: true }), AUTO_FETCH_MS)
    }
  }

  dispose() {
    backend.unwatchRepo(this.path).catch(() => {})
    if (this.#fetchTimer) clearInterval(this.#fetchTimer)
  }

  async refreshAll() {
    const [refs, log, status, oplog] = await Promise.all([
      backend.getRefs(this.path),
      backend.getLog(this.path),
      backend.getStatus(this.path),
      backend.getOplog(this.path),
    ])
    this.refs = refs
    this.log = log.commits
    this.truncated = log.truncated
    this.status = status
    this.oplog = oplog
    this.#reconcileSelection()
    await this.#reloadDiff()
  }

  async refreshStatus() {
    this.status = await backend.getStatus(this.path)
    this.#reconcileSelection()
    await this.#reloadDiff()
  }

  onChanged(kind: 'worktree' | 'git') {
    this.#pendingKinds.add(kind)
    if (this.#changeTimer) clearTimeout(this.#changeTimer)
    this.#changeTimer = setTimeout(() => {
      const kinds = this.#pendingKinds
      this.#pendingKinds = new Set()
      const p = kinds.has('git') ? this.refreshAll() : this.refreshStatus()
      p.catch(() => {})
    }, 180)
  }

  onProgress(e: ProgressEvent) {
    for (const [key, b] of Object.entries(this.busy)) {
      if (b.opId === e.opId) this.busy[key] = { ...b, percent: e.percent, phase: e.phase }
    }
  }

  #reconcileSelection() {
    if (this.selection.kind === 'commit' && !this.byId.has(this.selection.id)) this.selection = { kind: 'wip' }
    const p = this.fileSel
    if (!p || !this.files.some((f) => f.path === p)) {
      this.fileSel = this.changeFiles[0]?.path ?? null
      this.diffContext = CONTEXT
    }
  }

  async #reloadDiff() {
    const p = this.fileSel
    if (!p) {
      this.diff = null
      return
    }
    try {
      const d = await backend.getChangeDiff(this.path, p, this.diffContext)
      if (this.fileSel !== p) return
      this.diff = d
      this.diffError = null
    } catch (e) {
      if (this.fileSel !== p) return
      this.diff = null
      this.diffError = e as GitError
    }
  }

  // ---------- selection ----------

  setTab(tab: RepoTab) {
    this.tab = tab
    if (tab === 'history' && this.selection.kind === 'wip' && !this.previewOp) {
      // History opens on the newest commit, like a log does.
      const first = this.layout.rows.find((r) => !r.commit.wip && !r.commit.ghost)
      if (first) this.select({ kind: 'commit', id: first.commit.id })
    }
  }

  select(sel: Selection) {
    this.selection = sel
    if (sel.kind === 'commit') {
      this.commitFile = null
      this.commitDiff = null
      this.commitContext = CONTEXT
      this.#loadDetails(sel.id).then((d) => {
        // Open the first file so the diff area never starts empty.
        if (d?.files[0] && this.selection.kind === 'commit' && this.selection.id === sel.id && !this.commitFile) {
          this.selectCommitFile(d.files[0].path)
        }
      })
      this.#prefetchNeighbours(sel.id)
    } else {
      this.tab = 'changes'
    }
  }

  async #loadDetails(id: string): Promise<CommitDetails | null> {
    const cached = this.details.get(id)
    if (cached) return cached
    try {
      const d = await backend.getCommitDetails(this.path, id)
      this.details.set(id, d)
      return d
    } catch {
      return null
    }
  }

  /** Stepping through history never waits: the rows around the selection load ahead of time. */
  #prefetchNeighbours(id: string) {
    const i = this.layout.index.get(id)
    if (i === undefined) return
    for (const d of [-2, -1, 1, 2]) {
      const c = this.layout.rows[i + d]?.commit
      if (c && !c.wip && !c.ghost) this.#loadDetails(c.id)
    }
  }

  step(dir: 1 | -1) {
    const rows = this.layout.rows
    const current = this.selection.kind === 'wip' ? 0 : (this.layout.index.get(this.selection.id) ?? 0)
    let i = current + dir
    while (rows[i] && rows[i].commit.ghost) i += dir
    const row = rows[i]
    if (!row) return
    this.select(row.commit.wip ? { kind: 'wip' } : { kind: 'commit', id: row.commit.id })
  }

  selectFile(path: string) {
    if (this.fileSel !== path) this.diffContext = CONTEXT
    this.fileSel = path
    this.#reloadDiff()
  }

  /** Show more unchanged lines around every change in the file. */
  expandDiff() {
    this.diffContext += CONTEXT_STEP
    this.#reloadDiff()
  }

  async selectCommitFile(path: string) {
    if (this.selection.kind !== 'commit') return
    const id = this.selection.id
    if (this.commitFile !== path) this.commitContext = CONTEXT
    this.commitFile = path
    try {
      const d = await backend.getCommitDiff(this.path, id, path, this.commitContext)
      if (this.commitFile === path && this.selection.kind === 'commit' && this.selection.id === id) this.commitDiff = d
    } catch (e) {
      this.#app.fail(e)
    }
  }

  expandCommitDiff() {
    if (!this.commitFile) return
    this.commitContext += CONTEXT_STEP
    this.selectCommitFile(this.commitFile)
  }

  async loadIdentity() {
    try {
      this.identity = await backend.getIdentity(this.path)
    } catch {
      this.identity = null
    }
  }

  async setIdentity(name: string, email: string) {
    try {
      await backend.setIdentity(name, email)
      this.#app.toast({ kind: 'success', message: `Git will sign commits as ${name}` })
    } catch (e) {
      this.#app.fail(e)
    }
    await this.loadIdentity()
  }

  // ---------- actions ----------

  async #run(fn: () => Promise<OpResult>, opts: { toast?: boolean } = {}): Promise<OpResult | null> {
    try {
      const r = await fn()
      if (opts.toast !== false) this.#app.confirm(r, this)
      await this.refreshAll().catch(() => {})
      return r
    } catch (e) {
      this.#app.fail(e)
      await this.refreshAll().catch(() => {})
      return null
    }
  }

  stage(paths: string[]) {
    return this.#run(() => backend.stage(this.path, paths), { toast: false })
  }

  unstage(paths: string[]) {
    return this.#run(() => backend.unstage(this.path, paths), { toast: false })
  }

  fileCheck(f: FileChange): Check {
    if (f.conflict || !f.staged) return false
    return f.unstaged ? 'mixed' : true
  }

  /** A file's checkbox: a mixed or empty box stages the whole file; a full one unstages it. */
  toggleFile(f: FileChange) {
    if (f.conflict) return
    return this.fileCheck(f) === true ? this.unstage([f.path]) : this.stage([f.path])
  }

  toggleAll() {
    const files = this.files.filter((f) => !f.conflict)
    const todo = files.filter((f) => !f.staged || f.unstaged).map((f) => f.path)
    if (todo.length) return this.stage(todo)
    if (files.length) return this.unstage(files.map((f) => f.path))
  }

  /** Lines of the shown diff that are in the next commit. */
  stagedLines(d: FileDiff): Set<LineKey> {
    const out = new Set<LineKey>()
    d.hunks.forEach((h, hi) =>
      h.lines.forEach((l, li) => {
        if ((l.kind === 'add' || l.kind === 'del') && l.staged) out.add(lineKey(hi, li))
      }),
    )
    return out
  }

  /** Put these changed lines of the shown file into the next commit, or take them out. */
  async setLines(keys: LineKey[], include: boolean) {
    const d = this.diff
    if (!d || !keys.length) return
    const next = this.stagedLines(d)
    for (const k of keys) {
      if (include) next.add(k)
      else next.delete(k)
    }
    // Show the result at once; the refresh after the write confirms it.
    this.diff = {
      ...d,
      hunks: d.hunks.map((h, hi) => ({
        ...h,
        lines: h.lines.map((l, li) => (l.kind === 'add' || l.kind === 'del' ? { ...l, staged: next.has(lineKey(hi, li)) } : l)),
      })),
    }
    if (next.size === changedLines(d).size) return this.stage([d.path])
    const label = `${include ? 'Stage' : 'Unstage'} ${plural(keys.length, 'line')} in ${d.path}`
    const patch = next.size ? buildPatch(d, next, false) : null
    return this.#run(() => backend.setFileLines(this.path, d.path, patch, label), { toast: false })
  }

  /** Throw these changed lines away from the working tree. Undoable from the op log. */
  async discardLines(keys: LineKey[]) {
    const d = this.diff
    if (!d || !keys.length) return
    const patch = buildPatch(d, new Set(keys), true)
    if (!patch) return
    await this.#run(() => backend.applyPatch(this.path, patch, false, true, `Discard ${plural(keys.length, 'line')} in ${d.path}`))
  }

  async commit() {
    const message = this.body.trim() ? `${this.summary.trim()}\n\n${this.body.trim()}` : this.summary.trim()
    if (!message && !this.amend) return
    const r = await this.#run(() => backend.commit(this.path, message, this.amend))
    if (r) {
      this.summary = ''
      this.body = ''
      this.amend = false
    }
  }

  /** Fills the message box with the last commit's message when amending is switched on. */
  setAmend(on: boolean) {
    this.amend = on
    const last = this.head?.commit ? this.byId.get(this.head.commit) : null
    if (on && last && !this.summary && !this.body) {
      this.summary = last.summary
      this.body = last.body
    }
  }

  checkout(target: string) {
    return this.#run(() => backend.checkout(this.path, target))
  }

  createBranch(name: string, from: string, checkout: boolean) {
    return this.#run(() => backend.createBranch(this.path, name, from, checkout))
  }

  renameBranch(from: string, to: string) {
    return this.#run(() => backend.renameBranch(this.path, from, to))
  }

  async fetch(remote: string | null, opts: { quiet?: boolean } = {}) {
    if (this.busy.fetch || !this.remotes.length) return
    const opId = newOpId()
    this.busy.fetch = { label: 'Fetching', opId, percent: null, phase: '' }
    try {
      await backend.fetch(this.path, remote, opId)
      this.lastFetch = Date.now() / 1000
      this.offline = false
      if (!opts.quiet) this.#app.toast({ kind: 'success', message: 'Fetched from ' + (remote ?? 'all remotes') })
      await this.refreshAll().catch(() => {})
    } catch (e) {
      const err = e as GitError
      if (err.kind === 'network') this.offline = true
      if (!opts.quiet || err.kind === 'auth') this.#app.fail(err)
    } finally {
      delete this.busy.fetch
    }
  }

  /** Fast-forwards when possible; a diverged branch opens the preview to choose rebase or merge. */
  async pull() {
    const b = this.headBranch
    if (!b?.upstream || this.busy.pull) return
    if (b.ahead > 0 && b.behind > 0) {
      await this.startPreview({ kind: 'pull', mode: 'rebase' })
      return
    }
    const opId = newOpId()
    this.busy.pull = { label: 'Pulling', opId, percent: null, phase: '' }
    try {
      const r = await backend.runOperation(this.path, { kind: 'pull', mode: 'ffOnly' }, opId)
      this.lastFetch = Date.now() / 1000
      this.#app.confirm(r, this)
    } catch (e) {
      const err = e as GitError
      if (err.kind === 'divergent') await this.startPreview({ kind: 'pull', mode: 'rebase' })
      else this.#app.fail(err)
    } finally {
      delete this.busy.pull
      await this.refreshAll().catch(() => {})
    }
  }

  async push() {
    const b = this.headBranch
    const remote = b?.upstream?.split('/')[0] ?? this.primaryRemote?.name
    if (!b || !remote || this.busy.push) return
    const opId = newOpId()
    this.busy.push = { label: 'Pushing', opId, percent: null, phase: '' }
    try {
      const r = await backend.push(this.path, remote, b.name, !b.upstream, opId)
      this.#app.confirm(r, this)
    } catch (e) {
      const err = e as GitError
      if (err.kind === 'rejected') {
        this.#app.toast({
          kind: 'error',
          message: err.message,
          detail: err.detail,
          action: { label: 'Preview force push', run: () => this.startPreview({ kind: 'forcePush', remote, branch: b.name }) },
        })
      } else {
        this.#app.fail(err)
      }
    } finally {
      delete this.busy.push
      await this.refreshAll().catch(() => {})
    }
  }

  stashPush(message: string | null = null) {
    return this.#run(() => backend.stashPush(this.path, message, true))
  }

  stashApply(index: number, pop: boolean) {
    return this.#run(() => backend.stashApply(this.path, index, pop))
  }

  continueOp() {
    return this.#run(() => backend.continueOperation(this.path))
  }

  abortOp() {
    return this.#run(() => backend.abortOperation(this.path))
  }

  skipOp() {
    return this.#run(() => backend.skipOperation(this.path))
  }

  resolve(path: string, side: 'ours' | 'theirs' | 'mark') {
    return this.#run(() => backend.resolveConflict(this.path, path, side), { toast: false })
  }

  /** Keep the user's own version of a conflicted file, whichever side git calls it. */
  resolveMine(path: string) {
    return this.resolve(path, this.conflictSides.mineIsOurs ? 'ours' : 'theirs')
  }

  resolveOther(path: string) {
    return this.resolve(path, this.conflictSides.mineIsOurs ? 'theirs' : 'ours')
  }

  createTag(name: string, commit: string, message: string | null) {
    return this.#run(() => backend.createTag(this.path, name, commit, message))
  }

  deleteTag(name: string) {
    return this.#run(() => backend.deleteTag(this.path, name))
  }

  addRemote(name: string, url: string) {
    return this.#run(() => backend.addRemote(this.path, name, url))
  }

  openTerminal() {
    backend.openTerminal(this.path).catch((e) => this.#app.fail(e))
  }

  async undo(entryId: string | null = null) {
    // Earlier "Undo" offers point at entries this undo may consume.
    this.#app.toasts = this.#app.toasts.filter((t) => t.action?.label !== 'Undo')
    await this.#run(() => backend.undo(this.path, entryId))
  }

  async redo() {
    await this.#run(() => backend.redo(this.path))
  }

  async loadPullRequests() {
    for (const r of this.remotes) {
      if (r.host === 'other' || r.host === 'bitbucket') continue
      backend
        .listPullRequests(this.path, r.name)
        .then((list) => this.prs.set(r.name, list))
        .catch(() => {})
    }
  }

  // ---------- preview ----------

  async startPreview(op: Operation) {
    const seq = ++this.#previewSeq
    // Previews are drawn on the graph; a discard's readout reads fine from the Changes tab.
    if (op.kind !== 'discard') {
      this.tab = 'history'
      // The main area then explains the preview instead of showing an unrelated commit.
      this.selection = { kind: 'wip' }
    }
    this.previewOp = op
    this.previewLoading = true
    this.previewError = null
    try {
      const p = await backend.previewOperation(this.path, op)
      if (seq !== this.#previewSeq) return
      this.preview = p
    } catch (e) {
      if (seq !== this.#previewSeq) return
      this.preview = null
      this.previewError = e as GitError
    } finally {
      if (seq === this.#previewSeq) this.previewLoading = false
    }
  }

  cancelPreview() {
    this.#previewSeq++
    this.preview = null
    this.previewOp = null
    this.previewError = null
    this.previewLoading = false
    this.drag = null
  }

  async applyPreview() {
    const op = this.previewOp
    if (!op) return
    const opId = newOpId()
    this.busy.apply = { label: 'Applying', opId, percent: null, phase: '' }
    this.cancelPreview()
    try {
      const r = await backend.runOperation(this.path, op, opId)
      this.#app.confirm(r, this)
    } catch (e) {
      this.#app.fail(e)
    } finally {
      delete this.busy.apply
      await this.refreshAll().catch(() => {})
    }
  }

  // ---------- drag a branch onto another commit ----------

  startDrag(branch: string) {
    this.cancelPreview()
    this.drag = { branch, target: null }
  }

  /** Called while hovering a row during a drag; the preview follows the pointer. */
  dragOver(target: string) {
    const d = this.drag
    if (!d || d.target === target) return
    this.drag = { ...d, target }
    if (this.#dragTimer) clearTimeout(this.#dragTimer)
    const branchTip = this.refs?.branches.find((b) => b.name === d.branch)?.commit
    if (target === branchTip) {
      this.preview = null
      this.previewOp = null
      return
    }
    this.#dragTimer = setTimeout(() => this.startPreview({ kind: 'rebase', branch: d.branch, onto: target }), 90)
  }

  drop() {
    if (this.#dragTimer) clearTimeout(this.#dragTimer)
    const d = this.drag
    if (d?.target && !this.previewOp) this.startPreview({ kind: 'rebase', branch: d.branch, onto: d.target })
    this.drag = null
  }

  endDrag() {
    if (this.drag) this.cancelPreview()
  }
}

/** Files to show as one list entry each, conflicted first. */
export function fileLabel(f: FileChange): string {
  return f.origPath ? `${f.origPath} → ${f.path}` : f.path
}
