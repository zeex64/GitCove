<script lang="ts">
  import { Archive, ChevronRight, Ellipsis } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import type { FileChange } from '../api/types'
  import { fileMenu, stashMenu } from '../state/menus'
  import { plural, relativeTime } from '../util/format'
  import FileRow from './FileRow.svelte'
  import Checkbox from './Checkbox.svelte'
  import Popover from './Popover.svelte'

  let { repo }: { repo: RepoStore } = $props()

  let stashBtn = $state<HTMLButtonElement>()
  let stashOpen = $state(false)

  const stashes = $derived(repo.refs?.stashes ?? [])
  const busyOp = $derived(!!repo.status && repo.status.state !== 'clean')

  function kindOf(f: FileChange) {
    return f.unstaged ?? f.staged ?? 'modified'
  }

  function onListKey(e: KeyboardEvent) {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
    e.preventDefault()
    const files = repo.changeFiles
    const i = files.findIndex((f) => f.path === repo.fileSel)
    const next = files[Math.max(0, Math.min(files.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))]
    if (next) {
      repo.selectFile(next.path)
      queueMicrotask(() => (document.querySelector(`[data-file="${CSS.escape(next.path)}"] .row`) as HTMLElement | null)?.focus())
    }
  }
</script>

<div class="changes">
  {#if repo.dirty}
    <div class="head">
      <Checkbox
        checked={repo.allCheck}
        disabled={repo.files.every((f) => f.conflict)}
        label={repo.allCheck === true ? 'Leave every file out of the commit' : 'Include every file in the commit'}
        title={repo.conflicted.length ? 'Conflicted files join the commit once they’re resolved' : undefined}
        onchange={() => repo.toggleAll()}
      />
      <span class="count">
        {plural(repo.files.length, 'changed file')}{#if repo.conflicted.length}<span class="conf">{' · '}{plural(repo.conflicted.length, 'conflict')} to resolve</span>{/if}
      </span>
      <button class="icon-btn" title="Stash all changes" aria-label="Stash all changes" disabled={busyOp} onclick={() => repo.stashPush()}><Archive size={15} /></button>
    </div>

    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="list" role="listbox" aria-label="Changed files" tabindex="-1" onkeydown={onListKey}>
      {#each repo.changeFiles as f (f.path)}
        <div class="item" data-file={f.path}>
          <FileRow
            path={f.path}
            origPath={f.origPath}
            kind={kindOf(f)}
            conflict={!!f.conflict}
            checked={repo.fileCheck(f)}
            oncheck={() => repo.toggleFile(f)}
            selected={repo.fileSel === f.path}
            onselect={() => repo.selectFile(f.path)}
            ondiscard={f.conflict ? undefined : () => repo.startPreview({ kind: 'discard', paths: [f.path] })}
            oncontextmenu={(e) => app.openMenu(e, fileMenu(repo, f))}
          />
          {#if f.conflict}
            <div class="resolve">
              <button class="btn sm ghost" title="Keep your version of this file" onclick={() => repo.resolveMine(f.path)}>Keep yours</button>
              <button class="btn sm ghost" title="Keep {repo.conflictSides.other}’s version of this file" onclick={() => repo.resolveOther(f.path)}>Keep {repo.conflictSides.other}</button>
              <button class="btn sm quiet" onclick={() => repo.resolve(f.path, 'mark')}>Mark resolved</button>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <div class="head quiet"><span class="count">No changed files</span></div>
    <div class="list"></div>
  {/if}

  {#if stashes.length}
    <button class="stash-row" bind:this={stashBtn} onclick={() => (stashOpen = !stashOpen)} aria-haspopup="true" aria-expanded={stashOpen}>
      <Archive size={15} />
      <span>Stashed changes</span>
      <em class="num">{stashes.length}</em>
      <ChevronRight size={15} />
    </button>
  {/if}
</div>

<Popover bind:open={stashOpen} anchor={stashBtn} width={340}>
  <div class="pop-head">Stashed changes</div>
  <div class="stash-list">
    {#each stashes as s (s.commit)}
      <div class="stash">
        <span class="stash-main">
          <span class="stash-msg">{s.message.replace(/^(WIP on|On) [^:]+: /, '')}</span>
          <small>{relativeTime(s.time)}</small>
        </span>
        <button class="btn sm ghost" title="Apply these changes and remove the stash" onclick={() => { stashOpen = false; repo.stashApply(s.index, true) }}>Restore</button>
        <button class="icon-btn" aria-label="More stash actions" onclick={(e) => app.openMenu(e, stashMenu(repo, s))}><Ellipsis size={15} /></button>
      </div>
    {/each}
  </div>
</Popover>

<style>
  .changes {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .head {
    height: 40px;
    flex: none;
    padding: 0 8px 0 16px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid var(--line);
  }

  .count {
    flex: 1;
    min-width: 0;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count .conf {
    color: var(--warning);
    font-weight: 400;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px 0;
  }

  .list:focus {
    outline: none;
  }

  .resolve {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 4px 12px 8px 42px;
  }

  .head.quiet .count {
    color: var(--text-2);
    font-weight: 400;
  }

  .stash-row {
    height: 40px;
    flex: none;
    padding: 0 12px 0 16px;
    border: 0;
    border-top: 1px solid var(--line);
    background: transparent;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    color: var(--text);
    transition: background-color 120ms var(--ease-out);
  }

  .stash-row:hover {
    background: var(--bg-2);
  }

  .stash-row :global(svg) {
    color: var(--text-2);
    flex: none;
  }

  .stash-row span {
    flex: 1;
    font-weight: 500;
  }

  .stash-row em {
    font-style: normal;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-2);
    background: var(--bg-3);
    border-radius: 999px;
    padding: 1px 7px;
  }

  .pop-head {
    padding: 12px 14px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }

  .stash-list {
    padding: 2px 6px 8px;
    max-height: 320px;
    overflow-y: auto;
  }

  .stash {
    min-height: 44px;
    padding: 6px 4px 6px 8px;
    display: flex;
    align-items: center;
    gap: 6px;
    border-radius: var(--radius-sm);
  }

  .stash:hover {
    background: var(--bg-3);
  }

  .stash-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .stash-msg {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stash small {
    color: var(--text-2);
    font-size: 11.5px;
  }
</style>
