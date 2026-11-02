<script lang="ts">
  import type { GraphRow, Seg } from '../graph/layout'

  let { row, width, isHead }: { row: GraphRow; width: number; isHead: boolean } = $props()

  const H = 34
  const MID = H / 2
  const R = 7.5

  const x = (lane: number) => 14 + lane * 16

  function path(s: Seg, top: boolean): string {
    const x1 = x(s.from)
    const x2 = x(s.to)
    const y1 = top ? 0 : MID
    const y2 = top ? MID : H
    if (x1 === x2) return `M${x1} ${y1}V${y2}`
    // Curves leave or enter the node's row smoothly, like the rest of the category.
    return top ? `M${x1} ${y1}C${x1} ${y1 + 11} ${x2} ${y2 - 9} ${x2} ${y2}` : `M${x1} ${y1}C${x1} ${y1 + 9} ${x2} ${y2 - 11} ${x2} ${y2}`
  }

  const c = $derived(row.commit)
  const cx = $derived(x(row.lane))
  const color = $derived(`var(--lane-${row.color})`)
  const merge = $derived(c.parents.length > 1)
</script>

<svg {width} height={H} viewBox="0 0 {width} {H}" aria-hidden="true">
  {#each row.top as s, i (i)}
    <path d={path(s, true)} class="seg {s.style}" stroke="var(--lane-{s.color})" />
  {/each}
  {#each row.bottom as s, i (i)}
    <path d={path(s, false)} class="seg {s.style}" stroke="var(--lane-{s.color})" />
  {/each}

  {#if c.wip}
    <circle {cx} cy={MID} r={R - 1} class="node-wip" stroke={color} />
  {:else if c.ghost}
    <circle {cx} cy={MID} r={R - 0.5} class="node-ghost" class:conflict={c.conflicted} stroke={c.conflicted ? 'var(--warning)' : color} />
  {:else if c.void}
    <circle {cx} cy={MID} r={R - 1.5} class="node-void" />
    <path d="M{cx - 6} {MID}H{cx + 6}" class="strike" />
  {:else}
    {#if isHead}<circle {cx} cy={MID} r={R + 3.5} class="head-ring" stroke={color} />{/if}
    <circle {cx} cy={MID} r={merge ? R - 1.5 : R} fill={color} class="node" />
  {/if}
</svg>

<style>
  svg {
    display: block;
    overflow: visible;
    flex: none;
  }

  .seg {
    fill: none;
    stroke-width: 2.2;
    stroke-linecap: round;
  }

  .seg.ghost {
    stroke-dasharray: 3 4;
  }

  .seg.void {
    opacity: 0.32;
  }

  .node {
    stroke: var(--bg-0);
    stroke-width: 2;
  }

  .head-ring {
    fill: none;
    stroke-width: 1.6;
    opacity: 0.85;
  }

  .node-wip {
    fill: var(--bg-0);
    stroke-width: 2;
    stroke-dasharray: 3 2.5;
  }

  .node-ghost {
    fill: var(--bg-0);
    stroke-width: 2;
    stroke-dasharray: 3.5 2.5;
    animation: ghost-in 220ms var(--ease-out);
  }

  .node-ghost.conflict {
    stroke-dasharray: none;
  }

  .node-void {
    fill: var(--bg-0);
    stroke: var(--text-3);
    stroke-width: 1.6;
    opacity: 0.7;
  }

  .strike {
    stroke: var(--text-3);
    stroke-width: 1.6;
    stroke-linecap: round;
  }

  @keyframes ghost-in {
    from {
      opacity: 0;
      transform: scale(0.6);
    }
  }

  .node-ghost {
    transform-box: fill-box;
    transform-origin: center;
  }
</style>
