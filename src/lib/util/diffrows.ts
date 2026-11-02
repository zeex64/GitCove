import type { DiffLine, Hunk } from '../api/types'

/** One cell of a side-by-side row: a line and where it sits in the diff. */
export interface Cell {
  hi: number
  li: number
  line: DiffLine
}

export interface SplitRow {
  left: Cell | null
  right: Cell | null
  /** "No newline at end of file" and similar notes span both sides. */
  meta?: Cell
}

/**
 * Pairs a hunk's lines into side-by-side rows: context on both sides, and each run of
 * removals beside the run of additions that follows it, padded with blanks on the shorter side.
 */
export function splitRows(h: Hunk, hi: number): SplitRow[] {
  const rows: SplitRow[] = []
  let dels: Cell[] = []
  let adds: Cell[] = []
  const flush = () => {
    for (let i = 0; i < Math.max(dels.length, adds.length); i++) rows.push({ left: dels[i] ?? null, right: adds[i] ?? null })
    dels = []
    adds = []
  }
  h.lines.forEach((line, li) => {
    const cell = { hi, li, line }
    if (line.kind === 'del') {
      if (adds.length) flush()
      dels.push(cell)
    } else if (line.kind === 'add') {
      adds.push(cell)
    } else if (line.kind === 'context') {
      flush()
      rows.push({ left: cell, right: cell })
    } else {
      flush()
      rows.push({ left: null, right: null, meta: cell })
    }
  })
  flush()
  return rows
}

/** Unchanged lines hidden above hunk `i` (0 when the hunk starts the file or follows directly). */
export function hiddenBefore(hunks: Hunk[], i: number): number {
  const h = hunks[i]
  if (!h || h.oldStart === 0) return 0
  if (i === 0) return Math.max(0, h.oldStart - 1)
  const p = hunks[i - 1]
  return Math.max(0, h.oldStart - (p.oldStart + p.oldLines))
}
