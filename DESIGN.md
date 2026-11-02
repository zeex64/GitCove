---
name: GitCove
description: GitCove, the category-standard dark Git client played straight in GitHub Desktop's arrangement, rounded and calm, where nothing destructive happens unseen.
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
  syntax-keyword: "#ff8f7a"
  syntax-string: "#8fd6e8"
  syntax-function: "#c9a7ff"
  syntax-constant: "#f2c97a"
  syntax-comment: "#8b919b"
typography:
  display:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "24px"
    fontWeight: 650
    letterSpacing: "-0.015em"
  heading:
    fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: "18px"
    fontWeight: 650
    letterSpacing: "-0.01em"
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
    lineHeight: "22px"
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
    rounded: "{rounded.sm}"
    padding: "1px 5px"
  section-label:
    textColor: "{colors.text-2}"
    typography: "{typography.label}"
  toolbar:
    backgroundColor: "{colors.bg-1}"
    height: "56px"
  toolbar-segment:
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "0 10px 0 12px"
    height: "46px"
  toolbar-segment-hover:
    backgroundColor: "{colors.bg-2}"
  toolbar-search:
    backgroundColor: "{colors.bg-0}"
    textColor: "{colors.text-2}"
    rounded: "{rounded.md}"
    height: "32px"
  pane-tab:
    textColor: "{colors.text-2}"
    height: "42px"
  pane-tab-active:
    textColor: "{colors.text}"
  tab-underline:
    backgroundColor: "{colors.accent}"
    rounded: "{rounded.full}"
    height: "2px"
  count-pill:
    backgroundColor: "{colors.bg-3}"
    textColor: "{colors.text}"
    rounded: "{rounded.full}"
    padding: "0 6px"
    height: "18px"
  count-pill-sync:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-color}"
    rounded: "{rounded.full}"
    height: "18px"
  checkbox:
    backgroundColor: "{colors.bg-0}"
    rounded: "{rounded.sm}"
    size: "16px"
  checkbox-checked:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-color}"
  file-row:
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "0 8px 0 10px"
    height: "30px"
  file-row-selected:
    backgroundColor: "{colors.bg-3}"
  status-badge:
    backgroundColor: "{colors.warning}"
    textColor: "{colors.on-color}"
    rounded: "{rounded.sm}"
    size: "17px"
  commit-button:
    backgroundColor: "{colors.success}"
    textColor: "{colors.on-color}"
    rounded: "{rounded.md}"
    height: "38px"
  commit-button-disabled:
    backgroundColor: "{colors.bg-3}"
    textColor: "{colors.text-2}"
  segmented:
    backgroundColor: "{colors.bg-0}"
    rounded: "{rounded.base}"
    padding: "2px"
    height: "26px"
  segmented-active:
    backgroundColor: "{colors.bg-3}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
  pane-header:
    backgroundColor: "{colors.bg-1}"
    padding: "0 8px 0 16px"
    height: "40px"
  diff-line:
    backgroundColor: "{colors.bg-0}"
    typography: "{typography.mono}"
    height: "22px"
  diff-gutter-in:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-color}"
    width: "48px"
  diff-gutter-out:
    backgroundColor: "{colors.bg-1}"
    textColor: "{colors.text-3}"
    width: "48px"
  hunk-header:
    backgroundColor: "{colors.bg-1}"
    textColor: "{colors.text-2}"
    height: "32px"
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
  history-row:
    backgroundColor: "{colors.bg-0}"
    textColor: "{colors.text}"
    height: "50px"
  history-row-selected:
    backgroundColor: "{colors.accent-soft}"
  popover:
    backgroundColor: "{colors.bg-2}"
    rounded: "{rounded.lg}"
  popover-item:
    rounded: "{rounded.sm}"
    padding: "5px 9px"
    height: "32px"
  popover-item-hover:
    backgroundColor: "{colors.bg-3}"
  bordered-list:
    backgroundColor: "{colors.bg-1}"
    rounded: "{rounded.lg}"
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

# Design System: GitCove

## Overview

**Creative North Star: "The Graphite Workbench"**

