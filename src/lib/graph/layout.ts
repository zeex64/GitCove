import type { Commit } from '../api/types'

/** A commit as the graph draws it. Preview ghosts, replaced commits and the WIP row are flagged. */
export interface GraphCommit extends Commit {
  /** Will exist after the previewed operation; drawn hollow and dashed. */
  ghost?: boolean
  /** Stops being reachable after the previewed operation; drawn struck through. */
  void?: boolean
  /** A previewed step that would stop on a conflict. */
  conflicted?: boolean
  /** The uncommitted-changes row at the top. */
  wip?: boolean
}

export type SegStyle = 'solid' | 'ghost' | 'void'

/** One stroke inside a row. `top` strokes run from the row's top edge to its middle, `bottom` ones from the middle down. */
export interface Seg {
  from: number
  to: number
  color: number
  style: SegStyle
}

export interface GraphRow {
  commit: GraphCommit
  lane: number
  color: number
  top: Seg[]
  bottom: Seg[]
}

export interface GraphLayout {
  rows: GraphRow[]
  maxLanes: number
  /** Commit id → row index. */
  index: Map<string, number>
}

interface Lane {
  id: string
  color: number
  style: SegStyle
}

export const LANE_COLORS = 8

function styleOf(c: GraphCommit): SegStyle {
  if (c.ghost || c.wip) return 'ghost'
  if (c.void) return 'void'
  return 'solid'
}

function firstFree(lanes: (Lane | null)[], from = 0): number {
  for (let i = from; i < lanes.length; i++) if (lanes[i] === null) return i
  return Math.max(lanes.length, from)
}

/**
 * Assigns every commit a lane and the strokes that connect it to its children and parents.
 * `commits` must list children before parents (newest first, as `git log --date-order` does).
 * Commits on `mainChain` (the default branch's first-parent line) always take colour 0.
 */
export function layoutGraph(commits: GraphCommit[], mainChain: Set<string>): GraphLayout {
  const lanes: (Lane | null)[] = []
  const rows: GraphRow[] = []
  const index = new Map<string, number>()
  let nextColor = 1
  let maxLanes = 1

  const takeColor = (id: string) => {
    if (mainChain.has(id)) return 0
    const c = nextColor
    nextColor = nextColor + 1 >= LANE_COLORS ? 1 : nextColor + 1
    return c
  }

  // Lane 0 belongs to the default branch's first-parent line until that line ends,
  // so main stays leftmost even when newer commits on other branches come first.
  let mainPending = mainChain.size
  const allocate = (id: string) => {
    if (mainChain.has(id) && lanes[0] == null) return 0
    return firstFree(lanes, mainPending > 0 ? 1 : 0)
  }

  for (let i = 0; i < commits.length; i++) {
    const c = commits[i]
    index.set(c.id, i)

    const waiting: number[] = []
    for (let k = 0; k < lanes.length; k++) if (lanes[k]?.id === c.id) waiting.push(k)

    if (mainChain.has(c.id)) mainPending--

    let lane: number
    let color: number
    if (waiting.length === 0) {
      lane = allocate(c.id)
      color = takeColor(c.id)
    } else {
      // A fork point is waited on by several lanes; the main line keeps its lane.
      const mainLane = mainChain.has(c.id) ? waiting.find((k) => lanes[k]!.color === 0) : undefined
      lane = mainLane ?? waiting[0]
      color = mainChain.has(c.id) ? 0 : lanes[lane]!.color
    }

    const top: Seg[] = []
    const bottom: Seg[] = []

    for (let k = 0; k < lanes.length; k++) {
      const l = lanes[k]
      if (!l) continue
      if (l.id === c.id) {
        top.push({ from: k, to: lane, color: l.color, style: l.style })
      } else {
        top.push({ from: k, to: k, color: l.color, style: l.style })
        bottom.push({ from: k, to: k, color: l.color, style: l.style })
      }
    }
    for (const k of waiting) lanes[k] = null

    const style = styleOf(c)
    const [first, ...rest] = c.parents
    if (first !== undefined) {
      // Keep the lane even when another lane already waits for this parent:
      // branches run side by side and converge at their fork point.
      lanes[lane] = { id: first, color, style }
      bottom.push({ from: lane, to: lane, color, style })
    }
    for (const p of rest) {
      const existing = lanes.findIndex((l) => l?.id === p)
      if (existing >= 0) {
        bottom.push({ from: lane, to: existing, color: lanes[existing]!.color, style })
      } else {
        const k = allocate(p)
        const pc = takeColor(p)
        lanes[k] = { id: p, color: pc, style }
        bottom.push({ from: lane, to: k, color: pc, style })
      }
    }

    while (lanes.length && lanes[lanes.length - 1] === null) lanes.pop()
    maxLanes = Math.max(maxLanes, lanes.length, lane + 1)
    rows.push({ commit: c, lane, color, top, bottom })
  }

  return { rows, maxLanes, index }
}

/** The first-parent line of `tip`, used to keep the default branch on colour 0. */
export function firstParentChain(tip: string | null, byId: Map<string, Commit>): Set<string> {
  const chain = new Set<string>()
  let cur = tip
  while (cur && !chain.has(cur)) {
    chain.add(cur)
    cur = byId.get(cur)?.parents[0] ?? null
  }
  return chain
}
