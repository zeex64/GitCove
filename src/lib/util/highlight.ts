import type { HighlighterCore } from 'shiki/core'
import type { FileDiff } from '../api/types'

/** A coloured run of text; colours are `var(--syntax-…)` references owned by app.css. */
export interface Tok {
  text: string
  color?: string
  italic?: boolean
}

/** Tokens per hunk, per line index (null where a line stays plain). */
export type DiffTokens = (Tok[] | null)[][]

type Loader = () => Promise<{ default: unknown }>

/** Grammars load on first use, so only the languages a user actually diffs are fetched. */
const LOADERS: Record<string, Loader> = {
  typescript: () => import('shiki/langs/typescript.mjs'),
  tsx: () => import('shiki/langs/tsx.mjs'),
  javascript: () => import('shiki/langs/javascript.mjs'),
  jsx: () => import('shiki/langs/jsx.mjs'),
  svelte: () => import('shiki/langs/svelte.mjs'),
  vue: () => import('shiki/langs/vue.mjs'),
  rust: () => import('shiki/langs/rust.mjs'),
  python: () => import('shiki/langs/python.mjs'),
  go: () => import('shiki/langs/go.mjs'),
  java: () => import('shiki/langs/java.mjs'),
  kotlin: () => import('shiki/langs/kotlin.mjs'),
  swift: () => import('shiki/langs/swift.mjs'),
  c: () => import('shiki/langs/c.mjs'),
  cpp: () => import('shiki/langs/cpp.mjs'),
  csharp: () => import('shiki/langs/csharp.mjs'),
  ruby: () => import('shiki/langs/ruby.mjs'),
  php: () => import('shiki/langs/php.mjs'),
  shellscript: () => import('shiki/langs/shellscript.mjs'),
  fish: () => import('shiki/langs/fish.mjs'),
  json: () => import('shiki/langs/json.mjs'),
  jsonc: () => import('shiki/langs/jsonc.mjs'),
  yaml: () => import('shiki/langs/yaml.mjs'),
  toml: () => import('shiki/langs/toml.mjs'),
  html: () => import('shiki/langs/html.mjs'),
  css: () => import('shiki/langs/css.mjs'),
  scss: () => import('shiki/langs/scss.mjs'),
  markdown: () => import('shiki/langs/markdown.mjs'),
  sql: () => import('shiki/langs/sql.mjs'),
  lua: () => import('shiki/langs/lua.mjs'),
  xml: () => import('shiki/langs/xml.mjs'),
  ini: () => import('shiki/langs/ini.mjs'),
  docker: () => import('shiki/langs/docker.mjs'),
  make: () => import('shiki/langs/make.mjs'),
}

const BY_EXT: Record<string, string> = {
  ts: 'typescript', mts: 'typescript', cts: 'typescript', tsx: 'tsx',
  js: 'javascript', mjs: 'javascript', cjs: 'javascript', jsx: 'jsx',
  svelte: 'svelte', vue: 'vue', rs: 'rust', py: 'python', go: 'go', java: 'java',
  kt: 'kotlin', kts: 'kotlin', swift: 'swift', c: 'c', h: 'c',
  cc: 'cpp', cpp: 'cpp', cxx: 'cpp', hpp: 'cpp', hh: 'cpp', cs: 'csharp',
  rb: 'ruby', php: 'php', sh: 'shellscript', bash: 'shellscript', zsh: 'shellscript',
  fish: 'fish', json: 'json', jsonc: 'jsonc', yml: 'yaml', yaml: 'yaml', toml: 'toml',
  html: 'html', htm: 'html', css: 'css', scss: 'scss', md: 'markdown', markdown: 'markdown',
  sql: 'sql', lua: 'lua', xml: 'xml', svg: 'xml', ini: 'ini', cfg: 'ini', conf: 'ini',
}

const BY_NAME: Record<string, string> = { dockerfile: 'docker', makefile: 'make', 'tsconfig.json': 'jsonc' }

