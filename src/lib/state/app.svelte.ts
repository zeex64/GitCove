import { backend } from '../api'
import type { GitError, OpEntry, OpResult } from '../api/types'
import { RepoStore } from './repo.svelte'

export interface Toast {
  id: number
  kind: 'success' | 'error' | 'info'
  message: string
  detail?: string
  action?: { label: string; run: () => void }
}

export interface RecentRepo {
  path: string
  name: string
  openedAt: number
}

export interface Tab {
  id: string
  repo: RepoStore | null
}

export interface MenuItem {
  label: string
  run?: () => void
  danger?: boolean
  disabled?: boolean
  hint?: string
  separator?: boolean
}

export interface MenuState {
  x: number
  y: number
  items: MenuItem[]
}

export interface PromptField {
  name: string
  label: string
  value?: string
  placeholder?: string
  mono?: boolean
}

/** A small anchored form (new branch, tag, remote…), used instead of modal dialogs. */
export interface PromptState {
  x: number
  y: number
  title: string
  fields: PromptField[]
  confirm: string
  check?: { label: string; value: boolean }
  validate?: (values: Record<string, string>) => string | null
  onSubmit: (values: Record<string, string>, checked: boolean) => void
}

const KEY_TABS = 'gitcove.tabs'
const KEY_ACTIVE = 'gitcove.active'
const KEY_RECENT = 'gitcove.recent'
const KEY_PANES = 'gitcove.panes'

function load<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as T) : fallback
  } catch {
    return fallback
  }
}

function save(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    /* storage can be unavailable; the app still works for this session */
  }
}

let tabSeq = 0
let toastSeq = 0

class AppState {
  tabs = $state<Tab[]>([])
  activeId = $state<string | null>(null)
  recent = $state<RecentRepo[]>(load(KEY_RECENT, []))
  toasts = $state<Toast[]>([])
  paletteOpen = $state(false)
  menu = $state<MenuState | null>(null)
  prompt = $state<PromptState | null>(null)
  /** Opens the GitHub account popover from anywhere (palette, branch menu). */
  accountOpen = $state(false)
  /** A new tab should open its clone form on this source. */
  welcomeClone = $state<'github' | 'url' | null>(null)
  gitVersion = $state<string | null>(null)
  gitMissing = $state<GitError | null>(null)
  panes = $state({
    ...{ left: 340, split: false, wrap: true },
    ...load<Record<string, unknown>>(KEY_PANES, {}),
  } as { left: number; split: boolean; wrap: boolean })

  active = $derived(this.tabs.find((t) => t.id === this.activeId) ?? null)
  repo = $derived(this.active?.repo ?? null)

  async start() {
    try {
      this.gitVersion = await backend.gitVersion()
    } catch (e) {
      this.gitMissing = e as GitError
      return
    }

    import('./github.svelte').then(({ github }) => github.refresh())

    backend.onRepoChanged((e) => {
      for (const t of this.tabs) if (t.repo?.path === e.repo) t.repo.onChanged(e.kind)
    })
    backend.onProgress((e) => {
      for (const t of this.tabs) t.repo?.onProgress(e)
    })

    const saved = load<string[]>(KEY_TABS, [])
    const activePath = load<string | null>(KEY_ACTIVE, null)
    for (const path of saved) await this.openPath(path, { quiet: true, focus: path === activePath })
    const launched = await backend.startupPaths().catch(() => [])
    for (const path of launched) {
      const e = await this.openPath(path, { focus: true })
      if (e) this.fail(e)
    }
    if (this.tabs.length === 0) this.newTab()
    else if (!this.activeId) this.activeId = this.tabs[0].id
  }

  /** A new tab showing the clone form, on the GitHub list when signed in. */
  cloneInNewTab(source: 'github' | 'url' = 'github') {
    const empty = this.active && !this.active.repo
    if (!empty) this.newTab()
    this.welcomeClone = source
  }

  newTab() {
    const tab: Tab = { id: `tab-${++tabSeq}`, repo: null }
    this.tabs.push(tab)
    this.activeId = tab.id
    this.persistTabs()
  }

