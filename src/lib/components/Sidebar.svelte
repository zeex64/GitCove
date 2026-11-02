<script lang="ts">
  import {
    ChevronDown,
    GitBranch,
    Check,
    Cloud,
    GitPullRequest,
    GitPullRequestDraft,
    Tag,
    Archive,
    Plus,
    ArrowUp,
    ArrowDown,
    Search,
    PanelLeftClose,
    PanelLeft,
    Globe,
  } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import { beginBranchDrag } from '../state/drag.svelte'
  import { branchMenu, remoteBranchMenu, tagMenu, stashMenu, askNewBranch, hostName } from '../state/menus'
  import { copyText, openUrl } from '../api/system'
  import { middleTruncate, relativeTime } from '../util/format'
  import { github } from '../state/github.svelte'

  let { repo, collapsed = false }: { repo: RepoStore; collapsed?: boolean } = $props()

  type Section = 'local' | 'remote' | 'prs' | 'tags' | 'stashes'
  let open = $state<Record<Section, boolean>>({ local: true, remote: true, prs: true, tags: false, stashes: true })
  let remoteOpen = $state<Record<string, boolean>>({})
  let filter = $state('')
  let addLocal = $state<HTMLButtonElement>()
  let addRemote = $state<HTMLButtonElement>()

  const q = $derived(filter.trim().toLowerCase())
  const match = (s: string) => !q || s.toLowerCase().includes(q)
  const refs = $derived(repo.refs)
  const locals = $derived((refs?.branches ?? []).filter((b) => match(b.name)))
  const tags = $derived((refs?.tags ?? []).filter((t) => match(t.name)))
  const stashes = $derived((refs?.stashes ?? []).filter((s) => match(s.message)))
  const remotes = $derived(
    (refs?.remotes ?? []).map((r) => ({
      ...r,
      branches: (refs?.remoteBranches ?? []).filter((b) => b.remote === r.name && b.name !== 'HEAD' && match(b.fullName)),
    })),
  )
  const prCount = $derived([...repo.prs.values()].reduce((n, l) => n + l.items.length, 0))

  function selectCommit(id: string) {
    repo.select({ kind: 'commit', id })
  }

  function toggle(s: Section) {
    open[s] = !open[s]
  }

  function addRemoteForm() {
    if (!addRemote) return
    app.ask(addRemote, {
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

  function remoteMenu(e: MouseEvent, name: string, url: string, web: string | null) {
    app.openMenu(e, [
      { label: `Fetch ${name}`, run: () => repo.fetch(name) },
      { label: 'Copy URL', run: () => copyText(url) },
      { label: 'Open in browser', disabled: !web, run: () => web && openUrl(web) },
    ])
  }
</script>

{#if collapsed}
  <nav class="rail" aria-label="Repository sections">
    <button class="icon-btn" title="Show sidebar" aria-label="Show sidebar" onclick={() => { app.panes.sidebarCollapsed = false; app.persistPanes() }}><PanelLeft size={16} /></button>
    <span class="rail-sep"></span>
    {#each [{ s: 'local', icon: GitBranch, n: refs?.branches.length ?? 0, t: 'Local branches' }, { s: 'remote', icon: Cloud, n: refs?.remotes.length ?? 0, t: 'Remotes' }, { s: 'prs', icon: GitPullRequest, n: prCount, t: 'Pull requests' }, { s: 'tags', icon: Tag, n: refs?.tags.length ?? 0, t: 'Tags' }, { s: 'stashes', icon: Archive, n: refs?.stashes.length ?? 0, t: 'Stashes' }] as item (item.s)}
      <button class="rail-btn" title={item.t} aria-label={item.t} onclick={() => { app.panes.sidebarCollapsed = false; open[item.s as Section] = true; app.persistPanes() }}>
        <item.icon size={17} />
        {#if item.n}<em class="num">{item.n}</em>{/if}
      </button>
    {/each}
  </nav>
{:else}
  <aside class="sidebar" aria-label="Branches and remotes">
    <div class="top">
      <label class="filter">
        <Search size={13} />
        <input placeholder="Filter" bind:value={filter} spellcheck="false" aria-label="Filter branches, tags and stashes" />
      </label>
      <button class="icon-btn" title="Collapse sidebar" aria-label="Collapse sidebar" onclick={() => { app.panes.sidebarCollapsed = true; app.persistPanes() }}><PanelLeftClose size={15} /></button>
    </div>

    <div class="scroll">
      <!-- Local -->
      <div class="sec">
        <button class="sec-btn" onclick={() => toggle('local')} aria-expanded={open.local}>
          <ChevronDown size={13} class={open.local ? '' : 'closed'} /><span class="section-label">Local</span>
        </button>
        <span class="count num">{refs?.branches.length ?? 0}</span>
        <button class="icon-btn add" bind:this={addLocal} title="New branch" aria-label="New branch" disabled={!repo.head?.commit}
          onclick={() => addLocal && repo.head?.commit && askNewBranch(repo, addLocal, repo.head.commit, repo.head.branch ?? repo.head.commit.slice(0, 7))}><Plus size={14} /></button>
      </div>
      {#if open.local}
        {#each locals as b (b.name)}
          <div
            class="item"
            class:head={b.isHead}
            role="button"
            tabindex="0"
            title={b.upstream ? `${b.name} → ${b.upstream}` : `${b.name} (no upstream)`}
            onclick={() => selectCommit(b.commit)}
            ondblclick={() => !b.isHead && repo.checkout(b.name)}
            onmousedown={(e) => beginBranchDrag(e, repo, b.name)}
            onkeydown={(e) => e.key === 'Enter' && !b.isHead && repo.checkout(b.name)}
            oncontextmenu={(e) => app.openMenu(e, branchMenu(repo, b.name, { x: e.clientX, y: e.clientY }))}
          >
            {#if b.isHead}<Check size={14} class="ic head-ic" />{:else}<GitBranch size={14} class="ic" />{/if}
            <span class="name">{middleTruncate(b.name, 30)}</span>
            <span class="track num">
              {#if b.upstreamGone}<em class="gone">gone</em>{/if}
              {#if b.ahead}<em><ArrowUp size={11} />{b.ahead}</em>{/if}
              {#if b.behind}<em><ArrowDown size={11} />{b.behind}</em>{/if}
            </span>
          </div>
        {:else}
          <p class="none">{q ? 'No branches match.' : repo.head?.unborn ? `${repo.head.branch ?? 'main'} appears after the first commit.` : 'No local branches.'}</p>
        {/each}
      {/if}

      <!-- Remotes -->
      <div class="sec">
        <button class="sec-btn" onclick={() => toggle('remote')} aria-expanded={open.remote}>
          <ChevronDown size={13} class={open.remote ? '' : 'closed'} /><span class="section-label">Remote</span>
        </button>
        <span class="count num">{refs?.remotes.length ?? 0}</span>
        <button class="icon-btn add" bind:this={addRemote} title="Add remote" aria-label="Add remote" onclick={addRemoteForm}><Plus size={14} /></button>
      </div>
      {#if open.remote}
        {#each remotes as r (r.name)}
          {@const isOpen = remoteOpen[r.name] ?? r.name === (repo.primaryRemote?.name ?? '')}
          <div
            class="item remote"
            role="button"
            tabindex="0"
            title={r.fetchUrl}
            onclick={() => (remoteOpen[r.name] = !isOpen)}
            onkeydown={(e) => e.key === 'Enter' && (remoteOpen[r.name] = !isOpen)}
            oncontextmenu={(e) => remoteMenu(e, r.name, r.fetchUrl, r.webUrl)}
          >
            <ChevronDown size={13} class={isOpen ? 'ic' : 'ic closed'} />
            <span class="name">{r.name}</span>
            <span class="host">{r.host === 'other' ? '' : hostName(r.host)}</span>
          </div>
          {#if isOpen}
            {#each r.branches as rb (rb.fullName)}
              <div
                class="item nested"
                role="button"
                tabindex="0"
                title={rb.fullName}
                onclick={() => selectCommit(rb.commit)}
                ondblclick={() => repo.checkout(rb.fullName)}
                onkeydown={(e) => e.key === 'Enter' && repo.checkout(rb.fullName)}
                oncontextmenu={(e) => app.openMenu(e, remoteBranchMenu(repo, rb.fullName, { x: e.clientX, y: e.clientY }))}
              >
                <Cloud size={13} class="ic" />
                <span class="name">{middleTruncate(rb.name, 28)}</span>
              </div>
            {:else}
              <p class="none nested-none">{q ? 'No branches match.' : 'No branches fetched yet.'}</p>
            {/each}
          {/if}
        {:else}
          <p class="none">No remotes yet. Add one to push and pull.</p>
        {/each}
      {/if}

      <!-- Pull requests -->
      <div class="sec">
        <button class="sec-btn" onclick={() => toggle('prs')} aria-expanded={open.prs}>
          <ChevronDown size={13} class={open.prs ? '' : 'closed'} /><span class="section-label">Pull requests</span>
        </button>
        <span class="count num">{prCount}</span>
      </div>
      {#if open.prs}
        {#if !repo.remotes.some((r) => r.host !== 'other' && r.host !== 'bitbucket')}
          <p class="none">Pull requests appear here for GitHub, GitLab, Forgejo and Gitea remotes.</p>
        {/if}
        {#each [...repo.prs.values()] as list (list.remote)}
          {#if repo.prs.size > 1}<p class="sub-label">{list.remote} · {hostName(list.host)}</p>{/if}
          {#if list.status !== 'ok'}
            <p class="none">{list.message}</p>
            {#if list.status === 'private' && list.host === 'github' && !github.account}
              <button class="btn sm ghost signin" onclick={() => (app.accountOpen = true)}>Sign in to GitHub</button>
            {/if}
          {:else}
            {#each list.items as pr (pr.number)}
              <div
                class="item pr"
                role="button"
                tabindex="0"
                title={`${pr.title}\n${pr.sourceBranch} → ${pr.targetBranch} · ${pr.author} · updated ${relativeTime(pr.updatedAt)}`}
                onclick={() => openUrl(pr.url)}
                onkeydown={(e) => e.key === 'Enter' && openUrl(pr.url)}
                oncontextmenu={(e) =>
                  app.openMenu(e, [
                    { label: 'Open in browser', run: () => openUrl(pr.url) },
                    { label: `Check out ${pr.sourceBranch}`, run: () => repo.checkout(`${list.remote}/${pr.sourceBranch}`) },
                    { label: 'Copy link', run: () => copyText(pr.url) },
                  ])}
              >
                {#if pr.state === 'draft'}<GitPullRequestDraft size={14} class="ic draft" />{:else}<GitPullRequest size={14} class="ic open" />{/if}
                <span class="name">{pr.title}</span>
                <span class="host num">#{pr.number}</span>
              </div>
            {:else}
              <p class="none">No open pull requests on {list.remote}.</p>
            {/each}
          {/if}
        {/each}
      {/if}

      <!-- Tags -->
      <div class="sec">
        <button class="sec-btn" onclick={() => toggle('tags')} aria-expanded={open.tags}>
          <ChevronDown size={13} class={open.tags ? '' : 'closed'} /><span class="section-label">Tags</span>
        </button>
        <span class="count num">{refs?.tags.length ?? 0}</span>
      </div>
      {#if open.tags}
        {#each tags as t (t.name)}
          <div class="item" role="button" tabindex="0" onclick={() => selectCommit(t.commit)} onkeydown={(e) => e.key === 'Enter' && selectCommit(t.commit)} oncontextmenu={(e) => app.openMenu(e, tagMenu(repo, t.name))}>
            <Tag size={13} class="ic" /><span class="name">{t.name}</span>
          </div>
        {:else}
          <p class="none">{q ? 'No tags match.' : 'No tags.'}</p>
        {/each}
      {/if}

      <!-- Stashes -->
      <div class="sec">
        <button class="sec-btn" onclick={() => toggle('stashes')} aria-expanded={open.stashes}>
          <ChevronDown size={13} class={open.stashes ? '' : 'closed'} /><span class="section-label">Stashes</span>
        </button>
        <span class="count num">{refs?.stashes.length ?? 0}</span>
      </div>
      {#if open.stashes}
        {#each stashes as s (s.commit)}
          <div
            class="item stash"
            role="button"
            tabindex="0"
            title={`${s.message}\n${relativeTime(s.time)} · double-click to pop`}
            ondblclick={() => repo.stashApply(s.index, true)}
            onclick={(e) => app.openMenu(e, stashMenu(repo, s))}
            onkeydown={(e) => e.key === 'Enter' && repo.stashApply(s.index, true)}
            oncontextmenu={(e) => app.openMenu(e, stashMenu(repo, s))}
          >
            <Archive size={13} class="ic" />
            <span class="name">{s.message.replace(/^(WIP on|On) [^:]+: /, '')}</span>
            <span class="host num">{relativeTime(s.time).replace(' ago', '')}</span>
          </div>
        {:else}
          <p class="none">{q ? 'No stashes match.' : 'No stashes.'}</p>
        {/each}
      {/if}
    </div>
  </aside>
{/if}

<style>
  .sidebar {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg-1);
  }

  .top {
    flex: none;
    padding: 10px 8px 6px 10px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .filter {
    flex: 1;
    min-width: 0;
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--bg-0);
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-2);
    transition: border-color 120ms var(--ease-out);
  }

  .filter:focus-within {
    border-color: var(--accent);
  }

  .filter input {
    flex: 1;
    width: 100%;
    min-width: 0;
    border: 0;
    background: transparent;
    padding: 0;
    color: var(--text);
    font-size: 12.5px;
  }

  .filter input::placeholder {
    color: var(--text-3);
  }

  .filter input:focus-visible {
    outline: none;
  }

  .scroll {
    flex: 1;
    overflow-y: auto;
    padding-bottom: 16px;
  }

  .sec {
    height: 32px;
    margin-top: 6px;
    padding: 0 8px 0 6px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .sec-btn {
    border: 0;
    background: none;
    padding: 4px;
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-2);
    flex: 1;
    text-align: left;
  }

  .sec-btn:hover .section-label {
    color: var(--text);
  }

  .sec :global(svg) {
    transition: transform 150ms var(--ease-out);
  }

  :global(svg.closed) {
    transform: rotate(-90deg);
  }

  .count {
    font-size: 11.5px;
    color: var(--text-2);
    min-width: 16px;
    text-align: right;
  }

  .add {
    width: 22px;
    height: 22px;
    opacity: 0;
  }

  .sec:hover .add,
  .add:focus-visible {
    opacity: 1;
  }

  .item {
    height: 28px;
    margin: 0 6px;
    padding: 0 8px 0 12px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text);
  }

  .item:hover {
    background: var(--bg-2);
  }

  .item:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .item.head {
    background: var(--bg-3);
    font-weight: 600;
  }

  .item.nested {
    padding-left: 32px;
  }

  .item :global(.ic) {
    flex: none;
    color: var(--text-2);
  }

  .item :global(.head-ic) {
    color: var(--success);
  }

  .item :global(.ic.open) {
    color: var(--success);
  }

  .item :global(.ic.draft) {
    color: var(--text-2);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .track {
    display: flex;
    gap: 6px;
    flex: none;
  }

  .track em {
    font-style: normal;
    font-size: 11.5px;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    gap: 1px;
  }

  .track em.gone {
    color: var(--warning);
  }

  .host {
    flex: none;
    font-size: 11.5px;
    color: var(--text-2);
  }

  .none {
    padding: 2px 16px 6px 34px;
    color: var(--text-2);
    font-size: 12px;
    line-height: 1.45;
  }

  .signin {
    margin: 0 16px 8px 34px;
  }

  .nested-none {
    padding-left: 52px;
  }

  .sub-label {
    padding: 4px 16px 2px 22px;
    font-size: 11.5px;
    color: var(--text-2);
  }

  .rail {
    height: 100%;
    width: 100%;
    padding: 10px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    background: var(--bg-1);
  }

  .rail-sep {
    width: 22px;
    height: 1px;
    background: var(--line-strong);
    margin: 2px 0;
  }

  .rail-btn {
    position: relative;
    width: 36px;
    height: 36px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .rail-btn:hover {
    background: var(--bg-2);
    color: var(--text);
  }

  .rail-btn em {
    position: absolute;
    top: 2px;
    right: 1px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--bg-3);
    color: var(--text);
    font-style: normal;
    font-size: 11px;
    font-weight: 700;
    line-height: 16px;
  }
</style>
