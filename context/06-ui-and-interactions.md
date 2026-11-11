# 06 — UI and Interactions

The UI is built every frame from `App` methods. There is no retained widget tree and no global state outside `App`/`Tab`. This document maps the panels, controls, shortcuts, and dialogs to code.

## 1. Window chrome and panel layout

```
┌──────────────────────────────────────────────────────────────────┐
│ MenuBar: File | View | Settings | Help                           │
│ TabBar (only when tabs exist): [main ×] [chapter]                │
│ Toolbar: ▶ Compile | ✨ Fix with AI | ☐ Auto-compile | preview  │
├──────────────────────────────────────────────┬───────────────────┤
│ Formatting toolbar: B I U | Heading ▾ | ☐ Numbered | Undo/Redo  │
│                                              │ Preview toolbar   │
│ Editor area (TextEdit)                       │ (zoom/page/pan)   │
│                                              │ Preview pages     │
├──────────────────────────────────────────────┴───────────────────┤
│ Status bar: path • | Ln, Col | message | Compiling…/OK/Error     │
│ Output bar: ▼ Outputs                                            │
│ Output panel (resizable, optional): Compiler Output | AI Output  │
└──────────────────────────────────────────────────────────────────┘
```

Panel order in `App::ui`:

1. `Panel::top("menu_bar")`
2. `Panel::top("tab_bar")` (only with tabs)
3. `Panel::top("toolbar")`
4. `Panel::bottom("status_bar")`
5. `Panel::bottom("output_bar")` (only with tabs)
6. `Panel::bottom("output_panel")` when `show_outputs`
7. `CentralPanel` with the empty state or editor+preview

In fullscreen, panels are hidden except on macOS (`show_panels = !is_fullscreen || cfg!(target_os = "macos")`). A bottom-center toast says “Press Esc to exit fullscreen”.

## 2. Menu bar (gui/src/app/layout/menubar.rs)

| Menu | Item | Behaviour |
|---|---|---|
| File | New Document | Native save dialog; creates `<parent>/<stem>/<name>.tex` from the template. |
| File | Open… | Native open dialog (`.tex`, `.sty`, `.cls`); switches to an already-open path instead of duplicating. |
| File | Save | Saves the active tab (Save As if untitled). |
| File | Save As… | Native save dialog; renames the tab from the chosen stem. |
| File | Quit | Sends `ViewportCommand::Close`; close-request handling may prompt. |
| View | Toggle Preview | Shows/hides the right preview pane. |
| View | Fullscreen / F12 | Toggles viewport fullscreen. |
| Settings | Theme → System/Light/Dark | Updates `self.theme` and writes config immediately. |
| Settings | AI Integration | Opens the separate AI Settings viewport. |
| Help | Check for Updates | Starts a background GitHub API request. |
| Help | About | Opens the About window. |

The menu bar sets its own 13 px button/body font, 16×10 button padding, and 4 px item spacing.

## 3. Tab bar

- Each tab shows its **file stem** plus ` •` when dirty.
- The active tab shows a `×` close button. Closing a dirty tab asks an `rfd` Yes/No question.
- Drag-reorder starts after a 6 px horizontal movement from the press origin. While dragging, the tab is drawn as a floating overlay at the pointer and the original slot is a blank spacer.
- Tab width is heuristic: `title.chars().count() * 7.5 + 16 + (active ? 28 : 0)`, minimum 40 px. Long titles are not truncated with ellipsis; they widen the bar.
- Removing the active tab clamps `active_tab` to the new last index.

## 4. Main toolbar (gui/src/app/layout/toolbar.rs)

| Control | Enabled when | Action |
|---|---|---|
| `▶ Compile` / `⏳ Compiling…` | a tab has a path and is not already running | `trigger_compile()` |
| `✨ Fix with AI` / `⏳ Fixing…` | tab exists, LLM configured, not already running, and `error_message.is_some()` | `trigger_llm_correction()` |
| `Auto-compile` checkbox | always | Toggles the per-frame auto-compile timer. |
| Preview toggle (`◫`) | tabs exist | Same as View → Toggle Preview. |

The compile button is green and the AI button is purple; both colors depend on resolved theme. The compile label changes to `⏳ Compiling…` whenever `CompileStatus::Running`.

## 5. Formatting toolbar (gui/src/app/editor.rs)

