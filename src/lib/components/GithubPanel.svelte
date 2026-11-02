<script lang="ts">
  import { Copy, Check, ExternalLink, LogOut, Download, KeyRound, TriangleAlert, CloudOff } from '@lucide/svelte'
  import { app } from '../state/app.svelte'
  import { github, OAUTH_APP_URL } from '../state/github.svelte'
  import { copyText, openUrl } from '../api/system'
  import { initials } from '../util/format'

  let { onclone }: { onclone?: () => void } = $props()

  let clientId = $state('')
  let saving = $state(false)
  let copied = $state(false)
  let avatarFailed = $state(false)

  const s = $derived(github.status)
  const login = $derived(github.login)
  const account = $derived(github.account)

  async function save(e: SubmitEvent) {
    e.preventDefault()
    if (!clientId.trim()) return
    saving = true
    if (await github.setClientId(clientId)) clientId = ''
    saving = false
  }

  async function copyCode() {
    if (!login) return
    await copyText(login.code.userCode)
    copied = true
    setTimeout(() => (copied = false), 1400)
  }
</script>

<div class="gh">
  {#if !s}
    <div class="loading" aria-busy="true"><i></i><i></i></div>
  {:else if account}
    <div class="account">
      {#if account.avatarUrl && !avatarFailed}
        <img src={account.avatarUrl} alt="" width="40" height="40" onerror={() => (avatarFailed = true)} />
      {:else}
        <span class="avatar">{initials(account.name ?? account.login)}</span>
      {/if}
      <div class="who">
        <p class="name">{account.name ?? account.login}</p>
        <p class="login">@{account.login} on GitHub</p>
      </div>
    </div>
    {#if s.offline}
      <p class="note"><CloudOff size={13} />Can’t reach github.com right now. You stay signed in.</p>
    {/if}
    {#if s.storage === 'file'}
      <p class="note warn"><TriangleAlert size={13} />No system keychain answered, so the sign-in is saved in a private file in the app’s settings folder.</p>
    {/if}
    <div class="actions">
      <button class="btn primary" onclick={() => (onclone ? onclone() : app.cloneInNewTab('github'))}><Download size={14} />Clone a repository…</button>
      <button class="btn ghost" onclick={() => github.signOut()}><LogOut size={14} />Sign out</button>
    </div>
  {:else if !s.configured}
    <h3>Set up GitHub sign-in</h3>
    <p class="lede">GitHub needs to know which app is asking. Register a free OAuth App once, then sign in with a code.</p>
    <ol class="steps">
      <li>
        <span>Create an OAuth App on GitHub. Any name works; use <code>http://localhost</code> for both URLs.</span>
        <button class="btn sm ghost" onclick={() => openUrl(OAUTH_APP_URL)}>Open GitHub settings<ExternalLink size={12} /></button>
      </li>
      <li><span>On the new app’s page, tick <strong>Enable Device Flow</strong> and save.</span></li>
      <li>
        <span>Paste its Client ID here.</span>
        <form class="cid" onsubmit={save}>
          <input class="field mono" bind:value={clientId} placeholder="Ov23li…" spellcheck="false" autocomplete="off" aria-label="OAuth App client ID" />
          <button class="btn primary" disabled={!clientId.trim() || saving}>Save</button>
        </form>
      </li>
    </ol>
    {#if github.error}<p class="error" role="alert">{github.error.message}</p>{/if}
  {:else if login}
    <h3>Enter this code on GitHub</h3>
    <div class="code-row">
      <span class="code mono selectable">{login.code.userCode}</span>
      <button class="icon-btn" aria-label="Copy code" title="Copy code" onclick={copyCode}>
        {#if copied}<Check size={15} />{:else}<Copy size={15} />{/if}
      </button>
    </div>
    {#if login.done}
      <p class="error" role="alert">{login.message}</p>
      <div class="actions">
        <button class="btn primary" onclick={() => github.startLogin()}>Get a new code</button>
        <button class="btn quiet" onclick={() => github.cancelLogin()}>Cancel</button>
      </div>
    {:else}
      <p class="lede">Open <span class="mono">{login.code.verificationUri.replace('https://', '')}</span>, paste the code and approve access. This window carries on by itself.</p>
      <div class="actions">
        <button class="btn primary" onclick={() => github.copyAndOpen()}>Copy code and open GitHub<ExternalLink size={13} /></button>
        <button class="btn quiet" onclick={() => github.cancelLogin()}>Cancel</button>
      </div>
      <div class="waiting">
        <i class="bar"></i>
        <span>{login.message ?? 'Waiting for you to approve on GitHub…'}</span>
      </div>
    {/if}
  {:else}
    <h3>Sign in to GitHub</h3>
    <p class="lede">See and clone your repositories, and list pull requests on private ones. Pushing and pulling over HTTPS use this sign-in too.</p>
    <div class="actions">
      <button class="btn primary" onclick={() => github.startLogin()}><KeyRound size={14} />Sign in with GitHub</button>
    </div>
    {#if github.error}<p class="error" role="alert">{github.error.message}</p>{/if}
    {#if s.clientIdSource === 'user'}
      <p class="fine">Using the client ID you saved. You approve access on github.com; this app never sees your password.</p>
    {/if}
  {/if}
</div>

<style>
  .gh {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    font-size: 14px;
    font-weight: 600;
  }

  .lede {
    color: var(--text-2);
    line-height: 1.5;
  }

  .fine {
    color: var(--text-2);
    font-size: 12px;
    line-height: 1.45;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 2px;
  }

  .account {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .account img,
  .avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    flex: none;
  }

  .avatar {
    background: var(--bg-3);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
  }

  .who {
    min-width: 0;
  }

  .name {
    font-weight: 600;
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .login {
    color: var(--text-2);
  }

  .note {
    display: flex;
    gap: 7px;
    align-items: flex-start;
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .note :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .note.warn :global(svg) {
    color: var(--warning);
  }

  .steps {
    margin: 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    line-height: 1.5;
  }

  .steps li {
    padding-left: 2px;
  }

  .steps li::marker {
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }

  .steps li > span {
    display: block;
  }

  .steps .btn {
    margin-top: 6px;
  }

  code {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--bg-3);
  }

  .cid {
    margin-top: 6px;
    display: flex;
    gap: 6px;
  }

  .cid .field {
    background: var(--bg-1);
  }

  .code-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border: 1px dashed var(--accent-line);
    border-radius: var(--radius-md);
    background: var(--accent-soft);
  }

  .code {
    flex: 1;
    font-size: 24px;
    font-weight: 500;
    letter-spacing: 0.06em;
    color: var(--text);
  }

  .waiting {
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--text-2);
    font-size: 12.5px;
  }

  .bar {
    position: relative;
    height: 2px;
    border-radius: 999px;
    background: var(--bg-3);
    overflow: hidden;
  }

  .bar::after {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    width: 30%;
    background: var(--accent);
    border-radius: 999px;
    animation: slide 1.2s var(--ease-out) infinite;
  }

  .error {
    color: var(--danger);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .loading {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .loading i {
    height: 10px;
    border-radius: var(--radius-sm);
    background: var(--bg-3);
  }

  .loading i:first-child {
    width: 60%;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>
