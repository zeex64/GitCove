---
name: Repository Window (product name not yet chosen)
description: The category-standard dark Git client played straight, rounded and calm, where nothing destructive happens unseen.
colors:
  bg-0: "#151618"
  bg-1: "#1e1f22"
  bg-2: "#2b2d31"
  bg-3: "#35383e"
  bg-4: "#3e424a"
  line: "#33363c"
  line-strong: "#464a52"
  text: "#e3e5e8"
  text-2: "#a1a6ae"
  text-3: "#969ca5"
  accent: "#4fa3ff"
  accent-hover: "#6db3ff"
  accent-soft: "rgba(79, 163, 255, 0.14)"
  accent-line: "rgba(79, 163, 255, 0.5)"
  on-color: "#0e1013"
  success: "#3ddc97"
  success-hover: "#5be6a9"
  warning: "#f2c14e"
  warning-soft: "rgba(242, 193, 78, 0.12)"
  danger: "#ff6b6b"
  danger-hover: "#ff8585"
  danger-soft: "rgba(255, 107, 107, 0.13)"
  add-bg: "rgba(61, 220, 151, 0.11)"
  add-text: "#6ee7b3"
  del-bg: "rgba(255, 107, 107, 0.11)"
  del-text: "#ff9a9a"
  lane-0: "#4fa3ff"
  lane-1: "#f27a54"
  lane-2: "#b58cff"
  lane-3: "#3ddc97"
  lane-4: "#f2c14e"
  lane-5: "#5cd6e0"
  lane-6: "#f28cc0"
  lane-7: "#a3c76d"
typography:
  display:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "24px"
    fontWeight: 650
    letterSpacing: "-0.015em"
  title:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "15px"
    fontWeight: 600
    lineHeight: 1.35
  subtitle:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "14px"
    fontWeight: 600
    lineHeight: 1.5
  banner-title:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "13.5px"
    fontWeight: 600
  body:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.4
    fontFeature: "'cv11', 'ss01'"
  body-sm:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "12.5px"
    fontWeight: 400
  caption:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "12px"
    fontWeight: 400
  chip:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "11.5px"
    fontWeight: 600
  label:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "11px"
    fontWeight: 600
    letterSpacing: "0.06em"
  mono:
    fontFamily: "'JetBrains Mono', ui-monospace, 'SF Mono', 'Cascadia Code', monospace"
    fontSize: "12px"
    fontWeight: 400
rounded:
  sm: "6px"
  base: "8px"
  md: "10px"
  lg: "12px"
  full: "999px"
spacing:
  2xs: "4px"
  xs: "6px"
  sm: "8px"
  md: "10px"
  lg: "12px"
  xl: "16px"
  2xl: "24px"
components:
  button:
    backgroundColor: "{colors.bg-2}"
    textColor: "{colors.text}"
    rounded: "{rounded.base}"
    padding: "0 12px"
    height: "30px"
  button-hover:
    backgroundColor: "{colors.bg-3}"
  button-active:
    backgroundColor: "{colors.bg-4}"
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-color}"
    rounded: "{rounded.base}"
    padding: "0 12px"
    height: "30px"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
  button-success:
    backgroundColor: "{colors.success}"
    textColor: "{colors.on-color}"
    rounded: "{rounded.base}"
    height: "30px"
  button-success-hover:
    backgroundColor: "{colors.success-hover}"
  button-danger:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.on-color}"
    rounded: "{rounded.base}"
    height: "30px"
  button-danger-hover:
    backgroundColor: "{colors.danger-hover}"
  button-ghost:
    textColor: "{colors.text}"
    rounded: "{rounded.base}"
    height: "30px"
  button-quiet:
    textColor: "{colors.text-2}"
    rounded: "{rounded.base}"
    height: "30px"
  button-sm:
    typography: "{typography.caption}"
    rounded: "{rounded.sm}"
    padding: "0 8px"
    height: "24px"
  icon-button:
    textColor: "{colors.text-2}"
    rounded: "{rounded.sm}"
    size: "26px"
  icon-button-hover:
    backgroundColor: "{colors.bg-3}"
    textColor: "{colors.text}"
  field:
    backgroundColor: "{colors.bg-2}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "0 10px"
    height: "32px"
  kbd:
    textColor: "{colors.text-2}"
    padding: "1px 5px"
  section-label:
    textColor: "{colors.text-2}"
    typography: "{typography.label}"
  ref-chip-branch:
    textColor: "{colors.on-color}"
    typography: "{typography.chip}"
    rounded: "{rounded.sm}"
    padding: "0 7px"
    height: "20px"
  ref-chip-tag:
    backgroundColor: "{colors.bg-3}"
    textColor: "{colors.text}"
    typography: "{typography.chip}"
    rounded: "{rounded.sm}"
    height: "20px"
  graph-row:
    backgroundColor: "{colors.bg-0}"
    textColor: "{colors.text}"
    height: "34px"
  graph-row-selected:
    backgroundColor: "{colors.accent-soft}"
  context-menu:
    backgroundColor: "{colors.bg-2}"
    rounded: "{rounded.md}"
    padding: "5px"
  context-menu-item-active:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-color}"
    rounded: "{rounded.sm}"
    height: "28px"
  prompt:
    backgroundColor: "{colors.bg-2}"
    rounded: "{rounded.lg}"
    padding: "14px"
    width: "320px"
  toast:
    backgroundColor: "{colors.bg-2}"
    rounded: "{rounded.lg}"
    padding: "10px 8px 10px 12px"
    width: "380px"
  command-palette:
    backgroundColor: "{colors.bg-2}"
    rounded: "{rounded.lg}"
    width: "620px"
  command-palette-item-active:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-color}"
    rounded: "{rounded.sm}"
    height: "34px"
