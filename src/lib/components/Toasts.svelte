<script lang="ts">
  import { CircleCheck, CircleAlert, Info, X } from '@lucide/svelte'
  import { app } from '../state/app.svelte'

  let open = $state<Record<number, boolean>>({})
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}" role={t.kind === 'error' ? 'alert' : 'status'}>
      <span class="icon">
        {#if t.kind === 'success'}<CircleCheck size={16} />{:else if t.kind === 'error'}<CircleAlert size={16} />{:else}<Info size={16} />{/if}
      </span>
      <div class="body">
        <p>{t.message}</p>
        {#if t.detail && t.detail !== t.message}
          <button class="link" onclick={() => (open[t.id] = !open[t.id])}>{open[t.id] ? 'Hide details' : 'Show details'}</button>
          {#if open[t.id]}<pre class="mono selectable">{t.detail}</pre>{/if}
        {/if}
      </div>
      {#if t.action}
        <button
          class="btn sm ghost"
          onclick={() => {
            t.action?.run()
            app.dismiss(t.id)
          }}>{t.action.label}</button
        >
      {/if}
      <button class="icon-btn" aria-label="Dismiss" onclick={() => app.dismiss(t.id)}><X size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 38px;
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-end;
    pointer-events: none;
  }

  .toast {
    pointer-events: auto;
    width: 380px;
    padding: 10px 8px 10px 12px;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background: var(--bg-2);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
    animation: rise 200ms var(--ease-out);
  }

  .icon {
    display: flex;
    padding-top: 1px;
  }

  .success .icon {
    color: var(--success);
  }

  .error .icon {
    color: var(--danger);
  }

  .info .icon {
    color: var(--accent);
  }

  .body {
    flex: 1;
    min-width: 0;
    padding-top: 1px;
  }

  p {
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .link {
    margin-top: 4px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-size: 12px;
  }

  .link:hover {
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  pre {
    margin: 6px 0 0;
    max-height: 140px;
    overflow: auto;
    padding: 8px;
    background: var(--bg-0);
    border-radius: var(--radius-sm);
    white-space: pre-wrap;
    color: var(--text-2);
    font-size: 11.5px;
  }

  .btn.sm {
    margin-top: -1px;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
