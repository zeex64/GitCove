import type { Commit, FileChange, Stash } from '../api/types'
import { copyText, openFile, openUrl, revealFile } from '../api/system'
import { refNameError } from '../util/refname'
import { app, type MenuItem } from './app.svelte'
import type { RepoStore } from './repo.svelte'

const SEP: MenuItem = { label: '', separator: true }

type Point = { x: number; y: number }

export function askNewBranch(repo: RepoStore, at: Point | HTMLElement, from: string, fromLabel: string) {
  app.ask(at, {
    title: `New branch from ${fromLabel}`,
    fields: [{ name: 'name', label: 'Branch name', placeholder: 'feat/something', mono: true }],
    confirm: 'Create branch',
    check: { label: 'Switch to it', value: true },
    validate: (v) => refNameError(v.name, repo.refs?.branches.map((b) => b.name)),
    onSubmit: (v, checkout) => repo.createBranch(v.name, from, checkout),
  })
}

export function askNewTag(repo: RepoStore, at: Point, commit: Commit) {
  app.ask(at, {
    title: `Tag ${commit.shortId}`,
    fields: [
      { name: 'name', label: 'Tag name', placeholder: 'v1.0.0', mono: true },
    ],
    confirm: 'Create tag',
    validate: (v) => refNameError(v.name, repo.refs?.tags.map((t) => t.name), 'tag'),
    onSubmit: (v) => repo.createTag(v.name, commit.id, null),
  })
}

export function commitMenu(repo: RepoStore, c: Commit, at: Point): MenuItem[] {
  const cur = repo.head?.branch
  const onHead = repo.head?.commit === c.id
  const branchesHere = repo.refs?.branches.filter((b) => b.commit === c.id && !b.isHead) ?? []
  const items: MenuItem[] = []
  for (const b of branchesHere) items.push({ label: `Switch to ${b.name}`, run: () => repo.checkout(b.name) })
  items.push(
    { label: 'Check out this commit', hint: 'detached', disabled: onHead, run: () => repo.checkout(c.id) },
    { label: 'New branch here…', run: () => askNewBranch(repo, at, c.id, c.shortId) },
    { label: 'Tag this commit…', run: () => askNewTag(repo, at, c) },
    SEP,
  )
  if (cur) {
    for (const b of branchesHere) {
      items.push(
        { label: `Merge ${b.name} into ${cur}…`, run: () => repo.startPreview({ kind: 'merge', branch: b.name }) },
        { label: `Rebase ${cur} onto ${b.name}…`, run: () => repo.startPreview({ kind: 'rebase', branch: cur, onto: b.name }) },
      )
    }
    if (!branchesHere.length && !onHead) {
      items.push({ label: `Rebase ${cur} onto this commit…`, run: () => repo.startPreview({ kind: 'rebase', branch: cur, onto: c.id }) })
    }
  }
  items.push(
    { label: `Cherry-pick onto ${cur ?? 'HEAD'}…`, disabled: onHead, run: () => repo.startPreview({ kind: 'cherryPick', commit: c.id }) },
    { label: 'Revert this commit…', run: () => repo.startPreview({ kind: 'revert', commit: c.id }) },
    SEP,
    { label: `Reset ${cur ?? 'HEAD'} to here: keep changes staged…`, disabled: onHead, run: () => repo.startPreview({ kind: 'reset', target: c.id, mode: 'soft' }) },
    { label: `Reset ${cur ?? 'HEAD'} to here: keep changes unstaged…`, disabled: onHead, run: () => repo.startPreview({ kind: 'reset', target: c.id, mode: 'mixed' }) },
    { label: `Reset ${cur ?? 'HEAD'} to here: discard changes…`, danger: true, run: () => repo.startPreview({ kind: 'reset', target: c.id, mode: 'hard' }) },
    SEP,
    { label: 'Copy commit SHA', run: () => copyText(c.id) },
    { label: 'Copy message', run: () => copyText(c.body ? `${c.summary}\n\n${c.body}` : c.summary) },
  )
  const web = repo.primaryRemote?.webUrl
  if (web) {
    const path = repo.primaryRemote?.host === 'gitlab' ? '/-/commit/' : '/commit/'
    items.push({ label: 'Open on ' + hostName(repo.primaryRemote!.host), run: () => openUrl(web + path + c.id) })
  }
  return items
}

