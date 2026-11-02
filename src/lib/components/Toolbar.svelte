<script lang="ts">
  import {
    Undo2,
    Redo2,
    ArrowDownToLine,
    ArrowUpFromLine,
    RefreshCw,
    CloudUpload,
    CloudOff,
    GitBranch,
    ChevronDown,
    Search,
    FolderGit2,
    Check,
    TriangleAlert,
  } from '@lucide/svelte'
  import { onMount } from 'svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import Popover from './Popover.svelte'
  import BranchMenu from './BranchMenu.svelte'
  import { modKey, pickFolder } from '../api/system'
  import { relativeTime } from '../util/format'

  let { repo }: { repo: RepoStore } = $props()

  let repoBtn = $state<HTMLButtonElement>()
  let branchBtn = $state<HTMLButtonElement>()
  let undoBox = $state<HTMLDivElement>()
  let repoOpen = $state(false)
  let branchOpen = $state(false)
  let undoOpen = $state(false)
  let now = $state(Date.now())

  onMount(() => {
    const t = setInterval(() => (now = Date.now()), 30_000)
    return () => clearInterval(t)
  })

  const head = $derived(repo.head)
  const hb = $derived(repo.headBranch)
  const remote = $derived(hb?.upstream?.split('/')[0] ?? repo.primaryRemote?.name ?? null)
  const fetched = $derived.by(() => {
    void now
    if (repo.offline) return 'Offline. Couldn’t reach the remote'
    return repo.lastFetch ? `Last fetched ${relativeTime(repo.lastFetch)}` : 'Not fetched yet'
  })

  type Sync = { kind: 'fetch' | 'pull' | 'push' | 'publish' | 'none'; title: string; caption: string; busy: boolean; percent: number | null; ahead?: number; behind?: number }
  const sync = $derived.by<Sync>(() => {
    const b = repo.busy
    if (!remote) return { kind: 'none', title: 'No remote', caption: 'Add one from the branch menu', busy: false, percent: null }
    if (b.push) return { kind: 'push', title: `Pushing to ${remote}`, caption: b.push.phase || 'Sending commits…', busy: true, percent: b.push.percent }
    if (b.pull) return { kind: 'pull', title: `Pulling from ${remote}`, caption: b.pull.phase || 'Getting commits…', busy: true, percent: b.pull.percent }
    if (b.fetch) return { kind: 'fetch', title: `Fetching ${remote}`, caption: b.fetch.phase || 'Checking for new commits…', busy: true, percent: b.fetch.percent }
    if (hb && !hb.upstream && !head?.unborn) return { kind: 'publish', title: 'Publish branch', caption: `Put ${hb.name} on ${remote}`, busy: false, percent: null }
    if (hb && hb.behind > 0) return { kind: 'pull', title: `Pull ${remote}`, caption: fetched, busy: false, percent: null, behind: hb.behind, ahead: hb.ahead }
    if (hb && hb.ahead > 0) return { kind: 'push', title: `Push ${remote}`, caption: fetched, busy: false, percent: null, ahead: hb.ahead }
    return { kind: 'fetch', title: `Fetch ${remote}`, caption: fetched, busy: false, percent: null }
  })

  function runSync() {
    if (sync.busy) return
    if (sync.kind === 'pull') repo.pull()
    else if (sync.kind === 'push' || sync.kind === 'publish') repo.push()
    else if (sync.kind === 'fetch') repo.fetch(null)
    else branchOpen = true
  }

  async function openRepo() {
    repoOpen = false
    const path = await pickFolder('Open a repository')
    if (!path) return
    const e = await app.openPath(path)
    if (e) app.fail(e)
  }
</script>

