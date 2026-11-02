<script lang="ts">
  import {
    Undo2,
    Redo2,
    ArrowDownToLine,
    ArrowUpFromLine,
    GitBranchPlus,
    GitBranch,
    ChevronDown,
    Search,
    FolderGit2,
    Check,
    TriangleAlert,
    Cloud,
  } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import Popover from './Popover.svelte'
  import { modKey, pickFolder } from '../api/system'
  import { relativeTime, middleTruncate } from '../util/format'
  import { refNameError } from '../util/refname'

  let { repo }: { repo: RepoStore } = $props()

  let repoBtn = $state<HTMLButtonElement>()
  let branchBtn = $state<HTMLButtonElement>()
  let undoBtn = $state<HTMLDivElement>()
  let newBranchBtn = $state<HTMLButtonElement>()
  let repoOpen = $state(false)
  let branchOpen = $state(false)
  let undoOpen = $state(false)
  let filter = $state('')
  let branchIdx = $state(0)

  const head = $derived(repo.head)
  const hb = $derived(repo.headBranch)
  const upstreamRemote = $derived(hb?.upstream?.split('/')[0] ?? repo.primaryRemote?.name ?? null)
  const canPull = $derived(!!hb?.upstream && !repo.busy.pull)
  const canPush = $derived(!!hb && !!upstreamRemote && !repo.busy.push && !head?.unborn)

  const branchChoices = $derived.by(() => {
    const q = filter.trim().toLowerCase()
    const locals = (repo.refs?.branches ?? []).map((b) => ({ name: b.name, remote: false, current: b.isHead }))
    const localNames = new Set(locals.map((b) => b.name))
    const remotes = (repo.refs?.remoteBranches ?? [])
      .filter((r) => r.name !== 'HEAD' && !localNames.has(r.name))
      .map((r) => ({ name: r.fullName, remote: true, current: false }))
    return [...locals, ...remotes].filter((b) => !q || b.name.toLowerCase().includes(q))
  })

  $effect(() => {
    void filter
    branchIdx = 0
  })

  function switchTo(name: string) {
    branchOpen = false
    filter = ''
    repo.checkout(name)
  }

  function newBranch(anchor: HTMLElement | undefined) {
    if (!anchor) return
    branchOpen = false
    const from = head?.branch ?? head?.commit?.slice(0, 7) ?? 'HEAD'
    app.ask(anchor, {
      title: `New branch from ${from}`,
      fields: [{ name: 'name', label: 'Branch name', placeholder: 'feat/something', mono: true }],
      confirm: 'Create branch',
      check: { label: 'Switch to it', value: true },
      validate: (v) => refNameError(v.name, repo.refs?.branches.map((b) => b.name)),
      onSubmit: (v, checkout) => repo.createBranch(v.name, head?.commit ?? 'HEAD', checkout),
    })
  }

  async function openRepo() {
    repoOpen = false
    const path = await pickFolder('Open a repository')
    if (!path) return
    const e = await app.openPath(path)
    if (e) app.fail(e)
  }

  function onBranchKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      branchIdx = Math.min(branchChoices.length - 1, branchIdx + 1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      branchIdx = Math.max(0, branchIdx - 1)
    } else if (e.key === 'Enter' && branchChoices[branchIdx]) {
      switchTo(branchChoices[branchIdx].name)
    }
  }
</script>

