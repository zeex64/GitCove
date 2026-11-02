# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

A cross-platform desktop app (Linux, macOS, Windows) whose UI renders in Tauri's webview. It uses one design language across all three OSes rather than adapting per OS, so design work follows web rules, not iOS/Android native guidance.

## Stack

- **Shell and backend:** Tauri 2 (Rust core, system webview). Chosen by the user.
- **Frontend:** delegated. Svelte 5 + TypeScript on Vite. Reasoning: it is an official `create-tauri-app` template; fine-grained reactivity keeps long commit logs, large diffs and a live branch graph responsive without memoization work; and it has the least boilerplate for a project with one main developer. Virtualized lists are available through TanStack Virtual's Svelte adapter.
- **Git engine:** the user's installed `git` CLI, chosen by the user. The backend runs `git` and parses its output, so behavior matches the terminal: the user's config, hooks, SSH keys and credential helpers all apply. The operation log behind undo lives alongside it (see Capabilities).
- **Window frame:** custom title bar, chosen by the user. Repo tabs sit in the top strip with the app's own window controls; macOS keeps its native traffic lights.

## Users

The primary user is the person building it: a developer who works on Linux and uses git daily. The app is built for their own workflow first. Other developers may use it later, but no secondary audience has been confirmed or prioritized.

## Product Purpose

A modern desktop Git client in the same category as GitHub Desktop and GitKraken. It makes repository history easy to understand and makes risky operations safe. Success means the author reaches for it instead of the terminal or another GUI for everyday git work, on any of their machines.

## Positioning

Three commitments together set it apart from neighboring clients:

1. **History you can read.** The commit graph, branches, merges and rebases are understandable at a glance. The graph is the main way to see what a repository is doing, not a decorative side panel.
2. **Fearless operations.** Every operation can be undone, and destructive ones (rebase, reset, force-push, branch delete, discard) show a preview of what will happen first. Work cannot be lost through the app.
3. **First-class on Linux, equal everywhere.** Many well-regarded Git GUIs are Mac/Windows-first, GitHub-only, or paid. This one treats Linux as a primary platform, with full parity on macOS and Windows.

It also treats hosting providers as equals: GitHub, GitLab and Gitea/Forgejo integrations all have the same standing.

## Operating Context

- A desktop window, used alongside an editor and a terminal during normal development.
- Works on local repositories of any size. Remotes may be on GitHub, GitLab, Gitea/Forgejo, or any plain SSH/HTTPS git server.
- Hosting accounts are optional. Everything local, plus push/pull to any remote, must work without signing in.

## Capabilities and Constraints

**Confirmed direction**
- Visual commit history and branch graph as a core feature.
- Undo for every operation, and previews before destructive operations.
- Multi-host integration: GitHub, GitLab and Gitea/Forgejo, including viewing and creating pull/merge requests and cloning from an account.
- Built: GitHub sign-in through the OAuth device flow (token in the OS keychain), a picker of the user's repositories for cloning, pull requests on private GitHub repos, and the sign-in used for HTTPS fetch/pull/push to github.com.
- Built: GitHub Desktop-style Changes and History tabs. One checkbox per changed file (git's index stays the source of truth: checked means staged, mixed means partly staged), individual lines picked in or out by clicking the diff's line numbers (each pick is one undo step), syntax-coloured unified or side-by-side diffs with expandable context, a labelled toolbar with a smart Fetch/Pull/Push button, and setting the git name and email from the commit box when they're missing.
- Runs on Linux, macOS and Windows from one codebase.

**Implied baseline (from the category, not yet scoped):** staging (including hunks/lines), committing, branching, merging, rebasing, stashing, fetch/pull/push, and diff viewing.

**Open decisions**
- Logo and app icon (the name is settled; see Brand Commitments).
- Sign-in for GitLab and Gitea/Forgejo (GitHub is done).
- Who owns the GitHub OAuth App for distributed builds. Until one is baked in with `GITCOVE_GITHUB_CLIENT_ID`, each user registers their own and pastes its client ID.
- Licensing and distribution channels (packaging formats, stores, open source or not).
- Whether a light theme ever exists (dark is required; light is undecided).

## Brand Commitments

**Name: GitCove**, chosen by the user on 2026-11-01 (a cove: a sheltered place, matching "never lose work"). Written `GitCove` in UI and copy, `gitcove` in identifiers. `NewGitGUIProject` is only the folder name and must not appear in UI or copy. Caveat on record: Git's trademark policy disallows "Git" as a syllable in a product name, so wide distribution may need permission from the Software Freedom Conservancy or a rename. No logo or visual identity exists yet. The user's stated bar is a "modern, nice to use UI".

Set by the user (binding): **dark theme** and a **modern, rounded look**. Every visual direction must honor both.

Standing preference (chosen 2026-11-01 over more distinctive directions): **the category standard for dark Git clients, played straight**, executed at full craft with no novelty styling. The quality bar is **GitHub Desktop**: its clarity, plain-language copy and calm commit flow set the polish level to match.

Layout (chosen by the user on 2026-11-01, replacing the earlier GitKraken/Fork three-column layout): **GitHub Desktop's arrangement**. A left column with Changes and History tabs, the diff as the main area, branches, tags and pull requests in the Current branch menu, and a labelled toolbar. The commit graph lives in the History tab. The user rejects toolbars crammed with buttons; only the essential actions stay visible, and the rest live in the branch menu, a command palette and context menus.

## Evidence on Hand

None. The project folder was empty when this file was written. There are no screenshots, users, testimonials, benchmarks or performance figures. Future work must not invent any of these, or any claims about speed, adoption or comparisons with named competitors.

## Product Principles

1. **Never lose work.** Before the app does anything destructive, it explains what will change and offers a way back. Safety is the default, not a setting.
2. **Make the repository legible.** When a choice comes up, favor whatever helps the user understand the state of their history and branches over showing more controls.
3. **Built for real daily use.** Ground features in the author's actual workflow, not in a checklist borrowed from competitors.
4. **No second-class platform.** Linux, macOS and Windows get equal care, including packaging, credential storage, file dialogs, HiDPI and Wayland/X11 on Linux.
5. **Host-agnostic, account-optional.** Local git is complete without signing in. GitHub, GitLab and Gitea/Forgejo are peer integrations layered on top.
