const MIN = 60
const HOUR = 60 * MIN
const DAY = 24 * HOUR

/** "just now", "4 min ago", "3 h ago", "yesterday", "5 days ago", "3 weeks ago", then a date. */
export function relativeTime(unix: number, now = Date.now() / 1000): string {
  const d = Math.max(0, now - unix)
  if (d < 45) return 'just now'
  if (d < HOUR) return `${Math.round(d / MIN)} min ago`
  if (d < DAY) return `${Math.round(d / HOUR)} h ago`
  if (d < 2 * DAY) return 'yesterday'
  if (d < 7 * DAY) return `${Math.floor(d / DAY)} days ago`
  if (d < 30 * DAY) {
    const w = Math.floor(d / (7 * DAY))
    return w === 1 ? '1 week ago' : `${w} weeks ago`
  }
  return absoluteDate(unix, now)
}

export function absoluteDate(unix: number, now = Date.now() / 1000): string {
  const date = new Date(unix * 1000)
  const sameYear = date.getFullYear() === new Date(now * 1000).getFullYear()
  return date.toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    year: sameYear ? undefined : 'numeric',
  })
}

export function fullDate(unix: number): string {
  return new Date(unix * 1000).toLocaleString(undefined, {
    weekday: 'short',
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

/** "src/lib/graph/lanes.ts" → { dir: "src/lib/graph/", name: "lanes.ts" } */
export function splitPath(path: string): { dir: string; name: string } {
  const i = path.lastIndexOf('/')
  return i === -1 ? { dir: '', name: path } : { dir: path.slice(0, i + 1), name: path.slice(i + 1) }
}

export function plural(n: number, one: string, many = one + 's'): string {
  return `${n} ${n === 1 ? one : many}`
}

export function initials(name: string): string {
  const parts = name.replace(/\[bot\]$/, '').trim().split(/[\s._-]+/).filter(Boolean)
  if (parts.length === 0) return '?'
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase()
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase()
}

/** Shortens long branch names in the middle: "feature/very-long-name-here" → "feature/ver…me-here". */
export function middleTruncate(text: string, max = 28): string {
  if (text.length <= max) return text
  const keep = max - 1
  const head = Math.ceil(keep / 2)
  return text.slice(0, head) + '…' + text.slice(text.length - (keep - head))
}

export function repoNameFromUrl(url: string): string {
  const cleaned = url.trim().replace(/\/+$/, '').replace(/\.git$/, '')
  const i = Math.max(cleaned.lastIndexOf('/'), cleaned.lastIndexOf(':'))
  return i === -1 ? cleaned : cleaned.slice(i + 1)
}

let opCounter = 0
export function newOpId(): string {
  opCounter += 1
  return `op-${Date.now().toString(36)}-${opCounter}`
}