---

# Design System: Repository Window

## Overview

**Creative North Star: "The Graphite Workbench"**

A dark, rounded desktop Git client in the GitKraken and Fork layout lineage, executed at GitHub Desktop's level of calm: ref sidebar on the left, the commit graph and history in the centre, the changes panel on the right, a thin status bar below. Nothing about the styling is novel on purpose. The surfaces are four close graphite grounds separated by hairlines, the type is one neutral sans at a dense 13px, and colour is spent almost entirely on meaning: the branch lanes, the blue of selection and primary action, and the green, amber and red of state.

The one owned idea is visible rather than decorative: destructive operations are drawn on the graph before they run (dashed ghost commits, struck-through replaced commits, a consequence readout, Apply or Cancel), and every finished operation offers Undo. Everything else stays out of the way. Routine actions happen in place, in anchored popovers and context menus, never in modal dialogs; the toolbar holds only Undo/Redo, Pull, Push and Branch, and the rest lives in the Ctrl+K palette.

Density is desktop-tool density: 34px graph rows, 28px list and menu rows, 30px buttons. Corners are soft everywhere (6 to 12px), which is a binding user commitment alongside the dark theme.

**Key Characteristics:**
- Four graphite grounds (bg-0 to bg-3, bg-4 for pressed) separated by 1px hairlines, no gradients or textures.
- One accent blue for selection, focus and primary actions; semantic green, amber and red for state.
- Eight saturated lane colours; lane 0 always belongs to the default branch.
- Coloured fills always carry near-black text.
- Inter for everything; JetBrains Mono only for literal machine strings.
- Rounded throughout: 6px small controls and chips, 8px buttons, 10px inputs and menus, 12px popovers, banners and toasts.
- No modals for routine work; the command palette is the only scrimmed overlay.

## Colors

Graphite neutrals carry the structure; colour appears only where it means something.

### Primary
- **Signal Blue** (accent): the single interactive accent. Selected graph rows (as the 14% soft tint plus a 2px inset left bar when the list has focus), focus rings (2px outline, 1px offset), focused field borders, the primary button, active rows in context menus and the command palette, the preview banner, and the text caret. Hover deepens to **Lifted Blue** (accent-hover). **Blue Wash** (accent-soft) and **Blue Rule** (accent-line) are its tint and border forms.

### Secondary (semantic states)
- **Commit Green** (success): the Commit button, added-file badges, "no conflicts expected" readouts, success toasts.
- **Caution Amber** (warning): modified-file badges, conflicted state, the operation-in-progress banner (with **Amber Wash**, warning-soft), and conflict-predicting ghost nodes.
- **Alarm Red** (danger): destructive buttons and menu items, the danger variant of the preview banner (with **Red Wash**, danger-soft), deleted-file badges, error toasts.
- **Diff pair**: added lines sit on **Add Wash** (add-bg) with **Add Mint** (add-text) marks and line numbers; deleted lines on **Delete Wash** (del-bg) with **Delete Rose** (del-text). The gutter takes the same hue at a lower 7% alpha.

### Tertiary (graph lanes)
- **Lane palette** (lane-0 to lane-7): blue, orange, violet, green, amber, cyan, pink, lime, in that order. Lanes are 2.2px round-capped strokes with smooth cubic curves between lanes and round 7.5px nodes ringed in bg-0. **Lane 0 is reserved for the default branch's first-parent line**: it always takes colour 0 and the leftmost lane while that line continues; every other branch cycles colours 1 to 7. Branch ref chips fill with their lane's colour, so a chip and its line always match.
- In conflict views, the user's own side takes Signal Blue and the other side takes lane-2 violet (13% tint plus 2px inset bar), whichever of git's "ours"/"theirs" that happens to be.

