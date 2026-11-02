<script lang="ts">
  import { onMount } from 'svelte'
  import { FolderOpen, Download, FolderPlus, FolderGit2, X, TriangleAlert } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import { github } from '../state/github.svelte'
  import { backend } from '../api'
  import type { GitError, GithubRepo } from '../api/types'
  import { pickFolder } from '../api/system'
  import { newOpId, relativeTime, repoNameFromUrl } from '../util/format'
  import RepoPicker from './RepoPicker.svelte'
  import GithubPanel from './GithubPanel.svelte'

  const KEY_PARENT = 'gitcove.cloneParent'

  let error = $state<{ err: GitError; path: string } | null>(null)
  let cloning = $state(false)
  let source = $state<'github' | 'url'>('github')
  let selected = $state<GithubRepo | null>(null)
  let useSsh = $state(false)
  let url = $state('')
  let parent = $state('')
  let folder = $state('')
  let folderTouched = $state(false)
  let progress = $state<{ phase: string; percent: number | null } | null>(null)
  let cloneError = $state<GitError | null>(null)
  let opId = ''

  const cloneUrl = $derived(source === 'github' ? (selected ? (useSsh ? selected.sshUrl : selected.cloneUrl) : '') : url.trim())
  const ready = $derived(!!cloneUrl && !!parent && !!folder.trim() && !progress)
  const showDestination = $derived(source === 'url' || (!!github.account && !!selected))

  // "Clone a repository…" from the account menu or palette opens the form, even if it's already showing.
  $effect(() => {
    const want = app.welcomeClone
    if (!want) return
    cloning = true
    source = want
    app.welcomeClone = null
  })

  onMount(async () => {
    try {
      parent = localStorage.getItem(KEY_PARENT) ?? ''
    } catch {
      parent = ''
    }
    if (!parent) parent = await backend.defaultCloneDir().catch(() => '')
  })

  // A new pick or URL suggests its folder name until the user types their own.
  $effect(() => {
    if (folderTouched) return
    folder = source === 'github' ? (selected?.name ?? '') : repoNameFromUrl(url)
  })

  $effect(() => {
    let stop: (() => void) | undefined
    backend.onProgress((e) => {
      if (e.opId === opId) progress = { phase: e.phase, percent: e.percent }
    }).then((u) => (stop = u))
    return () => stop?.()
  })

  function openClone(to: 'github' | 'url') {
    cloning = true
    source = to
    cloneError = null
  }

  async function open(path?: string) {
    const p = path ?? (await pickFolder('Open a repository'))
    if (!p) return
    const e = await app.openPath(p)
    error = e ? { err: e, path: p } : null
  }

  async function init(path?: string) {
    const p = path ?? (await pickFolder('Choose a folder for the new repository'))
    if (!p) return
    try {
      const info = await backend.initRepo(p)
      error = null
      await app.openPath(info.path)
    } catch (e) {
      app.fail(e)
    }
  }

  async function chooseParent() {
    const p = await pickFolder('Clone into which folder?')
    if (p) parent = p
  }

  async function clone() {
    if (!ready) return
    opId = newOpId()
    progress = { phase: 'Connecting', percent: null }
    cloneError = null
    try {
      const info = await backend.cloneRepo(cloneUrl, `${parent.replace(/\/+$/, '')}/${folder.trim()}`, opId)
      try {
        localStorage.setItem(KEY_PARENT, parent)
      } catch {
        /* remembering the folder is a convenience */
      }
      cloning = false
      url = ''
      selected = null
      folderTouched = false
      await app.openPath(info.path)
    } catch (err) {
      cloneError = err as GitError
    } finally {
      progress = null
    }
  }
</script>

