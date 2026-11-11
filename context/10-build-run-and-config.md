# 10 — Build, Run, and Configuration

## 1. Prerequisites

| Requirement | Why |
|---|---|
| Rust toolchain with edition 2021 support | Workspace uses `edition = "2021"`. There is no `rust-toolchain.toml`; CI pins `1.99.1`, local users can use any recent stable. |
| `pkg-config` and X11/Wayland development headers on Linux | `eframe`/`winit` compile against them; CI installs the full list. |
| A TeX distribution with `latexmk` and `pdflatex` | Compilation. The preflight checks `pdflatex`; the compile command is `latexmk`. |
| `mutool`/`mudraw`, Ghostscript, or `pdftoppm` | Embedded PDF preview. |
| `pdfinfo` (poppler) | Page counts for preview navigation. |
| `tlmgr` (optional) | AI package installation. |
| Node 22 + npm (website only) | Website build. |

Linux development packages used by CI:

```text
pkg-config
libx11-dev libxcb1-dev libxcb-composite0-dev libxcb-cursor-dev
libxcb-image0-dev libxcb-icccm4-dev libxcb-keysyms1-dev
libxcb-randr0-dev libxcb-render0-dev libxcb-render-util0-dev
libxcb-shape0-dev libxcb-xfixes0-dev libxcb-xinerama0-dev
libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev
```

The GUI libraries (`libGL`, `libEGL`, `libdbus`, X11/Wayland client libraries) are `dlopen`ed at runtime by the egui/winit stack; the `.deb` lists them explicitly because `ldd` cannot see them.

## 2. Build and run

```bash
# workspace-wide
cargo build
cargo build --release

# run the GUI crate (workspace root)
cargo run -p lekhani-latex
cargo run -p lekhani-latex -- /path/to/document.tex
cargo run --release -p lekhani-latex
```

The binary is emitted as:

```text
target/debug/lekhani-latex
target/release/lekhani-latex
```

Platform targets used by release: `x86_64-pc-windows-msvc`, `i686-pc-windows-msvc`, `aarch64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin` (lipo to universal), and native `x86_64`/`aarch64` Linux.

### Quality commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

Clippy warnings are hard errors in CI. There are currently **no tests**, so `cargo test` succeeds without running anything.

### Release profile

```toml
[profile.release]
opt-level = 2
strip = true
```

No LTO, no codegen-units tuning, no panic strategy override.

## 3. Entry point details

`gui/src/main.rs`:

- `#![windows_subsystem = "windows"]` is **unconditional**. On Windows this hides the console even in debug builds, so `println!`/panic output may not be visible from a terminal; use a debugger or `RUST_BACKTRACE` written to a file if needed.
- On Linux, sets `MESA_LOG_FILE=/dev/null` if unset (silences Mesa/libEGL loader warnings).
- Loads `assets/logo.png` as the window icon; failure is non-fatal.
- Window: 1280×860, title `Lekhani Latex`.
- Initial style is dark; `App::new` applies the saved theme right away.
- `eframe::run_native` returns `Result<(), eframe::Error>`.

`gui/build.rs`:

- runs `winres` only when `CARGO_CFG_TARGET_OS == "windows"` and `../assets/logo.ico` exists;
- emits `GIT_BRANCH` from `git rev-parse --abbrev-ref HEAD` (or `unknown`), but nothing reads it;
- sets `cargo:rerun-if-changed` for itself and the icon.

## 4. Configuration

### Path

`dirs::config_dir()/lekhani-latex/config.json`:

| OS | Typical path |
|---|---|
| Linux | `~/.config/lekhani-latex/config.json` |
| macOS | `~/Library/Application Support/lekhani-latex/config.json` |
| Windows | `%APPDATA%\lekhani-latex\config.json` |

### Schema

```json
{
  "llm_api_key": "",
  "llm_settings_tab_index": 0,
  "llm_configured": false,
  "llm_model": "llama-3.2-3b",
  "llm_custom_model": "",
  "llm_endpoint_url": "http://localhost:1234/v1",
  "theme": "System"
}
```

Read/write functions in `gui/src/app/mod.rs`:

