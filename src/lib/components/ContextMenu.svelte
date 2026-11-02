<script lang="ts">
  import { app } from '../state/app.svelte'

  let el = $state<HTMLDivElement>()
  let pos = $state({ x: 0, y: 0 })
  let active = $state(-1)

  const menu = $derived(app.menu)
  const runnable = $derived(menu ? menu.items.map((it, i) => (!it.separator && !it.disabled ? i : -1)).filter((i) => i >= 0) : [])

  $effect(() => {
    if (!menu || !el) return
    const r = el.getBoundingClientRect()
    pos = {
      x: Math.min(menu.x, window.innerWidth - r.width - 8),
      y: menu.y + r.height > window.innerHeight - 8 ? Math.max(8, menu.y - r.height) : menu.y,
    }
    active = -1
    el.focus()
  })

  function close() {
    app.menu = null
  }

  function choose(i: number) {
    const it = menu?.items[i]
    if (!it || it.disabled || it.separator) return
    close()
    it.run?.()
  }

  function onKey(e: KeyboardEvent) {
    if (!menu) return
    if (e.key === 'Escape') close()
    else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      const at = runnable.indexOf(active)
      const next = e.key === 'ArrowDown' ? at + 1 : at - 1
      active = runnable[(next + runnable.length) % runnable.length]
    } else if (e.key === 'Enter' && active >= 0) {
      e.preventDefault()
      choose(active)
    }
  }
</script>

<svelte:window onmousedown={(e) => menu && el && !el.contains(e.target as Node) && close()} onblur={close} />

{#if menu}
  <div
    class="menu"
    role="menu"
    tabindex="-1"
    bind:this={el}
    style="left:{pos.x}px; top:{pos.y}px"
    onkeydown={onKey}
    oncontextmenu={(e) => e.preventDefault()}
  >
    {#each menu.items as it, i (i)}
      {#if it.separator}
        <div class="sep" role="separator"></div>
      {:else}
        <button
          role="menuitem"
          class:danger={it.danger}
          class:active={active === i}
          disabled={it.disabled}
          onmouseenter={() => (active = i)}
          onclick={() => choose(i)}
        >
          <span>{it.label}</span>
          {#if it.hint}<span class="hint">{it.hint}</span>{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 60;
    min-width: 220px;
    max-width: 340px;
    padding: 5px;
    background: var(--bg-2);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-pop);
    animation: pop 120ms var(--ease-out);
  }

  button {
    width: 100%;
    height: 28px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    text-align: left;
    white-space: nowrap;
  }

  button span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  button.active:not(:disabled) {
    background: var(--accent);
    color: var(--on-color);
  }

  button.danger {
    color: var(--danger);
  }

  button.danger.active:not(:disabled) {
    background: var(--danger);
    color: var(--on-color);
  }

  button:disabled {
    color: var(--text-3);
    opacity: 0.6;
  }

  .hint {
    color: var(--text-2);
    font-size: 12px;
  }

  button.active .hint {
    color: inherit;
    opacity: 0.75;
  }

  .sep {
    height: 1px;
    margin: 5px 6px;
    background: var(--line-strong);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
</style>
