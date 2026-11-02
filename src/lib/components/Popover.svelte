<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    open = $bindable(false),
    anchor,
    align = 'start',
    width = 300,
    children,
  }: {
    open: boolean
    anchor: HTMLElement | undefined
    align?: 'start' | 'center' | 'end'
    width?: number
    children: Snippet
  } = $props()

  let el = $state<HTMLDivElement>()
  let pos = $state({ x: 0, y: 0 })

  $effect(() => {
    if (!open || !anchor) return
    const r = anchor.getBoundingClientRect()
    const x = align === 'start' ? r.left : align === 'end' ? r.right - width : r.left + r.width / 2 - width / 2
    pos = { x: Math.max(8, Math.min(x, window.innerWidth - width - 8)), y: r.bottom + 6 }
  })

  function outside(e: MouseEvent) {
    if (!open) return
    const t = e.target as Node
    if (el?.contains(t) || anchor?.contains(t)) return
    open = false
  }
</script>

<svelte:window onmousedown={outside} onkeydown={(e) => open && e.key === 'Escape' && (open = false)} />

{#if open}
  <div class="pop" bind:this={el} style="left:{pos.x}px; top:{pos.y}px; width:{width}px">
    {@render children()}
  </div>
{/if}

<style>
  .pop {
    position: fixed;
    z-index: 45;
    max-height: min(520px, calc(100vh - 120px));
    display: flex;
    flex-direction: column;
    background: var(--bg-2);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    animation: pop 140ms var(--ease-out);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
</style>
