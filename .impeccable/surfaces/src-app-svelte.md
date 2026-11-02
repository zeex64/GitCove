---
version: 1
slug: "src-app-svelte"
primary_target: "src/App.svelte"
related_targets: []
---

# Main repository window

Mode: Operate. Scope: the whole main window (repo tabs, toolbar, ref sidebar, commit graph and list, changes panel, status bar, every repository state, first-run screen). Production build against real repositories. Composition reference: `.impeccable/mocks/decision/canon.html` (code-led; no comp).

Job: open a repo and, side by side, read where things stand and commit. Repos are small to mid-size (up to ~20k commits, a few dozen branches); several stay open as tabs.

Confirmed decisions: toolbar trimmed to Undo/Redo, Pull, Push, Branch (everything else in the Ctrl+K palette and context menus); a WIP row at the top of the graph; undo history in the Undo button's dropdown; destructive operations preview on the graph before running. Anti-goals: crammed toolbars, a hidden graph, spinners over content, modal dialogs for routine actions.

Out of scope: settings screen, hosting sign-in, pull request detail page.

Open: final product name (window shows the repo name only), whether a light theme ever exists.

## Direction contract

THESIS: The category-standard dark Git client played straight (ref sidebar, central commit graph, changes panel) whose one owned idea is that nothing destructive happens unseen: rebases, resets, force-pushes, deletes and discards are drawn on the graph before they run, and every operation can be undone. It refuses the button-crammed toolbar and the dumbed-down client that hides history.

OWN-WORLD: Graphite dark grounds #151618 / #1E1F22 / #2B2D31 / #35383E, text #E3E5E8 / #A1A6AE, hairlines #33363C. Rounded: 8px controls, 10px inputs and menus, 12px popovers. Lanes in saturated blue #4FA3FF, orange #F27A54, violet #B58CFF, green #3DDC97 (then amber, cyan, pink, lime), drawn as smooth curves with round nodes; coloured fills carry dark text. Inter throughout, JetBrains Mono only for hashes and code. Lucide icons, one stroke.

STORY: You open a repo tab and know at once where you stand: branch, ahead/behind, what landed, what is uncommitted. You stage lines and commit without losing the graph. Before anything risky you see its result drawn in place and apply or cancel; after anything, Undo names what it will reverse.

FIRST VIEWPORT: 38px title strip with repo tabs and window controls; 64px toolbar with repo and branch pickers left, Undo/Redo and Pull/Push/Branch as icon-over-label buttons centred, command search right. Three columns: 248px ref sidebar (Local, Remote, Pull requests, Tags, Stashes); the graph and commit list (graph, message with ref labels, author, date, SHA) topped by a WIP row; a 420px changes panel (unstaged, staged, diff, commit box with the green Commit button). 26px status bar.

FORM: canon, the standing exit, chosen over the ranked list (not on it). Signature interaction: the preview on the graph (ghost commits, struck replaced commits, a consequence readout, Apply/Cancel), live while dragging a branch onto another. Motion grammar: 150–200 ms ease-out state changes; the preview's ghost-in is the only authored transition. seed:bc0b368f42a18438

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
