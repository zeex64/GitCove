import type { RepoStore } from './repo.svelte'

/**
 * Dragging a branch label onto a commit, built on pointer events rather than HTML5 drag and drop
 * (which webviews intercept for file drops on some platforms).
 */
class DragState {
  active = $state(false)
  label = $state('')
  x = $state(0)
  y = $state(0)
}

export const dragState = new DragState()

export function beginBranchDrag(e: MouseEvent, repo: RepoStore, branch: string) {
  if (e.button !== 0) return
  const startX = e.clientX
  const startY = e.clientY
  let started = false

  const move = (ev: MouseEvent) => {
    if (!started) {
      if (Math.hypot(ev.clientX - startX, ev.clientY - startY) < 5) return
      started = true
      dragState.active = true
      dragState.label = branch
      repo.startDrag(branch)
      document.body.classList.add('dragging')
    }
    dragState.x = ev.clientX
    dragState.y = ev.clientY
  }

  const finish = (drop: boolean) => {
    window.removeEventListener('mousemove', move)
    window.removeEventListener('mouseup', up)
    window.removeEventListener('keydown', key)
    document.body.classList.remove('dragging')
    if (!started) return
    dragState.active = false
    if (drop && repo.drag?.target) repo.drop()
    else repo.endDrag()
  }

  const up = () => finish(true)
  const key = (ev: KeyboardEvent) => ev.key === 'Escape' && finish(false)

  window.addEventListener('mousemove', move)
  window.addEventListener('mouseup', up)
  window.addEventListener('keydown', key)
}