A dark, rounded desktop Git client in GitHub Desktop's arrangement, executed at GitHub Desktop's level of calm: a labelled toolbar across the top, a left column with Changes and History tabs, and the diff as the main area, roomy and full width. Branches, tags and pull requests live in the Current branch menu; there is no ref sidebar and no status bar. Nothing about the styling is novel on purpose. The surfaces are four close graphite grounds separated by hairlines, the type is one neutral sans at a dense 13px, and colour is spent almost entirely on meaning: the branch lanes, the blue of selection, inclusion and primary action, and the green, amber and red of state.

The one owned idea is visible rather than decorative: destructive operations are drawn on the graph before they run (dashed ghost commits, struck-through replaced commits, a consequence readout, Apply or Cancel), and every finished operation offers Undo. Everything else stays out of the way. Routine actions happen in place, in anchored popovers and context menus, never in modal dialogs; the toolbar holds four labelled segments (Current repository, Current branch, the Fetch/Pull/Push origin action, Undo) and a compact search, and the rest lives in the branch menu, the Ctrl+K palette and context menus.

Density is desktop-tool density: 30px file rows, 50px two-line history rows, 22px diff lines, 30px buttons. Corners are soft everywhere (6 to 12px), which is a binding user commitment alongside the dark theme.

**Key Characteristics:**
- Four graphite grounds (bg-0 to bg-3, bg-4 for pressed) separated by 1px hairlines, no gradients or textures.
- One accent blue for selection, focus, commit inclusion and primary actions; semantic green, amber and red for state.
- Eight saturated lane colours; lane 0 always belongs to the default branch.
- Coloured fills always carry near-black text.
- Inter for everything; JetBrains Mono only for literal machine strings, coloured in diffs by a restrained syntax palette.
- Rounded throughout: 6px small controls, checkboxes and chips, 8px buttons, 10px inputs, menus and toolbar segments, 12px popovers, banners, toasts and bordered lists; full-round pills and tab underlines.
- No modals for routine work; the command palette is the only scrimmed overlay.

## Colors

Graphite neutrals carry the structure; colour appears only where it means something.

### Primary
- **Signal Blue** (accent): the single interactive accent. Selected history rows (as the 14% soft tint plus a 2px inset left bar when the list has focus), the 2px underline of the active tab, ticked and mixed checkboxes, the diff gutter of lines that are in the commit, focus rings (2px outline, 1px offset), focused field borders, the primary button, the ahead/behind pills and progress line of the toolbar's origin action, active rows in context menus and the command palette, the preview banner, and the text caret. Hover deepens to **Lifted Blue** (accent-hover). **Blue Wash** (accent-soft) and **Blue Rule** (accent-line) are its tint and border forms.

### Secondary (semantic states)
- **Commit Green** (success): the full-width Commit button, added-file badges, the clean-state check, "no conflicts expected" readouts, the current-branch check in the branch menu, success toasts.
- **Caution Amber** (warning): modified-file badges, conflicted state (an amber-ringed badge), the operation-in-progress banner, the who-are-you prompt and conflict notes (on **Amber Wash**, warning-soft), the branch segment while a rebase is stopped, and conflict-predicting ghost nodes.
- **Alarm Red** (danger): destructive buttons and menu items, the danger variant of the preview banner (with **Red Wash**, danger-soft), deleted-file badges, error toasts.
- **Diff pair**: added lines sit on **Add Wash** (add-bg) with **Add Mint** (add-text) marks and line numbers; deleted lines on **Delete Wash** (del-bg) with **Delete Rose** (del-text). The gutter of an unpickable changed line takes the same hue at a lower 8% alpha.

### Tertiary (graph lanes)
- **Lane palette** (lane-0 to lane-7): blue, orange, violet, green, amber, cyan, pink, lime, in that order. Lanes are 2.2px round-capped strokes with smooth cubic curves between lanes, 16px apart, with round 7.5px nodes ringed in bg-0. **Lane 0 is reserved for the default branch's first-parent line**: it always takes colour 0 and the leftmost lane while that line continues; every other branch cycles colours 1 to 7. Branch ref chips fill with their lane's colour, so a chip and its line always match.
- In conflict views, the user's own side takes Signal Blue and the other side takes lane-2 violet (13% tint plus 2px inset bar), whichever of git's "ours"/"theirs" that happens to be.

### Code (syntax)
- A restrained palette for code in diffs, fed to the highlighter through CSS variables, each at least 5:1 on the diff floor: **Keyword Coral** (syntax-keyword), **String Ice** (syntax-string, also template expressions and links), **Function Lilac** (syntax-function), **Constant Wheat** (syntax-constant), **Comment Slate** (syntax-comment). Plain code and parameters stay Paper White; punctuation is Fog. Syntax colour sits on top of the diff washes and never replaces them.

