<script lang="ts">
  import { WrapText } from '@lucide/svelte'
  import { app } from '../state/app.svelte'

  function set(split: boolean) {
    app.panes.split = split
    app.persistPanes()
  }
</script>

<div class="opts">
  {#if !app.panes.split}
    <button
      class="icon-btn"
      class:on={app.panes.wrap}
      title={app.panes.wrap ? 'Don’t wrap long lines' : 'Wrap long lines'}
      aria-label="Wrap long lines"
      aria-pressed={app.panes.wrap}
      onclick={() => {
        app.panes.wrap = !app.panes.wrap
        app.persistPanes()
      }}
    ><WrapText size={14} /></button>
  {/if}
  <div class="seg" role="radiogroup" aria-label="Diff layout">
    <button role="radio" aria-checked={!app.panes.split} class:on={!app.panes.split} onclick={() => set(false)}>Unified</button>
    <button role="radio" aria-checked={app.panes.split} class:on={app.panes.split} onclick={() => set(true)}>Side by side</button>
  </div>
</div>

<style>
  .opts {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }

  .icon-btn.on {
    color: var(--accent);
  }

  .seg {
    height: 26px;
    padding: 2px;
    display: flex;
    gap: 2px;
    border-radius: var(--radius);
    background: var(--bg-0);
    border: 1px solid var(--line);
  }

  .seg button {
    padding: 0 9px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
    transition:
      background-color 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .seg button:hover:not(.on) {
    color: var(--text);
  }

  .seg button.on {
    background: var(--bg-3);
    color: var(--text);
  }
</style>
