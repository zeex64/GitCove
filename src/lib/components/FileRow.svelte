<script lang="ts">
  import { RotateCcw } from '@lucide/svelte'
  import type { ChangeKind } from '../api/types'
  import { splitPath } from '../util/format'
  import Checkbox from './Checkbox.svelte'

  let {
    path,
    origPath = null,
    kind,
    selected = false,
    conflict = false,
    stats = null,
    checked,
    oncheck,
    onselect,
    onopen,
    ondiscard,
    oncontextmenu,
  }: {
    path: string
    origPath?: string | null
    kind: ChangeKind
    selected?: boolean
    conflict?: boolean
    stats?: { additions: number; deletions: number } | null
    /** Shown as a checkbox when set: in the next commit, not, or partly. */
    checked?: boolean | 'mixed'
    oncheck?: () => void
    onselect: () => void
    onopen?: () => void
    ondiscard?: () => void
    oncontextmenu?: (e: MouseEvent) => void
  } = $props()

  const LETTER: Record<ChangeKind, string> = {
    added: 'A',
    untracked: 'A',
    modified: 'M',
    deleted: 'D',
    renamed: 'R',
    copied: 'C',
    typeChanged: 'T',
    conflicted: '!',
  }

  const TITLE: Record<ChangeKind, string> = {
    added: 'Added',
    untracked: 'New file',
    modified: 'Modified',
    deleted: 'Deleted',
    renamed: 'Renamed',
    copied: 'Copied',
    typeChanged: 'Type changed',
    conflicted: 'Conflicted',
  }

  const k = $derived<ChangeKind>(conflict ? 'conflicted' : kind)
  const parts = $derived(splitPath(path))
</script>

<div
  class="row"
  class:selected
  role="option"
  aria-selected={selected}
  tabindex="0"
  onclick={onselect}
  ondblclick={onopen}
  {oncontextmenu}
  onkeydown={(e) => {
    if (e.key === ' ' && oncheck && !conflict) {
      e.preventDefault()
      oncheck()
    } else if (e.key === 'Enter') onselect()
  }}
  title={origPath ? `${origPath} → ${path}` : path}
>
  {#if checked !== undefined && oncheck}
    <Checkbox
      {checked}
      disabled={conflict}
      label={checked === true ? `Leave ${parts.name} out of the commit` : `Include ${parts.name} in the commit`}
      title={conflict ? 'Resolve the conflict first' : checked === 'mixed' ? 'Partly included. Click to include the whole file' : undefined}
      onchange={oncheck}
    />
  {/if}
  <span class="path">
    {#if parts.dir}<span class="dir">{parts.dir}</span>{/if}<span class="name">{parts.name}</span>
  </span>
  {#if stats}
    <span class="stats mono num"><span class="a">+{stats.additions}</span> <span class="d">−{stats.deletions}</span></span>
  {/if}
  {#if ondiscard}
    <button class="icon-btn discard" aria-label="Discard changes to {parts.name}" title="Discard changes…" onclick={(e) => { e.stopPropagation(); ondiscard() }}><RotateCcw size={13} /></button>
  {/if}
  <span class="badge {k}" title={TITLE[k]}>{LETTER[k]}</span>
</div>

<style>
  .row {
    height: 30px;
    margin: 0 6px;
    padding: 0 8px 0 10px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .row:hover {
    background: var(--bg-2);
  }

  .row.selected {
    background: var(--bg-3);
  }

  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .badge {
    width: 17px;
    height: 17px;
    flex: none;
    border-radius: var(--radius-sm);
    font-size: 10.5px;
    font-weight: 800;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--on-color);
    background: var(--warning);
  }

  .badge.added,
  .badge.untracked {
    background: var(--success);
  }

  .badge.deleted {
    background: var(--danger);
  }

  .badge.renamed,
  .badge.copied,
  .badge.typeChanged {
    background: var(--accent);
  }

  .badge.conflicted {
    background: transparent;
    color: var(--warning);
    box-shadow: inset 0 0 0 1.5px var(--warning);
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .dir {
    color: var(--text-2);
  }

  .stats {
    flex: none;
    font-size: 11.5px;
  }

  .stats .a {
    color: var(--add-text);
  }

  .stats .d {
    color: var(--del-text);
  }

  .discard {
    width: 22px;
    height: 22px;
    opacity: 0;
  }

  .row:hover .discard,
  .row:focus-within .discard {
    opacity: 1;
  }
</style>