<div class="welcome">
  <div class="inner">
    <h1>Open a repository</h1>
    <p class="lede">Pick a folder that’s already a Git repository, clone one, or start a new one. You can also drop a folder onto this window.</p>

    <div class="actions">
      <button class="btn primary big" onclick={() => open()}><FolderOpen size={16} />Open folder…</button>
      <button class="btn big" onclick={() => (cloning ? (cloning = false) : openClone(github.account ? 'github' : 'url'))} aria-expanded={cloning}>
        <Download size={16} />Clone…
      </button>
      <button class="btn big" onclick={() => init()}><FolderPlus size={16} />New repository…</button>
    </div>

    {#if error}
      <div class="error" role="alert">
        <TriangleAlert size={16} />
        <div>
          <p>{error.err.message}</p>
          <p class="muted mono">{error.path}</p>
        </div>
        {#if error.err.kind === 'notARepo'}
          <button class="btn sm ghost" onclick={() => init(error!.path)}>Create a repository here</button>
        {/if}
      </div>
    {/if}

    {#if cloning}
      <section class="clone" aria-label="Clone a repository">
        <div class="tabs" role="tablist" aria-label="Clone from">
          <button role="tab" aria-selected={source === 'github'} class:on={source === 'github'} onclick={() => (source = 'github')}>
            Your GitHub repositories
          </button>
          <button role="tab" aria-selected={source === 'url'} class:on={source === 'url'} onclick={() => (source = 'url')}>From a URL</button>
        </div>

        {#if source === 'github'}
          {#if github.account}
            <RepoPicker bind:selected />
            <label class="check">
              <input type="checkbox" bind:checked={useSsh} />
              <span>Clone over SSH, using your SSH key instead of the GitHub sign-in</span>
            </label>
          {:else}
            <div class="signin"><GithubPanel onclone={() => (source = 'github')} /></div>
          {/if}
        {:else}
          <label>
            <span>Repository URL</span>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="field mono" bind:value={url} placeholder="https://github.com/owner/repo.git" spellcheck="false" autofocus />
          </label>
        {/if}

        {#if showDestination}
          <div class="row">
            <label class="grow">
              <span>Clone into</span>
              <button type="button" class="field pick" onclick={chooseParent} title={parent}>{parent || 'Choose a folder…'}</button>
            </label>
            <label>
              <span>Folder name</span>
              <input class="field mono" bind:value={folder} oninput={() => (folderTouched = true)} spellcheck="false" />
            </label>
          </div>
          {#if cloneError}<p class="clone-error" role="alert">{cloneError.message}</p>{/if}
          {#if progress}
            <div class="progress"><span>{progress.phase}{progress.percent != null ? ` ${progress.percent}%` : '…'}</span><i style="--p:{progress.percent ?? 15}%"></i></div>
          {/if}
          <div class="clone-actions">
            <button type="button" class="btn quiet" onclick={() => (cloning = false)}>Cancel</button>
            <button class="btn primary" disabled={!ready} onclick={clone}>
              {source === 'github' && selected ? `Clone ${selected.fullName}` : 'Clone'}
            </button>
          </div>
        {/if}
      </section>
    {/if}

    {#if app.recent.length}
      <h2>Recent</h2>
      <ul class="recent">
        {#each app.recent as r (r.path)}
          <li>
            <button class="recent-item" onclick={() => open(r.path)}>
              <FolderGit2 size={16} />
              <span class="r-main"><span class="r-name">{r.name}</span><span class="r-path">{r.path}</span></span>
              <span class="r-time">{relativeTime(r.openedAt / 1000)}</span>
            </button>
            <button class="icon-btn" aria-label="Remove from recent" title="Remove from recent" onclick={() => app.forget(r.path)}><X size={14} /></button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .welcome {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    justify-content: center;
    background: var(--bg-0);
    border-top: 1px solid var(--line);
  }

  .inner {
    width: min(640px, calc(100% - 48px));
    padding: 10vh 0 48px;
  }

  h1 {
    font-size: 24px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }

  .lede {
    margin-top: 8px;
    color: var(--text-2);
    font-size: 14px;
    line-height: 1.5;
    max-width: 60ch;
  }

  .actions {
    margin-top: 24px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .big {
    height: 38px;
    padding: 0 16px;
    border-radius: var(--radius-md);
  }

  .error {
    margin-top: 16px;
    padding: 12px;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    border: 1px solid rgba(242, 193, 78, 0.4);
    border-radius: var(--radius-lg);
    background: var(--warning-soft);
  }

  .error :global(svg) {
    color: var(--warning);
    flex: none;
    margin-top: 1px;
  }

  .error div {
    flex: 1;
    min-width: 0;
  }

  .muted {
    color: var(--text-2);
  }

  .clone {
    margin-top: 16px;
    padding: 14px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-lg);
    background: var(--bg-1);
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 2px;
    align-self: flex-start;
    border-radius: var(--radius);
    background: var(--bg-0);
    border: 1px solid var(--line-strong);
  }

  .tabs button {
    height: 26px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-weight: 500;
  }

  .tabs button.on {
    background: var(--bg-3);
    color: var(--text);
  }

  .tabs button:hover:not(.on) {
    color: var(--text);
  }

  .signin {
    padding: 4px 2px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label > span {
    font-size: 12px;
    color: var(--text-2);
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .check input {
    margin: 0;
    accent-color: var(--accent);
  }

  .check span {
    font-size: 12.5px;
  }

  .row {
    display: flex;
    gap: 10px;
  }

  .grow {
    flex: 1;
    min-width: 0;
  }

  .pick {
    text-align: left;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text);
  }

  .clone-error {
    color: var(--danger);
  }

  .progress {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-2);
    font-size: 12px;
  }

  .progress i {
    position: relative;
    flex: 1;
    height: 3px;
    border-radius: 999px;
    background: var(--bg-3);
    overflow: hidden;
  }

  .progress i::after {
    content: '';
    position: absolute;
    inset: 0;
    width: var(--p);
    background: var(--accent);
    transition: width 200ms var(--ease-out);
  }

  .clone-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  h2 {
    margin-top: 40px;
    margin-bottom: 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }

  .recent {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .recent li {
    display: flex;
    align-items: center;
    gap: 4px;
    border-radius: var(--radius);
  }

  .recent li:hover {
    background: var(--bg-1);
  }

  .recent-item {
    flex: 1;
    min-width: 0;
    height: 48px;
    padding: 0 12px;
    border: 0;
    background: transparent;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
  }

  .recent-item :global(svg) {
    color: var(--text-2);
    flex: none;
  }

  .r-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .r-name {
    font-weight: 500;
  }

  .r-path {
    font-size: 12px;
    color: var(--text-2);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .r-time {
    font-size: 12px;
    color: var(--text-2);
    flex: none;
  }

  .recent li .icon-btn {
    margin-right: 8px;
    opacity: 0;
  }

  .recent li:hover .icon-btn {
    opacity: 1;
  }
</style>
