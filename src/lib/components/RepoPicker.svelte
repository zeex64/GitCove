<script lang="ts">
  import { Search, Lock, RefreshCw, TriangleAlert } from '@lucide/svelte'
  import type { GithubRepo } from '../api/types'
  import { github } from '../state/github.svelte'
  import { relativeTime } from '../util/format'

  let { selected = $bindable(null) }: { selected: GithubRepo | null } = $props()

  let query = $state('')
  let list = $state<HTMLDivElement>()

  const SHOW = 200

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase()
    const all = github.repos ?? []
    return q ? all.filter((r) => r.fullName.toLowerCase().includes(q) || (r.description ?? '').toLowerCase().includes(q)) : all
  })
  const shown = $derived(filtered.slice(0, SHOW))

  $effect(() => {
    github.loadRepos()
  })

  function move(dir: 1 | -1) {
    if (!shown.length) return
    const i = selected ? shown.findIndex((r) => r.fullName === selected!.fullName) : -1
    selected = shown[Math.max(0, Math.min(shown.length - 1, i + dir))]
    queueMicrotask(() => list?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' }))
  }
</script>

<div class="picker">
  <div class="top">
    <label class="search">
      <Search size={14} />
      <!-- svelte-ignore a11y_autofocus -->
      <input
        bind:value={query}
        placeholder="Filter your repositories"
        spellcheck="false"
        autofocus
        aria-label="Filter repositories"
        onkeydown={(e) => {
          if (e.key === 'ArrowDown') {
            e.preventDefault()
            move(1)
          } else if (e.key === 'ArrowUp') {
            e.preventDefault()
            move(-1)
          }
        }}
      />
    </label>
    <button class="icon-btn" title="Refresh list" aria-label="Refresh list" disabled={github.reposLoading} onclick={() => github.loadRepos(true)}>
      <RefreshCw size={14} class={github.reposLoading ? 'spin' : ''} />
    </button>
  </div>

  <div class="list" bind:this={list} role="listbox" aria-label="Your GitHub repositories">
    {#if github.reposError}
      <p class="msg error"><TriangleAlert size={14} />{github.reposError.message}</p>
    {:else if !github.repos}
      {#each Array(6) as _, i (i)}
        <div class="sk"><i style="width:{35 + ((i * 23) % 40)}%"></i><i style="width:{55 + ((i * 17) % 30)}%"></i></div>
      {/each}
    {:else}
      {#each shown as r (r.fullName)}
        <button
          class="repo"
          role="option"
          aria-selected={selected?.fullName === r.fullName}
          onclick={() => (selected = r)}
          ondblclick={() => (selected = r)}
          tabindex="-1"
        >
          <span class="line1">
            {#if r.private}<Lock size={12} />{/if}
            <span class="full"><span class="owner">{r.owner}/</span><span class="name">{r.name}</span></span>
            {#if r.fork}<em>fork</em>{/if}
            {#if r.archived}<em>archived</em>{/if}
            <span class="when">{relativeTime(r.pushedAt)}</span>
          </span>
          {#if r.description}<span class="desc">{r.description}</span>{/if}
        </button>
      {:else}
        <p class="msg">{query ? `No repositories match “${query}”.` : 'This account has no repositories yet.'}</p>
      {/each}
      {#if filtered.length > SHOW}<p class="msg">Showing {SHOW} of {filtered.length}. Type to narrow the list.</p>{/if}
    {/if}
  </div>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-md);
    background: var(--bg-0);
    overflow: hidden;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 6px 6px 10px;
    border-bottom: 1px solid var(--line);
  }

  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
  }

  .search input {
    flex: 1;
    min-width: 0;
    height: 28px;
    border: 0;
    background: transparent;
    color: var(--text);
  }

  .search input::placeholder {
    color: var(--text-3);
  }

  /* The frame shows focus; the bare input inside it doesn't draw a second ring. */
  .search input:focus-visible {
    outline: none;
  }

  .picker:focus-within {
    border-color: var(--accent);
  }

  .list {
    height: 272px;
    overflow-y: auto;
    padding: 4px;
  }

  .repo {
    width: 100%;
    padding: 7px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
  }

  .repo:hover {
    background: var(--bg-2);
  }

  .repo[aria-selected='true'] {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent-line);
  }

  .line1 {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .line1 :global(svg) {
    color: var(--text-2);
    flex: none;
  }

  .full {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .owner {
    color: var(--text-2);
  }

  .name {
    font-weight: 600;
  }

  em {
    font-style: normal;
    font-size: 11px;
    color: var(--text-2);
    border: 1px solid var(--line-strong);
    border-radius: 999px;
    padding: 0 6px;
    flex: none;
  }

  .when {
    margin-left: auto;
    padding-left: 12px;
    font-size: 12px;
    color: var(--text-2);
    flex: none;
    font-variant-numeric: tabular-nums;
  }

  .desc {
    color: var(--text-2);
    font-size: 12.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .msg {
    padding: 14px 10px;
    color: var(--text-2);
    display: flex;
    gap: 8px;
    align-items: flex-start;
    line-height: 1.45;
  }

  .msg.error :global(svg) {
    color: var(--warning);
    flex: none;
    margin-top: 2px;
  }

  .sk {
    padding: 9px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .sk i {
    height: 9px;
    border-radius: var(--radius-sm);
    background: var(--bg-2);
  }

  .picker :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
