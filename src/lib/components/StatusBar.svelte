<script lang="ts">
  import { GitBranch, TriangleAlert, CloudOff, RefreshCw, ArrowUp, ArrowDown, Undo2 } from '@lucide/svelte'
  import { modKey } from '../api/system'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import { relativeTime } from '../util/format'

  let { repo }: { repo: RepoStore } = $props()

  let now = $state(Date.now() / 1000)
  $effect(() => {
    const t = setInterval(() => (now = Date.now() / 1000), 30_000)
    return () => clearInterval(t)
  })

  const head = $derived(repo.head)
  const hb = $derived(repo.headBranch)
  const busy = $derived(Object.values(repo.busy)[0] ?? null)
</script>

<footer class="status">
  <span class="item" class:warn={repo.detached}>
    {#if repo.detached}
      <TriangleAlert size={13} />Detached at <span class="mono">{head?.commit?.slice(0, 7)}</span>
    {:else}
      <GitBranch size={13} /><span class="strong">{repo.branchName ?? '—'}</span>
    {/if}
  </span>

  {#if hb?.upstream}
    <span class="item num" title="Compared with {hb.upstream}">
      <ArrowUp size={12} />{hb.ahead}
      <ArrowDown size={12} />{hb.behind}
      <span class="muted">{hb.upstream}</span>
    </span>
  {:else if hb && repo.remotes.length}
    <span class="item muted">Not published yet</span>
  {/if}

  {#if repo.remotes.length}
    {#if repo.offline}
      <span class="item warn" title="The last fetch couldn’t reach the remote"><CloudOff size={13} />Offline{repo.lastFetch ? `, last fetched ${relativeTime(repo.lastFetch, now)}` : ''}</span>
    {:else if repo.lastFetch}
      <button class="item link" onclick={() => repo.fetch(null)} title="Fetch now">
        <RefreshCw size={12} class={repo.busy.fetch ? 'spin' : ''} />Fetched {relativeTime(repo.lastFetch, now)}
      </button>
    {:else if repo.busy.fetch}
      <span class="item muted"><RefreshCw size={12} class="spin" />Fetching…</span>
    {/if}
  {/if}

  <span class="spacer"></span>

  {#if busy}
    <span class="item busy">
      {busy.label}{busy.phase ? `: ${busy.phase}` : ''}
      <i class="bar" style="--p:{busy.percent ?? 20}%"></i>
    </span>
  {/if}
  {#if repo.preview}
    <span class="item accent">Preview: nothing changes until you apply</span>
  {:else if repo.undoable && !repo.inOperation}
    <button class="item link undo-hint" onclick={() => repo.undo()} title="Undo this ({modKey} Z)">
      <Undo2 size={12} />Undo {repo.undoable.label}
    </button>
  {/if}
  {#if app.gitVersion}<span class="item muted">Git {app.gitVersion}</span>{/if}
</footer>

<style>
  .status {
    height: 26px;
    flex: none;
    padding: 0 12px;
    display: flex;
    align-items: center;
    gap: 16px;
    background: var(--bg-1);
    border-top: 1px solid var(--line);
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
  }

  .item {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .strong {
    color: var(--text);
    font-weight: 500;
  }

  .warn {
    color: var(--warning);
  }

  .accent {
    color: var(--accent);
  }

  .muted {
    color: var(--text-2);
  }

  .link {
    border: 0;
    background: none;
    padding: 0;
    color: var(--text-2);
    font-size: 12px;
  }

  .link:hover {
    color: var(--text);
  }

  .spacer {
    flex: 1;
  }

  .undo-hint {
    max-width: 420px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bar {
    position: relative;
    width: 80px;
    height: 3px;
    border-radius: 999px;
    background: var(--bg-3);
    overflow: hidden;
  }

  .bar::after {
    content: '';
    position: absolute;
    inset: 0;
    width: var(--p);
    background: var(--accent);
    transition: width 200ms var(--ease-out);
  }

  .status :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
