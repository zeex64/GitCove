import type { DiffLine, FileDiff } from '../api/types'

/** A line inside a diff, addressed by hunk and line index. */
export type LineKey = `${number}:${number}`

export function lineKey(hunk: number, line: number): LineKey {
  return `${hunk}:${line}`
}

/** Every changeable (+/-) line of the given hunks. */
export function changedLines(diff: FileDiff, hunks?: number[]): Set<LineKey> {
  const out = new Set<LineKey>()
  diff.hunks.forEach((h, hi) => {
    if (hunks && !hunks.includes(hi)) return
    h.lines.forEach((l, li) => {
      if (l.kind === 'add' || l.kind === 'del') out.add(lineKey(hi, li))
    })
  })
  return out
}

/**
 * Builds a patch containing only the selected +/- lines of `diff`.
 *
 * Forward patches (staging from the worktree diff) must match the index as their preimage,
 * so unselected removals stay as context and unselected additions are dropped.
 * Reverse patches (unstaging from the staged diff, or discarding from the worktree diff)
 * must match the target as their postimage, so unselected additions become context
 * and unselected removals are dropped.
 *
 * Returns null when nothing selected survives.
 */
export function buildPatch(diff: FileDiff, selected: Set<LineKey>, reverse: boolean): string | null {
  const oldPath = diff.oldPath ?? diff.path
  const isNew = diff.status === 'untracked' || diff.status === 'added'
  const isDeleted = diff.status === 'deleted'

  const out: string[] = []
  let delta = 0
  let any = false

  diff.hunks.forEach((h, hi) => {
    const body: string[] = []
    let oldCount = 0
    let newCount = 0
    let changed = false
    let lastKept = false

    const push = (marker: string, l: DiffLine) => {
      body.push(marker + l.text)
      lastKept = true
    }

    h.lines.forEach((l, li) => {
      const sel = selected.has(lineKey(hi, li))
      switch (l.kind) {
        case 'context':
          push(' ', l)
          oldCount++
          newCount++
          break
        case 'add':
          if (sel) {
            push('+', l)
            newCount++
            changed = true
          } else if (reverse) {
            push(' ', l)
            oldCount++
            newCount++
          } else {
            lastKept = false
          }
          break
        case 'del':
          if (sel) {
            push('-', l)
            oldCount++
            changed = true
          } else if (!reverse) {
            push(' ', l)
            oldCount++
            newCount++
          } else {
            lastKept = false
          }
          break
        case 'noNewline':
          if (lastKept) body.push('\\ No newline at end of file')
          break
      }
    })

    if (!changed) return
    any = true
    // The side git matches against is identical to the original hunk's, so its start is
    // unchanged; the other side shifts by what earlier kept hunks added or removed.
    const oldStart = reverse ? h.newStart - delta : h.oldStart
    const newStart = reverse ? h.newStart : h.oldStart + delta
    delta += newCount - oldCount
    out.push(`@@ -${oldStart},${oldCount} +${newStart},${newCount} @@`)
    out.push(...body)
  })

  if (!any) return null

  const header = [`diff --git a/${oldPath} b/${diff.path}`]
  if (isNew && !reverse) {
    header.push('new file mode 100644', '--- /dev/null', `+++ b/${diff.path}`)
  } else if (isDeleted && selectsWholeFile(diff, selected)) {
    header.push('deleted file mode 100644', `--- a/${oldPath}`, '+++ /dev/null')
  } else {
    header.push(`--- a/${oldPath}`, `+++ b/${diff.path}`)
  }
  return header.concat(out).join('\n') + '\n'
}

function selectsWholeFile(diff: FileDiff, selected: Set<LineKey>): boolean {
  return changedLines(diff).size === selected.size
}