| Control | Behaviour |
|---|---|
| `B` | Inserts `\textbf{}` at the cursor and moves the cursor inside the braces. |
| `I` | Inserts `\textit{}`. |
| `U` | Inserts `\underline{}`. |
| Heading dropdown | Shows the heading detected on the cursor line, or “Paragraph”. Items: Part, Chapter, Section, Subsection, Sub-subsection, Paragraph Header, Subparagraph, each at a decreasing font size. Chapter is disabled in `article` documents. |
| `Numbered` checkbox | Toggles the `*` on the current heading and stores `heading_numbered` for subsequent headings. |
| Undo / Redo | Calls `EditorBuffer::undo/redo`; disabled when the corresponding stack is empty. Hover text says Ctrl+Z / Ctrl+Y. |

Heading detection uses a regex on the current line only. `apply_heading` strips an existing heading command from the line, preserves indentation, and rewrites it. `is_article_document` looks for `\documentclass[...]{article}`; if no `\documentclass` is found it returns `true`, so chapter is disabled by default in fragments.

## 6. Editor area

`editor_area` wraps a `TextEdit::multiline` in a `ScrollArea::both` inside a framed background.

Key implementation details:

- The buffer `String` is moved out of `EditorBuffer` with `std::mem::take` and moved back after `TextEdit::show`.
- A custom `layouter` tokenizes the whole text with `core::lexer::tokenize`, maps token types to `SyntaxColors`, splits tokens around the current selection so the selection background is preserved, and uses `LayoutSection`s with `FontId::monospace(14.0)`.
- Error lines get a reddish background (`rgb(90,50,55)` dark / `rgb(255,200,200)` light). Selection background uses the global `selection.bg_fill`.
- The editor uses `lock_focus(true)` and `desired_width(f32::INFINITY)`; it fills the available width.
- A raw `Tab` key inserts **four spaces** at the cursor. It does not match surrounding indentation and does not support Shift+Tab dedent.
- The editor adds a custom context menu: “✨ Fix with AI” (only when configured; enabled only when there is an error), Cut, Copy, Paste, Select All. Cut/copy/paste/select-all reimplement selection handling with char/byte conversion; Paste uses `arboard`, not egui’s clipboard.
- Secondary-button press/release stores the previous `TextEdit` state so the context menu can act on the right-click selection.
- The context-menu “Fix with AI” picks the line under the mouse (stored in `ui.data` under `response.id.with("hover_line")`) and calls `trigger_llm_correction_for_line(line)`.

### Completion popup

- Completion is evaluated when the text changed or the popup is already visible, and only with editor focus and `cursor_pos > 0`.
- It looks backward from the cursor for the last `\`, and only offers matches if everything after it is alphanumeric.
- `find_completions(prefix)` strips the leading backslash and matches commands whose text starts with it (or whose full string starts with the prefix).
- The popup is an `egui::Area` at the cursor’s bottom-left, foreground order, minimum 180 px wide, maximum 300 px high, with a vertical `ScrollArea`.
- ArrowUp/ArrowDown/Enter/Escape are consumed from the input event queue **while the popup is visible**, before `TextEdit` sees them.
- Enter inserts the selected command, replaces the `\prefix` byte range, restores focus, and sets `completion_block_trigger` so the popup does not immediately reopen.
- Clicking outside closes the popup.
- The command list (`LATEX_COMMANDS`) is a static array with environments, text formatting, headings, references, math symbols, tables/figures, font sizes, and bibliography commands. It contains a malformed entry `"\\{figure}[h]"` (see gotchas).

## 7. Preview panel (gui/src/app/layout/preview_panel.rs)

Toolbar (enabled only when `preview.image_size.is_some()`):

| Control | Action |
|---|---|
| `−` / `+` | Zoom out/in by 0.25, clamped 0.25–4.0. |
| `NNN%` | Read-only zoom label. |
| `▶` | Next page; disabled if `num_pages` is known and `page + 1 >= num_pages`. |
| `Page N` | Read-only 1-based page label. |
| `◀` | Previous page; disabled at page 0. |
| `◀ ▶ ▲ ▼` | Scroll the preview by 50 px via `scroll_request`. |
| `↖` / `✋` | Select pan mode vs normal mode (selected state). |

Content:

- Pages are stacked vertically with a 16 px gap, centered horizontally, drawn on a white frame with a soft shadow.
- Only pages within one page-height of the viewport are rendered; others allocate placeholder space.
- “Rendering...” is shown while a texture is missing.
- No image → red `render_error` text or grey “No preview available”.
- Ctrl/Cmd+scroll zoom, pinch zoom (egui `zoom_delta`), middle-drag pan, and pan-mode primary-drag pan are all supported.
- The current page indicator follows the scroll offset.
- The preview pane is a resizable `Panel::right("preview_panel")` with default width `clamp(available/2, 200, 800)` and minimum 200.

## 8. Status bar

Format:

```text
<path or Untitled> • | Ln <line>, Col <col+1> | <status message><compile status>
```

Compile status suffixes:

| Status | Suffix |
|---|---|
| Idle | empty |
| Running | ` | Compiling…` |
| Success | ` | OK` |
| Failed | ` | Error` |

`Col` is displayed 1-based even though `cursor_line_col()` returns a 0-based char column.

## 9. Output panel

- Toggle via the `▼ Outputs` / `▶ Outputs` bar.
- Resizable bottom panel, default 150 px, minimum 60 px.
- Tabs: **Compiler Output** always; **AI Output** only when `llm_configured`.
- Logs are `(String, Color32)` pairs rendered in monospace 13 px. `Color32::WHITE` entries inherit the current text color.
- “Clear” empties only the selected tab’s log.
- The panel auto-opens (`show_outputs_requested`) after compile success/failure, AI results, and package installs.

## 10. Dialogs and viewports

### About

`egui::Window` anchored center, fixed 700×380 minimum, non-resizable, non-collapsible. Shows description, copyright, three external links, and a version grid. The copy button writes `Version: … (Stable)\nEnvironment: OS: … (…); Arch: …` to the clipboard. The three links use `https://github.com/example/lekhani-latex/...` placeholders.