<div class="toolbar">
  <button class="seg repo-seg" title={repo.path} bind:this={repoBtn} onclick={() => (repoOpen = !repoOpen)} aria-haspopup="true" aria-expanded={repoOpen}>
    <FolderGit2 size={18} />
    <span class="txt"><small>Current repository</small><b>{repo.name}</b></span>
    <ChevronDown size={15} class="caret" />
  </button>

  <span class="rule"></span>

  <button
    class="seg branch-seg"
    class:warn={repo.detached}
    bind:this={branchBtn}
    onclick={() => (branchOpen = !branchOpen)}
    aria-haspopup="true"
    aria-expanded={branchOpen}
    title={repo.detached ? 'HEAD is detached: new commits won’t be on any branch' : 'Switch branch, see pull requests and tags'}
  >
    {#if repo.detached}<TriangleAlert size={18} />{:else}<GitBranch size={18} />{/if}
    <span class="txt">
      <small>{repo.detached ? 'Detached HEAD' : repo.inOperation ? 'Rebasing branch' : 'Current branch'}</small>
      <b>{#if repo.detached}At {head?.commit?.slice(0, 7)}{:else}{repo.branchName ?? head?.branch ?? 'main'}{/if}</b>
    </span>
    <ChevronDown size={15} class="caret" />
  </button>

  <span class="rule"></span>

  <button class="seg sync-seg" class:busy={sync.busy} disabled={sync.busy} onclick={runSync} title={sync.kind === 'pull' && sync.ahead ? 'Your branch and the remote have both moved. Pulling shows a preview first.' : undefined}>
    {#if sync.kind === 'pull'}<ArrowDownToLine size={18} />{:else if sync.kind === 'push'}<ArrowUpFromLine size={18} />{:else if sync.kind === 'publish'}<CloudUpload size={18} />{:else if sync.kind === 'none'}<CloudOff size={18} />{:else}<RefreshCw size={18} class={sync.busy ? 'spin' : ''} />{/if}
    <span class="txt"><b>{sync.title}</b><small>{sync.caption}</small></span>
    {#if !sync.busy && (sync.behind || sync.ahead)}
      <span class="counts num">
        {#if sync.behind}<em><ArrowDownToLine size={11} />{sync.behind}</em>{/if}
        {#if sync.ahead}<em><ArrowUpFromLine size={11} />{sync.ahead}</em>{/if}
      </span>
    {/if}
    {#if sync.busy}<i class="progress" class:indet={sync.percent === null} style="--p:{sync.percent ?? 30}%"></i>{/if}
  </button>

  <span class="rule"></span>

  <div class="seg-split" bind:this={undoBox}>
    <button
      class="seg undo-seg"
      disabled={!repo.undoable}
      onclick={() => repo.undo()}
      title={repo.undoable ? `Undo ${repo.undoable.label} (${modKey} Z)` : 'Nothing to undo'}
    >
      <Undo2 size={18} />
      <span class="txt"><small>Undo · {modKey} Z</small><b>{repo.undoable ? repo.undoable.label : 'Nothing to undo'}</b></span>
    </button>
    <button class="caret-btn" aria-label="Undo history" title="Undo history" disabled={!repo.oplog.length} onclick={() => (undoOpen = !undoOpen)}>
      <ChevronDown size={15} />
    </button>
  </div>

  <button class="search" onclick={() => (app.paletteOpen = true)} title="Search branches, commits and commands">
    <Search size={14} />
    <span>Search</span>
    <span class="kbd">{modKey} K</span>
  </button>
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

<Popover bind:open={branchOpen} anchor={branchBtn} width={400}>
  <BranchMenu {repo} onclose={() => (branchOpen = false)} />
</Popover>

<Popover bind:open={undoOpen} anchor={undoBox} width={360}>
  <div class="pop-head">Undo history</div>
  <div class="list">
    {#if repo.redoable}
      <button class="item op" onclick={() => { undoOpen = false; repo.redo() }} title="Redo ({modKey} Shift Z)">
        <Redo2 size={14} />
        <span class="item-main"><span>Redo {repo.redoable.label}</span></span>
      </button>
    {/if}
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
  <div class="pop-foot hint">Choosing an entry undoes it and every newer step that can be undone. Fetches stay as they are.</div>
</Popover>

<style>
  .toolbar {
    height: 56px;
    flex: none;
    container-type: inline-size;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 10px 0 6px;
    background: var(--bg-1);
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }

  .seg {
    position: relative;
    height: 46px;
    min-width: 0;
    padding: 0 10px 0 12px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 11px;
    text-align: left;
    overflow: hidden;
    transition: background-color 120ms var(--ease-out);
  }

  .seg:hover:not(:disabled) {
    background: var(--bg-2);
  }

  .seg:active:not(:disabled) {
    background: var(--bg-3);
  }

  .seg[aria-expanded='true'] {
    background: var(--bg-2);
  }

  .seg:disabled {
    cursor: default;
  }

  .seg > :global(svg:first-child) {
    flex: none;
    color: var(--text-2);
  }

  .repo-seg {
    flex: 0 1 230px;
  }

  .branch-seg {
    flex: 0 1 260px;
  }

  .sync-seg {
    flex: 0 1 250px;
  }

  .seg-split {
    flex: 0 1 280px;
    min-width: 0;
    display: flex;
    align-items: center;
  }

  .undo-seg {
    flex: 1;
    border-radius: var(--radius-md) 0 0 var(--radius-md);
  }

  .undo-seg:disabled .txt,
  .undo-seg:disabled > :global(svg) {
    opacity: 0.5;
  }

  .caret-btn {
    width: 24px;
    height: 46px;
    flex: none;
    padding: 0;
    border: 0;
    border-radius: 0 var(--radius-md) var(--radius-md) 0;
    background: transparent;
    color: var(--text-2);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .caret-btn:hover:not(:disabled) {
    background: var(--bg-2);
    color: var(--text);
  }

  .caret-btn:disabled {
    opacity: 0.4;
  }

  .txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .txt small,
  .txt b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .txt small {
    font-size: 11.5px;
    color: var(--text-2);
  }

  .txt b {
    font-weight: 600;
  }

  .seg :global(.caret) {
    flex: none;
    color: var(--text-2);
  }

  .rule {
    width: 1px;
    height: 28px;
    flex: none;
    background: var(--line);
  }

  .branch-seg.warn {
    background: var(--warning-soft);
  }

  .branch-seg.warn > :global(svg:first-child) {
    color: var(--warning);
  }

  .counts {
    flex: none;
    display: flex;
    gap: 4px;
  }

  .counts em {
    height: 18px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-color);
    font-style: normal;
    font-size: 11px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .progress {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 4px;
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
    border-radius: 999px;
    background: var(--accent);
    transition: width 200ms var(--ease-out);
  }

  .progress.indet::after {
    width: 30%;
    animation: indet 1s var(--ease-out) infinite;
  }

  .seg :global(.spin) {
    animation: spin 1s linear infinite;
  }

  .search {
    margin-left: auto;
    height: 32px;
    flex: none;
    padding: 0 8px 0 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    background: var(--bg-0);
    color: var(--text-2);
    display: flex;
    align-items: center;
    gap: 8px;
    transition: border-color 120ms var(--ease-out);
  }

  .search:hover {
    border-color: var(--line-strong);
    color: var(--text);
  }

  .pop-head {
    padding: 12px 14px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }

  .list {
    max-height: 420px;
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

  .item:hover:not(:disabled) {
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

  .op.undone .item-main > span {
    text-decoration: line-through;
    color: var(--text-2);
  }

  .op:disabled:not(.undone) .item-main > span {
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

  @container (max-width: 1100px) {
    .search span:nth-child(2) {
      display: none;
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes indet {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>
