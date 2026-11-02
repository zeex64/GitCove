# GitCove

A cross-platform desktop Git client: Tauri 2 (Rust) with a Svelte 5 + TypeScript interface. It runs your installed `git`, so your config, hooks, SSH keys and credential helpers all apply.

## Run it

Prerequisites: Node 22+, Rust (stable), Git, and the [Tauri system dependencies](https://tauri.app/start/prerequisites/) for your OS (on Linux: WebKitGTK 4.1).

```bash
npm install
npm run tauri dev
```

If Rust was installed with rustup without editing your shell profile, put cargo on your PATH first, for example `export PATH="$HOME/.cargo/bin:$PATH"` (fish: `fish_add_path ~/.cargo/bin`).

Build an installable bundle:

```bash
npm run tauri build
```

## GitHub sign-in

The account button in the title bar signs in with GitHub's device flow: the app shows a short code, you approve it on github.com, and the app never sees your password. Once signed in you can pick one of your repositories to clone, private repositories list their pull requests, and HTTPS fetch/pull/push to github.com use the sign-in.

GitHub needs an OAuth App client ID to know which app is asking. Either:

- **In the app:** the account button walks you through it. Create an OAuth App at https://github.com/settings/applications/new (any name, `http://localhost` for both URLs), tick **Enable Device Flow** on its page, and paste its Client ID.
- **At build time:** set `GITCOVE_GITHUB_CLIENT_ID` when building, so every copy of the app has it built in.

The token is stored in the system keychain (Secret Service/KWallet on Linux, Keychain on macOS, Credential Manager on Windows). If no keychain answers, it falls back to a private file in the app's settings folder and the account panel says so. For github.com, git asks the app itself for credentials (as `GIT_ASKPASS`) and your own credential helpers are skipped for github.com only, so the token is never written into them.

## Design previews in a browser

`npm run dev` serves the interface at http://localhost:1420 with an in-memory demo repository instead of the Rust backend. Add `?scenario=` to see other states: `conflict`, `empty`, `detached`, `clean`, `offline`, `welcome`, `preview`, `commit`. GitHub account states: `?gh=setup` or `?gh=signedin`, with `&open=account`, `login`, `clone` or `clone-picked`.

## Tests

```bash
npm test                         # partial-staging patches against a throwaway repo
cd src-tauri && cargo test       # git backend: parsing, operations, preview, undo
npm run check                    # Svelte and TypeScript checks
```

## Where things live

- `src/lib/api/types.ts`, `src/lib/api/backend.ts`: the contract between the UI and the Rust backend.
- `src/lib/state/`: per-repository state, actions, previews and undo.
- `src/lib/graph/layout.ts`: commit graph lane layout.
- `src/lib/components/`: the window's panels.
- `src-tauri/src/`: git commands, operation log and undo, previews, file watching, pull request listing.
- `PRODUCT.md`: product decisions. `.impeccable/`: design records.