export function languageFor(path: string): string | null {
  const name = path.split('/').pop()?.toLowerCase() ?? ''
  if (BY_NAME[name]) return BY_NAME[name]
  const dot = name.lastIndexOf('.')
  return dot > 0 ? (BY_EXT[name.slice(dot + 1)] ?? null) : null
}

/** Beyond this many lines a diff stays plain rather than holding up the view. */
const MAX_LINES = 4000

let highlighter: Promise<HighlighterCore> | null = null
const loaded = new Set<string>()
const cache = new Map<string, DiffTokens>()

function getHighlighter(): Promise<HighlighterCore> {
  highlighter ??= (async () => {
    const [{ createHighlighterCore, createCssVariablesTheme }, { createJavaScriptRegexEngine }] = await Promise.all([
      import('shiki/core'),
      import('shiki/engine/javascript'),
    ])
    const theme = createCssVariablesTheme({ name: 'gitcove', variablePrefix: '--syntax-', fontStyle: true })
    return createHighlighterCore({ themes: [theme], langs: [], engine: createJavaScriptRegexEngine() })
  })()
  return highlighter
}

/** A cheap fingerprint of the diff's text, so a re-fetched but identical diff reuses its tokens. */
function fingerprint(diff: FileDiff, lang: string): string {
  let h = 2166136261
  for (const hunk of diff.hunks) {
    for (const l of hunk.lines) {
      const s = l.kind + l.text
      for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619)
    }
  }
  return `${lang}:${diff.path}:${h >>> 0}`
}

const clean = (t: string) => (t.endsWith('\r') ? t.slice(0, -1) : t)

/**
 * Colours every line of a diff. Each hunk's old side (context + removals) and new side
 * (context + additions) is tokenized as one block, so multi-line strings and comments
 * inside a hunk come out right.
 */
export async function highlightDiff(diff: FileDiff): Promise<DiffTokens | null> {
  const lang = languageFor(diff.path)
  if (!lang || diff.binary || diff.tooLarge) return null
  if (diff.hunks.reduce((n, h) => n + h.lines.length, 0) > MAX_LINES) return null
  const key = fingerprint(diff, lang)
  const hit = cache.get(key)
  if (hit) return hit

  const hl = await getHighlighter()
  if (!loaded.has(lang)) {
    const mod = await LOADERS[lang]()
    await hl.loadLanguage(mod.default as Parameters<HighlighterCore['loadLanguage']>[0])
    loaded.add(lang)
  }

  const convert = (t: { content: string; color?: string; fontStyle?: number }): Tok => ({
    text: t.content,
    color: t.color,
    italic: !!t.fontStyle && (t.fontStyle & 1) === 1,
  })

  const out: DiffTokens = diff.hunks.map((hunk) => {
    const oldIdx: number[] = []
    const newIdx: number[] = []
    const oldText: string[] = []
    const newText: string[] = []
    hunk.lines.forEach((l, i) => {
      if (l.kind === 'context' || l.kind === 'del') {
        oldIdx.push(i)
        oldText.push(clean(l.text))
      }
      if (l.kind === 'context' || l.kind === 'add') {
        newIdx.push(i)
        newText.push(clean(l.text))
      }
    })
    const res: (Tok[] | null)[] = hunk.lines.map(() => null)
    if (newText.length) {
      hl.codeToTokensBase(newText.join('\n'), { lang, theme: 'gitcove' }).forEach((line, j) => {
        res[newIdx[j]] = line.map(convert)
      })
    }
    if (oldText.length) {
      hl.codeToTokensBase(oldText.join('\n'), { lang, theme: 'gitcove' }).forEach((line, j) => {
        const i = oldIdx[j]
        if (hunk.lines[i].kind === 'del') res[i] = line.map(convert)
      })
    }
    return res
  })

  cache.set(key, out)
  if (cache.size > 60) cache.delete(cache.keys().next().value!)
  return out
}