| Function | Behaviour |
|---|---|
| `get_app_config_path` | Builds the path above; `None` if no config dir. |
| `get_llm_config_path` | Legacy `llm_config.json` path. |
| `read_app_config` | Reads JSON; if missing and legacy exists, migrates (writes new, removes old). Invalid JSON yields `{}`. |
| `write_app_config` | Creates parent dirs and writes pretty JSON; errors ignored. |
| `load_llm_settings` | Applies defaults for missing keys. |
| `load_initial_llm_settings` | Adds `LLM_API_KEY` fallback and sets `configured = true` when a key is found. |
| `save_llm_settings` | Writes all LLM keys. |
| `save_theme` / `load_theme` | Write/read `"System"`, `"Light"`, `"Dark"`. |

There is no schema version. A corrupt or partially written config is silently replaced by defaults on the next save.

### Environment variables

| Variable | Effect |
|---|---|
| `LLM_API_KEY` | Used at startup only when no stored key exists; enables AI without writing the key to disk. |
| `MESA_LOG_FILE` | Set to `/dev/null` on Linux only if not already set. |
| `RUST_BACKTRACE` | Standard Rust behaviour; CI sets `1`. |
| `CARGO_INCREMENTAL` | CI sets `0`; not read by the app. |

No `LEKHANI_*` env vars, CLI flags, or config overrides exist.

### Non-persisted state

Reset on every launch:

- open tabs and active tab;
- auto-compile checkbox;
- preview visibility (defaults on per tab), zoom, page, pan mode;
- output panel visibility/tab;
- heading “Numbered” state (defaults `true`);
- window size/position/fullscreen.

## 5. Runtime tool discovery

| Tool | Discovered by | Failure behaviour |
|---|---|---|
| `pdflatex` | `Command::new("pdflatex").arg("--version")` on startup and compile | Missing → installer dialog |
| `latexmk` | Not checked; run directly by `CompilerBridge` | Compile failure with “not found” |
| `mutool`, `mudraw`, `gswin64c`, `gswin32c`, `gs`, `pdftoppm` | First one spawnable with `--version` | No renderer → error + broken external fallback |
| `pdfinfo` | `pdfinfo --version` | No page count; unbounded next-page navigation |
| `tlmgr` | Not checked; run on AI `install` | Error shown in AI log |
| `cmd` | `open_externally` (all platforms) | Silent failure on non-Windows |
| `gsettings` | Linux theme poller every 2 s | Silent failure on non-GNOME desktops |

Tool lookup uses the process `PATH`. There is no settings field for custom tool paths.

## 6. Local verification checklist

After a change, verify at least:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --release -p lekhani-latex
```

Manual smoke test:

1. Launch with no arguments; use “New Document” and “Open File”.
2. Type LaTeX; confirm highlighting, `\` completion, Tab insertion.
3. Save; re-open the same file; confirm no duplicate tab.
4. Compile a valid document; confirm PDF preview page 0 and page navigation.
5. Introduce a syntax error; confirm red output lines, highlighted editor lines, and “Fix with AI” gating.
6. Toggle preview, zoom with buttons and Ctrl+scroll, pan with middle mouse.
7. Toggle theme across System/Light/Dark; restart and confirm persistence.
8. Close with a dirty tab; confirm the prompt and that `on_exit` saves after confirmation.
9. Toggle auto-compile; type and wait; confirm recompiles.
10. (Optional) Configure a local OpenAI-compatible endpoint and exercise correction/install.

## 7. Troubleshooting

| Symptom | Likely cause |
|---|---|
| “Missing LaTeX Compiler” though TeX is installed | `pdflatex` not on `PATH` (GUI-launched apps often have a shorter PATH). |
| Compile fails with `latexmk` not found | `pdflatex` exists but `latexmk` does not; install `latexmk`. |
| Compile succeeds but no preview | No rasterizer installed; install `mupdf-tools`, `ghostscript`, or `poppler-utils`. On Linux/macOS the external fallback cannot work. |
| Preview says “No PDF renderer found” | Renderer detection failed at startup; restart after installing. |
| Page count stuck / next page errors | `pdfinfo` missing. |
| Preview image stale after edits | Only a successful compile invalidates the preview; fix the compile error. |
| Build fails on Linux with X11/Wayland errors | Missing development headers listed above. |
| Windows console output invisible | `#![windows_subsystem = "windows"]` is unconditional. |
| AI button disabled | No LLM configured, no compile error, or a request is already running. |
| AI request hangs | No timeout is configured; the request thread waits on `ureq`. |
| App freezes when saving/compiling | Save and the `join()` of the previous compile run on the UI thread. |
| `cargo test` passes instantly | There are no tests. |
