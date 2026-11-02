<script lang="ts">
  import { Check, Cloud, Tag, TriangleAlert, ArrowLeft, Pencil } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore, RefLabel } from '../state/repo.svelte'
  import type { GraphRow } from '../graph/layout'
  import { beginBranchDrag } from '../state/drag.svelte'
  import { commitMenu, branchMenu, remoteBranchMenu, tagMenu } from '../state/menus'
  import { relativeTime, fullDate, initials, middleTruncate, plural } from '../util/format'
  import GraphCell from './GraphCell.svelte'
  import PreviewBanner from './PreviewBanner.svelte'
  import OperationBanner from './OperationBanner.svelte'
  import DiffView from './DiffView.svelte'

  let { repo }: { repo: RepoStore } = $props()

  const ROW = 34
  const OVERSCAN = 12

  let scroller = $state<HTMLDivElement>()
  let scrollTop = $state(0)
  let viewH = $state(600)

  const rows = $derived(repo.layout.rows)
  const graphW = $derived(Math.max(72, 28 + (Math.min(repo.layout.maxLanes, 12) - 1) * 16 + 8))
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN))
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + viewH) / ROW) + OVERSCAN))
  const visible = $derived(rows.slice(first, last))
  const selectedId = $derived(repo.selection.kind === 'wip' ? '__wip__' : repo.selection.id)
  const dragTarget = $derived(repo.drag?.target ?? null)
  const showBanner = $derived(!!(repo.preview || repo.previewLoading || repo.previewError || repo.drag))

  // Keep the selection in view when stepping with the keyboard.
  $effect(() => {
    const i = repo.layout.index.get(selectedId)
    if (i === undefined || !scroller) return
    const top = i * ROW
    if (top < scroller.scrollTop) scroller.scrollTop = top
    else if (top + ROW > scroller.scrollTop + viewH) scroller.scrollTop = top + ROW - viewH
  })

  // Arrow keys work through history as soon as the repo opens.
  let focusedOnce = false
  $effect(() => {
    if (focusedOnce || !scroller || repo.loading) return
    focusedOnce = true
    const active = document.activeElement
    if (!active || active === document.body || active.closest('.toolbar, .strip')) scroller.focus({ preventScroll: true })
  })

  // A new preview scrolls back to the top, where the ghost commits appear.
  $effect(() => {
    if (repo.preview?.created.length && scroller && !repo.drag) scroller.scrollTop = 0
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
      repo.select({ kind: 'wip' })
    } else if (e.key === 'End') {
      e.preventDefault()
      const lastRow = rows[rows.length - 1]
      if (lastRow && !lastRow.commit.wip) repo.select({ kind: 'commit', id: lastRow.commit.id })
    }
  }

  function clickRow(r: GraphRow) {
    if (r.commit.ghost) return
    repo.select(r.commit.wip ? { kind: 'wip' } : { kind: 'commit', id: r.commit.id })
    scroller?.focus()
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

  const wipCounts = $derived({ staged: repo.staged.length, unstaged: repo.unstaged.length, conflicted: repo.conflicted.length })
</script>

<section class="history" aria-label="History">
  {#if repo.status && repo.status.state !== 'clean'}
    <OperationBanner {repo} />
  {/if}
  {#if showBanner}
    <PreviewBanner {repo} />
  {/if}

  {#if repo.fullDiff && repo.diff}
    <div class="full">
      <header class="full-head">
        <button class="btn sm quiet" onclick={() => (repo.fullDiff = false)}><ArrowLeft size={14} />History</button>
        <span class="mono path">{repo.diff.path}</span>
        <span class="muted">{repo.fileSel?.staged ? 'Staged' : 'Unstaged'} changes</span>
      </header>
      <DiffView {repo} diff={repo.diff} editable={!repo.conflicted.some((f) => f.path === repo.fileSel?.path)} staged={!!repo.fileSel?.staged} sides={repo.conflictSides} />
    </div>
  {:else}
    <div class="cols" style="--graph:{graphW}px">
      <span class="section-label">Graph</span>
      <span class="section-label">Commit message</span>
      <span class="section-label">Author</span>
      <span class="section-label">Date</span>
      <span class="section-label">SHA</span>
    </div>

    {#if repo.loading && !repo.log.length}
      <div class="skeleton" aria-busy="true" aria-label="Loading history">
        {#each Array(14) as _, i (i)}
          <div class="sk-row" style="--w:{40 + ((i * 37) % 45)}%"><i class="dot"></i><i class="bar"></i></div>
        {/each}
      </div>
    {:else if repo.loadError}
      <div class="load-error" role="alert">
        <TriangleAlert size={18} />
        <div><p>Couldn’t read this repository.</p><p class="muted">{repo.loadError.message}</p></div>
        <button class="btn" onclick={() => repo.load()}>Try again</button>
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
              <GraphCell row={r} width={graphW} isHead={!c.wip && !c.ghost && repo.head?.commit === c.id} />
              <div class="msg">
                {#if c.wip}
                  {#if repo.summary.trim()}
                    <Pencil size={13} /><span class="summary">{repo.summary}</span>
                  {:else}
                    <span class="wip-label">Uncommitted changes</span>
                  {/if}
                  {#if repo.dirty}
                    <span class="counts num">
                      {#if wipCounts.conflicted}<em class="conf">{plural(wipCounts.conflicted, 'conflict')}</em>{/if}
                      {#if wipCounts.unstaged}<em>{wipCounts.unstaged} unstaged</em>{/if}
                      {#if wipCounts.staged}<em class="st">{wipCounts.staged} staged</em>{/if}
                    </span>
                  {:else}
                    <span class="muted">Working tree clean</span>
                  {/if}
                {:else}
                  {#each labels.slice(0, 3) as l (l.kind + l.name + (l.ghost ? 'g' : ''))}
                    <span
                      class="ref-chip {l.kind}"
                      class:ghost-chip={l.ghost}
                      class:struck={l.struck}
                      style={chipStyle(l, r.color)}
                      title={l.ghost ? `${l.name} moves here` : l.struck ? `${l.name} moves away` : l.name}
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
                      <span class="chip-text">{middleTruncate(l.name, 26)}</span>
                    </span>
                  {/each}
                  {#if labels.length > 3}<span class="more" title={labels.slice(3).map((l) => l.name).join(', ')}>+{labels.length - 3}</span>{/if}
                  {#if c.ghost}<span class="new-badge">{c.conflicted ? 'would conflict' : 'new'}</span>{/if}
                  <span class="summary">{c.summary}</span>
                {/if}
              </div>
              {#if c.wip}
                <span></span><span></span><span></span>
              {:else}
                <span class="author" title={c.authorEmail}>
                  <i class="avatar">{initials(c.authorName)}</i>
                  <span>{c.authorName}</span>
                </span>
                <span class="date num" title={fullDate(c.authorTime)}>{relativeTime(c.authorTime)}</span>
                <span class="mono sha">{c.shortId}</span>
              {/if}
            </div>
          {/each}
        </div>
        {#if repo.info.isEmpty && rows.length <= 1}
          <div class="empty-hint">
            <p>No commits yet.</p>
            <p class="muted">Stage files on the right and make the first commit.</p>
          </div>
        {/if}
        {#if repo.truncated}
          <p class="truncated muted" style="top:{rows.length * ROW + 8}px">Showing the newest {rows.length - 1} commits.</p>
        {/if}
      </div>
    {/if}
  {/if}
</section>

<style>
  .history {
    position: relative;
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg-0);
  }

  .cols,
  .row {
    display: grid;
    grid-template-columns: var(--graph) minmax(0, 1fr) 150px 104px 76px;
    align-items: center;
    padding-right: 14px;
  }

  .cols {
    height: 30px;
    flex: none;
    border-bottom: 1px solid var(--line);
  }

  .cols span:first-child {
    padding-left: 12px;
  }

  /* Narrow history: columns tighten first, then SHA, author and date step aside in turn. */
  @container (max-width: 900px) {
    .cols,
    .row {
      grid-template-columns: var(--graph) minmax(0, 1fr) 118px 92px 70px;
    }
  }

  @container (max-width: 680px) {
    .cols,
    .row {
      grid-template-columns: var(--graph) minmax(0, 1fr) 118px 92px 0;
    }

    .cols span:nth-child(5),
    .row > .sha {
      visibility: hidden;
      overflow: hidden;
    }
  }

  @container (max-width: 640px) {
    .cols,
    .row {
      grid-template-columns: var(--graph) minmax(0, 1fr) 0 88px 0;
    }

    .cols span:nth-child(3),
    .row > .author {
      visibility: hidden;
      overflow: hidden;
    }
  }

  @container (max-width: 420px) {
    .cols,
    .row {
      grid-template-columns: var(--graph) minmax(0, 1fr) 0 0 0;
    }

    .cols span:nth-child(4),
    .row > .date {
      visibility: hidden;
      overflow: hidden;
    }
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
    height: 34px;
    contain: layout style;
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
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding-right: 12px;
  }

  .summary {
    flex: 1 1 120px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .row.ghost .summary,
  .row.ghost .author,
  .row.ghost .date {
    color: var(--text-2);
  }

  .row.ghost {
    animation: ghost-row 220ms var(--ease-out);
  }

  .row.void .summary,
  .row.void .author,
  .row.void .date,
  .row.void .sha {
    text-decoration: line-through;
    text-decoration-color: rgba(255, 255, 255, 0.35);
    color: var(--text-3);
    opacity: 0.75;
  }

  .new-badge {
    flex: none;
    height: 18px;
    padding: 0 6px;
    border: 1px dashed var(--accent-line);
    border-radius: var(--radius-sm);
    color: var(--accent);
    font-size: 11px;
    font-weight: 600;
    line-height: 16px;
  }

  .wip-label {
    color: var(--text-2);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .row.wip :global(svg.lucide) {
    color: var(--text-2);
    flex: none;
  }

  .counts {
    display: flex;
    gap: 6px;
    flex: none;
  }

  .counts em {
    white-space: nowrap;
    font-style: normal;
    font-size: 11.5px;
    color: var(--text-2);
    background: var(--bg-2);
    border-radius: var(--radius-sm);
    padding: 1px 7px;
  }

  .counts em.st {
    color: var(--success);
  }

  .counts em.conf {
    color: var(--warning);
  }

  .muted {
    color: var(--text-2);
  }

  .ref-chip {
    cursor: grab;
    flex: 0 1 auto;
    min-width: 44px;
  }

  .chip-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .author {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    color: var(--text-2);
  }

  .author span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .avatar {
    width: 20px;
    height: 20px;
    flex: none;
    border-radius: 50%;
    background: var(--bg-3);
    color: var(--text);
    font-style: normal;
    font-size: 11px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .date {
    color: var(--text-2);
    white-space: nowrap;
  }

  .sha {
    color: var(--text-2);
  }

  .skeleton {
    padding-top: 4px;
  }

  .sk-row {
    height: 34px;
    display: flex;
    align-items: center;
    gap: 18px;
    padding-left: 10px;
  }

  .sk-row .dot {
    width: 15px;
    height: 15px;
    border-radius: 50%;
    background: var(--bg-2);
  }

  .sk-row .bar {
    height: 10px;
    width: var(--w);
    border-radius: var(--radius-sm);
    background: linear-gradient(90deg, var(--bg-2), var(--bg-3), var(--bg-2));
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }

  .load-error {
    margin: 24px;
    padding: 16px;
    display: flex;
    gap: 12px;
    align-items: flex-start;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    color: var(--text);
  }

  .load-error :global(svg) {
    color: var(--warning);
    flex: none;
  }

  .load-error div {
    flex: 1;
  }

  .empty-hint {
    position: absolute;
    top: 64px;
    left: 0;
    right: 0;
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

  .full {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .full-head {
    height: 40px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 12px;
    border-bottom: 1px solid var(--line);
  }

  .full-head .path {
    font-weight: 500;
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