### AI Settings

A separate immediate viewport (`ViewportId::from_hash_of("ai_settings_viewport")`), 650×450, non-resizable, no maximize/minimize. Contents:

- “Local LLM” / “API Key” segmented buttons (right-to-left layout, so API Key appears on the right).
- Local tab: model preset/custom model + Endpoint URL.
- API tab: model preset/custom model + masked API key.
- Save is enabled when the required fields are non-empty; Delete is shown when already configured.
- Closing the viewport or pressing Cancel **also saves** the current field values.
- Once `llm_configured` is true, the fields are disabled until Delete is pressed.
- The viewport uses the OS theme (`ctx.system_theme()`), not the app’s selected theme, for its light/dark background.

### Missing LaTeX Compiler

Borderless centered window. Explains that `pdflatex` is missing and offers Download/Cancel. States: download progress bar, “Download complete! Launching installer…”, or a red failure message with Retry/Cancel.

### Update

- `Checking`: centered spinner window.
- `Prompt(version)`: top-center banner with “New update available: vX” + Download Now / Skip.
- `Downloading(progress)`: centered progress window with Cancel (Cancel only resets UI; the download thread keeps running).

### Unsaved changes

Native `rfd` dialogs: one for window close (“quit without saving?”), one for closing a dirty tab.

## 11. Shortcuts

| Shortcut | Scope | Behaviour |
|---|---|---|
| `Ctrl/Cmd+O` | global | Open file dialog. |
| `Ctrl/Cmd+S` | tabs exist | Save (Save As if no path). |
| `Ctrl/Cmd+Z` | tabs exist | Undo. |
| `Ctrl/Cmd+Y` | tabs exist | Redo. |
| `F12` | global | Toggle fullscreen. |
| `Esc` | fullscreen | Exit fullscreen. |
| `Tab` | editor focused | Insert four spaces. |
| `↑` / `↓` | completion popup | Move selection. |
| `Enter` | completion popup | Accept completion. |
| `Esc` | completion popup | Close popup (does not reach the editor). |

Not implemented: Ctrl+Shift+Z redo, Ctrl+W close tab, Ctrl+N new document, Ctrl+Tab switching, Ctrl+F search, Ctrl+A is only in the context menu (the `TextEdit` may also handle it natively), file drag-and-drop, and per-tab keyboard focus history.

## 12. Empty state

With no tabs, the central panel shows “Lekhani Latex”, a grey hint, and two large buttons: “New Document” and “Open File”. The menu bar and toolbar remain visible; the tab bar and status/output panels do not.

## 13. Theme and fonts

- `apply_theme` runs every frame and sets the global egui style. Dark mode uses `panel_fill = rgb(20,20,25)`, `window_fill = rgb(25,25,30)`, grey selection, 8–12 px radii, and dark widget fills. Light mode uses default light visuals with `panel_fill = rgb(240,240,240)` and `window_fill = rgb(245,245,245)`.
- `System` follows `App::os_theme` (Linux poller) or `ctx.system_theme()`, defaulting to dark when neither is known.
- `App::new` appends `Hack` as a fallback in the proportional font family for geometric-shape glyphs.
- The initial `main.rs` style is dark; `App::new` immediately applies the saved theme.
