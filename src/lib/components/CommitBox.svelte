<script lang="ts">
  import { UserRound } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import type { RepoStore } from '../state/repo.svelte'
  import { github } from '../state/github.svelte'
  import { modKey } from '../api/system'
  import { initials, middleTruncate } from '../util/format'
  import Checkbox from './Checkbox.svelte'

  let { repo }: { repo: RepoStore } = $props()

  let winH = $state(window.innerHeight)
  let whoBtn = $state<HTMLButtonElement>()
  let avatarFailed = $state(false)

  const branch = $derived(repo.branchName ?? (repo.head?.detached ? 'detached HEAD' : 'main'))
  const busyOp = $derived(!!repo.status && repo.status.state !== 'clean')
  const included = $derived(repo.staged.length)
  const id = $derived(repo.identity)
  const unknown = $derived(!!id && (!id.name || !id.email))
  const canCommit = $derived((included > 0 || repo.amend) && repo.summary.trim().length > 0 && !busyOp && !unknown)
  const remaining = $derived(72 - repo.summary.length)
  const avatar = $derived(github.account?.avatarUrl && !avatarFailed ? github.account.avatarUrl : null)
  const who = $derived(id?.name ? `Committing as ${id.name}${id.email ? ` <${id.email}>` : ''}` : 'Git doesn’t know your name yet')

  const disabledWhy = $derived.by(() => {
    if (busyOp) return 'Finish or abort the operation in progress first'
    if (unknown) return 'Set your name and email first'
    if (!included && !repo.amend) return 'Tick the files to include in the commit'
    if (!repo.summary.trim()) return 'Write a summary first'
    return ''
  })

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      if (canCommit) repo.commit()
    }
  }

  function askIdentity() {
    if (!whoBtn) return
    app.ask(whoBtn, {
      title: 'Sign your commits',
      fields: [
        { name: 'name', label: 'Name', value: id?.name ?? github.account?.name ?? github.account?.login ?? '', placeholder: 'Your name' },
        { name: 'email', label: 'Email', value: id?.email ?? '', placeholder: 'you@example.com' },
      ],
      confirm: 'Save for all repositories',
      validate: (v) => (!v.name.trim() ? 'Enter a name.' : !/^\S+@\S+$/.test(v.email.trim()) ? 'Enter an email address.' : null),
      onSubmit: (v) => repo.setIdentity(v.name.trim(), v.email.trim()),
    })
  }
</script>

<svelte:window bind:innerHeight={winH} />

{#if busyOp}
  <div class="box busy"><p>Committing resumes once the operation in progress is finished or aborted.</p></div>
{:else}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="box" onkeydown={onKey} role="group" aria-label="Commit">
    {#if unknown}
      <div class="who-warn">
        <span>Git doesn’t know who you are yet, so it can’t sign commits.</span>
        <button class="btn sm ghost" bind:this={whoBtn} onclick={askIdentity}>Set name and email…</button>
      </div>
    {/if}
    <div class="summary-row">
      <span class="avatar" title={who}>
        {#if avatar}
          <img src={avatar} alt="" width="28" height="28" onerror={() => (avatarFailed = true)} />
        {:else if id?.name}
          {initials(id.name)}
        {:else}
          <UserRound size={15} />
        {/if}
      </span>
      <div class="summary-wrap">
        <input
          class="field summary"
          bind:value={repo.summary}
          placeholder={repo.info.isEmpty ? 'First commit summary' : 'Summary (required)'}
          aria-label="Commit summary"
          spellcheck="true"
        />
        {#if repo.summary.length > 50}<span class="count num" class:over={remaining < 0}>{remaining}</span>{/if}
      </div>
    </div>
    <textarea class="field desc" class:short={winH < 760} bind:value={repo.body} placeholder="Description" aria-label="Commit description"></textarea>
    <div class="row">
      {#if repo.head?.commit}
        <Checkbox
          checked={repo.amend}
          label="Amend last commit"
          text="Amend last commit"
          title="Replace the last commit with these changes and message"
          onchange={() => repo.setAmend(!repo.amend)}
        />
      {:else}
        <span></span>
      {/if}
      <span class="kbd">{modKey} Enter</span>
    </div>
    <button class="btn success commit-btn" disabled={!canCommit} title={disabledWhy || who} onclick={() => repo.commit()}>
      {#if repo.amend}Amend last commit{:else if !included}Tick the files to commit{:else}<span>Commit to <strong>{middleTruncate(branch, 28)}</strong></span>{/if}
    </button>
  </div>
{/if}

<style>
  .box {
    flex: none;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px solid var(--line);
    background: var(--bg-1);
  }

  .box.busy {
    padding: 14px 16px;
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .who-warn {
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    border-radius: var(--radius);
    background: var(--warning-soft);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .summary-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .avatar {
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: 50%;
    overflow: hidden;
    background: var(--bg-3);
    color: var(--text);
    font-size: 11px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .avatar img {
    display: block;
    width: 100%;
    height: 100%;
  }

  .summary-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  .summary {
    height: 34px;
    font-weight: 500;
    padding-right: 40px;
  }

  .count {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11.5px;
    color: var(--text-2);
  }

  .count.over {
    color: var(--warning);
  }

  .desc {
    height: 84px;
  }

  .desc.short {
    height: 52px;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }


  .commit-btn {
    height: 38px;
    border-radius: var(--radius-md);
  }

  .commit-btn strong {
    font-weight: 700;
  }

  .commit-btn:disabled {
    background: var(--bg-3);
    color: var(--text-2);
    opacity: 1;
  }
</style>
