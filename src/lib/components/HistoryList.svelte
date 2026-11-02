<script lang="ts">
  import { Check, Cloud, Tag, TriangleAlert, Pencil } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore, RefLabel } from '../state/repo.svelte'
  import type { GraphRow } from '../graph/layout'
  import { beginBranchDrag } from '../state/drag.svelte'
  import { commitMenu, branchMenu, remoteBranchMenu, tagMenu } from '../state/menus'
  import { relativeTime, fullDate, initials, middleTruncate, plural } from '../util/format'
  import GraphCell from './GraphCell.svelte'

  let { repo }: { repo: RepoStore } = $props()

  const ROW = 50
  const OVERSCAN = 10

  let scroller = $state<HTMLDivElement>()
  let scrollTop = $state(0)
  let viewH = $state(600)

  const rows = $derived(repo.layout.rows)
  const graphW = $derived(Math.max(44, 22 + (Math.min(repo.layout.maxLanes, 10) - 1) * 16 + 8))
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN))
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + viewH) / ROW) + OVERSCAN))
  const visible = $derived(rows.slice(first, last))
  const selectedId = $derived(repo.selection.kind === 'wip' ? '__wip__' : repo.selection.id)
  const dragTarget = $derived(repo.drag?.target ?? null)

  // Keep the selection in view when stepping with the keyboard.
  $effect(() => {
    const i = repo.layout.index.get(selectedId)
    if (i === undefined || !scroller) return
    const top = i * ROW
    if (top < scroller.scrollTop) scroller.scrollTop = top
    else if (top + ROW > scroller.scrollTop + viewH) scroller.scrollTop = top + ROW - viewH
  })

  // A new preview scrolls back to the top, where the ghost commits appear.
  $effect(() => {
    if (repo.preview?.created.length && scroller && !repo.drag) scroller.scrollTop = 0
  })

  // Arrow keys work through history as soon as the tab opens.
  $effect(() => {
    if (scroller && !repo.loading) scroller.focus({ preventScroll: true })
  })

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault()
      repo.step(1)
    } else if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault()
      repo.step(-1)
    } else if (e.key === 'Home') {
      e.preventDefault()
      const top = rows.find((r) => !r.commit.wip && !r.commit.ghost)
      if (top) repo.select({ kind: 'commit', id: top.commit.id })
    } else if (e.key === 'End') {
      e.preventDefault()
      const lastRow = rows[rows.length - 1]
      if (lastRow && !lastRow.commit.wip) repo.select({ kind: 'commit', id: lastRow.commit.id })
    }
  }

  function clickRow(r: GraphRow) {
    if (r.commit.ghost) return
    repo.select(r.commit.wip ? { kind: 'wip' } : { kind: 'commit', id: r.commit.id })
    if (!r.commit.wip) scroller?.focus()
  }

  function rowMenu(e: MouseEvent, r: GraphRow) {
    if (r.commit.wip || r.commit.ghost) return
    repo.select({ kind: 'commit', id: r.commit.id })
    app.openMenu(e, commitMenu(repo, r.commit, { x: e.clientX, y: e.clientY }))
  }

  function labelMenu(e: MouseEvent, l: RefLabel) {
    const at = { x: e.clientX, y: e.clientY }
    const items = l.kind === 'branch' ? branchMenu(repo, l.name, at) : l.kind === 'remote' ? remoteBranchMenu(repo, l.name, at) : tagMenu(repo, l.name)
    app.openMenu(e, items)
  }

  function chipStyle(l: RefLabel, color: number): string {
    const c = `var(--lane-${color})`
    if (l.kind === 'branch' && !l.ghost) return `background:${c}`
    if (l.kind === 'tag') return ''
    return `--c:${c}`
  }
</script>

