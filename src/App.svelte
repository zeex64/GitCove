<script lang="ts">
  import { onMount } from 'svelte'
  import { GitBranch, TriangleAlert } from '@lucide/svelte'
  import { app } from './lib/state/app.svelte'
  import { dragState } from './lib/state/drag.svelte'
  import { onFolderDrop, pickFolder } from './lib/api/system'
  import TitleBar from './lib/components/TitleBar.svelte'
  import Toolbar from './lib/components/Toolbar.svelte'
  import LeftPane from './lib/components/LeftPane.svelte'
  import MainPane from './lib/components/MainPane.svelte'
  import Welcome from './lib/components/Welcome.svelte'
  import Resizer from './lib/components/Resizer.svelte'
  import Toasts from './lib/components/Toasts.svelte'
  import ContextMenu from './lib/components/ContextMenu.svelte'
  import Prompt from './lib/components/Prompt.svelte'
  import CommandPalette from './lib/components/CommandPalette.svelte'

  let width = $state(window.innerWidth)
  let dropping = $state(false)

  const repo = $derived(app.repo)
  /** The diff keeps at least 520px; the left column gives way first. */
  const leftMax = $derived(Math.max(280, Math.min(640, width - 520)))
  const leftW = $derived(Math.min(leftMax, app.panes.left))

  onMount(() => {
    app.start()
    let stop: (() => void) | undefined
    onFolderDrop(
      async (paths) => {
        for (const p of paths) {
          const e = await app.openPath(p)
          if (e) app.fail(e)
        }
      },
      (over) => (dropping = over),
    ).then((u) => (stop = u))
    return () => stop?.()
  })

  function inField(e: KeyboardEvent) {
    const t = e.target as HTMLElement
    return t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable
  }

  async function onKey(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey
    const k = e.key.toLowerCase()
    if (mod && k === 'k') {
      e.preventDefault()
      app.paletteOpen = !app.paletteOpen
    } else if (mod && e.key === 'Tab') {
      e.preventDefault()
      app.cycleTab(e.shiftKey ? -1 : 1)
    } else if (mod && k === 't') {
      e.preventDefault()
      app.newTab()
    } else if (mod && k === 'w') {
      e.preventDefault()
      if (app.activeId) app.closeTab(app.activeId)
    } else if (mod && k === 'o') {
      e.preventDefault()
      const p = await pickFolder('Open a repository')
      if (p) {
        const err = await app.openPath(p)
        if (err) app.fail(err)
      }
    } else if (mod && (k === '1' || k === '2') && repo) {
      e.preventDefault()
      repo.setTab(k === '1' ? 'changes' : 'history')
    } else if (mod && k === 'z' && !inField(e) && repo) {
      e.preventDefault()
      if (e.shiftKey) repo.redo()
      else repo.undo()
    } else if (e.key === 'Escape' && !app.paletteOpen && !app.menu && !app.prompt && repo) {
      if (repo.preview || repo.previewLoading || repo.previewError) repo.cancelPreview()
    } else if (e.key === 'F5' && repo) {
      e.preventDefault()
      repo.refreshAll()
    }
  }
</script>

<svelte:window bind:innerWidth={width} onkeydown={onKey} />

<div class="window">
  <TitleBar />

  {#if app.gitMissing}
    <div class="blocked">
      <TriangleAlert size={22} />
      <h1>Git isn’t available</h1>
      <p>{app.gitMissing.message}</p>
    </div>
  {:else if repo}
    {#key app.activeId}
      <Toolbar {repo} />
      <div class="main" style="grid-template-columns:{leftW}px 1px minmax(0,1fr)">
        <LeftPane {repo} />
        <Resizer bind:value={() => leftW, (v) => (app.panes.left = v)} min={280} max={leftMax} onend={() => app.persistPanes()} />
        <MainPane {repo} />
      </div>
    {/key}
  {:else}
    <Welcome />
  {/if}

  {#if dragState.active}
    <div class="drag-chip" style="left:{dragState.x + 14}px; top:{dragState.y + 10}px">
      <GitBranch size={12} />{dragState.label}
    </div>
  {/if}

  {#if dropping}
    <div class="drop" aria-hidden="true"><p>Drop a folder to open it</p></div>
  {/if}

  <Toasts />
  <ContextMenu />
  <Prompt />
  <CommandPalette />
</div>

<style>
  .window {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .main {
    flex: 1;
    min-height: 0;
    display: grid;
  }

  .blocked {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    text-align: center;
    padding: 24px;
    border-top: 1px solid var(--line);
  }

  .blocked :global(svg) {
    color: var(--warning);
  }

  .blocked h1 {
    font-size: 24px;
    font-weight: 600;
  }

  .blocked p {
    color: var(--text-2);
    max-width: 52ch;
  }

  .drag-chip {
    position: fixed;
    z-index: 80;
    pointer-events: none;
    height: 22px;
    padding: 0 8px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--on-color);
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 5px;
    box-shadow: var(--shadow-pop);
  }

  .drop {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(21, 22, 24, 0.75);
    border: 2px dashed var(--accent);
    border-radius: var(--radius-lg);
    margin: 6px;
    pointer-events: none;
  }

  .drop p {
    font-size: 15px;
    font-weight: 600;
  }

  :global(body.dragging) {
    cursor: grabbing;
  }
</style>
