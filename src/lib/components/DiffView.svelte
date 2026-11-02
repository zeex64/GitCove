<script lang="ts">
  import { FileX2, FileWarning, TriangleAlert } from '@lucide/svelte'
  import type { DiffLine, FileDiff } from '../api/types'
  import type { RepoStore } from '../state/repo.svelte'
  import { lineKey, type LineKey } from '../util/patch'
  import { plural } from '../util/format'

  let {
    repo,
    diff,
    editable = false,
    staged = false,
    compact = false,
    wrap = false,
    sides = { ours: 'Yours', theirs: 'Theirs', mineIsOurs: true },
  }: {
    repo: RepoStore
    diff: FileDiff
    editable?: boolean
    staged?: boolean
    compact?: boolean
    wrap?: boolean
    /** Names for git's two conflict sides; during a rebase "ours" is the upstream, not the user. */
    sides?: { ours: string; theirs: string; mineIsOurs: boolean }
  } = $props()

  let anchor = $state<LineKey | null>(null)

  const sel = $derived(editable ? repo.lineSel : new Set<LineKey>())

  type Role = 'plain' | 'ours' | 'theirs' | 'base' | 'mark-ours' | 'mark-base' | 'mark-sep' | 'mark-theirs'

  /** Conflict markers split a hunk into labelled "yours" and "theirs" regions, whatever +/- git gave them. */
  function roles(lines: DiffLine[]): { role: Role; label: string }[] {
    let region: 'none' | 'ours' | 'base' | 'theirs' = 'none'
    return lines.map((l) => {
      const t = l.text.replace(/\r$/, '')
      if (/^<{7}( |$)/.test(t)) {
        region = 'ours'
        return { role: 'mark-ours', label: t.slice(8).trim() }
      }
      if (region !== 'none' && /^\|{7}( |$)/.test(t)) {
        region = 'base'
        return { role: 'mark-base', label: t.slice(8).trim() }
      }
      if (region !== 'none' && /^={7}$/.test(t)) {
        region = 'theirs'
        return { role: 'mark-sep', label: '' }
      }
      if (region !== 'none' && /^>{7}( |$)/.test(t)) {
        region = 'none'
        return { role: 'mark-theirs', label: t.slice(8).trim() }
      }
      return { role: region === 'none' ? 'plain' : region, label: '' }
    })
  }

  const hunkRoles = $derived(diff.hunks.map((h) => roles(h.lines)))
  const hasConflicts = $derived(hunkRoles.some((r) => r.some((x) => x.role === 'mark-ours')))

  function toggle(e: MouseEvent, hi: number, li: number) {
    if (!editable) return
    const k = lineKey(hi, li)
    repo.toggleLine(k, e.shiftKey ? anchor : null)
    anchor = k
  }
</script>

