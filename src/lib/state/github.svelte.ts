import { backend } from '../api'
import type { DeviceCode, GitError, GithubRepo, GithubStatus } from '../api/types'
import { copyText, openUrl } from '../api/system'
import { app } from './app.svelte'

export const OAUTH_APP_URL = 'https://github.com/settings/applications/new'

/** The signed-in GitHub account, the device-flow sign-in, and the repository list. */
class GithubState {
  status = $state<GithubStatus | null>(null)
  error = $state<GitError | null>(null)
  login = $state<{ code: DeviceCode; message: string | null; done: boolean } | null>(null)
  repos = $state.raw<GithubRepo[] | null>(null)
  reposLoading = $state(false)
  reposError = $state<GitError | null>(null)

  account = $derived(this.status?.account ?? null)

  #seq = 0

  async refresh() {
    try {
      this.status = await backend.githubStatus()
      this.error = null
    } catch (e) {
      this.error = e as GitError
    }
  }

  async setClientId(id: string): Promise<boolean> {
    try {
      this.status = await backend.githubSetClientId(id)
      this.error = null
      return true
    } catch (e) {
      this.error = e as GitError
      return false
    }
  }

  async startLogin() {
    this.error = null
    const seq = ++this.#seq
    try {
      const code = await backend.githubStartLogin()
      if (seq !== this.#seq) return
      this.login = { code, message: null, done: false }
      this.#schedule(seq, code.interval)
    } catch (e) {
      this.error = e as GitError
    }
  }

  /** The one-click path: the code goes to the clipboard and GitHub opens in the browser. */
  async copyAndOpen() {
    const l = this.login
    if (!l) return
    await copyText(l.code.userCode)
    await openUrl(l.code.verificationUri)
  }

  cancelLogin() {
    this.#seq++
    this.login = null
  }

  #schedule(seq: number, seconds: number) {
    setTimeout(() => this.#poll(seq, seconds), Math.max(1, seconds) * 1000)
  }

  async #poll(seq: number, interval: number) {
    const l = this.login
    if (seq !== this.#seq || !l) return
    try {
      const r = await backend.githubPollLogin(l.code.deviceCode)
      if (seq !== this.#seq) return
      switch (r.status) {
        case 'pending':
          this.#schedule(seq, interval)
          break
        case 'slowDown':
          this.#schedule(seq, r.interval ?? interval + 5)
          break
        case 'ok':
          this.login = null
          await this.refresh()
          app.toast({ kind: 'success', message: `Signed in to GitHub as @${r.account?.login ?? this.account?.login ?? 'you'}` })
          for (const t of app.tabs) t.repo?.loadPullRequests()
          this.loadRepos(true)
          break
        default:
          this.login = { ...l, message: r.message ?? 'Sign-in didn’t finish. Try again.', done: true }
      }
    } catch (e) {
      // A dropped connection shouldn't end the sign-in; GitHub's code stays valid for minutes.
      if (seq !== this.#seq) return
      this.login = { ...l, message: (e as GitError).message }
      this.#schedule(seq, interval * 2)
    }
  }

  async signOut() {
    try {
      await backend.githubSignOut()
      const login = this.account?.login
      this.repos = null
      await this.refresh()
      app.toast({ kind: 'info', message: login ? `Signed out of @${login}` : 'Signed out of GitHub' })
      for (const t of app.tabs) t.repo?.loadPullRequests()
    } catch (e) {
      app.fail(e)
    }
  }

  async loadRepos(force = false) {
    if (!this.account || this.reposLoading || (this.repos && !force)) return
    this.reposLoading = true
    this.reposError = null
    try {
      this.repos = await backend.githubListRepos()
    } catch (e) {
      this.reposError = e as GitError
      if ((e as GitError).kind === 'auth') await this.refresh()
    } finally {
      this.reposLoading = false
    }
  }
}

export const github = new GithubState()