export function branchMenu(repo: RepoStore, name: string, at: Point): MenuItem[] {
  const b = repo.refs?.branches.find((x) => x.name === name)
  if (!b) return []
  const cur = repo.head?.branch
  const items: MenuItem[] = []
  if (!b.isHead) items.push({ label: `Switch to ${name}`, run: () => repo.checkout(name) })
  if (cur && !b.isHead) {
    items.push(
      { label: `Merge ${name} into ${cur}…`, run: () => repo.startPreview({ kind: 'merge', branch: name }) },
      { label: `Rebase ${cur} onto ${name}…`, run: () => repo.startPreview({ kind: 'rebase', branch: cur, onto: name }) },
    )
  }
  if (b.isHead && b.upstream) {
    items.push({ label: 'Pull', run: () => repo.pull() }, { label: 'Push', run: () => repo.push() })
  }
  if (b.upstream) {
    const [remote, ...rest] = b.upstream.split('/')
    items.push({ label: `Force push to ${b.upstream}…`, danger: true, run: () => repo.startPreview({ kind: 'forcePush', remote, branch: name }) })
    void rest
  }
  items.push(
    SEP,
    { label: 'New branch from here…', run: () => askNewBranch(repo, at, b.commit, name) },
    {
      label: 'Rename…',
      run: () =>
        app.ask(at, {
          title: `Rename ${name}`,
          fields: [{ name: 'name', label: 'New name', value: name, mono: true }],
          confirm: 'Rename',
          validate: (v) => (v.name === name ? null : refNameError(v.name, repo.refs?.branches.map((x) => x.name))),
          onSubmit: (v) => v.name !== name && repo.renameBranch(name, v.name),
        }),
    },
    { label: 'Copy branch name', run: () => copyText(name) },
    SEP,
    {
      label: `Delete ${name}…`,
      danger: true,
      disabled: b.isHead,
      hint: b.isHead ? 'checked out' : undefined,
      run: () => repo.startPreview({ kind: 'deleteBranch', name, force: true }),
    },
  )
  return items
}

export function remoteBranchMenu(repo: RepoStore, fullName: string, at: Point): MenuItem[] {
  const cur = repo.head?.branch
  return [
    { label: `Check out ${fullName}`, hint: 'tracking branch', run: () => repo.checkout(fullName) },
    { label: 'New branch from here…', run: () => askNewBranch(repo, at, fullName, fullName) },
    SEP,
    { label: `Merge into ${cur}…`, disabled: !cur, run: () => repo.startPreview({ kind: 'merge', branch: fullName }) },
    { label: `Rebase ${cur} onto ${fullName}…`, disabled: !cur, run: () => cur && repo.startPreview({ kind: 'rebase', branch: cur, onto: fullName }) },
    SEP,
    { label: 'Copy branch name', run: () => copyText(fullName) },
  ]
}

export function tagMenu(repo: RepoStore, name: string): MenuItem[] {
  return [
    { label: `Check out ${name}`, hint: 'detached', run: () => repo.checkout(name) },
    { label: 'Copy tag name', run: () => copyText(name) },
    SEP,
    { label: `Delete tag ${name}`, danger: true, run: () => repo.deleteTag(name) },
  ]
}

export function stashMenu(repo: RepoStore, s: Stash): MenuItem[] {
  return [
    { label: 'Pop', hint: 'apply and remove', run: () => repo.stashApply(s.index, true) },
    { label: 'Apply', hint: 'keep the stash', run: () => repo.stashApply(s.index, false) },
    SEP,
    { label: 'Drop…', danger: true, run: () => repo.startPreview({ kind: 'dropStash', index: s.index }) },
  ]
}

export function fileMenu(repo: RepoStore, f: FileChange, staged: boolean): MenuItem[] {
  const abs = `${repo.path}/${f.path}`
  const items: MenuItem[] = []
  if (f.conflict) {
    items.push(
      { label: 'Use my version', run: () => repo.resolveMine(f.path) },
      { label: `Use ${repo.conflictSides.other}’s version`, run: () => repo.resolveOther(f.path) },
      { label: 'Mark as resolved', run: () => repo.resolve(f.path, 'mark') },
      SEP,
    )
  } else if (staged) {
    items.push({ label: 'Unstage', run: () => repo.unstage([f.path]) })
  } else {
    items.push({ label: 'Stage', run: () => repo.stage([f.path]) })
  }
  items.push(
    { label: 'Show full diff', run: () => { repo.selectFile(f.path, staged); repo.fullDiff = true } },
    { label: 'Open file', disabled: f.unstaged === 'deleted' || f.staged === 'deleted', run: () => openFile(abs) },
    { label: 'Reveal in file manager', run: () => revealFile(abs) },
    { label: 'Copy path', run: () => copyText(f.path) },
  )
  if (!staged && !f.conflict) {
    items.push(SEP, { label: 'Discard changes…', danger: true, run: () => repo.startPreview({ kind: 'discard', paths: [f.path] }) })
  }
  return items
}

export function hostName(host: string): string {
  return { github: 'GitHub', gitlab: 'GitLab', forgejo: 'Forgejo', gitea: 'Gitea', bitbucket: 'Bitbucket' }[host] ?? 'the web'
}
