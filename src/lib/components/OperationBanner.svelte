<script lang="ts">
  import { GitMerge, TriangleAlert, CircleCheck } from '@lucide/svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import { plural } from '../util/format'

  let { repo }: { repo: RepoStore } = $props()

  const s = $derived(repo.status)
  const op = $derived(s?.operation ?? null)
  const conflicts = $derived(repo.conflicted.length)

  const title = $derived.by(() => {
    switch (s?.state) {
      case 'rebasing':
        return `Rebasing ${op?.branch ?? 'your branch'}${op?.onto ? ` onto ${op.onto}` : ''}`
      case 'merging':
        return `Merging ${op?.onto ?? 'a branch'} into ${op?.branch ?? repo.head?.branch ?? 'HEAD'}`
      case 'cherryPicking':
        return `Cherry-picking ${op?.currentSummary ? `“${op.currentSummary}”` : 'a commit'}`
      case 'reverting':
        return `Reverting ${op?.currentSummary ? `“${op.currentSummary}”` : 'a commit'}`
      case 'bisecting':
        return 'Bisecting'
      default:
        return ''
    }
  })

  const canSkip = $derived(s?.state === 'rebasing' || s?.state === 'cherryPicking')
</script>

<div class="banner" role="region" aria-label="Operation in progress">
  <span class="icon"><GitMerge size={16} /></span>
  <div class="text">
    <p class="title">{title}</p>
    <p class="sub">
      {#if op?.step && op.total}<span class="num">Step {op.step} of {op.total}</span> · {/if}
      {#if s?.state === 'rebasing' && op?.currentSummary}stopped at “{op.currentSummary}”. {/if}
      {#if conflicts}
        <span class="warn"><TriangleAlert size={13} />{plural(conflicts, 'conflicted file')}: resolve them in the Changes list, then continue.</span>
      {:else if s?.state !== 'bisecting'}
        <span class="ok"><CircleCheck size={13} />No conflicts left. Continue when you’re ready.</span>
      {/if}
    </p>
  </div>
  {#if s?.state !== 'bisecting'}
    <button class="btn ghost" onclick={() => repo.abortOp()}>Abort</button>
    {#if canSkip}<button class="btn quiet" onclick={() => repo.skipOp()}>Skip commit</button>{/if}
    <button class="btn primary" disabled={conflicts > 0} onclick={() => repo.continueOp()}>Continue</button>
  {/if}
</div>

<style>
  .banner {
    flex: none;
    margin: 8px 8px 0;
    padding: 10px 10px 10px 12px;
    display: flex;
    align-items: center;
    gap: 12px;
    border: 1px solid rgba(242, 193, 78, 0.4);
    border-radius: var(--radius-lg);
    background: linear-gradient(0deg, var(--warning-soft), var(--warning-soft)), var(--bg-1);
  }

  .icon {
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 50%;
    background: var(--warning-soft);
    color: var(--warning);
    display: flex;
    align-items: center;
    justify-content: center;
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
  }

  .sub {
    color: var(--text-2);
    font-size: 12.5px;
  }

  .sub span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .warn {
    color: var(--warning);
  }

  .ok {
    color: var(--success);
  }
</style>
