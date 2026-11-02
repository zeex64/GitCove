<script lang="ts">
  import type { RepoStore, RepoTab } from '../state/repo.svelte'
  import { modKey } from '../api/system'
  import ChangesList from './ChangesList.svelte'
  import CommitBox from './CommitBox.svelte'
  import HistoryList from './HistoryList.svelte'

  let { repo }: { repo: RepoStore } = $props()

  const TABS: { id: RepoTab; label: string; key: string }[] = [
    { id: 'changes', label: 'Changes', key: '1' },
    { id: 'history', label: 'History', key: '2' },
  ]

  function onTabKey(e: KeyboardEvent) {
    if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
      e.preventDefault()
      repo.setTab(repo.tab === 'changes' ? 'history' : 'changes')
    }
  }
</script>

<aside class="left" aria-label="Changes and history">
  <div class="tabs" role="tablist" aria-label="View">
    {#each TABS as t (t.id)}
      <button
        role="tab"
        class="tab"
        class:on={repo.tab === t.id}
        aria-selected={repo.tab === t.id}
        tabindex={repo.tab === t.id ? 0 : -1}
        title="{t.label} ({modKey} {t.key})"
        onclick={() => repo.setTab(t.id)}
        onkeydown={onTabKey}
      >
        {t.label}
        {#if t.id === 'changes' && repo.files.length}<em class="num">{repo.files.length}</em>{/if}
      </button>
    {/each}
  </div>

  {#if repo.tab === 'changes'}
    <ChangesList {repo} />
    <CommitBox {repo} />
  {:else}
    <HistoryList {repo} />
  {/if}
</aside>

<style>
  .left {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-1);
  }

  .tabs {
    height: 42px;
    flex: none;
    display: grid;
    grid-template-columns: 1fr 1fr;
    border-bottom: 1px solid var(--line);
  }

  .tab {
    position: relative;
    border: 0;
    background: transparent;
    color: var(--text-2);
    font-weight: 500;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    transition:
      color 120ms var(--ease-out),
      background-color 120ms var(--ease-out);
  }

  .tab:hover {
    color: var(--text);
    background: rgba(255, 255, 255, 0.02);
  }

  .tab.on {
    color: var(--text);
    font-weight: 600;
  }

  .tab.on::after {
    content: '';
    position: absolute;
    left: 14px;
    right: 14px;
    bottom: -1px;
    height: 2px;
    border-radius: 999px;
    background: var(--accent);
  }

  .tab:focus-visible {
    outline-offset: -3px;
  }

  em {
    min-width: 20px;
    height: 18px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--bg-3);
    color: var(--text);
    font-style: normal;
    font-size: 11px;
    font-weight: 600;
    line-height: 18px;
    text-align: center;
  }
</style>
