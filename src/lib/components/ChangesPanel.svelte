<script lang="ts">
  import { ChevronDown, Archive, CircleCheck, GitBranchPlus, ArrowDownToLine, ArrowUpFromLine, Maximize2, WrapText } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import type { FileChange } from '../api/types'
  import { fileMenu, askNewBranch } from '../state/menus'
  import { modKey } from '../api/system'
  import { plural, relativeTime } from '../util/format'
  import FileRow from './FileRow.svelte'
  import DiffView from './DiffView.svelte'

  let { repo }: { repo: RepoStore } = $props()

  let openUnstaged = $state(true)
  let openStaged = $state(true)
  let summaryEl = $state<HTMLInputElement>()
  let winH = $state(window.innerHeight)
  let bodyOpen = $state(false)
  /** Short windows fold the description away so the diff keeps room; it opens on demand. */
  const showBody = $derived(winH >= 760 || bodyOpen || repo.body.length > 0)
  let branchBtn = $state<HTMLButtonElement>()

  const branch = $derived(repo.branchName ?? (repo.head?.detached ? 'detached HEAD' : 'main'))
  const busyOp = $derived(repo.status?.state && repo.status.state !== 'clean')
  const stagedCount = $derived(repo.staged.length)
  const canCommit = $derived((stagedCount > 0 || repo.amend) && repo.summary.trim().length > 0 && !busyOp)
  const sel = $derived(repo.fileSel)
  const selConflict = $derived(!!sel && repo.conflicted.some((f) => f.path === sel.path))
  const last = $derived(repo.head?.commit ? repo.byId.get(repo.head.commit) : null)
  const hb = $derived(repo.headBranch)
  const remaining = $derived(72 - repo.summary.length)

  const commitLabel = $derived.by(() => {
    if (busyOp) return 'Finish the operation above first'
    if (repo.amend) return 'Amend last commit'
    if (!stagedCount) return 'Stage files to commit'
    return `Commit ${plural(stagedCount, 'file')} to ${branch}`
  })

  function isSel(f: FileChange, staged: boolean) {
    return sel?.path === f.path && sel.staged === staged
  }

  function openFull(f: FileChange, staged: boolean) {
    repo.selectFile(f.path, staged)
    repo.fullDiff = true
  }

  function onCommitKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      if (canCommit) repo.commit()
    }
  }
</script>

<svelte:window bind:innerHeight={winH} />

