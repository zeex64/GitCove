<script lang="ts">
  import { GitBranch, GitBranchPlus, Check, Cloud, Tag, GitPullRequest, GitPullRequestDraft, Plus, ArrowUp, ArrowDown } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import { github } from '../state/github.svelte'
  import { branchMenu, remoteBranchMenu, tagMenu, askNewBranch, hostName } from '../state/menus'
  import { copyText, openUrl } from '../api/system'
  import { relativeTime } from '../util/format'

  let { repo, onclose }: { repo: RepoStore; onclose: () => void } = $props()

  type Item = { key: string; kind: 'local' | 'remote' | 'tag'; name: string; commit: string; current?: boolean; ahead?: number; behind?: number }

  let view = $state<'branches' | 'prs'>('branches')
  let filter = $state('')
  let idx = $state(0)
  let newBtn = $state<HTMLButtonElement>()
  let remoteBtn = $state<HTMLButtonElement>()

  const q = $derived(filter.trim().toLowerCase())
  const match = (s: string) => !q || s.toLowerCase().includes(q)
  const prLists = $derived([...repo.prs.values()])
  const prCount = $derived(prLists.reduce((n, l) => n + l.items.length, 0))

  const sections = $derived.by(() => {
    const refs = repo.refs
    if (!refs) return []
    const def = repo.defaultBranch?.name
    const locals = [...refs.branches]
      .sort((a, b) => Number(b.isHead) - Number(a.isHead) || Number(b.name === def) - Number(a.name === def) || a.name.localeCompare(b.name))
      .filter((b) => match(b.name))
      .map<Item>((b) => ({ key: 'l:' + b.name, kind: 'local', name: b.name, commit: b.commit, current: b.isHead, ahead: b.ahead, behind: b.behind }))
    const localNames = new Set(refs.branches.map((b) => b.name))
    const remotes = refs.remoteBranches
      .filter((r) => r.name !== 'HEAD' && !localNames.has(r.name) && match(r.fullName))
      .map<Item>((r) => ({ key: 'r:' + r.fullName, kind: 'remote', name: r.fullName, commit: r.commit }))
    const tags = refs.tags.filter((t) => match(t.name)).map<Item>((t) => ({ key: 't:' + t.name, kind: 'tag', name: t.name, commit: t.commit }))
    return [
      { title: 'Local branches', items: locals },
      { title: 'Remote branches', items: remotes },
      { title: 'Tags', items: tags },
    ].filter((s) => s.items.length)
  })
  const flat = $derived(sections.flatMap((s) => s.items))

  $effect(() => {
    void filter
    void view
    idx = 0
  })

  function activate(it: Item) {
    onclose()
    if (it.kind === 'tag') {
      repo.tab = 'history'
      repo.select({ kind: 'commit', id: it.commit })
    } else if (!it.current) {
      repo.checkout(it.name)
    }
  }

  function itemMenu(e: MouseEvent, it: Item) {
    const at = { x: e.clientX, y: e.clientY }
    app.openMenu(e, it.kind === 'local' ? branchMenu(repo, it.name, at) : it.kind === 'remote' ? remoteBranchMenu(repo, it.name, at) : tagMenu(repo, it.name))
  }

  function onKey(e: KeyboardEvent) {
    if (view !== 'branches') return
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      idx = Math.min(flat.length - 1, idx + 1)
      document.getElementById('bm-' + idx)?.scrollIntoView({ block: 'nearest' })
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      idx = Math.max(0, idx - 1)
      document.getElementById('bm-' + idx)?.scrollIntoView({ block: 'nearest' })
    } else if (e.key === 'Enter' && flat[idx]) {
      e.preventDefault()
      activate(flat[idx])
    }
  }

  function newBranch() {
    if (!newBtn || !repo.head?.commit) return
    const anchor = newBtn
    onclose()
    askNewBranch(repo, anchor, repo.head.commit, repo.head.branch ?? repo.head.commit.slice(0, 7))
  }

  function addRemote() {
    if (!remoteBtn) return
    const anchor = remoteBtn
    onclose()
    app.ask(anchor, {
      title: 'Add a remote',
      fields: [
        { name: 'name', label: 'Name', value: repo.remotes.length ? '' : 'origin', placeholder: 'origin', mono: true },
        { name: 'url', label: 'URL', placeholder: 'git@github.com:you/repo.git', mono: true },
      ],
      confirm: 'Add remote',
      validate: (v) => (repo.remotes.some((r) => r.name === v.name) ? 'A remote with that name already exists.' : null),
      onSubmit: (v) => repo.addRemote(v.name, v.url),
    })
  }
</script>

