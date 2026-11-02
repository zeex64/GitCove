<script lang="ts">
  import { ArrowDownToLine, ArrowUpFromLine, CloudUpload, GitBranchPlus, SquareTerminal, CircleCheck } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import { askNewBranch } from '../state/menus'
  import { plural, relativeTime, splitPath } from '../util/format'
  import PreviewBanner from './PreviewBanner.svelte'
  import OperationBanner from './OperationBanner.svelte'
  import DiffView from './DiffView.svelte'
  import DiffOptions from './DiffOptions.svelte'
  import CommitView from './CommitView.svelte'
  import PreviewDetails from './PreviewDetails.svelte'

  let { repo }: { repo: RepoStore } = $props()

  let branchBtn = $state<HTMLButtonElement>()

  const showPreview = $derived(!!(repo.preview || repo.previewLoading || repo.previewError || repo.drag))
  const d = $derived(repo.diff)
  const picking = $derived(!!d && !d.binary && !d.tooLarge && d.hunks.some((h) => h.lines.some((l) => l.staged !== undefined)))
  const inclusion = $derived.by(() => {
    if (!d || !picking) return null
    let total = 0
    let inn = 0
    for (const h of d.hunks) for (const l of h.lines) if (l.kind === 'add' || l.kind === 'del') { total++; if (l.staged) inn++ }
    if (inn === 0) return 'Not in the commit'
    if (inn === total) return 'All changes in the commit'
    return `${inn} of ${plural(total, 'changed line')} in the commit`
  })
  const parts = $derived(d ? splitPath(d.path) : null)
  const hb = $derived(repo.headBranch)
  const remote = $derived(hb?.upstream?.split('/')[0] ?? repo.primaryRemote?.name ?? null)
  const last = $derived(repo.head?.commit ? repo.byId.get(repo.head.commit) : null)
</script>

<main class="main-pane">
  {#if repo.status && repo.status.state !== 'clean'}
    <OperationBanner {repo} />
  {/if}
  {#if showPreview}
    <PreviewBanner {repo} />
  {/if}

  {#if repo.tab === 'history'}
    {#if repo.preview && repo.selection.kind === 'wip'}
      <PreviewDetails {repo} preview={repo.preview} />
    {:else if repo.selection.kind === 'commit'}
      {#key repo.selection.id}
        <CommitView {repo} id={repo.selection.id} />
      {/key}
    {:else}
      <p class="placeholder">{repo.previewLoading ? '' : 'Select a commit to see what it changed.'}</p>
    {/if}
  {:else if !repo.dirty}
    <div class="clean">
      <div class="clean-inner">
        <span class="clean-icon"><CircleCheck size={22} /></span>
        <h2>No local changes</h2>
        <p class="lede">
          {#if last}Everything is committed. The last commit was “{last.summary}”, {relativeTime(last.authorTime)}.{:else}Add or edit files in this folder and they’ll show up here, ready to commit.{/if}
        </p>
        <div class="next">
          {#if hb && hb.behind > 0}
            <div class="step">
              <div><p class="step-title">Pull {plural(hb.behind, 'commit')} from {remote}</p><p class="step-sub">{hb.upstream} has commits you don’t have yet.</p></div>
              <button class="btn primary" onclick={() => repo.pull()}><ArrowDownToLine size={14} />Pull {remote}</button>
            </div>
          {/if}
          {#if hb && hb.upstream && hb.ahead > 0}
            <div class="step">
              <div><p class="step-title">Push {plural(hb.ahead, 'commit')} to {remote}</p><p class="step-sub">They’re only on this computer so far.</p></div>
              <button class="btn primary" onclick={() => repo.push()}><ArrowUpFromLine size={14} />Push {remote}</button>
            </div>
          {:else if hb && !hb.upstream && remote}
            <div class="step">
              <div><p class="step-title">Publish {hb.name}</p><p class="step-sub">Put this branch on {remote} so others can see it.</p></div>
              <button class="btn primary" onclick={() => repo.push()}><CloudUpload size={14} />Publish branch</button>
            </div>
          {/if}
          {#if repo.head?.commit}
            <div class="step">
              <div><p class="step-title">Start something new</p><p class="step-sub">Make a branch from {repo.branchName ?? 'here'} for your next change.</p></div>
              <button class="btn ghost" bind:this={branchBtn} onclick={() => branchBtn && askNewBranch(repo, branchBtn, repo.head!.commit!, repo.branchName ?? 'HEAD')}>
                <GitBranchPlus size={14} />New branch
              </button>
            </div>
          {/if}
          <div class="step">
            <div><p class="step-title">Work in a terminal</p><p class="step-sub">Open your terminal at the root of {repo.name}.</p></div>
            <button class="btn ghost" onclick={() => repo.openTerminal()}><SquareTerminal size={14} />Open terminal</button>
          </div>
        </div>
      </div>
    </div>
  {:else if d && repo.fileSel === d.path}
    <div class="diff-head">
      <span class="mono path" title={d.oldPath ? `${d.oldPath} → ${d.path}` : d.path}>
        {#if parts?.dir}<span class="dir">{parts.dir}</span>{/if}<span>{parts?.name}</span>
      </span>
      {#if inclusion}<span class="incl" title="Click line numbers to choose which lines go in the commit">{inclusion}</span>{/if}
      <span class="mono num stat"><span class="a">+{d.additions}</span> <span class="dl">−{d.deletions}</span></span>
      <DiffOptions />
    </div>
    <DiffView
      diff={d}
      split={app.panes.split}
      wrap={app.panes.wrap}
      context={repo.diffContext}
      {picking}
      onpick={(keys, include) => repo.setLines(keys, include)}
      ondiscard={(keys) => repo.discardLines(keys)}
      onexpand={() => repo.expandDiff()}
      sides={repo.conflictSides}
    />
  {:else if repo.diffError}
    <p class="placeholder">{repo.diffError.message}</p>
  {:else if !repo.fileSel}
    <p class="placeholder">Select a file to see its changes.</p>
  {/if}
</main>

<style>
  .main-pane {
    position: relative;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-0);
  }

  .placeholder {
    margin: auto;
    padding: 24px;
    color: var(--text-2);
    text-align: center;
  }

  .diff-head {
    height: 40px;
    flex: none;
    padding: 0 8px 0 16px;
    display: flex;
    align-items: center;
    gap: 12px;
    border-bottom: 1px solid var(--line);
    background: var(--bg-1);
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-weight: 500;
  }

  .dir {
    color: var(--text-2);
    font-weight: 400;
  }

  .incl {
    flex: none;
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
  }

  .stat {
    flex: none;
    font-size: 11.5px;
  }

  .stat .a {
    color: var(--add-text);
  }

  .stat .dl {
    color: var(--del-text);
  }

  /* No local changes: what to do next, GitHub Desktop style. */
  .clean {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px 24px;
  }

  .clean-inner {
    width: min(560px, 100%);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .clean-icon {
    color: var(--success);
    margin-bottom: 4px;
  }

  h2 {
    font-size: 18px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  .lede {
    color: var(--text-2);
    line-height: 1.5;
    max-width: 60ch;
  }

  .next {
    margin-top: 16px;
    width: 100%;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--bg-1);
  }

  .step {
    padding: 14px 14px 14px 16px;
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .step + .step {
    border-top: 1px solid var(--line);
  }

  .step > div {
    flex: 1;
    min-width: 0;
  }

  .step-title {
    font-weight: 600;
  }

  .step-sub {
    margin-top: 2px;
    color: var(--text-2);
    font-size: 12.5px;
  }
</style>