<div class="history">
  {#if repo.loading && !repo.log.length}
    <div class="skeleton" aria-busy="true" aria-label="Loading history">
      {#each Array(10) as _, i (i)}
        <div class="sk-row"><i class="dot"></i><span><i class="bar" style="--w:{55 + ((i * 37) % 35)}%"></i><i class="bar sm"></i></span></div>
      {/each}
    </div>
  {:else if repo.loadError}
    <div class="load-error" role="alert">
      <TriangleAlert size={18} />
      <div><p>Couldn’t read this repository.</p><p class="muted">{repo.loadError.message}</p></div>
      <button class="btn sm" onclick={() => repo.load()}>Try again</button>
    </div>
  {:else}
    <div
      class="scroller"
      class:dragging={!!repo.drag}
      bind:this={scroller}
      bind:clientHeight={viewH}
      onscroll={() => (scrollTop = scroller!.scrollTop)}
      onkeydown={onKey}
      tabindex="0"
      role="listbox"
      aria-label="Commits"
      aria-activedescendant={'row-' + selectedId}
    >
      <div class="inner" style="height:{rows.length * ROW}px; --graph:{graphW}px">
        {#each visible as r, k (r.commit.id)}
          {@const i = first + k}
          {@const c = r.commit}
          {@const labels = repo.labels.get(c.id) ?? []}
          <div
            id={'row-' + c.id}
            class="row"
            class:selected={selectedId === c.id}
            class:ghost={c.ghost}
            class:void={c.void}
            class:wip={c.wip}
            class:target={dragTarget === c.id}
            style="transform:translateY({i * ROW}px)"
            role="option"
            tabindex="-1"
            aria-selected={selectedId === c.id}
            onmousedown={(e) => e.button === 0 && clickRow(r)}
            onmouseenter={() => repo.drag && !c.wip && !c.ghost && repo.dragOver(c.id)}
            oncontextmenu={(e) => rowMenu(e, r)}
            ondblclick={() => {
              const b = labels.find((l) => l.kind === 'branch' && !l.isHead && !l.ghost)
              if (b) repo.checkout(b.name)
            }}
          >
            <GraphCell row={r} width={graphW} height={ROW} isHead={!c.wip && !c.ghost && repo.head?.commit === c.id} />
            <div class="msg">
              {#if c.wip}
                <div class="l1">
                  {#if repo.summary.trim()}<Pencil size={12} /><span class="summary">{repo.summary}</span>{:else}<span class="summary wip-label">Uncommitted changes</span>{/if}
                </div>
                <div class="l2">
                  {#if repo.dirty}
                    {#if repo.conflicted.length}<em class="conf">{plural(repo.conflicted.length, 'conflict')}</em>{/if}
                    <span>{plural(repo.files.length, 'changed file')}</span>
                    {#if repo.staged.length}<span class="dotsep">·</span><em class="st">{repo.staged.length} in commit</em>{/if}
                  {:else}
                    <span>Working tree clean</span>
                  {/if}
                </div>
              {:else}
                <div class="l1">
                  {#each labels.slice(0, 1) as l (l.kind + l.name + (l.ghost ? 'g' : ''))}
                    <span
                      class="ref-chip {l.kind}"
                      class:ghost-chip={l.ghost}
                      class:struck={l.struck}
                      style={chipStyle(l, r.color)}
                      title={l.ghost ? `${l.name} moves here` : l.struck ? `${l.name} moves away` : l.kind === 'branch' ? `${l.name} · drag onto another commit to rebase` : l.name}
                      role="button"
                      tabindex="-1"
                      onmousedown={(e) => {
                        if (l.kind === 'branch' && !l.ghost && !l.struck) {
                          e.stopPropagation()
                          beginBranchDrag(e, repo, l.name)
                        }
                      }}
                      onclick={() => clickRow(r)}
                      onkeydown={(e) => e.key === 'Enter' && clickRow(r)}
                      oncontextmenu={(e) => !l.ghost && labelMenu(e, l)}
                    >
                      {#if l.isHead && !l.ghost}<Check size={11} strokeWidth={3} />{:else if l.kind === 'remote'}<Cloud size={11} />{:else if l.kind === 'tag'}<Tag size={11} />{/if}
                      <span class="chip-text">{middleTruncate(l.name, 22)}</span>
                    </span>
                  {/each}
                  {#if labels.length > 1}<span class="more" title={labels.slice(1).map((l) => l.name).join(', ')}>+{labels.length - 1}</span>{/if}
                  {#if c.ghost}<span class="new-badge" style="--c:{c.conflicted ? 'var(--warning)' : `var(--lane-${r.color})`}">{c.conflicted ? 'would conflict' : 'new'}</span>{/if}
                  <span class="summary" title={c.summary}>{c.summary}</span>
                </div>
                <div class="l2">
                  <i class="avatar">{initials(c.authorName)}</i>
                  <span class="author" title={c.authorEmail}>{c.authorName}</span>
                  <span class="dotsep">·</span>
                  <span class="date num" title={fullDate(c.authorTime)}>{relativeTime(c.authorTime)}</span>
                </div>
              {/if}
            </div>
          </div>
        {/each}
      </div>
      {#if repo.info.isEmpty && rows.length <= 1}
        <div class="empty-hint">
          <p>No commits yet.</p>
          <p class="muted">Make the first commit from the Changes tab.</p>
        </div>
      {/if}
      {#if repo.truncated}
        <p class="truncated muted" style="top:{rows.length * ROW + 8}px">Showing the newest {rows.length - 1} commits.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .history {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-0);
  }

  .scroller {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .scroller:focus-visible {
    outline: none;
  }

  .inner {
    position: relative;
  }

  .row {
    position: absolute;
    left: 0;
    right: 0;
    height: 50px;
    display: grid;
    grid-template-columns: var(--graph) minmax(0, 1fr);
    align-items: center;
    padding-right: 12px;
    contain: layout style;
    border-bottom: 1px solid rgba(255, 255, 255, 0.025);
  }

  .row:hover {
    background: rgba(255, 255, 255, 0.025);
  }

  .row.selected {
    background: var(--accent-soft);
  }

  .scroller:focus .row.selected {
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .row.target {
    background: rgba(79, 163, 255, 0.2);
    box-shadow: inset 0 0 0 1px var(--accent-line);
  }

  .dragging .row {
    cursor: copy;
  }

  .msg {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .l1,
  .l2 {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    white-space: nowrap;
  }

  .l2 {
    font-size: 12px;
    color: var(--text-2);
  }

  .summary {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 500;
  }

  .wip-label {
    color: var(--text-2);
  }

  .row.wip :global(svg.lucide) {
    color: var(--text-2);
    flex: none;
  }

  .l2 em {
    font-style: normal;
  }

  .l2 em.st {
    color: var(--success);
  }

  .l2 em.conf {
    color: var(--warning);
  }

  .dotsep {
    color: var(--text-3);
  }

  .author {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .date {
    flex: none;
  }

  .avatar {
    width: 18px;
    height: 18px;
    flex: none;
    border-radius: 50%;
    background: var(--bg-3);
    color: var(--text);
    font-style: normal;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: -0.03em;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .row.ghost .summary,
  .row.ghost .l2 {
    color: var(--text-2);
  }

  .row.ghost {
    animation: ghost-row 220ms var(--ease-out);
  }

  .row.void .summary,
  .row.void .l2 {
    text-decoration: line-through;
    text-decoration-color: rgba(255, 255, 255, 0.35);
    color: var(--text-3);
    opacity: 0.75;
  }

  .new-badge {
    flex: none;
    height: 18px;
    padding: 0 6px;
    border: 1px dashed var(--c);
    border-radius: var(--radius-sm);
    color: var(--c);
    font-size: 11px;
    font-weight: 600;
    line-height: 16px;
  }

  /* One chip at its natural width; the summary gives way first. */
  .ref-chip {
    cursor: grab;
    flex: none;
    max-width: 140px;
  }

  .chip-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ref-chip.remote,
  .ref-chip.ghost-chip {
    background: transparent;
    color: var(--c);
    box-shadow: inset 0 0 0 1px var(--c);
  }

  .ref-chip.ghost-chip {
    box-shadow: none;
    outline: 1px dashed var(--c);
    outline-offset: -1px;
  }

  .ref-chip.tag {
    background: var(--bg-3);
    color: var(--text);
  }

  .ref-chip.struck {
    text-decoration: line-through;
    opacity: 0.45;
  }

  .ref-chip.remote,
  .ref-chip.tag {
    cursor: default;
  }

  .more {
    font-size: 11.5px;
    color: var(--text-2);
    flex: none;
  }

  .muted {
    color: var(--text-2);
  }

  .skeleton {
    padding-top: 4px;
  }

  .sk-row {
    height: 50px;
    display: flex;
    align-items: center;
    gap: 14px;
    padding-left: 14px;
  }

  .sk-row span {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .sk-row .dot {
    width: 15px;
    height: 15px;
    border-radius: 50%;
    background: var(--bg-2);
    flex: none;
  }

  .sk-row .bar {
    height: 9px;
    width: var(--w);
    border-radius: var(--radius-sm);
    background: linear-gradient(90deg, var(--bg-2), var(--bg-3), var(--bg-2));
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }

  .sk-row .bar.sm {
    width: 35%;
    height: 7px;
  }

  .load-error {
    margin: 16px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: flex-start;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
  }

  .load-error :global(svg) {
    color: var(--warning);
  }

  .empty-hint {
    position: absolute;
    top: 72px;
    left: 0;
    right: 0;
    padding: 0 16px;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .empty-hint p:first-child {
    font-size: 14px;
    font-weight: 600;
  }

  .truncated {
    position: absolute;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 12px;
    padding-bottom: 16px;
  }

  @keyframes ghost-row {
    from {
      opacity: 0;
    }
  }

  @keyframes shimmer {
    to {
      background-position: -200% 0;
    }
  }
</style>
