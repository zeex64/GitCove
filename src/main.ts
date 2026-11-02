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
  const tab = params.get('tab')
  const split = params.get('split')
  const picked = params.has('picked')
  const summary = params.get('summary')
  if (scenario !== 'preview' && scenario !== 'commit' && !tab && !split && !picked && !summary && open !== 'branch' && open !== 'undo') return
  const { app } = await import('./lib/state/app.svelte')
  for (let i = 0; i < 50 && (!app.repo || app.repo.loading); i++) await new Promise((r) => setTimeout(r, 60))
  const repo = app.repo
  if (!repo) return
  if (split !== null) app.panes.split = split === '1'
  if (tab === 'history') repo.setTab('history')
  if (summary) repo.summary = summary
  if (picked) {
    // Some lines of the open file in the commit, some left out.
    for (let i = 0; i < 50 && !repo.diff; i++) await new Promise((r) => setTimeout(r, 60))
    const d = repo.diff
    if (d) repo.setLines(d.hunks[0].lines.flatMap((l, li) => (l.kind === 'add' && li < 6) || l.kind === 'del' ? [`0:${li}` as const] : []), true)
  }
  if (open === 'branch' || open === 'undo') setTimeout(() => (document.querySelector(open === 'branch' ? '.branch-seg' : '.caret-btn') as HTMLElement | null)?.click(), 300)
  if (scenario === 'preview') repo.startPreview({ kind: 'rebase', branch: 'feat/undo-log', onto: 'main' })
  else if (scenario === 'commit') {
    const c = repo.log.find((x) => x.summary.startsWith('Record undo'))
    if (c) {
      repo.setTab('history')
      repo.select({ kind: 'commit', id: c.id })
      setTimeout(() => repo.selectCommitFile('src/lib/graph/lanes.ts'), 200)
    }
  }
}

start()
