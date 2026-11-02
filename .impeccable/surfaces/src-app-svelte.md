---
version: 1
slug: "src-app-svelte"
primary_target: "src/App.svelte"
related_targets: []
---

# Main repository window

Mode: Operate. Scope: the whole main window (repo tabs, labelled toolbar, Changes and History tabs, diff and commit views, every repository state, first-run screen). Production build against real repositories. Composition reference: GitHub Desktop's Changes and History screens (user-supplied screenshots, 2026-11-01), carried in GitCove's own graphite world.

Job: open a repo and read where things stand, review each change in a roomy diff, pick exactly what goes into the commit, and commit. Repos are small to mid-size (up to ~20k commits, a few dozen branches); several stay open as tabs.

Confirmed decisions (2026-11-01, replacing the earlier three-column layout at the user's request): GitHub Desktop's model. A left column with **Changes** and **History** tabs; the main area holds the big diff. Changes: one list of changed files with a checkbox each (checked = staged in git's index; mixed = partly staged), a stashed-changes row, and a commit box with the user's avatar and a full-width "Commit to <branch>" button. History: the commit graph as two-line rows (summary with ref chips; avatar, author, time), then the selected commit's details, files and diff in the main area. Diffs: unified or side-by-side, syntax coloured, context expandable, and changed lines included or excluded by clicking the line-number gutter. Toolbar: labelled segments (Current repository, Current branch, a Fetch/Pull/Push origin button with last-fetched time, Undo naming what it reverses) plus command search. Branches, remote branches, pull requests and tags live in the Current branch menu; there is no ref sidebar and no status bar. Kept: the WIP row atop the graph, undo history in the Undo segment's dropdown, previews drawn on the graph before destructive operations, the Ctrl+K palette.

Anti-goals: crammed toolbars, spinners over content, modal dialogs for routine actions, a cramped diff.

Out of scope: settings screen, pull request detail page.

Open: whether a light theme ever exists.

## Direction contract

THESIS: GitHub Desktop's calm, diff-first layout played straight in GitCove's dark graphite world, whose one owned idea is that nothing destructive happens unseen: rebases, resets, force-pushes, deletes and discards are drawn on the graph before they run, and every operation can be undone. It refuses the button-crammed toolbar and the cramped diff.

OWN-WORLD: Graphite dark grounds #151618 / #1E1F22 / #2B2D31 / #35383E, text #E3E5E8 / #A1A6AE, hairlines #33363C. Rounded: 8px controls, 10px inputs and menus, 12px popovers. Lanes in saturated blue #4FA3FF, orange #F27A54, violet #B58CFF, green #3DDC97 (then amber, cyan, pink, lime), drawn as smooth curves with round nodes; coloured fills carry dark text. Inter throughout, JetBrains Mono only for hashes and code; code in diffs takes a restrained syntax palette. Lucide icons, one stroke.

STORY: You open a repo tab and the toolbar tells you where you stand: which repository, which branch, whether to fetch, pull or push, and what Undo would reverse. Changes lists every edited file; you tick what goes in, read each diff at full width, click lines in or out, and commit to the named branch. History shows the graph; pick a commit and read it the same way. Before anything risky you see its result drawn on the graph and apply or cancel.

FIRST VIEWPORT: 38px title strip with repo tabs and window controls; 56px toolbar of labelled segments (Current repository, Current branch, Fetch/Pull/Push origin, Undo) with command search right. A 340px left column (resizable) with Changes / History tabs; the main area fills the rest with the selected file's diff (or the selected commit), under any operation or preview banner.

FORM: canon, the standing exit, now in GitHub Desktop's arrangement. Signature interaction: the preview on the graph (ghost commits, struck replaced commits, a consequence readout, Apply/Cancel), live while dragging a branch chip onto another commit. Motion grammar: 120–200 ms ease-out state changes; the preview's ghost-in is the only authored transition. seed:bc0b368f42a18438

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