### Neutral
- **Graphite Night** (bg-0): the app floor, the history list, the title strip, inset field wells such as the sidebar filter and the toolbar search.
- **Graphite** (bg-1): sidebar, toolbar, changes panel, status bar, banner base.
- **Raised Graphite** (bg-2): buttons at rest, fields, menus, popovers, toasts, the palette.
- **Hover Graphite** (bg-3) and **Pressed Graphite** (bg-4): hover and active states; bg-3 also fills tag chips and neutral chips.
- **Hairline** (line) and **Strong Hairline** (line-strong): region dividers and list borders; strong hairline for ghost-button, field, menu and popover borders.
- **Paper White** (text), **Fog** (text-2), **Ash** (text-3): primary text, secondary text and labels, placeholders and struck or disabled content.
- **Ink** (on-color): the near-black text that sits on every coloured fill.

### Named Rules
**The Dark Ink Rule.** Every coloured fill (primary, success and danger buttons, branch ref chips, file-status badges, active menu and palette rows) carries Ink text, never white.

**The One Blue Rule.** Signal Blue means "selected, focused, or the main action here". It is not used for decoration, headings or idle icons.

**The Lane Zero Rule.** Colour 0 and the leftmost lane belong to the default branch. No other branch may take them.

## Typography

**UI Font:** Inter Variable (with Inter, system-ui, -apple-system, Segoe UI)
**Mono Font:** JetBrains Mono (with ui-monospace, SF Mono, Cascadia Code), ligatures off

**Character:** One neutral, highly legible sans carries the whole app at a dense 13px base with Inter's open-digit and alternate-glyph features (cv11, ss01). Mono is a functional tool, not a voice.

### Hierarchy
- **Display** (650, 24px, -0.015em): the first-run welcome heading only.
- **Title** (600, 15px, 1.35): the selected commit's summary in the commit detail panel.
- **Subtitle** (600, 14px): empty-state titles and the welcome lede (lede at 400, 1.5, max 60ch); the palette's search input is set at 14px too.
- **Banner title** (600, 13.5px): preview and operation banner headlines.
- **Body** (400, 13px, 1.4): everything by default: commit messages, list rows, menus, buttons (buttons at 500, coloured buttons at 600).
- **Body small** (12.5px): banner readouts and sub-lines, sidebar filter input.
- **Caption** (12px): small buttons, hints and shortcuts in menus and the palette, toast links, field labels in prompts.
- **Chip** (600, 11.5px): ref chips, conflict marker labels, counts, palette group names.
- **Label** (600, 11px, 0.06em, uppercase): list-section headers (sidebar sections, Unstaged/Staged files, graph column headers). Keyboard hints (kbd) also use 11px at 500, not uppercase.
- **Mono** (400, 12px): hashes, diff lines and hunk headers, file paths in diff headers, and branch names or SHAs quoted inside a sentence.

### Named Rules
**The Literal String Rule.** Mono marks a literal machine string: a SHA, a code line, a path in a diff header, a ref name quoted inside prose. Lists of branches, files and ref chips stay in Inter. Mono never appears as styling for headings or labels.

**The Section Header Rule.** The uppercase tracked label names a list group (Local, Remote, Unstaged files, graph columns). It never sits above a heading as an eyebrow.

## Layout

A fixed desktop chrome around a three-column workspace:

- **Title strip** 38px on bg-0 with repo tabs (28px) and the app's own window controls (46px wide).
- **Toolbar** 64px on bg-1: repo and branch pickers left, icon-over-label action buttons centred (62 by 48px; Undo carries a dropdown for history), command search right (32px, up to 300px). A container query at 1180px drops the repo pill's text and the search's placeholder text.
- **Main grid**: sidebar, 1px resizer, history (minmax(0, 1fr)), 1px resizer, changes panel.
  - **Sidebar** defaults to 248px. Below a 1200px window, or when the user collapses it, it becomes a 52px icon rail.
  - **Changes panel** auto-sizes until the user drags it: about 28% of the window, at least 420px (360px below 1200px), clamped so the history keeps at least 480px and the panel never exceeds 960px. Once dragged, the user's width persists (capped at 380px below 1200px).
  - **History** is a size container. Columns are graph, message, author 150px, date 104px, SHA 76px; they tighten at 900px, then the SHA steps aside at 680px, the author at 640px, the date at 420px, leaving graph and message.
  - Below 760px of window height the changes panel compresses its lists, diff header and commit box.