<div class="toolbar">
  <div class="left">
    <button class="pill repo-pill" title={repo.path} bind:this={repoBtn} onclick={() => (repoOpen = !repoOpen)} aria-haspopup="true" aria-expanded={repoOpen}>
      <FolderGit2 size={15} />
      <span class="pill-text">{repo.name}</span>
      <ChevronDown size={14} />
    </button>
    <button
      class="pill"
      class:warn={repo.detached}
      bind:this={branchBtn}
      onclick={() => (branchOpen = !branchOpen)}
      aria-haspopup="true"
      aria-expanded={branchOpen}
      title={repo.detached ? 'HEAD is detached: new commits won’t be on any branch' : 'Switch branch'}
    >
      {#if repo.detached}<TriangleAlert size={15} />{:else}<GitBranch size={15} />{/if}
      <span class="pill-text">
        {#if repo.detached}Detached at {head?.commit?.slice(0, 7)}{:else}{middleTruncate(repo.branchName ?? head?.branch ?? 'main', 30)}{/if}
      </span>
      <ChevronDown size={14} />
    </button>
  </div>

  <div class="tools">
    <div class="split" bind:this={undoBtn}>
      <button
        class="tool"
        disabled={!repo.undoable}
        onclick={() => repo.undo()}
        title={repo.undoable ? `Undo: ${repo.undoable.label} (${modKey} Z)` : 'Nothing to undo'}
      >
        <Undo2 size={18} /><span>Undo</span>
      </button>
      <button class="caret" aria-label="Undo history" disabled={!repo.oplog.length} onclick={() => (undoOpen = !undoOpen)}>
        <ChevronDown size={13} />
      </button>
    </div>
    <button
      class="tool"
      disabled={!repo.redoable}
      onclick={() => repo.redo()}
      title={repo.redoable ? `Redo: ${repo.redoable.label} (${modKey} Shift Z)` : 'Nothing to redo'}
    >
      <Redo2 size={18} /><span>Redo</span>
    </button>

    <div class="sep"></div>

    <button
      class="tool"
      disabled={!canPull}
      onclick={() => repo.pull()}
      title={!hb?.upstream ? 'This branch has no upstream to pull from' : `Pull from ${hb.upstream}`}
    >
      <ArrowDownToLine size={18} />
      <span>{repo.busy.pull ? 'Pulling' : 'Pull'}</span>
      {#if hb && hb.behind > 0}<em class="badge num">{hb.behind}</em>{/if}
      {#if repo.busy.pull}<i class="progress" style="--p:{repo.busy.pull.percent ?? 30}%"></i>{/if}
    </button>
    <button
      class="tool"
      disabled={!canPush}
      onclick={() => repo.push()}
      title={!upstreamRemote ? 'No remote set up' : hb?.upstream ? `Push to ${hb.upstream}` : `Push and track ${upstreamRemote}/${hb?.name}`}
    >
      <ArrowUpFromLine size={18} />
      <span>{repo.busy.push ? 'Pushing' : 'Push'}</span>
      {#if hb && (hb.ahead > 0 || (!hb.upstream && upstreamRemote))}<em class="badge num">{hb.upstream ? hb.ahead : 'new'}</em>{/if}
      {#if repo.busy.push}<i class="progress" style="--p:{repo.busy.push.percent ?? 30}%"></i>{/if}
    </button>
    <button class="tool" bind:this={newBranchBtn} onclick={() => newBranch(newBranchBtn)} disabled={head?.unborn} title="Create a branch here">
      <GitBranchPlus size={18} /><span>Branch</span>
    </button>
  </div>

  <div class="right">
    <button class="search" onclick={() => (app.paletteOpen = true)}>
      <Search size={14} />
      <span>Search or run a command</span>
      <span class="kbd">{modKey} K</span>
    </button>
  </div>
</div>

<Popover bind:open={repoOpen} anchor={repoBtn} width={320}>
  <div class="pop-head">Repositories</div>
  <div class="list">
    {#each app.recent as r (r.path)}
      <button class="item" onclick={() => { repoOpen = false; app.openPath(r.path) }}>
        <FolderGit2 size={15} />
        <span class="item-main"><span>{r.name}</span><small>{r.path}</small></span>
        {#if r.path === repo.path}<Check size={14} />{/if}
      </button>
    {/each}
  </div>
  <div class="pop-foot">
    <button class="btn sm ghost" onclick={openRepo}>Open folder…</button>
    <button class="btn sm quiet" onclick={() => { repoOpen = false; app.newTab() }}>New tab</button>
  </div>
</Popover>

<Popover bind:open={branchOpen} anchor={branchBtn} width={340}>
  <div class="filter">
    <!-- svelte-ignore a11y_autofocus -->
    <input class="field" placeholder="Switch to branch…" bind:value={filter} onkeydown={onBranchKey} autofocus spellcheck="false" />
  </div>
  <div class="list">
    {#each branchChoices as b, i (b.name)}
      <button class="item" class:active={i === branchIdx} onmouseenter={() => (branchIdx = i)} onclick={() => switchTo(b.name)} disabled={b.current}>
        {#if b.remote}<Cloud size={14} />{:else}<GitBranch size={14} />{/if}
        <span class="item-main"><span class="mono-ish">{b.name}</span></span>
        {#if b.current}<small class="cur">current</small>{/if}
      </button>
    {:else}
      <p class="empty">No branches match “{filter}”.</p>
    {/each}
  </div>
  <div class="pop-foot">
    <button class="btn sm ghost" onclick={() => newBranch(branchBtn)}><GitBranchPlus size={14} />New branch…</button>
  </div>
</Popover>

<Popover bind:open={undoOpen} anchor={undoBtn} width={360} align="center">
  <div class="pop-head">Undo history</div>
  <div class="list">
    {#each repo.oplog as e (e.id)}
      <button
        class="item op"
        class:undone={e.undone}
        disabled={!e.undoable || e.undone}
        title={e.reason ?? (e.undone ? 'Already undone' : 'Undo back to here')}
        onclick={() => {
          undoOpen = false
          repo.undo(e.id)
        }}
      >
        <span class="item-main"><span>{e.label}</span>{#if !e.undoable && e.reason}<small>{e.reason}</small>{/if}</span>
        <small class="num">{relativeTime(e.time)}</small>
      </button>
    {/each}
  </div>
  <div class="pop-foot hint">Choosing an entry undoes it and everything after it.</div>
</Popover>

<style>
  .toolbar {
    height: 64px;
    flex: none;
    container-type: inline-size;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    padding: 0 12px;
    background: var(--bg-1);
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }

  .left {
    display: flex;
    gap: 8px;
    min-width: 0;
  }

  .pill {
    height: 32px;
    min-width: 0;
    max-width: 260px;
    padding: 0 10px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: var(--bg-2);
    display: flex;
    align-items: center;
    gap: 7px;
    font-weight: 500;
    color: var(--text);
    transition: background-color 120ms var(--ease-out);
  }

  .pill:hover {
    background: var(--bg-3);
  }

  .pill :global(svg) {
    flex: none;
    color: var(--text-2);
  }

  .pill.warn {
    border-color: rgba(242, 193, 78, 0.45);
    background: var(--warning-soft);
  }

  .pill.warn :global(svg:first-child) {
    color: var(--warning);
  }

  .pill-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The tab already names the repo; on narrow windows its pill gives the branch the room. */
  @container (max-width: 1180px) {
    .repo-pill .pill-text {
      display: none;
    }

    .right .search span:nth-child(2) {
      display: none;
    }

    .right .search {
      width: auto;
    }
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .tool {
    position: relative;
    width: 62px;
    height: 48px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    font-size: 11.5px;
    font-weight: 500;
    overflow: hidden;
    transition: background-color 120ms var(--ease-out);
  }

  .tool :global(svg) {
    color: var(--text-2);
  }

  .tool:hover:not(:disabled) {
    background: var(--bg-2);
  }

  .tool:hover:not(:disabled) :global(svg) {
    color: var(--text);
  }

  .tool:active:not(:disabled) {
    background: var(--bg-3);
  }

  .tool:disabled {
    opacity: 0.4;
  }

  .split {
    display: flex;
    align-items: center;
  }

  .split .tool {
    width: 54px;
    border-radius: var(--radius-md) 0 0 var(--radius-md);
  }

  .caret {
    width: 18px;
    height: 48px;
    border: 0;
    border-radius: 0 var(--radius-md) var(--radius-md) 0;
    background: transparent;
    color: var(--text-2);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .caret:hover:not(:disabled) {
    background: var(--bg-2);
    color: var(--text);
  }

  .caret:disabled {
    opacity: 0.4;
  }

  .badge {
    position: absolute;
    top: 4px;
    right: 9px;
    min-width: 17px;
    height: 16px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-color);
    font-style: normal;
    font-size: 10.5px;
    font-weight: 700;
    line-height: 16px;
  }

  .progress {
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: 3px;
    height: 2px;
    border-radius: 999px;
    background: var(--bg-3);
    overflow: hidden;
  }

  .progress::after {
    content: '';
    position: absolute;
    inset: 0;
    width: var(--p);
    background: var(--accent);
    transition: width 200ms var(--ease-out);
  }

  .sep {
    width: 1px;
    height: 30px;
    margin: 0 8px;
    background: var(--line-strong);
  }

  .right {
    display: flex;
    justify-content: flex-end;
  }

  .search {
    height: 32px;
    width: min(300px, 100%);
    padding: 0 8px 0 10px;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-md);
    background: var(--bg-0);
    color: var(--text-2);
    display: flex;
    align-items: center;
    gap: 8px;
    transition: border-color 120ms var(--ease-out);
  }

  .search:hover {
    border-color: var(--text-3);
  }

  .search span:nth-child(2) {
    flex: 1;
    text-align: left;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .pop-head {
    padding: 12px 14px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }

  .filter {
    padding: 10px;
    border-bottom: 1px solid var(--line-strong);
  }

  .filter .field {
    background: var(--bg-1);
  }

  .list {
    overflow-y: auto;
    padding: 4px 5px;
  }

  .item {
    width: 100%;
    min-height: 32px;
    padding: 5px 9px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    display: flex;
    align-items: center;
    gap: 9px;
    text-align: left;
  }

  .item :global(svg) {
    flex: none;
    color: var(--text-2);
  }

  .item:hover:not(:disabled),
  .item.active:not(:disabled) {
    background: var(--bg-3);
  }

  .item:disabled {
    cursor: default;
  }

  .item-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .item-main > span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item small {
    color: var(--text-2);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cur {
    flex: none;
  }

  .op.undone .item-main > span {
    text-decoration: line-through;
    color: var(--text-2);
  }

  .op:disabled:not(.undone) .item-main > span {
    color: var(--text-2);
  }

  .empty {
    padding: 10px;
    color: var(--text-2);
  }

  .pop-foot {
    padding: 8px 10px;
    border-top: 1px solid var(--line-strong);
    display: flex;
    gap: 6px;
  }

  .pop-foot.hint {
    color: var(--text-2);
    font-size: 12px;
  }
</style>