### Neutral
- **Graphite Night** (bg-0): the app floor, the diff and history floors, the title strip, unticked checkboxes, inset wells such as the toolbar search and the segmented control.
- **Graphite** (bg-1): the toolbar, the left column, pane headers (diff header, commit header, hunk headers), the commit files column, the gutter of lines left out of the commit, bordered lists, banner base.
- **Raised Graphite** (bg-2): buttons at rest, fields, menus, popovers, toasts, the palette; the hover of toolbar segments and file rows.
- **Hover Graphite** (bg-3) and **Pressed Graphite** (bg-4): hover and active states; bg-3 also marks the selected file row, the active segment option, neutral count pills, tag chips, avatars' empty state and the disabled Commit button.
- **Hairline** (line) and **Strong Hairline** (line-strong): region dividers, toolbar separators and list borders; strong hairline for ghost-button, field, checkbox, menu and popover borders.
- **Paper White** (text), **Fog** (text-2), **Ash** (text-3): primary text, secondary text and captions, placeholders, line numbers and struck or disabled content.
- **Ink** (on-color): the near-black text that sits on every coloured fill.

### Named Rules
**The Dark Ink Rule.** Every coloured fill (primary, success and danger buttons, branch ref chips, file-status badges, ticked checkboxes, the in-commit gutter, sync count pills, active menu and palette rows) carries Ink text or glyphs, never white.

**The One Blue Rule.** Signal Blue means "selected, focused, included, or the main action here". It is not used for decoration, headings or idle icons.

**The Inclusion Gutter Rule.** In the workspace's lists and diffs, the commit-inclusion gutter is the one place Signal Blue fills a row element: line numbers of lines in the commit sit on solid Signal Blue in Ink at weight 600, lines left out sit on bg-1 in Ash. Selected rows use Blue Wash (history) or a bg-3 step (files, popover items), never a solid blue fill; solid blue rows belong only to the floating context menu and palette.

**The Lane Zero Rule.** Colour 0 and the leftmost lane belong to the default branch. No other branch may take them.

## Typography

**UI Font:** Inter Variable (with Inter, system-ui, -apple-system, Segoe UI)
**Mono Font:** JetBrains Mono (with ui-monospace, SF Mono, Cascadia Code), ligatures off

**Character:** One neutral, highly legible sans carries the whole app at a dense 13px base with Inter's open-digit and alternate-glyph features (cv11, ss01). Mono is a functional tool, not a voice.

### Hierarchy
- **Display** (650, 24px, -0.015em): the first-run welcome heading only.
- **Heading** (650, 18px, -0.01em): main-area headings ("No local changes", "What happens if you apply this"), followed by a Fog lede at 1.5 line height, max about 60ch.
- **Title** (600, 15px, 1.35): the selected commit's summary in the commit header.
- **Subtitle** (600, 14px): small empty-state titles and the welcome lede (lede at 400, 1.5, max 60ch); the palette's search input is set at 14px too.
- **Banner title** (600, 13.5px): preview and operation banner headlines.
- **Body** (400, 13px, 1.4): everything by default: commit messages, list rows, menus, buttons (buttons at 500, coloured buttons at 600); toolbar segment values and active tabs at 600, inactive tabs at 500.
- **Body small** (12.5px): banner readouts, the commit meta row, next-step sub-lines, notes and hints.
- **Caption** (12px): small buttons, the diff inclusion readout, segmented-control options, history sub-lines, shortcuts in menus and the palette, toast links, field labels in prompts.
- **Chip** (600, 11.5px): ref chips, conflict marker labels, palette group names; at 400 it also sets toolbar captions ("Current branch"), diff stats, character counts and popover sub-lines.
- **Label** (600, 11px, 0.06em, uppercase): list-section headers (Local branches, Remote branches, Tags in the branch menu; the grouped lists of the preview details). Count pills use 11px at 600 to 700, not uppercase; keyboard hints (kbd) 11px at 500.
- **Mono** (400, 12px, 22px line): hashes, diff lines and hunk headers (11.5px), file paths in diff headers, and branch names or SHAs quoted inside a sentence. Diff line numbers are 11px.

