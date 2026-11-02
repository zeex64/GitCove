<script lang="ts">
  import { Plus, Minus, RotateCcw } from '@lucide/svelte'
  import type { ChangeKind } from '../api/types'
  import { splitPath } from '../util/format'

  let {
    path,
    origPath = null,
    kind,
    selected = false,
    staged = false,
    conflict = false,
    stats = null,
    onselect,
    onopen,
    ontoggle,
    ondiscard,
    oncontextmenu,
  }: {
    path: string
    origPath?: string | null
    kind: ChangeKind
    selected?: boolean
    staged?: boolean
    conflict?: boolean
    stats?: { additions: number; deletions: number } | null
    onselect: () => void
    onopen?: () => void
    ontoggle?: () => void
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
    untracked: 'New file, not yet tracked',
    modified: 'Modified',
    deleted: 'Deleted',
    renamed: 'Renamed',
    copied: 'Copied',
    typeChanged: 'Type changed',
    conflicted: 'Conflicted',
  }

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
  oncontextmenu={oncontextmenu}
  onkeydown={(e) => {
    if (e.key === ' ' && ontoggle) {
      e.preventDefault()
      ontoggle()
    } else if (e.key === 'Enter') onselect()
  }}
  title={origPath ? `${origPath} → ${path}` : path}
>
  <span class="badge {conflict ? 'conflicted' : kind}" title={TITLE[conflict ? 'conflicted' : kind]}>{LETTER[conflict ? 'conflicted' : kind]}</span>
  <span class="path">
    {#if parts.dir}<span class="dir">{parts.dir}</span>{/if}<span class="name">{parts.name}</span>
  </span>
  {#if stats}
    <span class="stats mono num"><span class="a">+{stats.additions}</span> <span class="d">−{stats.deletions}</span></span>
  {/if}
  <span class="actions">
    {#if ondiscard}
      <button class="icon-btn" aria-label="Discard changes" title="Discard changes…" onclick={(e) => { e.stopPropagation(); ondiscard() }}><RotateCcw size={14} /></button>
    {/if}
    {#if ontoggle}
      <button
        class="icon-btn toggle"
        aria-label={staged ? 'Unstage' : 'Stage'}
        title={staged ? 'Unstage (Space)' : 'Stage (Space)'}
        onclick={(e) => { e.stopPropagation(); ontoggle() }}
      >
        {#if staged}<Minus size={15} />{:else}<Plus size={15} />{/if}
      </button>
    {/if}
  </span>
</div>

<style>
  .row {
    height: 30px;
    margin: 0 6px;
    padding: 0 6px 0 10px;
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
    direction: ltr;
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

  .actions {
    display: flex;
    gap: 2px;
    flex: none;
    opacity: 0;
  }

  .row:hover .actions,
  .row.selected .actions,
  .row:focus-within .actions {
    opacity: 1;
  }

  .toggle:hover:not(:disabled) {
    background: var(--accent);
    color: var(--on-color);
  }
</style>