- **Status bar** 26px on bg-1.

**Spacing rhythm** runs on small steps: 4, 6, 8, 10, 12, 16, 24px. Gaps inside rows are 4 to 8px; panel padding is 10 to 16px; banners sit 8px in from the edges. **Fixed row heights** set the vertical rhythm: graph rows 34px, sidebar items and menu rows 28px, section headers 32px, palette results 34px, column header 30px.

## Elevation & Depth

Depth is tonal by default: regions are separated by ground steps and 1px hairlines, not shadows. Exactly one shadow exists, and it only appears on things that float above the workspace.

### Shadow Vocabulary
- **Pop** (`box-shadow: 0 14px 36px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.35)`): context menus, anchored prompts and popovers, toasts, the command palette.

### Motion
One easing for everything: a fast ease-out (`cubic-bezier(0.16, 1, 0.3, 1)`). Hover and colour changes run 120ms. Floating surfaces enter with a short fade plus a 3 to 8px translate: menus 120ms, prompts and popovers 140ms, the palette 150ms (its scrim fades 120ms), diff toolbars 160ms, banners drop in at 180ms, toasts rise 200ms. Chevrons rotate in 150ms; progress bars ease their width over 200ms. The signature transition is the preview's **ghost-in** (220ms): ghost nodes scale up from 0.6 while fading in, and ghost rows fade in alongside. Loading uses a 1s linear spinner, a 1.4s shimmer for skeleton rows, and a 1s indeterminate bar inside the preview banner. Under reduced motion every animation and transition collapses to 1ms.

### Named Rules
**The Float-Only Shadow Rule.** Only surfaces that float above the workspace get the Pop shadow. Panels, banners, rows and buttons stay flat.

## Shapes

Soft, consistent rounding is a binding brand commitment. Small controls, chips, menu rows, list items and icon buttons use 6px; buttons and segmented controls 8px; inputs and context menus 10px; banners, toasts, prompts, popovers and the palette 12px. Graph nodes, avatars and banner icon discs are full circles. Borders are always 1px hairlines; dashed 1px outlines mark things that do not exist yet (ghost chips, the "new" badge, WIP and ghost nodes). Struck-through text and a horizontal strike through a hollow node mark things that are about to be replaced.

## Components

### Buttons
Quiet, compact and uniform: one shape for every button.
- **Shape:** gently rounded (8px), 30px tall, 12px side padding, weight 500, 6px icon gap.
- **Default:** Raised Graphite with Paper White text; hover to bg-3, pressed to bg-4; disabled at 45% opacity.
- **Primary / Success / Danger:** Signal Blue, Commit Green or Alarm Red fill with Ink text at weight 600, each with a lighter hover. Success is reserved for the Commit action; danger for confirming a destructive step.
- **Ghost:** transparent with a Strong Hairline border (secondary actions such as Cancel or "Use theirs").
- **Quiet:** transparent with Fog text, filling on hover (Stage all, Discard hunk).
- **Small:** 24px tall, 8px padding, 12px text, 6px radius.
- **Icon button:** 26px square, 6px radius, Fog icon, fills bg-3 on hover.
- **Toolbar actions:** Lucide icon over a short label, 62 by 48px, single stroke weight.

### Chips
- **Branch ref chip:** 20px, 6px radius, 11.5px weight 600, filled with its lane colour and Ink text, max 220px with ellipsis. Draggable onto another row to start a preview.
- **Remote chip:** transparent with a 1px inset ring and text in the lane colour, plus a cloud icon.
- **Tag chip:** bg-3 fill with Paper White text and a tag icon.
- **Ghost chip:** dashed outline in the lane colour (where a ref will land). **Struck chip:** line-through at 45% opacity (where it is leaving).

### Inputs / Fields
- **Style:** 32px, 10px radius, Strong Hairline border, Raised Graphite fill (bg-1 inside popovers so the field reads against the bg-2 surface), Ash placeholder.
- **Focus:** the border turns Signal Blue; no outline glow. Inset wells (sidebar filter, toolbar search) use bg-0 with a Hairline border.
- **Textarea:** same shape, 8px by 10px padding, line-height 1.45, no resize handle.
- **Keyboard hint:** 11px Inter, Strong Hairline border, 5px radius.

### Navigation
- **Sidebar:** bg-1. Collapsible sections (Local, Remote, Pull requests, Tags, Stashes) under uppercase section labels with counts; 28px items with 6px radius; items hover to bg-2; the checked-out branch sits on bg-3 at weight 600 with ahead/behind counts (sidebar rows never take Signal Blue). An add button appears on section hover.
- **Rail:** below 1200px the sidebar becomes a 52px column of icon buttons that reopen a section.
- **Repo tabs:** in the 38px title strip; the active tab is a raised pill with a coloured status dot.

