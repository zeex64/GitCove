<script lang="ts">
  import { Eye, TriangleAlert, CircleCheck, Info, X, MousePointer2 } from '@lucide/svelte'
  import type { Operation } from '../api/types'
  import type { RepoStore } from '../state/repo.svelte'
  import { plural } from '../util/format'

  let { repo }: { repo: RepoStore } = $props()

  const p = $derived(repo.preview)
  const op = $derived(repo.previewOp)
  const cur = $derived(repo.head?.branch ?? null)

  /** Rebase and merge of the current branch are two answers to the same question; offer both. */
  const alternatives = $derived.by((): { label: string; op: Operation; on: boolean }[] => {
    if (!op || !cur) return []
    if (op.kind === 'rebase' && op.branch === cur) {
      return [
        { label: 'Rebase', op, on: true },
        { label: 'Merge', op: { kind: 'merge', branch: op.onto }, on: false },
      ]
    }
    if (op.kind === 'merge') {
      return [
        { label: 'Rebase', op: { kind: 'rebase', branch: cur, onto: op.branch }, on: false },
        { label: 'Merge', op, on: true },
      ]
    }
    if (op.kind === 'pull') {
      return [
        { label: 'Rebase', op: { kind: 'pull', mode: 'rebase' }, on: op.mode === 'rebase' },
        { label: 'Merge', op: { kind: 'pull', mode: 'merge' }, on: op.mode === 'merge' },
      ]
    }
    return []
  })

  const danger = $derived(
    !!p &&
      (p.irreversible ||
        op?.kind === 'deleteBranch' ||
        op?.kind === 'discard' ||
        op?.kind === 'dropStash' ||
        (op?.kind === 'reset' && op.mode === 'hard')),
  )

  const applyLabel = $derived.by(() => {
    switch (op?.kind) {
      case 'rebase':
        return 'Apply rebase'
      case 'merge':
        return p?.created.length ? 'Merge' : 'Fast-forward'
      case 'reset':
        return 'Reset'
      case 'forcePush':
        return 'Force push'
      case 'deleteBranch':
        return 'Delete branch'
      case 'discard':
        return 'Discard changes'
      case 'cherryPick':
        return 'Cherry-pick'
      case 'revert':
        return 'Revert'
      case 'pull':
        return op.mode === 'merge' ? 'Pull and merge' : 'Pull and rebase'
      case 'dropStash':
        return 'Drop stash'
      default:
        return 'Apply'
    }
  })

  const changeLine = $derived.by(() => {
    if (!p || !op) return null
    const replays = p.created.filter((c) => c.replaces).length
    const isRebase = op.kind === 'rebase' || (op.kind === 'pull' && op.mode === 'rebase')
    const isMerge = op.kind === 'merge' || (op.kind === 'pull' && op.mode !== 'rebase')
    if (isRebase) return replays ? `${plural(replays, 'commit')} rewritten` : 'Already up to date'
    if (isMerge) return p.created.length ? '1 merge commit' : 'Fast-forward, no merge commit'
    switch (op.kind) {
      case 'reset':
        return p.replaced.length ? `${plural(p.replaced.length, 'commit')} leave the branch` : 'The branch doesn’t move'
      case 'deleteBranch':
        return p.replaced.length ? `${plural(p.replaced.length, 'commit')} only on this branch` : 'No commits are lost'
      case 'cherryPick':
      case 'revert':
        return '1 new commit'
      default:
        return null
    }
  })
</script>

