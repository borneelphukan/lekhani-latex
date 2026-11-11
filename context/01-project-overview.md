# 01 — Project Overview

## 1. What Lekhani Latex is

Lekhani Latex is a native desktop LaTeX editor written in Rust with `eframe`/`egui`. It targets people who want a small, fast editor with an inline PDF preview rather than a full TeX IDE.

Core capabilities:

- syntax-highlighted `.tex` editing with dark/light/system themes;
- multiple open documents in a draggable tab bar;
- compile with the local TeX distribution and show the resulting PDF inline;
- page navigation, zoom (buttons, pinch, Ctrl+scroll), and pan;
- LaTeX command autocompletion after `\`;
- per-document undo/redo;
- optional auto-compile while typing;
- optional “Fix with AI” using an OpenAI-compatible chat-completions endpoint;
- an update checker and a guided TeX-distribution downloader.

## 2. Product boundaries

The app is an **editor + orchestrator**, not a TeX engine or PDF library:

| Concern | Owned by | Notes |
|---|---|---|
| Text editing | `core` | Plain `String` plus a byte cursor, line index, and a simple `EditToken` undo stack. Not a rope, no incremental parsing. |
| LaTeX compilation | `compiler` | Spawns `latexmk -pdf -interaction=nonstopmode` (configurable through `CompilerConfig`) and parses stdout/stderr + the `.log` file. |
| PDF rasterization | external CLI | `mutool`/`mudraw`, Ghostscript, or `pdftoppm`. The app never links a PDF library. |
| Page count | external CLI | `pdfinfo` from poppler. Without it the preview cannot know the real page count. |
| TeX installation | external installer | If `pdflatex` is missing, the app downloads a platform installer; it does not implement TeX itself. |
| AI | external HTTP API | One request/response per correction. No streaming, no tool protocol, no conversation history. |
| Packaging/updates | shell scripts + GitHub Actions | The app only downloads an installer and launches it. |

## 3. Platform support

| Platform | Architectures shipped | Packaging |
|---|---|---|
| Windows | x64, x86, ARM64 | Inno Setup installer + portable zip |
| Linux | x86_64, ARM64 | Debian (`amd64` only in CI), RPM, tarball, AppImage |
| macOS | universal (x86_64 + arm64) | `.dmg` containing `Lekhani Latex.app` |

Runtime prerequisites on the user's machine:

- a TeX distribution exposing `latexmk` (and `pdflatex` for the preflight check);
- a PDF rasterizer for the embedded preview;
- `pdfinfo` (poppler) for page counts;
- for AI package installation, a TeX Live installation with `tlmgr` on `PATH`.

None of these are bundled. The `.deb` and RPM declare TeX and poppler as **recommendations**, so the editor installs and runs without them.

## 4. Identity and links

| Item | Value |
|---|---|
| Real repository | `https://github.com/borneelphukan/lekhani-latex` |
| Homepage / Pages | `https://borneelphukan.github.io/lekhani-latex/` |
| Crate/binary name | `lekhani-latex` |
| Workspace members | `lekhani-core`, `lekhani-compiler`, `lekhani-latex` |
| Version | `0.0.7` in `[workspace.package]` |
| License | MIT declared in `Cargo.toml` and AppStream metadata; **no root `LICENSE` file exists** |

Several in-app URLs still use `github.com/example/lekhani-latex` (update checker, About dialog). They are placeholders and currently do not resolve to this project. The website and AppStream metadata use the real `borneelphukan` URLs.

## 5. High-level architecture

```text
gui (lekhani-latex)
├── main.rs                 eframe entry, window/icon/style
├── app/mod.rs              App state, frame lifecycle, dialogs, update/LLM handling
├── app/tab.rs              Tab: buffer + compiler + preview + logs
├── app/editor.rs           TextEdit, syntax layout, completion popup, context menu
├── app/layout/*            Menu, toolbar, status bar, preview panel
├── components/*            Thin egui widget factories
└── preview.rs              External renderer discovery + per-page PNG rendering

compiler (lekhani-compiler)
├── compiler/bridge.rs      Per-tab compile thread, process kill, CompileEvent
└── compiler/parser.rs      Warning/error extraction from streams and .log

core (lekhani-core)
├── buffer/{mod,edit,cursor}.rs   EditorBuffer, save, undo/redo, cursor math
├── lexer.rs                Regex tokenizer for syntax colors
├── completions.rs          Static LaTeX command list + prefix search
└── types.rs                Theme, SyntaxColors, CompilerConfig, AppError
```

The dependency direction is `gui → compiler → core` and `gui → core`. Both library crates depend on `egui` only for color/syntax types (`core`) and compiler events (`compiler`).

## 6. What is intentionally naive

These choices are useful context before “improving” something:

- The editor is a single `egui::TextEdit::multiline` with a custom layouter. Syntax highlighting re-tokenizes the whole buffer on every layout/layout pass.
- The undo model records only operations performed through `EditorBuffer` helpers. Typing inside the `TextEdit` is detected as a generic change; undo history is therefore mostly populated by toolbar/heading/AI/completion edits, not by ordinary keystrokes.
- Every compile is a fresh subprocess. There is no persistent TeX server, no dependency graph, and no bibtex/makeindex orchestration beyond what `latexmk` does.
- Preview pages are rendered on demand and cached in memory per tab. There is no disk cache.
- The app has no settings UI beyond theme and AI; auto-compile is a toolbar checkbox that resets to `false` on every launch.
- There is no test suite, no benchmark, and no fuzzing. CI validates formatting, clippy, and a trivially passing test command.

## 7. Where to start reading

1. [02-project-structure.md](02-project-structure.md) for the file map.
2. [03-architecture.md](03-architecture.md) for threads and frame order.
3. [05-compile-and-preview-pipeline.md](05-compile-and-preview-pipeline.md) for the core behaviour.
4. [14-conventions-and-gotchas.md](14-conventions-and-gotchas.md) before changing URLs, saving, or preview code.
