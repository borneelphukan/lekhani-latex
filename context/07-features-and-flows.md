# 07 — Features and End-to-End Flows

This document traces each user-facing feature from input to state change. For the lower-level compile/render mechanics, see [05-compile-and-preview-pipeline.md](05-compile-and-preview-pipeline.md).

## 1. New document

```
File → New Document / empty-state “New Document”
  └─ file_dialog_action = NewDocument
       └─ App::new_document()
            ├─ rfd save dialog, filter “LaTeX” (.tex), default name document.tex
            ├─ project_tex_path(chosen) = <parent>/<stem>/<chosen filename>
            ├─ EditorBuffer::new() (template article)
            ├─ buf.save_as(tex_path)
            ├─ Tab::new_empty(tex_path) (title = stem, dirty = false)
            └─ push tab, make it active
```

The directory `<parent>/<stem>/` is created by `save_as` → `write_atomically` → `create_dir_all(parent)`. The chosen filename is preserved, but the file always lands one directory deeper than the dialog suggested.

Failure to create the file surfaces on the current active tab’s `error_message`; if there were no tabs, `active_tab_mut()` would panic — in practice the File menu’s New action is reachable with zero tabs, which is a latent bug. (The empty-state button calls `new_document()` directly and only fails after a dialog.)

## 2. Open file

```
File → Open… / empty-state “Open File”
  └─ App::open_file()
       ├─ rfd open dialog (.tex, .sty, .cls)
       ├─ if path already open: switch to that tab (no duplicate)
       └─ Tab::load(path): read UTF-8, title = stem, dirty = false
```

If reading fails, `Tab::load` falls back to the template buffer with the requested path and shows “Opened <path>” as the status message; the user may overwrite the unreadable file on save.

Only the first positional CLI argument is opened at startup (`std::env::args().skip(1).next()`); additional arguments are ignored.

## 3. Save / Save As

| Action | Flow |
|---|---|
| Save with path | `EditorBuffer::save()` → `write_atomically`; status “Saved”; if auto-compile is on, compile. |
| Save without path | Falls through to Save As. |
| Save As | Native save dialog → `project_tex_path` → `save_as`; title updates from the chosen stem; status “Saved as <nested path>”; optional auto-compile. |

`write_atomically` writes `<file>.tex.tmp`, removes the original, then renames. A crash between remove and rename loses the original. There is no backup file, no fsync, and no permission preservation beyond the default umask.

## 4. Compile

Flow is covered in [05](05-compile-and-preview-pipeline.md). User-visible behaviour:

1. If `pdflatex` is missing, the “Missing LaTeX Compiler” dialog appears instead of compiling.
2. A tab with no path shows “Please save the file before compiling.”
3. The buffer text is written to disk before compiling (even if `dirty` is false).
4. Preview is forced visible.
5. While running, the toolbar shows `⏳ Compiling…`; the status bar shows `Compiling…`.
6. Warnings append yellow lines immediately; success replaces the log with one green duration line; failure builds red per-error lines and highlights the offending source lines.
7. On success, the preview resets to page 0 with a fresh texture cache.
8. On failure, the last preview remains.

The elapsed-time display uses `1.23s` for ≥1 s and `123ms` below 1 s.

## 5. Auto-compile

- Toggle in the main toolbar. Not persisted; starts `false` every launch.
- After each edit, `last_content_change` is set.
- When idle ≥ 0.8 s and the previous auto-compile was ≥ 1.2 s ago, the active tab is written directly and compiled.
- Because the timer is global (not per-tab), switching tabs does not reset it; only the active tab is auto-compiled.
- Auto-compile does not run when there are no tabs; it does not require `dirty` to update `last_auto_compile`.
- Save with auto-compile on also compiles, so typing + saving can produce two compiles in quick succession.

## 6. Tabs

- Open tabs are independent documents; each has its own compiler and preview state.
- Reopening a path switches to the existing tab.
- Dirty tabs show ` •`; closing asks for confirmation.
- Drag tabs horizontally to reorder; the active index is updated.
- Closing the last tab returns to the empty state.
- There is no “close all”, “reopen closed”, or session restore.

## 7. Undo / redo

- Toolbar buttons and `Cmd/Ctrl+Z` / `Cmd/Ctrl+Y` call `EditorBuffer::undo/redo`.
- History is a capped stack of `EditToken`s (1024).
- History is populated by `insert_str`, `backspace`, `delete`, `replace_all`, and formatting helpers. Ordinary keystrokes inside `TextEdit` mutate the string without pushing tokens, so they are not individually undoable through this stack. egui’s own text-edit undo may still apply to the focused widget, but it is not synchronized with `EditorBuffer`’s stack.
- Redo is cleared on any new token.
- Undo/redo marks the buffer dirty even if it returns to the saved content.

## 8. Syntax highlighting

- `core::lexer::tokenize` uses one multiline regex: `\\([a-zA-Z]+|[^a-zA-Z])|%.*$|\$\$|\$|[{}]`.
- Token types: `Command`, `MathDollar`, `MathDoubleDollar`, `OpenBrace`, `CloseBrace`, `Comment`, `Text`.
- The layouter maps each to `SyntaxColors` and paints the whole buffer on every layout pass.
- Comments are `%.*$` in multiline mode, so they run to end of line.
- `$$` is matched before `$`.
- A backslash followed by a non-letter (e.g. `\\`, `\{`) is one command token.
- There is no string/verbatim/math-environment awareness.
- Selection is layered by splitting tokens at selection boundaries; error backgrounds take precedence only when the token is not selected.