### Named Rules
**The Literal String Rule.** Mono marks a literal machine string: a SHA, a code line, a path in a diff header, a ref name quoted inside prose. Lists of branches, files and ref chips stay in Inter. Mono never appears as styling for headings or labels.

**The Section Header Rule.** The uppercase tracked label names a list group (Local branches, Remote branches, Tags, "2 new commits" over a preview list). It never sits above a heading as an eyebrow.

## Layout

A fixed desktop chrome around a two-column workspace, after GitHub Desktop:

- **Title strip** 38px on bg-0 with repo tabs (28px) and the app's own window controls (46px wide).
- **Toolbar** 56px on bg-1 between hairlines: labelled segments left to right (Current repository, Current branch, the origin action, Undo with its history caret), separated by 28px hairline rules, then a compact Search button pushed right. Segments flex and truncate (repo up to 230px, branch 260px, origin 250px, Undo 280px). A container query at 1100px drops the search's label, leaving the icon and shortcut.
- **Main grid**: left column, 1px resizer, main area (minmax(0, 1fr)).
  - **Left column** defaults to 340px and is resizable from 280px to min(640px, window − 520px), so the diff always keeps at least 520px; the left column gives way first. It stacks the 42px Changes / History tabs, then either the changes list (40px header, file rows, a 40px stashed-changes row, the commit box) or the history list.
  - **Main area** fills the rest on bg-0: any operation or preview banner, then a 40px pane header and the diff, the selected commit, the preview details, or the "No local changes" next steps.
  - **Commit view** splits under its header into a files column (minmax(200px, 280px), bg-1) and the diff.
  - Below 760px of window height the commit description shrinks from 84px to 52px.

**Spacing rhythm** runs on small steps: 4, 6, 8, 10, 12, 16, 24px. Gaps inside rows are 6 to 10px; pane padding is 12 to 16px; banners sit 8px in from the edges; main-area readouts (clean state, preview details) centre in a 560 to 680px column with 24 to 32px padding. **Fixed row heights** set the vertical rhythm: pane headers 40px, file rows 30px, history rows 50px, diff lines 22px, hunk headers 32px, popover items 32px, context-menu rows 28px, palette results 34px.

## Elevation & Depth

Depth is tonal by default: regions are separated by ground steps and 1px hairlines, not shadows. Exactly one shadow exists, and it only appears on things that float above the workspace.

### Shadow Vocabulary
- **Pop** (`box-shadow: 0 14px 36px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.35)`): context menus, anchored prompts and popovers (branch menu, undo history, stashes), toasts, the command palette, the drag chip.

### Motion
One easing for everything: a fast ease-out (`cubic-bezier(0.16, 1, 0.3, 1)`). Hover and colour changes run 120ms. Floating surfaces enter with a short fade plus a small translate: menus 120ms, prompts and popovers 140ms (4px drop), the palette 150ms (its scrim fades 120ms), banners drop in at 180ms, toasts rise 200ms. Chevrons rotate in 150ms; progress lines ease their width over 200ms, with a 1s indeterminate sweep when the total is unknown. The signature transition is the preview's **ghost-in** (220ms): ghost nodes scale up from 0.6 while fading in, and ghost rows fade in alongside. Loading uses a 1s linear spinner in the origin segment and a 1.4s shimmer for skeleton rows. Under reduced motion every animation and transition collapses to 1ms.

### Named Rules
**The Float-Only Shadow Rule.** Only surfaces that float above the workspace get the Pop shadow. Panels, banners, rows, toolbar segments and buttons stay flat.

## Shapes

Soft, consistent rounding is a binding brand commitment. Small controls, checkboxes, status badges, chips, list and popover rows, keyboard hints and icon buttons use 6px; buttons, the segmented control and warning notes 8px; inputs, context menus, toolbar segments, the search button and the Commit button 10px; banners, toasts, prompts, popovers, the palette and bordered lists 12px. Count pills, the 2px tab underlines and progress lines are full-round; graph nodes, avatars and banner icon discs are full circles. Borders are always 1px hairlines (1.5px on checkboxes); dashed 1px outlines mark things that do not exist yet (ghost chips, the "new" badge, WIP and ghost nodes). Struck-through text and a horizontal strike through a hollow node mark things that are about to be replaced. Blank halves in the side-by-side diff are faintly striped at -45deg.

## Components

