import { isDesktop } from './index'

export const isMac = typeof navigator !== 'undefined' && /Mac/.test(navigator.userAgent)
export const modKey = isMac ? '⌘' : 'Ctrl'

/** Folder picker. In browser previews it hands out the demo repositories in turn. */
export async function pickFolder(title: string): Promise<string | null> {
  if (isDesktop) {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const picked = await open({ directory: true, multiple: false, title })
    return typeof picked === 'string' ? picked : null
  }
  const { DEMO_ROOT } = await import('./demo')
  const names = ['dotfiles', 'blog', 'GitCove']
  const name = names[pickCount % names.length]
  pickCount++
  return `${DEMO_ROOT}/${name}`
}
let pickCount = 0

export async function openFile(path: string) {
  if (!isDesktop) return
  const { openPath } = await import('@tauri-apps/plugin-opener')
  await openPath(path)
}

export async function revealFile(path: string) {
  if (!isDesktop) return
  const { revealItemInDir } = await import('@tauri-apps/plugin-opener')
  await revealItemInDir(path)
}

export async function openUrl(url: string) {
  if (!isDesktop) {
    window.open(url, '_blank', 'noopener')
    return
  }
  const { openUrl } = await import('@tauri-apps/plugin-opener')
  await openUrl(url)
}

export async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text)
  } catch {
    /* clipboard can be unavailable without focus; nothing else to do */
  }
}

export async function windowAction(action: 'minimize' | 'toggleMaximize' | 'close') {
  if (!isDesktop) return
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow()[action]()
}

export async function startWindowDrag() {
  if (!isDesktop) return
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  await getCurrentWindow().startDragging()
}

/** Folders dropped onto the window. */
export async function onFolderDrop(handler: (paths: string[]) => void, hover: (over: boolean) => void) {
  if (!isDesktop) return () => {}
  const { getCurrentWebview } = await import('@tauri-apps/api/webview')
  return getCurrentWebview().onDragDropEvent((e) => {
    if (e.payload.type === 'over' || e.payload.type === 'enter') hover(true)
    else if (e.payload.type === 'leave') hover(false)
    else if (e.payload.type === 'drop') {
      hover(false)
      handler(e.payload.paths)
    }
  })
}
