<script lang="ts">
  import { Copy, Check, Ellipsis, GitBranchPlus, ChevronDown } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import { commitMenu, askNewBranch } from '../state/menus'
  import { copyText } from '../api/system'
  import { fullDate, initials, plural, relativeTime } from '../util/format'
  import FileRow from './FileRow.svelte'
  import DiffView from './DiffView.svelte'
  import DiffOptions from './DiffOptions.svelte'

  let { repo, id }: { repo: RepoStore; id: string } = $props()

  let copied = $state(false)
  let bodyOpen = $state(false)
  let moreBtn = $state<HTMLButtonElement>()
  let branchBtn = $state<HTMLButtonElement>()

  const d = $derived(repo.details.get(id) ?? null)
  const c = $derived(d?.commit ?? repo.byId.get(id) ?? null)
  const labels = $derived(repo.labels.get(id) ?? [])
  const totals = $derived(d ? d.files.reduce((t, f) => ({ a: t.a + f.additions, d: t.d + f.deletions }), { a: 0, d: 0 }) : null)
  const longBody = $derived(!!c?.body && (c.body.split('\n').length > 4 || c.body.length > 320))

  $effect(() => {
    void id
    bodyOpen = false
  })

  function copySha() {
    if (!c) return
    copyText(c.id)
    copied = true
    setTimeout(() => (copied = false), 1200)
  }

  function more() {
    if (!c || !moreBtn) return
    const r = moreBtn.getBoundingClientRect()
    app.menu = { x: r.right - 260, y: r.bottom + 4, items: commitMenu(repo, c, { x: r.left, y: r.bottom + 6 }) }
  }
</script>

{#if c}
  <section class="commit" aria-label="Commit {c.shortId}">
    <header class="head">
      <div class="title-row">
        <h2 class="selectable">{c.summary}</h2>
        <div class="actions">
          <button class="btn sm ghost" bind:this={branchBtn} onclick={() => branchBtn && askNewBranch(repo, branchBtn, c.id, c.shortId)}>
            <GitBranchPlus size={14} />Branch here
          </button>
          <button class="icon-btn" bind:this={moreBtn} aria-label="More actions for this commit" onclick={more}><Ellipsis size={16} /></button>
        </div>
      </div>
      {#if c.body}
        <p class="body selectable" class:clamped={longBody && !bodyOpen}>{c.body}</p>
        {#if longBody}
          <button class="more-body" onclick={() => (bodyOpen = !bodyOpen)} aria-expanded={bodyOpen}>
            <ChevronDown size={13} class={bodyOpen ? 'up' : ''} />{bodyOpen ? 'Show less' : 'Show the full description'}
          </button>
        {/if}
      {/if}
      <div class="meta">
        <span class="who" title={c.authorEmail}><i class="avatar">{initials(c.authorName)}</i>{c.authorName}</span>
        <button class="sha mono" onclick={copySha} title="Copy the full SHA">
          {c.shortId}{#if copied}<Check size={12} />{:else}<Copy size={12} />{/if}
        </button>
        <span class="when" title={fullDate(c.authorTime)}>{relativeTime(c.authorTime)}</span>
        {#if d}
          <span class="files-count">{plural(d.files.length, 'changed file')}</span>
          {#if totals}<span class="mono num tot"><span class="a">+{totals.a}</span> <span class="dl">−{totals.d}</span></span>{/if}
        {/if}
        {#each labels as l (l.kind + l.name)}
          <span class="ref-chip {l.kind}">{l.name}</span>
        {/each}
      </div>
    </header>

    <div class="body-split">
      <div class="files" role="listbox" aria-label="Files in this commit">
        {#if d}
          {#each d.files as f (f.path)}
            <FileRow
              path={f.path}
              origPath={f.oldPath}
              kind={f.status}
              stats={f.binary ? null : { additions: f.additions, deletions: f.deletions }}
              selected={repo.commitFile === f.path}
              onselect={() => repo.selectCommitFile(f.path)}
            />
          {/each}
        {:else}
          {#each Array(3) as _, i (i)}<div class="sk"><i style="width:{50 + i * 15}%"></i></div>{/each}
        {/if}
      </div>
      <div class="diff-area">
        {#if repo.commitDiff && repo.commitFile}
          <div class="diff-head">
            <span class="mono path" title={repo.commitFile}>{repo.commitFile}</span>
            <DiffOptions />
          </div>
          <DiffView
            diff={repo.commitDiff}
            split={app.panes.split}
            wrap={app.panes.wrap}
            context={repo.commitContext}
            onexpand={() => repo.expandCommitDiff()}
          />
        {:else if d && !d.files.length}
          <p class="placeholder">This commit doesn’t change any files.</p>
        {/if}
      </div>
    </div>
  </section>
{/if}

<style>
  .commit {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .head {
    flex: none;
    padding: 14px 16px 12px 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-bottom: 1px solid var(--line);
    background: var(--bg-1);
  }

  .title-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }

  h2 {
    flex: 1;
    min-width: 0;
    font-size: 15px;
    font-weight: 600;
    line-height: 1.35;
    padding-top: 2px;
  }

  .actions {
    display: flex;
    gap: 4px;
    flex: none;
  }

  .body {
    white-space: pre-wrap;
    color: var(--text-2);
    line-height: 1.5;
    max-width: 90ch;
  }

  .body.clamped {
    display: -webkit-box;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .more-body {
    align-self: flex-start;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-2);
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
  }

  .more-body:hover {
    color: var(--text);
  }

  .more-body :global(svg) {
    transition: transform 150ms var(--ease-out);
  }

  .more-body :global(svg.up) {
    transform: rotate(180deg);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    color: var(--text-2);
    font-size: 12.5px;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--text);
  }

  .avatar {
    width: 20px;
    height: 20px;
    flex: none;
    border-radius: 50%;
    background: var(--bg-3);
    font-style: normal;
    font-size: 11px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .sha {
    height: 22px;
    padding: 0 7px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--bg-2);
    color: var(--text);
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .sha:hover {
    background: var(--bg-3);
  }

  .tot {
    font-size: 11.5px;
  }

  .tot .a {
    color: var(--add-text);
  }

  .tot .dl {
    color: var(--del-text);
  }

  .meta .ref-chip {
    background: var(--bg-3);
    color: var(--text);
  }

  .body-split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(200px, 280px) minmax(0, 1fr);
  }

  .files {
    min-height: 0;
    overflow-y: auto;
    padding: 6px 0;
    border-right: 1px solid var(--line);
    background: var(--bg-1);
  }

  .sk {
    height: 30px;
    padding: 0 16px;
    display: flex;
    align-items: center;
  }

  .sk i {
    height: 9px;
    border-radius: var(--radius-sm);
    background: var(--bg-2);
  }

  .diff-area {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .diff-head {
    height: 40px;
    flex: none;
    padding: 0 8px 0 16px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid var(--line);
    background: var(--bg-1);
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .placeholder {
    margin: auto;
    color: var(--text-2);
    padding: 16px;
  }
</style>
