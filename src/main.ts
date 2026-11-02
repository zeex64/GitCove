import '@fontsource-variable/inter'
import '@fontsource/jetbrains-mono/400.css'
import '@fontsource/jetbrains-mono/500.css'
import './app.css'
import { mount } from 'svelte'
import { initBackend, isDesktop } from './lib/api'
import App from './App.svelte'

async function start() {
  await initBackend()
  if (!isDesktop) {
    // Browser previews open the demo repository unless a scenario asks for the empty start.
    const { DEMO_ROOT } = await import('./lib/api/demo')
    const scenario = new URLSearchParams(location.search).get('scenario')
    localStorage.setItem('gitcove.tabs', JSON.stringify(scenario === 'welcome' ? [] : [`${DEMO_ROOT}/GitCove`, `${DEMO_ROOT}/dotfiles`]))
    localStorage.setItem('gitcove.active', JSON.stringify(`${DEMO_ROOT}/GitCove`))
    if (scenario === 'welcome') localStorage.removeItem('gitcove.recent')
  }
  mount(App, { target: document.getElementById('app')! })
  if (!isDesktop) stageScenario()
}

/** Browser-preview scenarios that need a moment of interaction: a live preview, a selected commit. */
async function stageScenario() {
  const params = new URLSearchParams(location.search)
  const open = params.get('open')
  if (open) {
    const { app } = await import('./lib/state/app.svelte')
    const { github } = await import('./lib/state/github.svelte')
    for (let i = 0; i < 50 && !github.status; i++) await new Promise((r) => setTimeout(r, 60))
    if (open === 'account' || open === 'login') app.accountOpen = true
    if (open === 'login') github.startLogin()
    if (open === 'clone') app.cloneInNewTab('github')
    if (open === 'clone-picked') {
      app.cloneInNewTab('github')
      setTimeout(() => (document.querySelector('.repo') as HTMLElement | null)?.click(), 900)
    }
  }
  const scenario = params.get('scenario')
  if (scenario !== 'preview' && scenario !== 'commit') return
  const { app } = await import('./lib/state/app.svelte')
  for (let i = 0; i < 50 && (!app.repo || app.repo.loading); i++) await new Promise((r) => setTimeout(r, 60))
  const repo = app.repo
  if (!repo) return
  if (scenario === 'preview') repo.startPreview({ kind: 'rebase', branch: 'feat/undo-log', onto: 'main' })
  else {
    const c = repo.log.find((x) => x.summary.startsWith('Record undo'))
    if (c) {
      repo.select({ kind: 'commit', id: c.id })
      setTimeout(() => repo.selectCommitFile('src/lib/graph/lanes.ts'), 200)
    }
  }
}

start()
