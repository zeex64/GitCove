<script lang="ts">
  import { Copy, Check, ArrowLeft, Ellipsis, GitBranchPlus } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import { commitMenu, askNewBranch } from '../state/menus'
  import { copyText } from '../api/system'
  import { fullDate, initials, plural, relativeTime } from '../util/format'
  import FileRow from './FileRow.svelte'
  import DiffView from './DiffView.svelte'

  let { repo, id }: { repo: RepoStore; id: string } = $props()

  let copied = $state(false)
  let moreBtn = $state<HTMLButtonElement>()
  let branchBtn = $state<HTMLButtonElement>()

  const d = $derived(repo.details.get(id) ?? null)
  const c = $derived(d?.commit ?? repo.byId.get(id) ?? null)
  const labels = $derived(repo.labels.get(id) ?? [])
  const totals = $derived(
    d ? d.files.reduce((t, f) => ({ a: t.a + f.additions, d: t.d + f.deletions }), { a: 0, d: 0 }) : null,
  )

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

<section class="panel" aria-label="Commit details">
  <header class="head">
    <button class="btn sm quiet" onclick={() => repo.select({ kind: 'wip' })} title="Back to working changes">
      <ArrowLeft size={14} />Working changes
    </button>
    <div class="head-actions">
      {#if c}
        <button class="btn sm ghost" bind:this={branchBtn} onclick={() => branchBtn && askNewBranch(repo, branchBtn, c.id, c.shortId)}>
          <GitBranchPlus size={14} />Branch here
        </button>
        <button class="icon-btn" bind:this={moreBtn} aria-label="More actions" onclick={more}><Ellipsis size={16} /></button>
      {/if}
    </div>
  </header>

  {#if c}
    <div class="meta">
      <h2 class="selectable">{c.summary}</h2>
      {#if c.body}<p class="body selectable">{c.body}</p>{/if}
      {#if labels.length}
        <div class="labels">
          {#each labels as l (l.kind + l.name)}
            <span class="ref-chip {l.kind}">{l.name}</span>
          {/each}
        </div>
      {/if}
      <dl>
        <dt>Author</dt>
        <dd class="author">
          <i class="avatar">{initials(c.authorName)}</i>
          <span><span>{c.authorName}</span> <span class="muted selectable">{c.authorEmail}</span></span>
        </dd>
        <dt>Date</dt>
        <dd title={fullDate(c.authorTime)}>{fullDate(c.authorTime)} <span class="muted">({relativeTime(c.authorTime)})</span></dd>
        <dt>Commit</dt>
        <dd>
          <button class="sha mono" onclick={copySha} title="Copy full SHA">
            {c.id.slice(0, 12)}
            {#if copied}<Check size={13} />{:else}<Copy size={13} />{/if}
          </button>
        </dd>
        {#if c.parents.length}
          <dt>{c.parents.length > 1 ? 'Parents' : 'Parent'}</dt>
          <dd class="parents">
            {#each c.parents as p (p)}
              <button class="sha mono" onclick={() => repo.select({ kind: 'commit', id: p })}>{p.slice(0, 7)}</button>
            {/each}
          </dd>
        {/if}
      </dl>
    </div>

    <div class="files">
      <div class="files-head">
        <span class="section-label">{d ? plural(d.files.length, 'file') + ' changed' : 'Files changed'}</span>
        {#if totals}<span class="mono num tot"><span class="a">+{totals.a}</span> <span class="dl">−{totals.d}</span></span>{/if}
      </div>
      <div class="file-list">
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
    </div>

    <div class="diff-area">
      {#if repo.commitDiff && repo.commitFile}
        <div class="diff-head"><span class="mono">{repo.commitFile}</span></div>
        <DiffView {repo} diff={repo.commitDiff} compact wrap={app.panes.wrap} />
      {:else}
        <p class="placeholder">Select a file to see what this commit changed.</p>
      {/if}
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
    padding: 0 10px 0 8px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--line);
  }

  .head-actions {
    display: flex;
    gap: 4px;
  }

  .meta {
    flex: none;
    padding: 14px 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-bottom: 1px solid var(--line);
    max-height: 45%;
    overflow-y: auto;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
    line-height: 1.35;
  }

  .body {
    white-space: pre-wrap;
    color: var(--text-2);
    line-height: 1.5;
  }

  .labels {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .labels .ref-chip {
    background: var(--bg-3);
    color: var(--text);
  }

  dl {
    margin: 4px 0 0;
    display: grid;
    grid-template-columns: 64px 1fr;
    gap: 6px 10px;
    align-items: center;
  }

  dt {
    color: var(--text-2);
    font-size: 12px;
  }

  dd {
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .author {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .author > span {
    overflow: hidden;
    text-overflow: ellipsis;
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

  .muted {
    color: var(--text-2);
  }

  .sha {
    height: 22px;
    padding: 0 6px;
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

  .parents {
    display: flex;
    gap: 4px;
  }

  .files {
    flex: 0 1 auto;
    max-height: 30%;
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--line);
    min-height: 0;
  }

  .files-head {
    height: 34px;
    flex: none;
    padding: 0 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
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

  .file-list {
    overflow-y: auto;
    padding-bottom: 6px;
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
    flex: 1;
    min-height: 100px;
    display: flex;
    flex-direction: column;
  }

  .diff-head {
    height: 34px;
    flex: none;
    padding: 0 14px;
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--line);
    overflow: hidden;
    white-space: nowrap;
  }

  .placeholder {
    margin: auto;
    color: var(--text-2);
    padding: 16px;
    text-align: center;
  }
</style>
