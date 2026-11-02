<script lang="ts">
  import { Check, Minus } from '@lucide/svelte'

  let {
    checked,
    label,
    text,
    disabled = false,
    title,
    onchange,
  }: {
    checked: boolean | 'mixed'
    label: string
    /** A visible label beside the box; clicking it toggles too. */
    text?: string
    disabled?: boolean
    title?: string
    onchange: () => void
  } = $props()
</script>

<button
  class="cbw"
  class:on={checked !== false}
  class:labelled={!!text}
  role="checkbox"
  aria-checked={checked === 'mixed' ? 'mixed' : checked}
  aria-label={text ? undefined : label}
  title={title ?? (text ? undefined : label)}
  {disabled}
  onclick={(e) => {
    e.stopPropagation()
    onchange()
  }}
  onmousedown={(e) => e.stopPropagation()}
>
  <span class="cb">
    {#if checked === true}<Check size={12} strokeWidth={3.2} />{:else if checked === 'mixed'}<Minus size={12} strokeWidth={3.2} />{/if}
  </span>
  {#if text}<span class="text">{text}</span>{/if}
</button>

<style>
  .cbw {
    flex: none;
    padding: 0;
    border: 0;
    background: none;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
  }

  .cb {
    width: 16px;
    height: 16px;
    flex: none;
    border: 1.5px solid var(--line-strong);
    border-radius: var(--radius-sm);
    background: var(--bg-0);
    color: var(--on-color);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition:
      background-color 120ms var(--ease-out),
      border-color 120ms var(--ease-out);
  }

  .cbw:hover:not(:disabled) .cb {
    border-color: var(--text-3);
  }

  .cbw:hover:not(:disabled) .text {
    color: var(--text);
  }

  .on .cb {
    border-color: var(--accent);
    background: var(--accent);
  }

  .on:hover:not(:disabled) .cb {
    border-color: var(--accent-hover);
    background: var(--accent-hover);
  }

  .cbw:disabled {
    opacity: 0.4;
  }

  .cbw:focus-visible {
    outline: none;
  }

  .cbw:focus-visible .cb {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .text {
    transition: color 120ms var(--ease-out);
  }
</style>