<div class="diff" class:compact class:wrap>
  {#if diff.binary}
    <div class="note"><FileX2 size={18} /><p>Binary file. There’s no text diff to show.</p></div>
  {:else if diff.tooLarge}
    <div class="note"><FileWarning size={18} /><p>This diff is too large to show here. Open the file in your editor to review it.</p></div>
  {:else if diff.hunks.length === 0}
    <div class="note"><p>No line changes. Only the file’s mode or name changed.</p></div>
  {:else}
    <div class="scroll">
      {#if hasConflicts}
        <p class="conflict-note"><TriangleAlert size={14} />Conflicts: keep one side from the list, or edit the file and mark it resolved.</p>
      {/if}
      {#each diff.hunks as h, hi (hi)}
        <div class="hunk-head">
          <span class="mono" title={h.header}>{h.header}</span>
          {#if editable}
            <span class="hunk-actions">
              {#if !staged}
                <button class="btn sm quiet" onclick={() => {
                  repo.lineSel = new Set(h.lines.flatMap((l, li) => (l.kind === 'add' || l.kind === 'del' ? [lineKey(hi, li)] : [])))
                  repo.discardLines()
                }}>Discard hunk</button>
              {/if}
              <button class="btn sm ghost" onclick={() => repo.applyLines([hi])}>{staged ? 'Unstage hunk' : 'Stage hunk'}</button>
            </span>
          {/if}
        </div>
        <div class="lines mono">
          {#each h.lines as l, li (li)}
            {@const k = lineKey(hi, li)}
            {@const change = l.kind === 'add' || l.kind === 'del'}
            {@const r = hunkRoles[hi][li]}
            {#if l.kind === 'noNewline'}
              <div class="line meta"><span class="no"></span><span class="no"></span><span class="mark"></span><span class="text">No newline at end of file</span></div>
            {:else if r.role.startsWith('mark-')}
              <div class="line marker {r.role}" class:mine-ours={sides.mineIsOurs}>
                <span class="no num"></span><span class="no num">{l.newNo ?? ''}</span><span class="mark"></span>
                <span class="marker-label">
                  {#if r.role === 'mark-ours'}{sides.ours}{r.label && !r.label.startsWith('HEAD') ? ` · ${r.label}` : ''}
                  {:else if r.role === 'mark-base'}Common ancestor{r.label ? ` · ${r.label}` : ''}
                  {:else if r.role === 'mark-sep'}{sides.theirs} below
                  {:else}End of {sides.theirs.charAt(0).toLowerCase() + sides.theirs.slice(1)}{r.label && sides.theirs === 'Theirs' ? ` · ${r.label}` : ''}{/if}
                </span>
              </div>
            {:else}
              <div
                class="line {r.role === 'plain' ? l.kind : r.role}"
                class:mine-ours={sides.mineIsOurs}
                class:selected={sel.has(k)}
              >
                <button
                  class="gutter"
                  class:pick={editable && change}
                  tabindex="-1"
                  disabled={!editable || !change}
                  aria-label={change ? 'Select line' : undefined}
                  onmousedown={(e) => change && toggle(e, hi, li)}
                >
                  <span class="no num">{l.oldNo ?? ''}</span><span class="no num">{l.newNo ?? ''}</span>
                </button>
                <span class="mark">{r.role !== 'plain' ? '' : l.kind === 'add' ? '+' : l.kind === 'del' ? '−' : ''}</span>
                <span class="text selectable">{l.text.endsWith('\r') ? l.text.slice(0, -1) : l.text}</span>
              </div>
            {/if}
          {/each}
        </div>
      {/each}
    </div>
    {#if editable && sel.size}
      <div class="selbar">
        <span>{plural(sel.size, 'line')} selected</span>
        <button class="btn sm quiet" onclick={() => repo.clearLines()}>Clear</button>
        {#if !staged}<button class="btn sm ghost" onclick={() => repo.discardLines()}>Discard lines</button>{/if}
        <button class="btn sm primary" onclick={() => repo.applyLines()}>{staged ? 'Unstage lines' : 'Stage lines'}</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .diff {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-0);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding-bottom: 12px;
  }

  .note {
    margin: 16px;
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-2);
  }

  .conflict-note {
    margin: 8px 10px;
    padding: 8px 10px;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    border-radius: var(--radius);
    background: var(--warning-soft);
    color: var(--text);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .conflict-note :global(svg) {
    color: var(--warning);
    flex: none;
    margin-top: 2px;
  }

  .hunk-head {
    position: sticky;
    left: 0;
    min-height: 32px;
    padding: 4px 8px 4px 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    background: var(--bg-1);
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    color: var(--text-2);
  }

  .hunk-head .mono {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
  }

  .hunk-actions {
    display: flex;
    gap: 4px;
    flex: none;
  }

  .lines {
    min-width: max-content;
  }

  .wrap .lines {
    min-width: 0;
  }

  .line {
    display: flex;
    min-height: 20px;
    line-height: 20px;
  }

  .gutter {
    display: flex;
    flex: none;
    align-self: stretch;
    padding: 0;
    border: 0;
    background: var(--bg-0);
    color: var(--text-3);
    font: inherit;
    cursor: default;
    position: sticky;
    left: 0;
  }

  .gutter.pick {
    cursor: pointer;
  }

  .gutter.pick:hover {
    background: var(--bg-3);
    color: var(--text);
  }

  .no {
    width: 44px;
    flex: none;
    padding: 0 8px 0 0;
    text-align: right;
    font-size: 11px;
  }

  .compact .no {
    width: 36px;
  }

  .compact .hunk-head {
    min-height: 28px;
    padding-top: 2px;
    padding-bottom: 2px;
  }

  .mark {
    width: 18px;
    flex: none;
    text-align: center;
    color: var(--text-3);
  }

  .text {
    white-space: pre;
    padding-right: 24px;
    color: var(--text);
  }

  .wrap .text {
    flex: 1;
    min-width: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding-right: 12px;
  }

  .line.add {
    background: var(--add-bg);
  }

  .line.add .gutter {
    background: rgba(61, 220, 151, 0.07);
  }

  .line.add .mark,
  .line.add .no {
    color: var(--add-text);
  }

  .line.del {
    background: var(--del-bg);
  }

  .line.del .gutter {
    background: rgba(255, 107, 107, 0.07);
  }

  .line.del .mark,
  .line.del .no {
    color: var(--del-text);
  }

  /* Conflict regions: the user's side in the accent, the other side in violet, markers as labelled rules. */
  .line.ours.mine-ours,
  .line.theirs:not(.mine-ours) {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .line.theirs.mine-ours,
  .line.ours:not(.mine-ours) {
    background: rgba(181, 140, 255, 0.13);
    box-shadow: inset 2px 0 0 var(--lane-2);
  }

  .line.base {
    background: rgba(255, 255, 255, 0.03);
    box-shadow: inset 2px 0 0 var(--text-3);
  }

  .line.ours .gutter,
  .line.theirs .gutter,
  .line.base .gutter {
    background: transparent;
  }

  .line.marker {
    min-height: 24px;
    line-height: 24px;
    background: var(--bg-1);
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }

  .marker-label {
    font-family: var(--font-ui);
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mine-ours.mark-ours .marker-label,
  .mark-theirs:not(.mine-ours) .marker-label,
  .mark-sep:not(.mine-ours) .marker-label {
    color: var(--accent);
  }

  .mark-ours:not(.mine-ours) .marker-label,
  .mine-ours.mark-theirs .marker-label,
  .mine-ours.mark-sep .marker-label {
    color: var(--lane-2);
  }

  .mark-base .marker-label {
    color: var(--text-2);
  }

  .line.selected {
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .line.selected .gutter {
    background: var(--accent);
  }

  .line.selected .gutter .no {
    color: var(--on-color);
  }

  .line.meta .text {
    color: var(--text-3);
    font-style: italic;
  }

  .selbar {
    position: absolute;
    left: 50%;
    bottom: 12px;
    transform: translateX(-50%);
    padding: 6px 6px 6px 12px;
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    background: var(--bg-2);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
    animation: rise 160ms var(--ease-out);
  }

  .selbar span {
    margin-right: 6px;
    color: var(--text-2);
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 6px);
    }
  }
</style>
