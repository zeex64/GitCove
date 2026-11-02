import { defineConfig, type Plugin } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

/**
 * WebKitGTK (Tauri's Linux webview) can run modules from its disk cache after a restart without
 * asking the server, so a component's old code asks for styles the server no longer has and the
 * window renders unstyled. In dev, nothing the server sends may be stored.
 */
function noStoreInDev(): Plugin {
  return {
    name: 'gitcove:no-store-in-dev',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use((_req, res, next) => {
        const set = res.setHeader.bind(res)
        res.setHeader = ((name: string, value: number | string | readonly string[]) =>
          set(name, name.toLowerCase() === 'cache-control' ? 'no-store' : value)) as typeof res.setHeader
        res.setHeader('Cache-Control', 'no-store')
        next()
      })
    },
  }
}

// Tauri expects a fixed port and must see Rust compile errors in the terminal.
export default defineConfig({
  plugins: [svelte(), noStoreInDev()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  // Syntax grammars are split per language and load only when a diff needs them; the largest (C++) is ~800 kB.
  build: { target: 'es2022', chunkSizeWarningLimit: 1000 },
})
