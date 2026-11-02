<script lang="ts">
  import { ArrowRight, TriangleAlert, Archive, Trash2, CloudAlert } from '@lucide/svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import type { Preview } from '../api/types'
  import { plural } from '../util/format'

  let { repo, preview }: { repo: RepoStore; preview: Preview } = $props()

  const replaced = $derived(preview.replaced.map((id) => repo.byId.get(id)).filter((c) => !!c))
  const short = (id: string | null) => (id ? (repo.byId.get(id)?.shortId ?? id.slice(0, 7)) : '')
</script>

<div class="details">
  <div class="inner">
    <h2>What happens if you apply this</h2>
    <p class="lede">Nothing has changed yet. The graph on the left shows the result: dashed commits are new, struck-through ones are replaced.</p>

    {#if preview.created.length}
      <section>
        <h3 class="section-label">{plural(preview.created.length, 'new commit')}</h3>
        <ul>
          {#each preview.created as c (c.id)}
            <li><i class="node ghost" class:conflict={c.conflicted}></i><span class="summary">{c.summary}</span>{#if c.conflicted}<em class="warn">would conflict</em>{/if}</li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if replaced.length}
      <section>
        <h3 class="section-label">{plural(replaced.length, 'commit')} replaced</h3>
        <ul>
          {#each replaced as c (c.id)}
            <li class="struck"><i class="node void"></i><span class="summary">{c.summary}</span><span class="mono sha">{c.shortId}</span></li>
          {/each}
        </ul>
        <p class="hint">Their changes live on in the new commits above, and Undo brings the originals back.</p>
      </section>
    {/if}

    {#if preview.moves.length}
      <section>
        <h3 class="section-label">Branches that move</h3>
        <ul>
          {#each preview.moves as m (m.ref)}
            <li>
              <span class="ref">{m.ref}</span>
              {#if m.from && m.to}<span class="mono sha">{short(m.from)}</span><ArrowRight size={13} /><span class="mono sha">{short(m.to)}</span>
              {:else if m.to}<span class="muted">created at</span><span class="mono sha">{short(m.to)}</span>
              {:else}<span class="danger">deleted</span>{/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if preview.conflicts.files.length}
      <section>
        <h3 class="section-label">Expected conflicts</h3>
        <ul>
          {#each preview.conflicts.files as f (f)}
            <li><TriangleAlert size={14} class="ic warn-ic" /><span class="mono">{f}</span></li>
          {/each}
        </ul>
        <p class="hint">You’ll be able to resolve them file by file, or abort and be back where you started.</p>
      </section>
    {/if}

    {#if preview.remoteCommitsLost.length}
      <section>
        <h3 class="section-label danger">Removed from the remote</h3>
        <ul>
          {#each preview.remoteCommitsLost as c (c.id)}
            <li><CloudAlert size={14} class="ic danger-ic" /><span class="summary">{c.summary}</span><span class="mono sha">{c.shortId}</span></li>
          {/each}
        </ul>
        <p class="hint">Anyone who already pulled these keeps them; this can’t be undone from here.</p>
      </section>
    {/if}

    {#if preview.needsStash || preview.discardedFiles.length}
      <section>
        <h3 class="section-label">Your uncommitted changes</h3>
        <ul>
          {#if preview.needsStash}
            <li><Archive size={14} class="ic" /><span>Set aside first and put back afterwards.</span></li>
          {/if}
          {#each preview.discardedFiles as f (f)}
            <li><Trash2 size={14} class="ic danger-ic" /><span class="mono">{f}</span><span class="muted">thrown away (Undo restores it)</span></li>
          {/each}
        </ul>
      </section>
    {/if}

    {#each preview.notes as n (n)}
      <p class="note">{n}</p>
    {/each}
  </div>
</div>

<style>
  .details {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 28px 24px 40px;
  }

  .inner {
    width: min(680px, 100%);
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  h2 {
    font-size: 18px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  .lede {
    margin-top: -14px;
    color: var(--text-2);
    line-height: 1.5;
    max-width: 62ch;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  h3.danger {
    color: var(--danger);
  }

  ul {
    margin: 0;
    padding: 4px;
    list-style: none;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--bg-1);
  }

  li {
    min-height: 32px;
    padding: 4px 10px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-radius: var(--radius-sm);
  }

  li + li {
    border-top: 1px solid var(--line);
    border-radius: 0;
  }

  .node {
    width: 12px;
    height: 12px;
    flex: none;
    border-radius: 50%;
  }

  .node.ghost {
    border: 1.5px dashed var(--accent);
  }

  .node.ghost.conflict {
    border-color: var(--warning);
  }

  .node.void {
    border: 1.5px solid var(--text-3);
    background: linear-gradient(var(--text-3), var(--text-3)) center / 100% 1.5px no-repeat;
  }

  .summary {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .struck .summary {
    text-decoration: line-through;
    text-decoration-color: rgba(255, 255, 255, 0.35);
    color: var(--text-2);
  }

  .sha {
    color: var(--text-2);
  }

  .ref {
    font-weight: 600;
    margin-right: 4px;
  }

  li :global(svg) {
    flex: none;
    color: var(--text-2);
  }

  li :global(.warn-ic) {
    color: var(--warning);
  }

  li :global(.danger-ic) {
    color: var(--danger);
  }

  em.warn {
    font-style: normal;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--warning);
  }

  .muted {
    color: var(--text-2);
  }

  .danger {
    color: var(--danger);
  }

  .hint,
  .note {
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 1.45;
  }
</style>