<section class="panel" aria-label="Working changes">
  <header class="head">
    <h2>
      {#if repo.dirty}{plural(repo.files.length, 'file change')} on <span class="mono-b">{branch}</span>{:else}Working changes{/if}
    </h2>
    <button class="icon-btn" title="Stash all changes" aria-label="Stash all changes" disabled={!repo.dirty || !!busyOp} onclick={() => repo.stashPush()}><Archive size={15} /></button>
  </header>

  {#if !repo.dirty}
    <div class="clean">
      <span class="clean-icon"><CircleCheck size={22} /></span>
      <p class="clean-title">No uncommitted changes</p>
      {#if last}
        <p class="muted">Last commit: “{last.summary}”, {relativeTime(last.authorTime)}</p>
      {:else}
        <p class="muted">Add or edit files in this folder and they’ll appear here.</p>
      {/if}
      <div class="clean-actions">
        {#if hb && hb.behind > 0}<button class="btn ghost" onclick={() => repo.pull()}><ArrowDownToLine size={14} />Pull {hb.behind}</button>{/if}
        {#if hb && hb.ahead > 0}<button class="btn ghost" onclick={() => repo.push()}><ArrowUpFromLine size={14} />Push {hb.ahead}</button>{/if}
        {#if repo.head?.commit}
          <button class="btn ghost" bind:this={branchBtn} onclick={() => branchBtn && askNewBranch(repo, branchBtn, repo.head!.commit!, branch)}>
            <GitBranchPlus size={14} />New branch
          </button>
        {/if}
      </div>
    </div>
  {:else}
    <div class="lists">
      {#if repo.conflicted.length}
        <div class="sec">
          <span class="section-label conf">Conflicted ({repo.conflicted.length})</span>
        </div>
        {#each repo.conflicted as f (f.path)}
          <div class="conflict-row">
            <FileRow
              path={f.path}
              kind="conflicted"
              conflict
              selected={isSel(f, false)}
              onselect={() => repo.selectFile(f.path, false)}
              onopen={() => openFull(f, false)}
              oncontextmenu={(e) => app.openMenu(e, fileMenu(repo, f, false))}
            />
            <div class="resolve">
              <button class="btn sm ghost" title="Keep your version of this file" onclick={() => repo.resolveMine(f.path)}>Use mine</button>
              <button class="btn sm ghost" title="Keep {repo.conflictSides.other}’s version of this file" onclick={() => repo.resolveOther(f.path)}>Use {repo.conflictSides.other}</button>
              <button class="btn sm ghost" onclick={() => repo.resolve(f.path, 'mark')}>Mark resolved</button>
            </div>
          </div>
        {/each}
      {/if}

      <div class="sec">
        <button class="sec-toggle" onclick={() => (openUnstaged = !openUnstaged)} aria-expanded={openUnstaged}>
          <ChevronDown size={14} class={openUnstaged ? '' : 'closed'} />
          <span class="section-label">Unstaged files ({repo.unstaged.length})</span>
        </button>
        {#if repo.unstaged.length}<button class="btn sm quiet" onclick={() => repo.stageAll()}>Stage all</button>{/if}
      </div>
      {#if openUnstaged}
        {#each repo.unstaged as f (f.path)}
          <FileRow
            path={f.path}
            origPath={f.origPath}
            kind={f.unstaged!}
            selected={isSel(f, false)}
            onselect={() => repo.selectFile(f.path, false)}
            onopen={() => openFull(f, false)}
            ontoggle={() => repo.stage([f.path])}
            ondiscard={() => repo.startPreview({ kind: 'discard', paths: [f.path] })}
            oncontextmenu={(e) => app.openMenu(e, fileMenu(repo, f, false))}
          />
        {:else}
          <p class="none">Everything is staged.</p>
        {/each}
      {/if}

      <div class="sec">
        <button class="sec-toggle" onclick={() => (openStaged = !openStaged)} aria-expanded={openStaged}>
          <ChevronDown size={14} class={openStaged ? '' : 'closed'} />
          <span class="section-label">Staged files ({repo.staged.length})</span>
        </button>
        {#if repo.staged.length}<button class="btn sm quiet" onclick={() => repo.unstageAll()}>Unstage all</button>{/if}
      </div>
      {#if openStaged}
        {#each repo.staged as f (f.path)}
          <FileRow
            path={f.path}
            origPath={f.origPath}
            kind={f.staged!}
            staged
            selected={isSel(f, true)}
            onselect={() => repo.selectFile(f.path, true)}
            onopen={() => openFull(f, true)}
            ontoggle={() => repo.unstage([f.path])}
            oncontextmenu={(e) => app.openMenu(e, fileMenu(repo, f, true))}
          />
        {:else}
          <p class="none">Stage files or lines to include them in the next commit.</p>
        {/each}
      {/if}
    </div>

    <div class="diff-area">
      {#if repo.diff && sel && !repo.fullDiff}
        <div class="diff-head">
          <span class="mono path" title={repo.diff.path}>{repo.diff.path}</span>
          <span class="mono num stat"><span class="a">+{repo.diff.additions}</span> <span class="d">−{repo.diff.deletions}</span></span>
          <button
            class="icon-btn"
            class:on={app.panes.wrap}
            title={app.panes.wrap ? 'Don’t wrap long lines' : 'Wrap long lines'}
            aria-label="Wrap long lines"
            aria-pressed={app.panes.wrap}
            onclick={() => { app.panes.wrap = !app.panes.wrap; app.persistPanes() }}
          ><WrapText size={14} /></button>
          <button class="icon-btn" title="Open full diff" aria-label="Open full diff" onclick={() => (repo.fullDiff = true)}><Maximize2 size={14} /></button>
        </div>
        <DiffView {repo} diff={repo.diff} editable={!selConflict} staged={sel.staged} compact wrap={app.panes.wrap} sides={repo.conflictSides} />
      {:else if repo.fullDiff}
        <p class="placeholder">Showing the full diff in the main area.</p>
      {:else if repo.diffError}
        <p class="placeholder">{repo.diffError.message}</p>
      {:else}
        <p class="placeholder">Select a file to see its changes.</p>
      {/if}
    </div>
  {/if}

  {#if !busyOp}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="commit" onkeydown={onCommitKey} role="group" aria-label="Commit">
      <div class="summary-wrap">
        <input
          class="field summary"
          bind:this={summaryEl}
          bind:value={repo.summary}
          placeholder={repo.info.isEmpty ? 'First commit summary' : 'Commit summary'}
          aria-label="Commit summary"
          spellcheck="true"
        />
        {#if repo.summary.length > 50}<span class="count num" class:over={remaining < 0}>{remaining}</span>{/if}
      </div>
      {#if showBody}
        <!-- svelte-ignore a11y_autofocus -->
        <textarea class="field" rows="3" bind:value={repo.body} placeholder="Description (optional)" aria-label="Commit description" autofocus={bodyOpen}></textarea>
      {/if}
      <div class="commit-row">
        <label class="amend" title={repo.head?.commit ? 'Replace the last commit with these changes and message' : 'There’s no commit to amend yet'}>
          <input type="checkbox" checked={repo.amend} disabled={!repo.head?.commit} onchange={(e) => repo.setAmend(e.currentTarget.checked)} />
          <span>Amend last commit</span>
        </label>
        {#if !showBody}<button class="btn sm quiet" onclick={() => (bodyOpen = true)}>Add description</button>{/if}
        <span class="kbd">{modKey} Enter</span>
      </div>
      <button class="btn success commit-btn" disabled={!canCommit} onclick={() => repo.commit()}>{commitLabel}</button>
    </div>
  {:else}
    <div class="commit busy">
      <p class="muted">Committing resumes once the operation above is finished or aborted.</p>
    </div>
  {/if}
</section>

<style>
  .panel {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-1);
  }

  .head {
    height: 48px;
    flex: none;
    padding: 0 10px 0 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    border-bottom: 1px solid var(--line);
  }

  h2 {
    font-size: 13.5px;
    font-weight: 600;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .mono-b {
    font-family: var(--font-mono);
    font-size: 12.5px;
    font-weight: 500;
  }

  .lists {
    flex: 0 1 auto;
    min-height: 98px;
    max-height: 42%;
    overflow-y: auto;
    scroll-snap-type: y proximity;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--line);
  }

  .sec {
    height: 34px;
    padding: 0 10px 0 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    position: sticky;
    top: 0;
    background: var(--bg-1);
    z-index: 1;
  }

  .sec-toggle {
    border: 0;
    background: none;
    padding: 4px;
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-2);
  }

  .sec-toggle :global(svg) {
    transition: transform 150ms var(--ease-out);
  }

  .sec-toggle :global(svg.closed) {
    transform: rotate(-90deg);
  }

  .section-label.conf {
    padding-left: 6px;
    color: var(--warning);
  }

  .none {
    padding: 4px 16px 8px 34px;
    color: var(--text-2);
    font-size: 12.5px;
  }

  .conflict-row {
    padding-bottom: 6px;
  }

  .resolve {
    display: flex;
    gap: 4px;
    padding: 2px 12px 0 39px;
  }

  .diff-area {
    flex: 1;
    min-height: 160px;
    display: flex;
    flex-direction: column;
  }

  .diff-head {
    height: 34px;
    flex: none;
    padding: 0 6px 0 14px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid var(--line);
  }

  .diff-head .path {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }

  .stat {
    font-size: 11.5px;
  }

  .icon-btn.on {
    color: var(--accent);
  }

  .stat .a {
    color: var(--add-text);
  }

  .stat .d {
    color: var(--del-text);
  }

  .placeholder {
    margin: auto;
    color: var(--text-2);
    padding: 16px;
    text-align: center;
  }

  .clean {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 24px;
    text-align: center;
  }

  .clean-icon {
    color: var(--success);
    margin-bottom: 4px;
  }

  .clean-title {
    font-size: 14px;
    font-weight: 600;
  }

  .muted {
    color: var(--text-2);
  }

  .clean-actions {
    margin-top: 12px;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .commit {
    flex: none;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px solid var(--line);
    background: var(--bg-1);
  }

  .commit.busy {
    padding: 16px;
  }

  .summary-wrap {
    position: relative;
  }

  .summary {
    height: 36px;
    font-weight: 500;
    padding-right: 40px;
  }

  .count {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11.5px;
    color: var(--text-2);
  }

  .count.over {
    color: var(--warning);
  }

  .commit-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .commit-row .amend {
    margin-right: auto;
  }

  .amend {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
  }

  .amend input {
    margin: 0;
    accent-color: var(--accent);
  }

  /* Short windows keep room for the diff. */
  @media (max-height: 760px) {
    .lists {
      max-height: 27%;
    }

    .diff-head {
      height: 30px;
    }

    .commit {
      padding: 10px 12px;
    }

    .commit.busy {
      padding: 8px 12px;
      font-size: 12px;
    }
  }

  .commit-btn {
    height: 38px;
    border-radius: var(--radius-md);
  }

  .commit-btn:disabled {
    background: var(--bg-3);
    color: var(--text-2);
    opacity: 1;
  }
</style>
