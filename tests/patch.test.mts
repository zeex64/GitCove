// Partial staging must produce patches git accepts. Runs against a throwaway repo:
//   npm test
import { execSync } from 'node:child_process'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { buildPatch, lineKey } from '../src/lib/util/patch.ts'

const dir = mkdtempSync(join(process.env.TMPDIR ?? tmpdir(), 'pt-'))
const git = (args: string, input?: string) => execSync(`git ${args}`, { cwd: dir, input, encoding: 'utf8' })
git('init -q'); git('config user.email t@t'); git('config user.name t')
const base = Array.from({ length: 30 }, (_, i) => `line ${i + 1}`)
writeFileSync(join(dir, 'f.txt'), base.join('\n') + '\n')
git('add f.txt'); git('commit -qm base')

function parse(raw: string) {
  const hunks: any[] = []
  let h: any = null
  let o = 0, n = 0
  for (const line of raw.split('\n')) {
    const m = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/.exec(line)
    if (m) { h = { header: line, oldStart: +m[1], oldLines: +(m[2] ?? 1), newStart: +m[3], newLines: +(m[4] ?? 1), lines: [] }; hunks.push(h); o = h.oldStart; n = h.newStart; continue }
    if (!h) continue
    if (line.startsWith('+')) h.lines.push({ kind: 'add', oldNo: null, newNo: n++, text: line.slice(1) })
    else if (line.startsWith('-')) h.lines.push({ kind: 'del', oldNo: o++, newNo: null, text: line.slice(1) })
    else if (line.startsWith(' ')) h.lines.push({ kind: 'context', oldNo: o++, newNo: n++, text: line.slice(1) })
    else if (line.startsWith('\\')) h.lines.push({ kind: 'noNewline', oldNo: null, newNo: null, text: '' })
  }
  return { path: 'f.txt', oldPath: null, status: 'modified', binary: false, tooLarge: false, additions: 0, deletions: 0, hunks }
}

// Worktree: change line 3, insert after 10, delete 20-21, change 28.
const work = [...base]
work[2] = 'line 3 CHANGED'
work.splice(10, 0, 'inserted A', 'inserted B')
work.splice(21, 2)
work[work.length - 3] = 'line 28 CHANGED'
writeFileSync(join(dir, 'f.txt'), work.join('\n') + '\n')

let pass = 0, fail = 0
const check = (name: string, ok: boolean, extra = '') => { ok ? pass++ : fail++; console.log(`${ok ? 'ok  ' : 'FAIL'} ${name} ${extra}`) }

// 1) Stage only "inserted B" (second add of hunk containing inserts).
let d = parse(git('diff -U3 -- f.txt'))
const hi = d.hunks.findIndex((h: any) => h.lines.some((l: any) => l.text === 'inserted B'))
const li = d.hunks[hi].lines.findIndex((l: any) => l.text === 'inserted B')
let patch = buildPatch(d as any, new Set([lineKey(hi, li)]), false)!
git('apply --cached --recount --whitespace=nowarn -', patch)
let staged = git('diff --cached -U0 -- f.txt')
check('stage single added line', staged.includes('+inserted B') && !staged.includes('+inserted A') && !staged.includes('CHANGED'))

// 2) Stage the deletion of line 21 only (one of two removed lines), from a later hunk.
d = parse(git('diff -U3 -- f.txt'))
let h2 = d.hunks.findIndex((h: any) => h.lines.some((l: any) => l.kind === 'del' && l.text === 'line 21'))
let l2 = d.hunks[h2].lines.findIndex((l: any) => l.kind === 'del' && l.text === 'line 21')
patch = buildPatch(d as any, new Set([lineKey(h2, l2)]), false)!
git('apply --cached --recount --whitespace=nowarn -', patch)
staged = git('diff --cached -U0 -- f.txt')
check('stage one removed line in later hunk', staged.includes('-line 21') && !staged.includes('-line 20'))

// 3) Unstage "inserted B" again from the staged diff (reverse).
d = parse(git('diff --cached -U3 -- f.txt'))
const h3 = d.hunks.findIndex((h: any) => h.lines.some((l: any) => l.text === 'inserted B'))
const l3 = d.hunks[h3].lines.findIndex((l: any) => l.text === 'inserted B')
patch = buildPatch(d as any, new Set([lineKey(h3, l3)]), true)!
git('apply --cached --reverse --recount --whitespace=nowarn -', patch)
staged = git('diff --cached -U0 -- f.txt')
check('unstage line from staged diff', !staged.includes('inserted B') && staged.includes('-line 21'))

// 4) Discard "line 28 CHANGED" from the worktree (reverse, not cached).
d = parse(git('diff -U3 -- f.txt'))
const h4 = d.hunks.findIndex((h: any) => h.lines.some((l: any) => l.text === 'line 28 CHANGED'))
const sel4 = new Set<any>()
d.hunks[h4].lines.forEach((l: any, i: number) => { if (l.text === 'line 28 CHANGED' || (l.kind === 'del' && l.text === 'line 28')) sel4.add(lineKey(h4, i)) })
patch = buildPatch(d as any, sel4, true)!
git('apply --reverse --recount --whitespace=nowarn -', patch)
const wt = git('diff -U0 -- f.txt')
check('discard a change in the worktree', !wt.includes('line 28 CHANGED') && wt.includes('line 3 CHANGED'))

// 5) Stage a whole hunk in the middle while another selection exists elsewhere.
d = parse(git('diff -U3 -- f.txt'))
const h5 = d.hunks.findIndex((h: any) => h.lines.some((l: any) => l.text === 'line 3 CHANGED'))
const sel5 = new Set<any>()
d.hunks[h5].lines.forEach((l: any, i: number) => { if (l.kind !== 'context') sel5.add(lineKey(h5, i)) })
patch = buildPatch(d as any, sel5, false)!
git('apply --cached --recount --whitespace=nowarn -', patch)
staged = git('diff --cached -U0 -- f.txt')
check('stage whole first hunk', staged.includes('+line 3 CHANGED'))

// 6) New untracked file: stage two of three lines.
writeFileSync(join(dir, 'new.txt'), 'a\nb\nc\n')
const raw = execSync('git diff --no-index -- /dev/null new.txt || true', { cwd: dir, encoding: 'utf8' })
const dn: any = parse(raw); dn.path = 'new.txt'; dn.status = 'untracked'
patch = buildPatch(dn, new Set([lineKey(0, 0), lineKey(0, 2)]), false)!
git('apply --cached --recount --whitespace=nowarn -', patch)
const shown = git('show :new.txt')
check('stage partial untracked file', shown === 'a\nc\n', JSON.stringify(shown))

console.log(`\n${pass} passed, ${fail} failed`)
process.exit(fail ? 1 : 0)
