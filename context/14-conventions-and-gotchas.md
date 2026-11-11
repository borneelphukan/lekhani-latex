# 14 — Conventions, Gotchas, and Platform Traps

These are behaviours, known bugs, and codebase conventions future agents should know before changing code. Verify still-true items against the current source; the pack was written at `7eba198`.

## 1. Repository-URL drift (`example/lekhani-latex`)

The app and the website point at different repositories:

| Location | URL |
|---|---|
| Update check | `https://api.github.com/repos/example/lekhani-latex/releases/latest` |
| Update download | `https://github.com/example/lekhani-latex/releases/download/v{version}/lekhani-latexSetup.exe` |
| About window links | three `https://github.com/example/lekhani-latex/...` hyperlinks |
| Website, Inno URLs, AppStream, Cargo metadata, install.sh, build-repo | `https://github.com/borneelphukan/lekhani-latex` and `https://borneelphukan.github.io/lekhani-latex/` |

Practical consequence: **Help → Check for Updates cannot work** for this repository. If you fix it, use the `borneelphukan` owner and construct per-platform asset URLs (or query the release API's asset list) rather than the single hard-coded Windows `.exe`.

## 2. Compiler preflight checks a different tool than it runs

- `App::ui` (startup once) and `App::trigger_compile` run `pdflatex --version`.
- `CompilerConfig::default()` runs `latexmk -pdf -interaction=nonstopmode`.

A system with `pdflatex` but no `latexmk` does not get the helpful installer dialog; it gets a compile failure. A system with `latexmk` but no `pdflatex` is blocked by the preflight. Any change to the command must touch both.

## 3. File-saving is not crash-safe, and compile bypasses save entirely

```rust
let tmp = path.with_extension("tex.tmp");
fs::write(&tmp, text)?;
let _ = fs::remove_file(path);
fs::rename(&tmp, path)?;
```

- `with_extension("tex.tmp")` changes `document.tex` → `document.tex.tmp`, but `readme.md` → `readme.tex.tmp` if ever used on a non-`.tex` path.
- Removing the destination **before** renaming means a crash between the two leaves the original file **gone**. True atomic replacement would use rename-with-replace semantics (e.g. a platform-specific atomic rename) without the preceding `remove_file`.
- `trigger_compile` and the auto-compile path call `std::fs::write` directly, so they have no temp file at all.
- Config writes (`write_app_config`) are plain `fs::write` too, with errors ignored.

## 4. New document / save-as nests the file in a project folder

`project_tex_path` turns a dialog choice of `/home/me/paper/main.tex` into `/home/me/paper/main/main.tex`. Users often do not expect the extra folder.

`new_document` reports creation failures by assigning `self.active_tab_mut().error_message`, which **panics when no tabs are open** — reachable via File → New Document from the empty state. Use a non-tab status/error channel (or guard the call) when fixing this.

## 5. `open_externally()` is Windows code on every platform

```rust
Command::new("cmd").args(["/c", "start", "", path_str])
```

There is no `#[cfg(windows)]` around it. On Linux/macOS the spawn fails silently, and `PreviewEvent::Unsupported` calls it. The intended behaviour (“open in the system's default viewer”, per the root README) does not happen. Correct implementation would be `xdg-open` / `open` per OS.

## 6. Preview temp files are never cleaned

Rendered pages are written to `std::env::temp_dir()/lekhani_preview_<pid>_<page>.png`. Nothing deletes them, on success or failure, and not even when a compile clears the in-memory cache. Long sessions or many recompiles accumulate PNG files.

There is also a **name collision hazard**: the filename does not include the PDF path or tab identity. Two tabs rendering the same page index race on the same temp file.

## 7. Renderer detection accepts a tool that merely spawns

`find_renderer` calls `<tool> --version` and accepts any `output().is_ok()`. It ignores exit status and stderr. A broken or non-PDF `mutool` in `PATH` wins discovery and all pages then fail. The chosen renderer is cached per tab for the app's lifetime.

`pdfinfo` is required for the page count; when it is missing, `num_pages` stays `None` and “next page” is never disabled, so users can scroll into non-existent pages that error at render time.

`image_size` is set once from the first decoded page and reused for all pages, so mixed page sizes are displayed at the first page's dimensions.

## 8. Auto-compile and LLM state are global, not per tab

- `auto_compile`, `last_content_change`, and `last_auto_compile` live on `App`, not `Tab`. Switching tabs does not reset the timers, and only the active tab can ever auto-compile.
- `llm_correction_in_progress` and the LLM channel are app-wide. The result handler applies text to `self.active_tab_mut()` when the message is drained, not to the tab that issued the request. Switching tabs mid-request can edit the wrong document.
- The output panel visibility and selected tab (`show_outputs`, `output_panel_tab`) are global too, even though logs are per tab.

## 9. Compile cancellation is kill-by-PID, and Drop does nothing

`CompilerBridge::compile` kills the previous child with `kill -9`/`taskkill /F`, then joins the old thread (which can block the UI briefly). `impl Drop for CompilerBridge` is empty, and `PreviewViewer::drop` is empty with a comment that render threads are “daemon-like”, so closing the app neither stops children nor waits for threads.

There is no timeout: a hung `latexmk` runs until killed or the app exits.

## 10. Success is judged by exit status + log text

`success = output.status.success() && !read_log_fatal(log)`, and then the PDF must exist. Conversely, a nonzero exit with a valid PDF is treated as failure. `read_log_fatal` searches for the substring `Fatal error occurred` anywhere in the log.

## 11. Update download has a non-functional “Cancel”

`UpdateState::Downloading` offers a Cancel button that only resets the UI state. The download thread keeps streaming, keeps sending progress, and on completion will spawn the installer and `std::process::exit(0)` even though the user cancelled. The same is true in the compiler-download dialog (its Downloading state has no Cancel at all).

## 12. AI settings save on Cancel and lock after save

- Closing the AI Settings viewport or clicking **Cancel** calls `save_llm_settings()`, so “Cancel” is really “Apply and close”.
- When `llm_configured` is true, all inputs are inside `add_enabled_ui(!self.llm_configured)`; changing a model, endpoint, or key requires pressing **Delete** first.
- The API-tab endpoint is ignored at request time: local mode uses the stored URL, API mode hard-codes `https://api.openai.com/v1/chat/completions`.
- The stored default local endpoint (`http://localhost:1234/v1`) lacks `/chat/completions`, so the default configuration cannot work until the user enters the full path.

## 13. AI package install has no elevation and no recompile

`LlmAction::InstallPackage` runs `tlmgr install <package>` directly. On system TeX Live installations it usually fails with a permission error; the app shows the stderr but never retries, elevates, or recompiles after success.

## 14. The completion list has a malformed entry

`LATEX_COMMANDS` in `core/src/completions.rs` contains:

```rust
"\\{figure}[h]",
```

which renders as `\{figure}[h]`, not `\begin{figure}[h]`. It is also inconsistent with the surrounding `\begin{…}` entries. Fix it when touching completions, and be aware existing tests/docs do not cover it.

Completion matching is case-sensitive `starts_with`; there is no fuzzy match, no `\end{…}` pairing, and no snippet expansion (inserted `{}` does not tab between placeholders).

## 15. Editor undo history and ordinary typing

`EditorBuffer` undo tokens are pushed by the buffer helpers (`insert_str`, `backspace`, `delete`, `replace_all`). Typing into `TextEdit` mutates the moved-out `String` and then sets `tab.buffer.text`/`cursor` directly; it does **not** push an `EditToken`. So `EditorBuffer::undo` only undoes structural edits, completions, clipboard operations routed through the buffer, heading changes, and AI replacements — not keystrokes.

`replace_all` (used for AI corrections) pushes two tokens: delete-all then insert-all. Undo therefore behaves oddly around a large AI replacement.

## 16. `App::new_document` and heading helpers unwrap regexes at runtime

Several regexes are constructed with `Regex::new(...).unwrap()` inside per-call functions (`is_article_document`, `detect_current_heading`, `apply_heading`, `toggle_heading_starred`). The patterns are static and valid, but each call recompiles the regex, unlike `error_line_regex()` which uses a `OnceLock`. Prefer the cached pattern style for new code.

## 17. Windows subsystem hides output even in debug

`gui/src/main.rs` starts with an unconditional `#![windows_subsystem = "windows"]`, not `#[cfg(not(debug_assertions))]`. On Windows there is no console even in debug builds, so `println!`, `eprintln!`, and panic text are invisible when launched from a terminal.

## 18. Linux theme poller assumes GNOME and never stops

`App::new` spawns an infinite loop on Linux calling `gsettings get org.gnome.desktop.interface color-scheme` every two seconds, forever, regardless of the selected theme. On non-GNOME desktops the command fails silently. It keeps running until process exit.

`apply_theme`'s System branch treats “no theme information” as **dark** (`sys.is_none()` → dark).

## 19. UI access to the active tab can panic

`active_tab()` / `active_tab_mut()` index `tabs[active_tab]` with no bounds check. Most paths guard with `tabs.is_empty()`, but `new_document`'s error branch (item 4) is an example of a missing guard. Any new code that touches `active_tab_mut()` after a fallible operation needs to re-check for zero tabs. Closing the last tab only clamps `active_tab` when the vec is non-empty.

## 20. `egui` input consumption while completion is visible

While `completion_visible` is true, the editor loop **removes** ArrowUp/ArrowDown/Enter/Escape from `ctx.input_mut(|i| i.events)`. This also affects menus, dialogs, and text fields drawn later in the same frame. Completion must therefore be closed carefully whenever focus moves.

## 21. `arboard` clipboard vs egui clipboard

Context-menu Copy/Select All use egui (`ui.ctx().copy_text` / TextEdit state), but Paste creates a fresh `arboard::Clipboard` every time and can fail silently (the failure is ignored and nothing is inserted). Clipboard handling is not unified.

## 22. No tests, no LICENSE, stray files

- Zero `#[test]`/`#[cfg(test)]` occurrences in `core`, `compiler`, or `gui`; CI’s `cargo test` gate is trivially green.
- `Cargo.toml` and packaging metadata declare MIT, but no root `LICENSE` file exists; every packaging script checks `if [ -f LICENSE ]` / `skipifsourcedoesntexist`.
- `document.log` is committed at the repo root. It is leftover build output, not a test fixture.
- `assets/lekhani-latex.desktop` duplicates the file in `packaging/debian/`; packaging uses the latter.
- `website/package-lock.json` and `website/pnpm-lock.yaml` are both committed; npm is authoritative.
- The release workflow's `metainfo` release entry in git still says `0.0.5`; `build-deb.sh` rewrites it per build, `build-rpm.sh` does not.

## 23. Release/tagging assumptions

- `release.yml` reads the version from `Cargo.toml`, but **never compares it to the pushed tag**. A `v1.0.0` tag on a `0.0.7` tree creates a release named `v1.0.0` with artifacts named `0.0.7`.
- Non-tag pushes and even PRs run the full packaging matrix and produce workflow artifacts; only the weekly schedule skips the expensive jobs (`debian-x86_64`, `website`, `publish-pages` still run).
- The apt repository is amd64-only while ARM64 RPM/tarball/AppImage builds are produced.
- No artifact is signed; no checksum is published automatically on the website.
- `metadata` derives `numeric` as `major.minor.patch.0` — the fourth field is hard-coded, so a four-component crate version would be flattened.

## 24. CI pins Rust `1.99.1` in four places

`ci.yml` (`check`), `release.yml` (`ci`), and each packaging job pin the toolchain. There is no `rust-toolchain.toml`, so local builds can differ from CI. When bumping, update **every** pin, not just one.

## 25. Clippy discipline

CI runs `cargo clippy --workspace --all-targets -- -D warnings` with no `allow`s and no lint configuration file, so any new clippy warning in any crate breaks both `ci.yml` and `release.yml`'s `ci` gate. At the same time, the codebase suppresses dead code with `#[allow(dead_code)]` and `#![allow(dead_code)]` (notably in `core/src/buffer/mod.rs`, `core/src/types.rs`, and `core/src/lexer.rs`), and several unused helpers exist (`CompilerBridge::reset_status`, `AppError` variants, `EditKind`-adjacent types, `CursorPos`, most `components::*` helpers, and much of `buffer/cursor.rs`). Do not assume a helper is wired up because it compiles.

## 26. Style and size conventions

- `App`/`Tab` methods live in separate files as `impl App` / `impl Tab` blocks. Rust module privacy lets child modules (`app::layout::*`, `app::editor`) touch private fields; adding a new panel usually means adding a method, not a widget struct.
- Widgets are created through `components::button::{standard,toolbar,icon,borderless}` and `components::dropdown::dropdown`; `components::textfield` is mostly unused.
- Hard-coded colours and sizes are inline (`Color32::from_rgb(...)`, `vec2(28.0, 28.0)`, corner radii 4/6/8/12). There is no theme token layer.
- Icons are text glyphs (`▶`, `⏳`, `✨`, `−`, `+`, `◀`, `▶`, `▲`, `▼`, `↖`, `✋`, `◫`, `×`, `✔`), which is why the `Hack` fallback font is installed.
- Deferred local flags (`do_cut`, `close_requested`, `remove_tab`, `keep_visible`) are the idiomatic way to mutate state after a UI closure.

## 27. Documentation drift

The root `README.md` is user-facing and stale in specific ways:

| README claim | Actual behaviour |
|---|---|
| “A LaTeX distribution … providing `pdflatex`” | Preflight checks `pdflatex`, but compilation runs `latexmk`. |
| “Compile button — Run `pdflatex` on the current file” | Runs `latexmk`. |
| “Atomic file saves” | Temp write + **remove original** + rename; compile/auto-compile write directly. |
| “If none are found, the PDF opens in the system's default viewer” | `open_externally()` is Windows-only and fails silently elsewhere. |
| No mention of AI, updates, theme persistence, or packaging | Implemented and in CI. |

Update the README together with the relevant `context/` document, not instead of it.