### Graph Rows (signature)
- 34px rows on Graphite Night. A WIP row sits on top with a dashed hollow node and staged/unstaged counts.
- Hover adds a 2.5% white wash; selection uses Blue Wash plus a 2px inset Signal Blue bar when the list is focused; a drop target gets a 20% blue fill with a Blue Rule ring.
- Node sizes: 7.5px radius commits, smaller for merges, a 1.6px ring 3.5px outside the HEAD commit.
- Ghost commits: dashed hollow nodes and dashed lane strokes, Fog text, a dashed "new" badge. Replaced commits: hollow Ash node with a strike, row text struck through in Ash.

### Preview Banner (signature)
Sits 8px in above the history: 12px radius, Blue Rule border, Blue Wash over bg-1 (danger variant: red border and Red Wash). A 30px circular icon disc, a 13.5px title prefixed by "Preview:" in Fog, a readout line of green/amber/red consequences, a segmented control for alternatives (Rebase / Merge), then Cancel and the primary Apply. It says plainly that the user can undo.

### Operation Banner
Same shape as the preview banner in amber: shown while a merge or rebase is stopped, with the conflict count and Abort (ghost), Skip commit (quiet, when available) and Continue (primary, disabled while conflicts remain).

### Conflict Regions
Diff lines between conflict markers are grouped and labelled by the user's side, not git's: the user's side ("Yours") in Blue Wash with a blue inset bar, the other side in a violet wash with a lane-2 bar, the base in a faint white wash with an Ash bar. Markers become 24px labelled rules on bg-1 with 11.5px Inter labels. Conflicted files list with ghost small buttons to keep one side.

### Toasts
Bottom right above the status bar, 380px, bg-2, 12px radius, Strong Hairline border, Pop shadow, rising in over 200ms. A coloured status icon (green, red, blue), the message, and an action button, usually Undo naming what it reverses; error output appears in a scrollable bg-0 block.

### Context Menus
bg-2, 10px radius, 5px padding, Pop shadow, 220 to 340px wide. 28px rows with right-aligned shortcut hints; the active row fills Signal Blue with Ink text; destructive items are red and fill red when active; separators are Strong Hairline rules.

### Anchored Prompts
Routine inputs (new branch, tag, rename branch) open as a 320px popover anchored to the control that asked: bg-2, 12px radius, 14px padding, Pop shadow, a 13px title, labelled fields, and right-aligned actions. No scrim.

### Command Palette
Ctrl+K opens a 620px panel 90px from the top over a light 45% scrim: 48px search row at 14px, grouped results in 34px rows, the active result filled Signal Blue with Ink text, shortcut hints at 12px.

## Do's and Don'ts

### Do:
- **Do** put Ink (#0e1013) text on every coloured fill: buttons, ref chips, file badges, active menu rows.
- **Do** reserve Signal Blue for selection, focus and the primary action in a group.
- **Do** keep lane colour 0 and the leftmost lane for the default branch, and colour branch chips with their lane colour.
- **Do** draw destructive operations on the graph first (dashed ghosts, struck replacements, a readout, Apply/Cancel) and offer Undo in a toast afterward.
- **Do** handle routine input in anchored popovers, context menus and the Ctrl+K palette.
- **Do** separate regions with ground steps and 1px hairlines; give the Pop shadow only to floating surfaces.
- **Do** use the shared control vocabulary: 30px buttons at 8px radius, 24px small buttons, 26px icon buttons, 32px fields at 10px radius.
- **Do** label conflict regions by the user's side (Yours in blue, the other side in violet).
- **Do** use the single ease-out curve and keep state changes between 120 and 220ms.

### Don't:
- **Don't** open modal dialogs for routine actions; the palette is the only scrimmed overlay.
- **Don't** crowd the toolbar beyond Undo/Redo, Pull, Push and Branch; further actions belong in the palette and context menus.
- **Don't** set white text on coloured fills.
- **Don't** use Signal Blue as decoration, or for idle icons and headings.
- **Don't** use JetBrains Mono for headings, labels, lists of refs or file lists.
- **Don't** put the uppercase section label above a heading as an eyebrow.
- **Don't** use sharp corners; nothing in the app is square-cornered except 1px rules.
- **Don't** add a light theme or per-OS styling without a product decision; the app is dark and uses one design language across Linux, macOS and Windows.
- **Don't** show the placeholder folder name as a product name or invent a logo.
