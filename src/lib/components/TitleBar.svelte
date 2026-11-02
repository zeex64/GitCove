<script lang="ts">
  import { Plus, X, Minus, Square, Copy } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import { isMac, startWindowDrag, windowAction } from '../api/system'
  import AccountButton from './AccountButton.svelte'

  let maximized = $state(false)

  function onStripMouseDown(e: MouseEvent) {
    if (e.button !== 0 || e.target !== e.currentTarget) return
    if (e.detail === 2) {
      windowAction('toggleMaximize')
      maximized = !maximized
    } else startWindowDrag()
  }

  function tabMenu(e: MouseEvent, id: string) {
    const tab = app.tabs.find((t) => t.id === id)
    app.openMenu(e, [
      { label: 'Close tab', hint: 'Ctrl W', run: () => app.closeTab(id) },
      {
        label: 'Close other tabs',
        disabled: app.tabs.length < 2,
        run: () => app.tabs.filter((t) => t.id !== id).forEach((t) => app.closeTab(t.id)),
      },
      { separator: true, label: '' },
      { label: 'Open terminal here', disabled: !tab?.repo, run: () => tab?.repo?.openTerminal() },
    ])
  }
</script>

<header class="strip" class:mac={isMac} onmousedown={onStripMouseDown} role="toolbar" tabindex="-1" aria-label="Repositories">
  {#each app.tabs as tab (tab.id)}
    {@const active = tab.id === app.activeId}
    <div
      class="tab"
      class:active
      role="tab"
      tabindex="0"
      aria-selected={active}
      onmousedown={(e) => {
        if (e.button === 1) {
          e.preventDefault()
          app.closeTab(tab.id)
        } else if (e.button === 0) app.activate(tab.id)
      }}
      onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && app.activate(tab.id)}
      oncontextmenu={(e) => tabMenu(e, tab.id)}
      title={tab.repo?.path ?? 'New tab'}
    >
      {#if tab.repo?.dirty}<span class="dot" aria-label="Uncommitted changes"></span>{/if}
      <span class="name">{tab.repo?.name ?? 'New tab'}</span>
      <button
        class="close"
        aria-label="Close tab"
        onmousedown={(e) => e.stopPropagation()}
        onclick={(e) => {
          e.stopPropagation()
          app.closeTab(tab.id)
        }}><X size={13} /></button
      >
    </div>
  {/each}
  <button class="icon-btn add" aria-label="New tab" title="New tab (Ctrl T)" onclick={() => app.newTab()}><Plus size={15} /></button>

  <AccountButton />

  {#if !isMac}
    <div class="controls">
      <button aria-label="Minimize" onclick={() => windowAction('minimize')}><Minus size={15} /></button>
      <button
        aria-label={maximized ? 'Restore' : 'Maximize'}
        onclick={() => {
          windowAction('toggleMaximize')
          maximized = !maximized
        }}
      >
        {#if maximized}<Copy size={12} />{:else}<Square size={12} />{/if}
      </button>
      <button class="close-win" aria-label="Close window" onclick={() => windowAction('close')}><X size={16} /></button>
    </div>
  {/if}
</header>

<style>
  .strip {
    height: 38px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 0 0 10px;
    background: var(--bg-0);
  }

  .strip.mac {
    padding-left: 80px;
    padding-right: 8px;
  }

  .tab {
    height: 28px;
    max-width: 220px;
    min-width: 96px;
    padding: 0 6px 0 12px;
    border-radius: var(--radius);
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
    transition:
      background-color 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .tab:hover {
    background: var(--bg-1);
    color: var(--text);
  }

  .tab.active {
    background: var(--bg-2);
    color: var(--text);
  }

  .name {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-weight: 500;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--warning);
    flex: none;
  }

  .close {
    width: 20px;
    height: 20px;
    border: 0;
    padding: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    flex: none;
  }

  .tab:hover .close,
  .tab.active .close {
    opacity: 1;
  }

  .close:hover {
    background: var(--bg-4);
    color: var(--text);
  }

  .add {
    width: 28px;
    height: 28px;
  }

  .controls {
    margin-left: 6px;
    display: flex;
    height: 100%;
  }

  .controls button {
    width: 46px;
    height: 100%;
    border: 0;
    background: transparent;
    color: var(--text-2);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .controls button:hover {
    background: var(--bg-2);
    color: var(--text);
  }

  /* Window-close red is the desktop convention, not a state colour. */
  .controls .close-win:hover {
    background: #e5484d;
    color: var(--text);
  }
</style>
