<script lang="ts">
  import { FileX2, FileWarning, TriangleAlert, ChevronsUpDown, ChevronDown } from '@lucide/svelte'
  import type { DiffLine, FileDiff } from '../api/types'
  import { app } from '../state/app.svelte'
  import { lineKey, type LineKey } from '../util/patch'
  import { highlightDiff, type DiffTokens } from '../util/highlight'
  import { splitRows, hiddenBefore, type Cell } from '../util/diffrows'
  import Checkbox from './Checkbox.svelte'

  let {
    diff,
    split = false,
    wrap = false,
    context = 3,
    picking = false,
    onpick,
    ondiscard,
    onexpand,
    sides = { ours: 'Yours', theirs: 'Theirs', mineIsOurs: true },
  }: {
    diff: FileDiff
    split?: boolean
    wrap?: boolean
    /** Unchanged lines asked for around each change; fewer at the end means the file ends there. */
    context?: number
    /** Changed lines go in or out of the next commit by clicking their line numbers. */
    picking?: boolean
    onpick?: (keys: LineKey[], include: boolean) => void
    ondiscard?: (keys: LineKey[]) => void
    onexpand?: () => void
    /** Names for git's two conflict sides; during a rebase "ours" is the upstream, not the user. */
    sides?: { ours: string; theirs: string; mineIsOurs: boolean }
  } = $props()

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
  const sideBySide = $derived(split && !hasConflicts)
  const rows = $derived(sideBySide ? diff.hunks.map((h, hi) => splitRows(h, hi)) : [])
  const atEnd = $derived.by(() => {
    const last = diff.hunks[diff.hunks.length - 1]
    if (!last) return true
    let trailing = 0
    for (let i = last.lines.length - 1; i >= 0 && last.lines[i].kind !== 'add' && last.lines[i].kind !== 'del'; i--) {
      if (last.lines[i].kind === 'context') trailing++
    }
    return trailing < context
  })

  // Colours land a moment after the text; tokens only apply to the diff shape they were made for.
  const shape = (d: FileDiff) => d.path + '|' + d.hunks.map((h) => `${h.header}:${h.lines.length}`).join('|')
  let tokens = $state.raw<DiffTokens | null>(null)
  let tokShape = $state('')
  const tok = $derived(tokens && tokShape === shape(diff) ? tokens : null)
  $effect(() => {
    const d = diff
    let live = true
    highlightDiff(d)
      .then((t) => {
        if (!live) return
        tokens = t
        tokShape = shape(d)
      })
      .catch(() => {})
    return () => {
      live = false
    }
  })

  const isChange = (l: DiffLine) => l.kind === 'add' || l.kind === 'del'
  const clean = (t: string) => (t.endsWith('\r') ? t.slice(0, -1) : t)

  // ---- picking lines in and out of the commit ----

  let drag = $state<{ include: boolean; keys: Set<LineKey> } | null>(null)
  let anchor: LineKey | null = null
  let scrollEl = $state<HTMLDivElement>()

  // Keyboard: the line numbers are one tab stop; arrows move between changed lines.
  let focusKey = $state<LineKey | null>(null)
  const path = $derived(diff.path)
  const firstChange = $derived.by<LineKey | null>(() => {
    for (let hi = 0; hi < diff.hunks.length; hi++) {
      const li = diff.hunks[hi].lines.findIndex(isChange)
      if (li !== -1) return lineKey(hi, li)
    }
    return null
  })
  const tabKey = $derived(focusKey ?? firstChange)
  $effect(() => {
    void path
    focusKey = null
  })

  function gutterKey(e: KeyboardEvent, hi: number, li: number, l: DiffLine) {
    if (!picking || !isChange(l)) return
    const k = lineKey(hi, li)
    if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault()
      onpick?.([k], !l.staged)
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      const all = [...(scrollEl?.querySelectorAll<HTMLButtonElement>('.gutter.pick') ?? [])]
      const next = all[all.findIndex((b) => b.dataset.key === k) + (e.key === 'ArrowDown' ? 1 : -1)]
      if (!next) return
      focusKey = next.dataset.key as LineKey
      next.focus()
      next.scrollIntoView({ block: 'nearest' })
      // Shift carries this line's choice on to the next one.
      if (e.shiftKey) onpick?.([focusKey], !!l.staged)
    }
  }

  function isIn(l: DiffLine, k: LineKey): boolean {
    return drag?.keys.has(k) ? drag.include : !!l.staged
  }

  /** Every changed line between two keys, in document order. */
  function rangeKeys(a: LineKey, b: LineKey): LineKey[] {
    const [ah, al] = a.split(':').map(Number)
    const [bh, bl] = b.split(':').map(Number)
    const [s, e] = ah < bh || (ah === bh && al <= bl) ? [[ah, al], [bh, bl]] : [[bh, bl], [ah, al]]
    const out: LineKey[] = []
    for (let hi = s[0]; hi <= e[0]; hi++) {
      const lines = diff.hunks[hi].lines
      const from = hi === s[0] ? s[1] : 0
      const to = hi === e[0] ? e[1] : lines.length - 1
      for (let li = from; li <= to; li++) if (isChange(lines[li])) out.push(lineKey(hi, li))
    }
    return out
  }

  function down(e: MouseEvent, hi: number, li: number, l: DiffLine) {
    if (!picking || !isChange(l) || e.button !== 0) return
    e.preventDefault()
    const k = lineKey(hi, li)
    const include = !l.staged
    if (e.shiftKey && anchor) {
      onpick?.(rangeKeys(anchor, k), include)
      anchor = k
      return
    }
    anchor = k
    drag = { include, keys: new Set([k]) }
    window.addEventListener('mouseup', up, { once: true })
  }

  function enter(hi: number, li: number, l: DiffLine) {
    if (!drag || !isChange(l)) return
    const keys = new Set(drag.keys)
    keys.add(lineKey(hi, li))
    drag = { ...drag, keys }
  }

  function up() {
    const d = drag
    drag = null
    if (d?.keys.size) onpick?.([...d.keys], d.include)
  }

  function hunkKeys(hi: number): LineKey[] {
    return diff.hunks[hi].lines.flatMap((l, li) => (isChange(l) ? [lineKey(hi, li)] : []))
  }

  function hunkCheck(hi: number): boolean | 'mixed' {
    const lines = diff.hunks[hi].lines.filter(isChange)
    const n = lines.filter((l) => l.staged).length
    return n === 0 ? false : n === lines.length ? true : 'mixed'
  }

  function lineMenu(e: MouseEvent, hi: number, li: number, l: DiffLine) {
    if (!ondiscard || !isChange(l)) return
    e.preventDefault()
    app.openMenu(e, [
      { label: 'Discard this line', danger: true, run: () => ondiscard([lineKey(hi, li)]) },
      { label: 'Discard this hunk', danger: true, run: () => ondiscard(hunkKeys(hi)) },
    ])
  }