<div class="bm">
  <div class="views" role="tablist" aria-label="Branch menu">
    <button role="tab" aria-selected={view === 'branches'} class:on={view === 'branches'} onclick={() => (view = 'branches')}>Branches</button>
    <button role="tab" aria-selected={view === 'prs'} class:on={view === 'prs'} onclick={() => (view = 'prs')}>
      Pull requests{#if prCount}<em class="num">{prCount}</em>{/if}
    </button>
  </div>

  {#if view === 'branches'}
    <div class="filter">
      <!-- svelte-ignore a11y_autofocus -->
      <input class="field" placeholder="Filter branches and tags" bind:value={filter} onkeydown={onKey} autofocus spellcheck="false" aria-label="Filter branches and tags" />
    </div>
    <div class="list" role="listbox" aria-label="Branches and tags">
      {#each sections as s (s.title)}
        <p class="sec section-label">{s.title}</p>
        {#each s.items as it (it.key)}
          {@const i = flat.indexOf(it)}
          <button
            id={'bm-' + i}
            class="item"
            class:active={i === idx}
            class:current={it.current}
            role="option"
            aria-selected={i === idx}
            onmouseenter={() => (idx = i)}
            onclick={() => activate(it)}
            oncontextmenu={(e) => itemMenu(e, it)}
            title={it.kind === 'tag' ? `Show ${it.name} in history` : it.current ? 'Current branch' : `Switch to ${it.name}`}
          >
            {#if it.current}<Check size={14} class="cur-ic" />{:else if it.kind === 'remote'}<Cloud size={14} />{:else if it.kind === 'tag'}<Tag size={14} />{:else}<GitBranch size={14} />{/if}
            <span class="name">{it.name}</span>
            {#if it.ahead}<span class="ab num" title="{it.ahead} ahead of upstream"><ArrowUp size={11} />{it.ahead}</span>{/if}
            {#if it.behind}<span class="ab num" title="{it.behind} behind upstream"><ArrowDown size={11} />{it.behind}</span>{/if}
          </button>
        {/each}
      {:else}
        <p class="empty">{q ? `Nothing matches “${filter}”.` : 'No branches yet. Make the first commit to create one.'}</p>
      {/each}
    </div>
  {:else}
    <div class="list prs">
      {#if !repo.remotes.some((r) => r.host !== 'other' && r.host !== 'bitbucket')}
        <p class="empty">Pull requests show up here for GitHub, GitLab, Forgejo and Gitea remotes.</p>
      {/if}
      {#each prLists as list (list.remote)}
        {#if prLists.length > 1}<p class="sec section-label">{list.remote} · {hostName(list.host)}</p>{/if}
        {#if list.status !== 'ok'}
          <p class="empty">{list.message}</p>
          {#if list.status === 'private' && list.host === 'github' && !github.account}
            <button class="btn sm ghost signin" onclick={() => { onclose(); app.accountOpen = true }}>Sign in to GitHub</button>
          {/if}
        {:else}
          {#each list.items as pr (pr.number)}
            <button
              class="item pr"
              title={`${pr.sourceBranch} → ${pr.targetBranch} · updated ${relativeTime(pr.updatedAt)}`}
              onclick={() => openUrl(pr.url)}
              oncontextmenu={(e) =>
                app.openMenu(e, [
                  { label: 'Open in browser', run: () => openUrl(pr.url) },
                  { label: `Check out ${pr.sourceBranch}`, run: () => { onclose(); repo.checkout(`${list.remote}/${pr.sourceBranch}`) } },
                  { label: 'Copy link', run: () => copyText(pr.url) },
                ])}
            >
              {#if pr.state === 'draft'}<GitPullRequestDraft size={14} />{:else}<GitPullRequest size={14} class="open" />{/if}
              <span class="pr-main">
                <span class="name">{pr.title}</span>
                <small>#{pr.number} · {pr.author} · {pr.sourceBranch}</small>
              </span>
            </button>
          {:else}
            <p class="empty">No open pull requests on {list.remote}.</p>
          {/each}
        {/if}
      {/each}
    </div>
  {/if}

  <div class="foot">
    <button class="btn sm ghost" bind:this={newBtn} disabled={!repo.head?.commit} onclick={newBranch}><GitBranchPlus size={14} />New branch…</button>
    <button class="btn sm quiet" bind:this={remoteBtn} onclick={addRemote}><Plus size={14} />Add remote…</button>
  </div>
</div>

<style>
  .bm {
    display: flex;
    flex-direction: column;
    max-height: min(560px, calc(100vh - 140px));
  }

  .views {
    flex: none;
    display: grid;
    grid-template-columns: 1fr 1fr;
    border-bottom: 1px solid var(--line-strong);
  }

  .views button {
    position: relative;
    height: 38px;
    border: 0;
    background: transparent;
    color: var(--text-2);
    font-weight: 500;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .views button:hover {
    color: var(--text);
  }

  .views button.on {
    color: var(--text);
    font-weight: 600;
  }

  .views button.on::after {
    content: '';
    position: absolute;
    left: 16px;
    right: 16px;
    bottom: -1px;
    height: 2px;
    border-radius: 999px;
    background: var(--accent);
  }

  .views em {
    font-style: normal;
    font-size: 11px;
    font-weight: 600;
    background: var(--bg-3);
    border-radius: 999px;
    padding: 0 6px;
    line-height: 17px;
  }

  .filter {
    flex: none;
    padding: 10px;
  }

  .filter .field {
    background: var(--bg-1);
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 5px 6px;
  }

  .list.prs {
    padding-top: 6px;
  }

  .sec {
    padding: 10px 9px 4px;
  }

  .item {
    width: 100%;
    min-height: 32px;
    padding: 5px 9px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    display: flex;
    align-items: center;
    gap: 9px;
    text-align: left;
  }

  .item :global(svg) {
    flex: none;
    color: var(--text-2);
  }

  .item :global(svg.cur-ic) {
    color: var(--success);
  }

  .item :global(svg.open) {
    color: var(--success);
  }

  .item.active {
    background: var(--bg-3);
  }

  .item.current .name {
    font-weight: 600;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ab {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    font-size: 11.5px;
    color: var(--text-2);
  }

  .item.pr:hover {
    background: var(--bg-3);
  }

  .pr-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .pr-main small {
    color: var(--text-2);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    padding: 10px 9px;
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .signin {
    margin: 0 9px 8px;
  }

  .foot {
    flex: none;
    padding: 8px 10px;
    border-top: 1px solid var(--line-strong);
    display: flex;
    gap: 6px;
  }
</style>
