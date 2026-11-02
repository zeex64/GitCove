<script lang="ts">
  import { Search, GitBranch, GitCommitHorizontal, Zap, Cloud } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import { askNewBranch } from '../state/menus'
  import { github } from '../state/github.svelte'
  import { modKey, pickFolder } from '../api/system'
  import { relativeTime } from '../util/format'

  interface Item {
    id: string
    group: 'Actions' | 'Branches' | 'Commits'
    label: string
    hint?: string
    run: () => void
  }

  let query = $state('')
  let idx = $state(0)
  let input = $state<HTMLInputElement>()
  let list = $state<HTMLDivElement>()

  const repo = $derived(app.repo)

  const actions = $derived.by((): Item[] => {
    const r = repo
    const center = { x: window.innerWidth / 2 - 160, y: 120 }
    const items: (Item | false)[] = [
      { id: 'open', group: 'Actions', label: 'Open repository…', hint: `${modKey} O`, run: async () => { const p = await pickFolder('Open a repository'); if (p) { const e = await app.openPath(p); if (e) app.fail(e) } } },
      { id: 'tab', group: 'Actions', label: 'New tab', hint: `${modKey} T`, run: () => app.newTab() },
      { id: 'ghclone', group: 'Actions', label: github.account ? 'Clone from GitHub…' : 'Clone a repository…', run: () => app.cloneInNewTab(github.account ? 'github' : 'url') },
      !github.account && { id: 'ghin', group: 'Actions', label: 'Sign in to GitHub…', run: () => { app.accountOpen = true; if (github.status?.configured) github.startLogin() } },
      !!github.account && { id: 'ghout', group: 'Actions', label: `Sign out of GitHub (@${github.account?.login})`, run: () => github.signOut() },
      !!r && { id: 'fetch', group: 'Actions', label: 'Fetch all remotes', run: () => r.fetch(null) },
      !!r?.headBranch?.upstream && { id: 'pull', group: 'Actions', label: 'Pull', run: () => r.pull() },
      !!r?.headBranch && { id: 'push', group: 'Actions', label: 'Push', run: () => r.push() },
      !!r?.headBranch?.upstream && {
        id: 'force',
        group: 'Actions',
        label: 'Force push…',
        hint: 'preview first',
        run: () => r.startPreview({ kind: 'forcePush', remote: r.headBranch!.upstream!.split('/')[0], branch: r.headBranch!.name }),
      },
      !!r?.head?.commit && { id: 'branch', group: 'Actions', label: 'New branch…', run: () => askNewBranch(r, center, r.head!.commit!, r.head!.branch ?? 'HEAD') },
      !!r?.dirty && { id: 'stash', group: 'Actions', label: 'Stash all changes', run: () => r.stashPush() },
      !!r?.refs?.stashes.length && { id: 'pop', group: 'Actions', label: 'Pop latest stash', run: () => r.stashApply(0, true) },
      !!r?.unstaged.length && { id: 'stageall', group: 'Actions', label: 'Stage all changes', run: () => r.stageAll() },
      !!r?.staged.length && { id: 'unstageall', group: 'Actions', label: 'Unstage all changes', run: () => r.unstageAll() },
      !!r?.unstaged.length && { id: 'discardall', group: 'Actions', label: 'Discard all unstaged changes…', hint: 'preview first', run: () => r.startPreview({ kind: 'discard', paths: r.unstaged.map((f) => f.path) }) },
      !!r?.undoable && { id: 'undo', group: 'Actions', label: `Undo ${r.undoable.label}`, hint: `${modKey} Z`, run: () => r.undo() },
      !!r?.redoable && { id: 'redo', group: 'Actions', label: `Redo ${r.redoable.label}`, hint: `${modKey} Shift Z`, run: () => r.redo() },
      !!r && { id: 'term', group: 'Actions', label: 'Open terminal here', run: () => r.openTerminal() },
      !!r && { id: 'close', group: 'Actions', label: 'Close tab', hint: `${modKey} W`, run: () => app.closeTab(app.activeId!) },
    ]
    return items.filter((x): x is Item => !!x)
  })

  const results = $derived.by((): Item[] => {
    const q = query.trim().toLowerCase()
    const r = repo
    const out: Item[] = actions.filter((a) => !q || a.label.toLowerCase().includes(q))
    if (r) {
      const branches = [
        ...(r.refs?.branches ?? []).filter((b) => !b.isHead).map((b) => ({ name: b.name, remote: false })),
        ...(r.refs?.remoteBranches ?? []).filter((b) => b.name !== 'HEAD' && !r.refs?.branches.some((x) => x.name === b.name)).map((b) => ({ name: b.fullName, remote: true })),
      ]
      for (const b of branches) {
        if (q && !b.name.toLowerCase().includes(q)) continue
        out.push({ id: 'b:' + b.name, group: 'Branches', label: `Switch to ${b.name}`, hint: b.remote ? 'remote' : undefined, run: () => r.checkout(b.name) })
        if (!q && out.length > 24) break
      }
      if (q.length >= 2) {
        let n = 0
        for (const c of r.log) {
          if (c.summary.toLowerCase().includes(q) || c.id.startsWith(q) || c.authorName.toLowerCase().includes(q)) {
            out.push({ id: 'c:' + c.id, group: 'Commits', label: c.summary, hint: `${c.shortId} · ${relativeTime(c.authorTime)}`, run: () => r.select({ kind: 'commit', id: c.id }) })
            if (++n >= 30) break
          }
        }
      }
    }
    return out
  })

  $effect(() => {
    if (app.paletteOpen) {
      query = ''
      idx = 0
      queueMicrotask(() => input?.focus())
    }
  })

  $effect(() => {
    void query
    idx = 0
  })

  $effect(() => {
    list?.querySelector(`[data-i="${idx}"]`)?.scrollIntoView({ block: 'nearest' })
  })

  function run(it: Item) {
    app.paletteOpen = false
    it.run()
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') app.paletteOpen = false
    else if (e.key === 'ArrowDown') {
      e.preventDefault()
      idx = Math.min(results.length - 1, idx + 1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      idx = Math.max(0, idx - 1)
    } else if (e.key === 'Enter' && results[idx]) {
      e.preventDefault()
      run(results[idx])
    }
  }
</script>

{#if app.paletteOpen}
  <div class="scrim" role="presentation" onmousedown={(e) => e.target === e.currentTarget && (app.paletteOpen = false)}>
    <div class="palette" role="dialog" aria-label="Command palette">
      <label class="search">
        <Search size={16} />
        <input bind:this={input} bind:value={query} onkeydown={onKey} placeholder="Run a command, switch branch, or find a commit" spellcheck="false" aria-label="Search" />
        <span class="kbd">Esc</span>
      </label>
      <div class="results" bind:this={list} role="listbox">
        {#each results as it, i (it.id)}
          {#if i === 0 || results[i - 1].group !== it.group}<p class="group">{it.group}</p>{/if}
          <button class="res" class:active={i === idx} data-i={i} role="option" aria-selected={i === idx} onmouseenter={() => (idx = i)} onclick={() => run(it)}>
            {#if it.group === 'Actions'}<Zap size={14} />{:else if it.group === 'Branches'}{#if it.hint === 'remote'}<Cloud size={14} />{:else}<GitBranch size={14} />{/if}{:else}<GitCommitHorizontal size={14} />{/if}
            <span class="label">{it.label}</span>
            {#if it.hint}<span class="hint">{it.hint}</span>{/if}
          </button>
        {:else}
          <p class="empty">Nothing matches “{query}”.</p>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: color-mix(in srgb, var(--bg-0) 60%, transparent);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 90px;
    animation: fade 120ms var(--ease-out);
  }

  .palette {
    width: min(620px, calc(100vw - 48px));
    max-height: min(520px, calc(100vh - 140px));
    display: flex;
    flex-direction: column;
    background: var(--bg-2);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    animation: pop 150ms var(--ease-out);
  }

  .search {
    height: 48px;
    flex: none;
    padding: 0 14px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid var(--line-strong);
    color: var(--text-2);
  }

  .search input {
    flex: 1;
    border: 0;
    background: transparent;
    font-size: 14px;
    color: var(--text);
  }

  .search input::placeholder {
    color: var(--text-3);
  }

  .search input:focus-visible {
    outline: none;
  }

  .results {
    overflow-y: auto;
    padding: 4px 6px 8px;
  }

  .group {
    padding: 10px 10px 4px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-2);
  }

  .res {
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
  }

  .res :global(svg) {
    color: var(--text-2);
    flex: none;
  }

  .res.active {
    background: var(--accent);
    color: var(--on-color);
  }

  .res.active :global(svg),
  .res.active .hint {
    color: var(--on-color);
    opacity: 0.8;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .hint {
    font-size: 12px;
    color: var(--text-2);
    flex: none;
  }

  .empty {
    padding: 16px;
    color: var(--text-2);
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.99);
    }
  }
</style>