<div class="banner" class:danger={danger && !!p} role="region" aria-label="Preview">
  {#if repo.drag && !p && !repo.previewLoading}
    <span class="icon"><MousePointer2 size={16} /></span>
    <div class="text">
      <p class="title">Drop on a commit to rebase <strong>{repo.drag.branch}</strong> onto it</p>
      <p class="sub">You’ll see the result here before anything changes. Esc cancels.</p>
    </div>
  {:else if repo.previewLoading && !p}
    <span class="icon"><Eye size={16} /></span>
    <div class="text">
      <p class="title">Working out what would change…</p>
      <i class="indeterminate"></i>
    </div>
    <button class="btn ghost" onclick={() => repo.cancelPreview()}>Cancel</button>
  {:else if repo.previewError}
    <span class="icon warn"><TriangleAlert size={16} /></span>
    <div class="text">
      <p class="title">This can’t be previewed</p>
      <p class="sub">{repo.previewError.message}</p>
    </div>
    <button class="btn ghost" onclick={() => repo.cancelPreview()}>Dismiss</button>
  {:else if p}
    <span class="icon"><Eye size={16} /></span>
    <div class="text">
      <p class="title">
        <span class="kicker-free">Preview:</span>
        {p.title}
        {#if repo.previewLoading}<span class="updating">updating…</span>{/if}
      </p>
      <p class="readout">
        {#if changeLine}<span>{changeLine}</span>{/if}
        {#if op?.kind === 'rebase' || op?.kind === 'merge' || op?.kind === 'cherryPick' || op?.kind === 'revert' || op?.kind === 'pull'}
          {#if !p.conflicts.predicted}
            <span class="note"><Info size={13} />Conflicts can’t be predicted with this Git version</span>
          {:else if p.conflicts.files.length}
            <span class="warn" title={p.conflicts.files.join('\n')}><TriangleAlert size={13} />Conflicts expected in {plural(p.conflicts.files.length, 'file')}</span>
          {:else}
            <span class="ok"><CircleCheck size={13} />No conflicts expected</span>
          {/if}
        {/if}
        {#if p.needsStash}<span>Uncommitted changes are set aside and restored</span>{/if}
        {#if p.discardedFiles.length}
          <span class="bad" title={p.discardedFiles.join('\n')}>Throws away changes in {plural(p.discardedFiles.length, 'file')}</span>
        {/if}
        {#if p.remoteCommitsLost.length}
          <span class="bad" title={p.remoteCommitsLost.map((c) => `${c.shortId} ${c.summary}`).join('\n')}>
            Removes {plural(p.remoteCommitsLost.length, 'commit')} from the remote
          </span>
        {/if}
        {#if p.irreversible}
          <span class="bad">Can’t be undone afterwards</span>
        {:else}
          <span class="muted">You can undo this</span>
        {/if}
        {#each p.notes.filter((n) => n !== 'You can undo this.') as n (n)}<span class="muted">{n}</span>{/each}
      </p>
    </div>
    {#if alternatives.length}
      <div class="seg" role="radiogroup" aria-label="How to combine">
        {#each alternatives as a (a.label)}
          <button role="radio" aria-checked={a.on} class:on={a.on} onclick={() => !a.on && repo.startPreview(a.op)}>{a.label}</button>
        {/each}
      </div>
    {/if}
    <button class="btn ghost" onclick={() => repo.cancelPreview()}>Cancel</button>
    <button class="btn {danger ? 'danger' : 'primary'}" onclick={() => repo.applyPreview()} disabled={repo.previewLoading}>{applyLabel}</button>
    <button class="icon-btn" aria-label="Close preview" onclick={() => repo.cancelPreview()}><X size={15} /></button>
  {/if}
</div>

<style>
  .banner {
    flex: none;
    min-height: 60px;
    margin: 8px 8px 0;
    padding: 10px 10px 10px 12px;
    display: flex;
    align-items: center;
    gap: 12px;
    border: 1px solid var(--accent-line);
    border-radius: var(--radius-lg);
    background: linear-gradient(0deg, var(--accent-soft), var(--accent-soft)), var(--bg-1);
    animation: drop 180ms var(--ease-out);
  }

  .banner.danger {
    border-color: rgba(255, 107, 107, 0.45);
    background: linear-gradient(0deg, var(--danger-soft), var(--danger-soft)), var(--bg-1);
  }

  .icon {
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .danger .icon {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .icon.warn {
    background: var(--warning-soft);
    color: var(--warning);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .title {
    font-weight: 600;
    font-size: 13.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title strong {
    font-family: var(--font-mono);
    font-size: 12.5px;
  }

  .kicker-free {
    color: var(--text-2);
    font-weight: 500;
  }

  .updating {
    color: var(--text-2);
    font-weight: 400;
    font-size: 12px;
    margin-left: 6px;
  }

  .sub {
    color: var(--text-2);
  }

  .readout {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    color: var(--text);
    font-size: 12.5px;
  }

  .readout span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .readout .ok {
    color: var(--success);
  }

  .readout .warn {
    color: var(--warning);
  }

  .readout .bad {
    color: var(--danger-hover);
  }

  .readout .muted,
  .readout .note {
    color: var(--text-2);
  }

  .seg {
    display: flex;
    padding: 2px;
    border-radius: var(--radius);
    background: var(--bg-0);
    border: 1px solid var(--line-strong);
  }

  .seg button {
    height: 24px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-weight: 500;
  }

  .seg button.on {
    background: var(--bg-3);
    color: var(--text);
  }

  .seg button:hover:not(.on) {
    color: var(--text);
  }

  .indeterminate {
    position: relative;
    height: 2px;
    width: 220px;
    margin-top: 4px;
    border-radius: 999px;
    background: var(--bg-3);
    overflow: hidden;
  }

  .indeterminate::after {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    width: 35%;
    background: var(--accent);
    border-radius: 999px;
    animation: slide 1s var(--ease-out) infinite;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(300%);
    }
  }

  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>