</script>

{#snippet code(hi: number, li: number, l: DiffLine)}
  {@const toks = tok?.[hi]?.[li]}
  <span class="text selectable">{#if toks}{#each toks as t, ti (ti)}<span class:it={t.italic} style:color={t.color}>{t.text}</span>{/each}{:else}{clean(l.text)}{/if}</span>
{/snippet}

{#snippet gutter(hi: number, li: number, l: DiffLine, nos: (number | null)[])}
  {@const pick = picking && isChange(l)}
  <button
    class="gutter"
    class:pick
    data-key={lineKey(hi, li)}
    tabindex={pick && lineKey(hi, li) === tabKey ? 0 : -1}
    disabled={!pick}
    aria-label={pick ? (l.staged ? 'Leave this line out of the commit' : 'Include this line in the commit') : undefined}
    aria-pressed={pick ? !!l.staged : undefined}
    onmousedown={(e) => down(e, hi, li, l)}
    onmouseenter={() => enter(hi, li, l)}
    onkeydown={(e) => gutterKey(e, hi, li, l)}
    onfocus={() => (focusKey = lineKey(hi, li))}
  >
    {#each nos as n, i (i)}<span class="no num">{n ?? ''}</span>{/each}
  </button>
{/snippet}

{#snippet half(cell: Cell | null, side: 'left' | 'right')}
  {#if cell}
    {@const l = cell.line}
    {@const k = lineKey(cell.hi, cell.li)}
    {@const pick = picking && isChange(l)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="half {isChange(l) ? l.kind : 'context'}" class:in={pick && isIn(l, k)} class:out={pick && !isIn(l, k)} oncontextmenu={(e) => lineMenu(e, cell.hi, cell.li, l)}>
      {@render gutter(cell.hi, cell.li, l, [side === 'left' ? l.oldNo : l.newNo])}
      <span class="mark">{l.kind === 'add' ? '+' : l.kind === 'del' ? '−' : ''}</span>
      {@render code(cell.hi, cell.li, l)}
    </div>
  {:else}
    <div class="half blank"></div>
  {/if}
{/snippet}

<div class="diff" class:wrap={wrap || sideBySide} class:split={sideBySide}>
  {#if diff.binary}
    <div class="note"><FileX2 size={18} /><p>Binary file. There’s no text diff to show.</p></div>
  {:else if diff.tooLarge}
    <div class="note"><FileWarning size={18} /><p>This diff is too large to show here. Open the file in your editor to review it.</p></div>
  {:else if diff.hunks.length === 0}
    <div class="note"><p>No line changes. Only the file’s mode or name changed.</p></div>
  {:else}
    <div class="scroll" class:dragging={!!drag} bind:this={scrollEl}>
      {#if hasConflicts}
        <p class="conflict-note"><TriangleAlert size={14} />Conflicts: keep one side from the list, or edit the file and mark it resolved.</p>
      {/if}
      {#each diff.hunks as h, hi (hi)}
        {@const hidden = hiddenBefore(diff.hunks, hi)}
        <div class="hunk-head">
          {#if onexpand && hidden > 0}
            <button class="expand" onclick={onexpand} title="Show more unchanged lines ({hidden} hidden)" aria-label="Show more unchanged lines"><ChevronsUpDown size={14} /></button>
          {:else}
            <span class="expand-spacer"></span>
          {/if}
          {#if picking && hunkKeys(hi).length}
            <Checkbox checked={hunkCheck(hi)} label={hunkCheck(hi) === true ? 'Leave this hunk out of the commit' : 'Include this hunk in the commit'} onchange={() => onpick?.(hunkKeys(hi), hunkCheck(hi) !== true)} />
          {/if}
          <span class="mono hh" title={h.header}>{h.header}</span>
        </div>

        {#if sideBySide}
          <div class="lines">
            {#each rows[hi] as row, ri (ri)}
              <div class="srow">
                {#if row.meta}
                  <div class="smeta">No newline at end of file</div>
                {:else}
                  {@render half(row.left, 'left')}
                  {@render half(row.right, 'right')}
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="lines mono">
            {#each h.lines as l, li (li)}
              {@const r = hunkRoles[hi][li]}
              {@const k = lineKey(hi, li)}
              {@const pick = picking && isChange(l)}
              {#if l.kind === 'noNewline'}
                <div class="line meta"><span class="gutter"><span class="no"></span><span class="no"></span></span><span class="mark"></span><span class="text">No newline at end of file</span></div>
              {:else if r.role.startsWith('mark-')}
                <div class="line marker {r.role}" class:mine-ours={sides.mineIsOurs}>
                  <span class="gutter"><span class="no num"></span><span class="no num">{l.newNo ?? ''}</span></span><span class="mark"></span>
                  <span class="marker-label">
                    {#if r.role === 'mark-ours'}{sides.ours}{r.label && !r.label.startsWith('HEAD') ? ` · ${r.label}` : ''}
                    {:else if r.role === 'mark-base'}Common ancestor{r.label ? ` · ${r.label}` : ''}
                    {:else if r.role === 'mark-sep'}{sides.theirs} below
                    {:else}End of {sides.theirs.charAt(0).toLowerCase() + sides.theirs.slice(1)}{r.label && sides.theirs === 'Theirs' ? ` · ${r.label}` : ''}{/if}
                  </span>
                </div>
              {:else}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  class="line {r.role === 'plain' ? l.kind : r.role}"
                  class:mine-ours={sides.mineIsOurs}
                  class:in={pick && isIn(l, k)}
                  class:out={pick && !isIn(l, k)}
                  oncontextmenu={(e) => lineMenu(e, hi, li, l)}
                >
                  {@render gutter(hi, li, l, [l.oldNo, l.newNo])}
                  <span class="mark">{r.role !== 'plain' ? '' : l.kind === 'add' ? '+' : l.kind === 'del' ? '−' : ''}</span>
                  {@render code(hi, li, l)}
                </div>
              {/if}
            {/each}
          </div>
        {/if}
      {/each}
      {#if onexpand && !atEnd}
        <div class="hunk-head end">
          <button class="expand" onclick={onexpand} title="Show more unchanged lines below" aria-label="Show more unchanged lines below"><ChevronDown size={14} /></button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .diff {
    position: relative;
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-0);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding-bottom: 24px;
  }

  .scroll.dragging {
    cursor: pointer;
    user-select: none;
  }

  .note {
    margin: 24px;
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-2);
  }

  .conflict-note {
    margin: 10px 12px;
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
    padding: 4px 12px 4px 0;
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-1);
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    color: var(--text-2);
  }

  .hunk-head.end {
    border-bottom: 0;
  }

  .expand,
  .expand-spacer {
    width: 40px;
    height: 24px;
    flex: none;
  }

  .expand {
    margin-left: 4px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      background-color 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .expand:hover {
    background: var(--bg-3);
    color: var(--text);
  }

  .hh {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
  }

  .lines {
    min-width: max-content;
  }

  .wrap .lines {
    min-width: 0;
  }

  .line,
  .half {
    display: flex;
    min-height: 22px;
    line-height: 22px;
    font-family: var(--font-mono);
    font-size: 12px;
    font-variant-ligatures: none;
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
    z-index: 1;
    transition: background-color 120ms var(--ease-out);
  }

  .gutter.pick {
    cursor: pointer;
  }

  .gutter.pick:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .in .gutter.pick:focus-visible {
    outline-color: var(--text);
  }

  .no {
    width: 48px;
    flex: none;
    padding: 0 10px 0 0;
    text-align: right;
    font-size: 11px;
  }

  .mark {
    width: 20px;
    flex: none;
    text-align: center;
    color: var(--text-3);
  }

  .text {
    white-space: pre;
    padding-right: 24px;
    color: var(--syntax-foreground);
  }

  .text .it {
    font-style: italic;
  }

  .wrap .text {
    flex: 1;
    min-width: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding-right: 12px;
  }

  /* Changed lines: a wash for the line, a stronger tint in the gutter. */
  .add {
    background: var(--add-bg);
  }

  .add .gutter {
    background: rgba(61, 220, 151, 0.08);
  }

  .add .mark,
  .add .no {
    color: var(--add-text);
  }

  .del {
    background: var(--del-bg);
  }

  .del .gutter {
    background: rgba(255, 107, 107, 0.08);
  }

  .del .mark,
  .del .no {
    color: var(--del-text);
  }

  /* In the commit: the line number column turns Signal Blue, as in GitHub Desktop. */
  .in .gutter {
    background: var(--accent);
  }

  .in .gutter .no {
    color: var(--on-color);
    font-weight: 600;
  }

  .in .gutter.pick:hover {
    background: var(--accent-hover);
  }

  .out .gutter {
    background: var(--bg-1);
  }

  .out .gutter .no,
  .out .mark {
    color: var(--text-3);
  }

  .out .gutter.pick:hover {
    background: var(--bg-3);
  }

  /* Side by side: two equal columns split by a hairline. */
  .srow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }

  .half {
    min-width: 0;
  }

  .half:first-child {
    border-right: 1px solid var(--line);
  }

  .half.blank {
    background: repeating-linear-gradient(-45deg, transparent 0 6px, rgba(255, 255, 255, 0.018) 6px 12px);
  }

  .smeta {
    grid-column: 1 / -1;
    padding-left: 68px;
    font-size: 12px;
    font-style: italic;
    color: var(--text-3);
    line-height: 22px;
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
    min-height: 26px;
    line-height: 26px;
    background: var(--bg-1);
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }

  .line.marker .gutter {
    background: transparent;
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

  .line.meta .text {
    color: var(--text-3);
    font-style: italic;
    font-family: var(--font-ui);
  }
</style>