  /** Opens a repository into the active empty tab, an existing tab for that repo, or a new tab. */
  async openPath(path: string, opts: { quiet?: boolean; focus?: boolean } = {}): Promise<GitError | null> {
    try {
      const info = await backend.openRepo(path)
      const existing = this.tabs.find((t) => t.repo?.path === info.path)
      if (existing) {
        if (opts.focus !== false) this.activeId = existing.id
        return null
      }
      const store = new RepoStore(info, this)
      const empty = this.active && !this.active.repo ? this.active : null
      if (empty && !opts.quiet) {
        empty.repo = store
      } else {
        const tab: Tab = { id: `tab-${++tabSeq}`, repo: store }
        this.tabs.push(tab)
        if (opts.focus !== false) this.activeId = tab.id
      }
      this.remember(info.path, info.name)
      this.persistTabs()
      await store.load()
      return null
    } catch (e) {
      if (!opts.quiet) return e as GitError
      return null
    }
  }

  closeTab(id: string) {
    const i = this.tabs.findIndex((t) => t.id === id)
    if (i === -1) return
    this.tabs[i].repo?.dispose()
    this.tabs.splice(i, 1)
    if (this.activeId === id) this.activeId = this.tabs[Math.min(i, this.tabs.length - 1)]?.id ?? null
    if (this.tabs.length === 0) this.newTab()
    this.persistTabs()
  }

  activate(id: string) {
    this.activeId = id
    save(KEY_ACTIVE, this.active?.repo?.path ?? null)
  }

  cycleTab(dir: 1 | -1) {
    if (this.tabs.length < 2) return
    const i = this.tabs.findIndex((t) => t.id === this.activeId)
    this.activate(this.tabs[(i + dir + this.tabs.length) % this.tabs.length].id)
  }

  remember(path: string, name: string) {
    this.recent = [{ path, name, openedAt: Date.now() }, ...this.recent.filter((r) => r.path !== path)].slice(0, 12)
    save(KEY_RECENT, this.recent)
  }

  forget(path: string) {
    this.recent = this.recent.filter((r) => r.path !== path)
    save(KEY_RECENT, this.recent)
  }

  persistTabs() {
    save(
      KEY_TABS,
      this.tabs.map((t) => t.repo?.path).filter((p): p is string => !!p),
    )
    save(KEY_ACTIVE, this.active?.repo?.path ?? null)
  }

  persistPanes() {
    save(KEY_PANES, this.panes)
  }

  toast(t: Omit<Toast, 'id'>, ms = t.kind === 'error' ? 9000 : 5000) {
    const id = ++toastSeq
    this.toasts.push({ ...t, id })
    if (this.toasts.length > 4) this.toasts.shift()
    setTimeout(() => this.dismiss(id), ms)
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id)
  }

  /** Success toast for an operation, with an Undo action when the op log says it can be undone. */
  confirm(result: OpResult, store: RepoStore) {
    const entry: OpEntry | null = result.entry
    this.toast({
      kind: 'success',
      message: result.message,
      action: entry?.undoable ? { label: 'Undo', run: () => store.undo(entry.id) } : undefined,
    })
  }

  fail(err: unknown) {
    const e = err as Partial<GitError>
    this.toast({ kind: 'error', message: e.message ?? String(err), detail: e.detail })
  }

  openMenu(e: MouseEvent, items: MenuItem[]) {
    e.preventDefault()
    e.stopPropagation()
    this.menu = { x: e.clientX, y: e.clientY, items }
  }

  /** Opens a prompt anchored under an element, or at a point. */
  ask(anchor: HTMLElement | { x: number; y: number }, p: Omit<PromptState, 'x' | 'y'>) {
    const pt =
      anchor instanceof HTMLElement
        ? (() => {
            const r = anchor.getBoundingClientRect()
            return { x: r.left, y: r.bottom + 6 }
          })()
        : anchor
    this.menu = null
    this.prompt = { ...p, ...pt }
  }
}

export const app = new AppState()
export type { AppState }
