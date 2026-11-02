import type { Backend } from './backend'
import { tauriBackend } from './tauri'

/** True inside the desktop app; false when the UI runs in a plain browser (design previews). */
export const isDesktop = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

let selected: Backend = tauriBackend

/** Browser previews swap in the in-memory demo repository before the app mounts. */
export async function initBackend(): Promise<void> {
  if (!isDesktop) {
    const { createDemoBackend } = await import('./demo')
    selected = createDemoBackend()
  }
}

export const backend: Backend = new Proxy({} as Backend, {
  get(_t, prop) {
    return (selected as unknown as Record<string | symbol, unknown>)[prop]
  },
})

export type { Backend }