### Toolbar (labelled segments)
Calm and readable: every control says what it is.
- **Segment:** 46px tall, 10px radius, transparent at rest, bg-2 on hover and while its popover is open, bg-3 pressed. A Fog Lucide icon, then a two-line label: an 11.5px Fog caption over a 13px weight-600 value (Current repository / GitCove), then a Fog caret.
- **Origin action:** one smart segment that becomes Fetch, Pull or Push origin, its value line on top with "Last fetched …" as the caption, Signal Blue count pills with Ink arrows for ahead/behind, a spinning icon while busy, and a 2px full-round progress line along its bottom edge.
- **Undo:** a split segment. The main part's caption names the shortcut, its value names what Undo would reverse; a 24px caret half opens the undo history popover (32px rows, undone entries struck through in Fog, relative times right-aligned).
- **Branch segment:** turns Amber Wash with an amber icon while a rebase is stopped or HEAD is detached.
- **Search:** a 32px bg-0 well with a Hairline border (Strong Hairline on hover), icon, label and kbd hint.

### Buttons
Quiet, compact and uniform: one shape for every button.
- **Shape:** gently rounded (8px), 30px tall, 12px side padding, weight 500, 6px icon gap.
- **Default:** Raised Graphite with Paper White text; hover to bg-3, pressed to bg-4; disabled at 45% opacity.
- **Primary / Success / Danger:** Signal Blue, Commit Green or Alarm Red fill with Ink text at weight 600, each with a lighter hover. Success is reserved for the Commit action; danger for confirming a destructive step.
- **Ghost:** transparent with a Strong Hairline border (secondary actions such as Cancel, New branch, "Keep yours").
- **Quiet:** transparent with Fog text, filling on hover (Mark resolved, Add remote).
- **Small:** 24px tall, 8px padding, 12px text, 6px radius.
- **Icon button:** 26px square, 6px radius, Fog icon, fills bg-3 on hover; a toggled icon button (wrap) turns its icon Signal Blue.
- **Commit button:** full width, 38px, 10px radius, Commit Green with Ink "Commit to **branch**" (branch at 700, middle-truncated). With nothing ticked it reads "Tick the files to commit" on bg-3 in Fog at full opacity.

### Tabs
- **Pane tabs** (Changes / History): two equal 42px columns over a Hairline. Inactive in Fog at 500, hover to Paper White over a 2% white wash; active in Paper White at 600 with a 2px full-round Signal Blue underline inset 14px from each side. A neutral count pill follows the label.
- **Menu tabs** (Branches / Pull requests): the same treatment at 38px inside the branch menu, underline inset 16px.
- **Segmented control** (Unified / Side by side): a 26px bg-0 well with a Hairline border and 8px radius, 2px padding; options 12px at 500 in Fog, the active option on bg-3 in Paper White at 6px radius.

### Checkbox and File Rows
- **Checkbox:** 16px, 6px radius, 1.5px Strong Hairline border on bg-0 (border lightens to Ash on hover). Ticked or mixed fills Signal Blue with an Ink check or dash. Focus ring sits 2px out.
- **Changes header:** 40px, a tri-state header checkbox and the count ("3 changed files", conflicts appended in amber), a stash action right.
- **File row:** 30px, 6px radius, inset 6px; checkbox, path (directory in Fog, file name in Paper White), then the status badge on the right. Hover bg-2, selected bg-3. In the commit view the row shows +/− stats before the badge.
- **Status badge:** 17px, 6px radius, a single 10.5px letter in Ink on amber (modified), green (added, untracked), red (deleted) or blue (renamed, copied); conflicted is an amber 1.5px ring with an amber letter.
- **Conflicted files** get a row of small buttons beneath: "Keep yours" and "Keep <other>" (ghost) and "Mark resolved" (quiet).
- **Stashed changes:** a 40px row above the commit box with a neutral count pill, opening a popover of stashes with Restore.

### Commit Box
bg-1 under a Hairline, 12px padding, 8px gaps. A 28px avatar beside a 34px summary field with a right-aligned character counter (amber when long), an 84px description, an "Amend last commit" control with its kbd hint, then the Commit button. When git has no name or email, an Amber Wash note (8px radius) asks who you are with a small ghost "Set name and email…" button.