## 9. Autocompletion

- Trigger: after any change or while already visible, look back from the cursor for `\` and ensure the suffix is alphanumeric.
- Popup position: cursor line's bottom-left.
- Navigation: ↑/↓/Enter/Esc are consumed only while visible.
- Accepting replaces the byte range from `\` to the cursor with the full command (which may include `{}` placeholders).
- `completion_block_trigger` prevents immediate reopening after accepting.
- Clicking elsewhere closes the popup.
- Matching is case-sensitive `starts_with`; there is no fuzzy matching and no ranking.
- The list is static and contains a malformed `"\\{figure}[h]"` entry.

## 10. Headings and inline formatting

- Bold/italic/underline insert fixed commands and place the cursor between braces.
- Heading dropdown detects the current line’s command (if any), shows a label, and rewrites the line to the selected level.
- The “Numbered” checkbox removes/adds `*` on the current heading and controls new headings.
- Chapter is disabled when the document class is `article` (or when no `\documentclass` is found).
- `apply_heading(None, …)` strips a heading command and leaves the text content.
- Indentation before the heading is preserved.
- Cursor placement after applying a heading is at the closing brace.

## 11. Preview controls

| Interaction | Result |
|---|---|
| `−` / `+` | 0.25 zoom step, 0.25–4.0. |
| Ctrl/Cmd+wheel | 0.9/1.1 multiplicative zoom; wheel events consumed. |
| Trackpad pinch | egui `zoom_delta()` multiplicative zoom. |
| Page ◀/▶ | Move one page; next disabled at known end. |
| Arrow buttons | Scroll 50 px. |
| Middle-drag | Pan. |
| Pan mode + left-drag | Pan. |
| Scroll | Free vertical/horizontal scroll; current page derived from offset. |
| Initial open | If zoom is exactly 1.0 and the page is wider than the pane, fit width once. |

The page count comes from `pdfinfo`; if it is missing, the UI cannot disable “next” at the end.

## 12. AI correction and package install

Detailed in [08-ai-and-llm.md](08-ai-and-llm.md). Summary:

- “Fix with AI” in the toolbar fixes the whole document.
- “✨ Fix with AI” in the editor context menu fixes the hovered line.
- Both require `llm_configured` and a compile error.
- The response either replaces text (whole document or one line) or triggers `tlmgr install <package>`.
- Results are written to the AI output tab; compiler errors remain in the compiler tab.

## 13. Theme switching

- Settings → Theme writes `config.json` immediately.
- `apply_theme` runs every frame, so the change is instant.
- System mode follows the OS theme; on Linux a background `gsettings` poller updates it every 2 s.
- Syntax colors resolve from the actual egui dark/light mode, not the configured enum.

## 14. Update check

```
Help → Check for Updates
  └─ check_for_updates(ctx)
       ├─ state = Checking
       └─ thread: GET https://api.github.com/repos/example/lekhani-latex/releases/latest
            ├─ naive string scan for "tag_name"
            ├─ compare tag (minus leading v) with CARGO_PKG_VERSION
            └─ send CheckResult(available, version)
```

- The check is manual only; there is no startup or periodic check.
- Any version inequality counts as an update; there is no semver ordering.
- If the request or parse fails, the thread still sends `CheckResult(false, None)` after a 1-second artificial delay. The user sees “No update available”, which is a false negative and does not distinguish a network failure from being up to date.
- The URL is a placeholder and does not point at this repository.

If an update is offered, “Download Now” fetches a Windows `.exe` from `github.com/example/lekhani-latex/releases/download/v<version>/lekhani-latexSetup.exe`, saves it to the downloads directory, spawns it, and exits the app. This path is compiled on every platform and is not platform-aware.

## 15. Missing compiler download

- Triggered at startup/compile when `pdflatex --version` fails.
- Download URL by OS:
  - Windows: `protext-3.2-021024.zip` from CTAN;
  - Linux: `install-tl-unx.tar.gz`;
  - macOS: `mactex-basic.pkg`.
- Progress is reported only when `Content-Length` is present.
- On completion:
  - Windows: spawn the downloaded executable;
  - macOS: `open` the `.pkg`;
  - Linux: find one of `gnome-terminal`, `konsole`, `xfce4-terminal`, `alacritty`, `xterm` and run a shell that extracts the tarball and runs `sudo ./install-tl`.
- No checksum/signature verification, no resume, no cancel of the background download.

## 16. About and version info

- About window displays a fixed description, copyright, placeholder links, and `CARGO_PKG_VERSION`.
- “Copy Version Info” copies OS/family/arch and version.
- No build commit/hash is displayed even though `build.rs` emits `GIT_BRANCH`.

## 17. Feature gaps worth knowing

- No search/replace, spell check, or multi-cursor.
- No project/file tree; one file per tab.
- No bibliography/glossary UI.
- No PDF export beyond what `latexmk` writes.
- No settings persistence for auto-compile, window size, open tabs, zoom, or split ratio.
- No crash reporting, logging, or telemetry.
- No tests for any of these flows.
