<script lang="ts">
  import { UserRound } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import { github } from '../state/github.svelte'
  import Popover from './Popover.svelte'
  import GithubPanel from './GithubPanel.svelte'

  let btn = $state<HTMLButtonElement>()
  let avatarFailed = $state(false)

  const account = $derived(github.account)
</script>

<button
  class="account-btn"
  class:signed-in={!!account}
  bind:this={btn}
  onmousedown={(e) => e.stopPropagation()}
  onclick={() => (app.accountOpen = !app.accountOpen)}
  aria-haspopup="true"
  aria-expanded={app.accountOpen}
  title={account ? `GitHub: @${account.login}` : 'Sign in to GitHub'}
>
  {#if account?.avatarUrl && !avatarFailed}
    <img src={account.avatarUrl} alt="" width="20" height="20" onerror={() => (avatarFailed = true)} />
  {:else}
    <UserRound size={15} />
  {/if}
  <span>{account ? account.login : 'Sign in'}</span>
</button>

<Popover bind:open={app.accountOpen} anchor={btn} width={360} align="end">
  <div class="pad"><GithubPanel onclone={() => { app.accountOpen = false; app.cloneInNewTab('github') }} /></div>
</Popover>

<style>
  .account-btn {
    height: 28px;
    max-width: 180px;
    padding: 0 10px 0 6px;
    margin-left: auto;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    display: flex;
    align-items: center;
    gap: 7px;
    font-weight: 500;
    flex: none;
    transition:
      background-color 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .account-btn:hover,
  .account-btn[aria-expanded='true'] {
    background: var(--bg-2);
    color: var(--text);
  }

  .account-btn span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-btn img {
    width: 20px;
    height: 20px;
    border-radius: 50%;
  }

  .pad {
    padding: 16px;
    overflow-y: auto;
  }
</style>