### Chips and Pills
- **Branch ref chip:** 20px, 6px radius, 11.5px weight 600, filled with its lane colour and Ink text. In history rows one chip shows at its natural width (max 140px) with "+N" in Fog for the rest. Draggable onto another row to start a preview.
- **Remote chip:** transparent with a 1px inset ring and text in the lane colour, plus a cloud icon.
- **Tag chip:** bg-3 fill with Paper White text and a tag icon.
- **Ghost chip:** dashed outline in the lane colour (where a ref will land). **Struck chip:** line-through at 45% opacity (where it is leaving).
- **Count pill:** 18px, full-round, 11px; bg-3 with Paper White for neutral counts (tabs, stashes, pull requests), Signal Blue with Ink only for the origin action's ahead/behind counts.

### Inputs / Fields
- **Style:** 32px, 10px radius, Strong Hairline border, Raised Graphite fill (bg-1 inside popovers so the field reads against the bg-2 surface), Ash placeholder.
- **Focus:** the border turns Signal Blue; no outline glow.
- **Textarea:** same shape, 8px by 10px padding, line-height 1.45, no resize handle.
- **Keyboard hint:** 11px Inter, Strong Hairline border, 6px radius.

### Branch Menu
The Current branch segment opens a 12px-radius popover: Branches / Pull requests tabs, a filter field, then Local branches, Remote branches and Tags under section labels in 32px rows (Fog branch icons; the current branch with a green check, weight 600 and ahead/behind; the keyboard-active row on bg-3). Pull requests list with an open-state icon and a sub-line. A Strong Hairline footer holds New branch (small ghost) and Add remote (small quiet).

### History Rows (signature)
- 50px two-line rows on Graphite Night with a faint 2.5% white bottom rule: line one is the ref chip, "+N" and the summary at 500; line two is an 18px avatar, author and relative time in 12px Fog. The WIP row sits on top with a dashed hollow node, "Uncommitted changes", and changed/included counts.
- Hover adds a 2.5% white wash; selection uses Blue Wash plus a 2px inset Signal Blue bar when the list is focused; a drop target gets a 20% blue fill with a Blue Rule ring.
- Node sizes: 7.5px radius commits, smaller for merges, a 1.6px ring 3.5px outside the HEAD commit.
- Ghost commits: dashed hollow nodes and dashed lane strokes, Fog text, a dashed "new" badge. Replaced commits: hollow Ash node with a strike, row text struck through in Ash.

### Diff (signature)
- **Pane header:** 40px on bg-1: the path (directory in Fog, file at 500), the inclusion readout ("6 of 11 changed lines in the commit", "Not in the commit") in 12px Fog, +/− stats, then the wrap toggle and the Unified / Side-by-side control.
- **Lines:** 12px mono on a 22px line, two 48px number columns (11px, right-aligned) and a 20px mark column; text in the syntax palette over the add/delete washes.
- **Inclusion gutter:** clicking or dragging over the line numbers of a changed line puts it in or out of the commit. In: solid Signal Blue with Ink numerals at 600 (Lifted Blue on hover). Out: bg-1 with Ash numerals (bg-3 on hover). The gutter is sticky on horizontal scroll and carries roving keyboard focus (2px focus ring, Paper White on blue).
- **Hunk header:** 32px on bg-1 between hairlines: a 40px expand-context control, the hunk's tri-state checkbox, and the 11.5px mono hunk range in Fog.
- **Side by side:** two equal columns split by a Hairline; blank halves striped.
- **Conflict regions:** labelled by the user's side, not git's: the user's side ("Yours") in Blue Wash with a blue inset bar, the other side in a violet wash with a lane-2 bar, the base in a faint white wash with an Ash bar. Markers become 26px labelled rules on bg-1 with 11.5px Inter labels; a conflict note in Amber Wash heads the file.

### Commit View
A bg-1 header (14 to 20px padding): the summary in Title with Branch here and a more menu, the body in Fog clamped to four lines with a "Show the full description" toggle, then a 12.5px meta row: 20px avatar and author, a bg-2 SHA button with copy icon, time, file count, +/− totals and neutral ref chips. Below, the files column beside the diff.

### Main-Area Readouts
- **No local changes:** a green check, the Heading, a Fog lede naming the last commit, then a 12px-radius bordered list on bg-1 of next steps (title at 600, 12.5px Fog sub-line, one button each; Push origin is the primary).
- **Preview details:** "What happens if you apply this" in Heading over a lede, then section-labelled bordered lists (bg-1, 12px radius, hairlines between rows) of new commits (dashed blue nodes, amber when a conflict is expected), replaced commits (struck), branches that move, expected conflicts, items removed from the remote (label in red) and uncommitted changes.

### Preview Banner (signature)
Sits 8px in above the main area: 12px radius, Blue Rule border, Blue Wash over bg-1 (danger variant: red border and Red Wash). A 30px circular icon disc, a 13.5px title prefixed by "Preview:" in Fog, a readout line of green/amber/red consequences, a segmented control for alternatives (Rebase / Merge), then Cancel and the primary Apply. It says plainly that the user can undo.

### Operation Banner
Same shape as the preview banner in amber: shown while a merge or rebase is stopped, with the conflict count and Abort (ghost), Skip commit (quiet, when available) and Continue (primary, disabled while conflicts remain).

### Toasts
Bottom right, 380px, bg-2, 12px radius, Strong Hairline border, Pop shadow, rising in over 200ms. A coloured status icon (green, red, blue), the message, and an action button, usually Undo naming what it reverses; error output appears in a scrollable bg-0 block.

### Context Menus
bg-2, 10px radius, 5px padding, Pop shadow, 220 to 340px wide. 28px rows with right-aligned shortcut hints; the active row fills Signal Blue with Ink text; destructive items are red and fill red when active; separators are Strong Hairline rules.

### Popovers and Anchored Prompts
Popovers (branch menu, undo history, stashes) and routine inputs (new branch, tag, rename branch, git identity) open anchored to the control that asked: bg-2, Strong Hairline border, 12px radius, Pop shadow, a 140ms fade and 4px drop. Popover rows are 32px at 6px radius, hovering to bg-3; heads are 12px Fog at 600; footers sit behind a Strong Hairline. Prompts are 320px with 14px padding, a 13px title, labelled fields and right-aligned actions. No scrim.

### Command Palette
Ctrl+K opens a 620px panel 90px from the top over a light 45% scrim: 48px search row at 14px, grouped results in 34px rows, the active result filled Signal Blue with Ink text, shortcut hints at 12px.

## Do's and Don'ts

### Do:
- **Do** put Ink (#0e1013) text on every coloured fill: buttons, ref chips, file badges, ticked checkboxes, the in-commit gutter, active menu rows.
- **Do** reserve Signal Blue for selection, focus, commit inclusion and the primary action in a group.
- **Do** mark what goes into the commit with checkboxes and the Signal Blue gutter, and say it in words in the diff header.
- **Do** keep lane colour 0 and the leftmost lane for the default branch, and colour branch chips with their lane colour.
- **Do** draw destructive operations on the graph first (dashed ghosts, struck replacements, a readout, Apply/Cancel) and offer Undo in a toast afterward.
- **Do** label toolbar segments with a caption over a value, and put branches, tags and pull requests in the branch menu.
- **Do** handle routine input in anchored popovers, context menus and the Ctrl+K palette.
- **Do** separate regions with ground steps and 1px hairlines; give the Pop shadow only to floating surfaces.
- **Do** use the shared control vocabulary: 30px buttons at 8px radius, 24px small buttons, 26px icon buttons, 32px fields at 10px radius, 16px checkboxes at 6px.
- **Do** keep the diff roomy: the main area never drops below 520px, and the left column gives way first.
- **Do** label conflict regions by the user's side (Yours in blue, the other side in violet).
- **Do** use the single ease-out curve and keep state changes between 120 and 220ms.

### Don't:
- **Don't** open modal dialogs for routine actions; the palette is the only scrimmed overlay.
- **Don't** crowd the toolbar beyond its four labelled segments and search; further actions belong in the branch menu, the palette and context menus.
- **Don't** bring back a ref sidebar or a status bar; their content lives in the branch menu, the toolbar and toasts.
- **Don't** set white text on coloured fills.
- **Don't** use Signal Blue as decoration, or for idle icons and headings, or as a solid fill for selected list rows.
- **Don't** use JetBrains Mono for headings, labels, lists of refs or file lists.
- **Don't** put the uppercase section label above a heading as an eyebrow.
- **Don't** use sharp corners; nothing in the app is square-cornered except 1px rules and the diff's line bands.
- **Don't** add a light theme or per-OS styling without a product decision; the app is dark and uses one design language across Linux, macOS and Windows.
- **Don't** show the folder name as the product name or invent a logo; the product is written GitCove.
