# 02 — Project Structure

## 1. Top-level tree

```text
lekhani-latex/
├── Cargo.toml                 workspace: members, versions, shared dependencies, release profile
├── Cargo.lock                 committed lockfile (MSRV not pinned by a rust-toolchain file)
├── README.md                  user-facing feature/usage document (partly stale)
├── document.log               stray TeX log accidentally tracked at repo root
├── .cargo/audit.toml          cargo-audit configuration (empty ignore list)
├── .github/workflows/         ci.yml, audit.yml, release.yml, website.yml
├── assets/
│   ├── cover.png              marketing/README image, not used by the app
│   ├── logo.png               app window icon, .deb/RPM/AppImage/DMG icon source
│   ├── logo.ico               Windows executable/installer icon
│   └── lekhani-latex.desktop  duplicate desktop entry (packaging copies the packaging/ one)
├── core/                      library crate: editor model, lexer, completions, shared types
├── compiler/                  library crate: compile thread and log parser
├── gui/                       binary crate `lekhani-latex`
│   ├── build.rs               Windows icon via winres; sets GIT_BRANCH env (currently unused)
│   └── src/
│       ├── main.rs            eframe entry + initial style/icon
│       ├── preview.rs         external PDF renderer + page texture pipeline
│       ├── app/
│       │   ├── mod.rs         App state, lifecycle, dialogs, update/LLM/package handling
│       │   ├── tab.rs         per-document state container
│       │   ├── editor.rs      text editor, layouter, completion, context menu
│       │   └── layout/
│       │       ├── menubar.rs     menus + file actions + compile/AI triggers
│       │       ├── toolbar.rs     compile/AI buttons, auto-compile, preview toggle
│       │       ├── statusbar.rs   path, dirty marker, Ln/Col, compile status
│       │       └── preview_panel.rs preview toolbar and page list
│       └── components/        button/dropdown/textfield factories
├── installer/lekhani-latex.iss Inno Setup script
├── packaging/
│   ├── debian/                build-deb.sh, desktop, AppStream, maintainer scripts, copyright
│   ├── rpm/                   build-rpm.sh
│   ├── linux/                 build-tarball.sh, build-appimage.sh
│   ├── macos/                 build-dmg.sh
│   └── apt/                   build-repo.py, install.sh
└── website/                   React/Vite landing page + download page
```

## 2. Workspace crates

| Crate | Path | Binary/lib | Responsibility | Key dependencies |
|---|---|---|---|---|
| `lekhani-core` | `core/` | lib | `EditorBuffer`, cursor math, undo/redo, `Theme`, `CompilerConfig`, lexer, completion list | `egui`, `regex` |
| `lekhani-compiler` | `compiler/` | lib | `CompilerBridge`, `CompileEvent`, log parsing | `lekhani-core` |
| `lekhani-latex` | `gui/` | bin | All UI, preview, config, LLM, updates, installers | `lekhani-core`, `lekhani-compiler`, `egui`, `eframe`, `winit`, `rfd`, `ureq`, `dirs`, `arboard`, `serde_json`, `image`, `regex` |

The workspace pins `egui = "0.34"` and `eframe = "0.34"` with:

```toml
eframe = { version = "0.34", default-features = false,
           features = ["default_fonts", "wayland", "web_screen_reader", "wgpu", "x11"] }
winit = "0.30"
```

`accesskit` is deliberately **off** because (per the Cargo comment) it pulls `zbus`/`atspi` and `quick-xml 0.39`, which has open advisories. `winit` is declared explicitly to restore Wayland dynamic loading and client-side decorations that `default-features = false` would otherwise drop.

## 3. Where behaviour lives

| Behaviour | Primary location | Secondary/notes |
|---|---|---|
| Window, icon, startup style | `gui/src/main.rs` | `App::new` re-applies saved theme. |
| Global app state, frame order, dialogs | `gui/src/app/mod.rs` | `impl eframe::App::ui`, not `update`. |
| Open/save/new/compile triggers | `gui/src/app/layout/menubar.rs` | File menu actions and `trigger_compile`. |
| Compile status/auto-compile/preview toggle | `gui/src/app/layout/toolbar.rs` | Auto-compile logic actually lives in `App::ui`. |
| Tab model and per-tab state | `gui/src/app/tab.rs` | Also constructs default `CompilerBridge`. |
| Text editing and completion popup | `gui/src/app/editor.rs` | Includes the AI context-menu entry. |
| Syntax tokenization | `core/src/lexer.rs` | Regex over whole text; maps to `SyntaxColors`. |
| Undo/redo and cursor movement | `core/src/buffer/{edit,cursor}.rs` | `replace_all` pushes delete+insert tokens. |
| Compile subprocess | `compiler/src/compiler/bridge.rs` | Kills previous PID, removes stale PDF/log. |
| Error/warning extraction | `compiler/src/compiler/parser.rs` | Streams first, `.log` fallback, truncation. |
| PDF renderer discovery/rendering | `gui/src/preview.rs` | `open_externally` is Windows-only code. |
| Theme definitions/syntax colors | `core/src/types.rs` | `Theme::resolve`, `syntax_colors`. |
| Persisted settings | `gui/src/app/mod.rs` | `get_app_config_path`, `read_app_config`, writers. |
| Update check/download | `gui/src/app/mod.rs` | `check_for_updates`, `download_update`. |
| TeX installer download | `gui/src/app/mod.rs` | `start_compiler_download`. |
| LLM request/prompt/response | `gui/src/app/layout/menubar.rs` | `do_llm_correction` spawns the thread. |
| LLM settings window | `gui/src/app/mod.rs` | `llm_settings_dialog` uses an immediate viewport. |
| Packaging metadata/scripts | `packaging/**` | Scripts read a prebuilt binary. |
| Release orchestration | `.github/workflows/release.yml` | Version read from `Cargo.toml`. |
| Landing/download site | `website/src/**` | `HashRouter`, base `/lekhani-latex/`. |

## 4. Module and privacy patterns

- `gui/src/app/mod.rs` declares `mod editor; pub mod layout; pub mod tab;`. Layout modules are children of `app`, so they can access private `App` fields and methods without `pub(crate)` on every field. The root README calls this “module-per-feature”.
- `components/` contains only thin constructors (`standard`, `toolbar`, `icon`, `dropdown`, `password`, etc.), not stateful widgets.
- `core` and `compiler` are ordinary libraries. `core/src/lib.rs` re-exports modules, not symbols; callers use full paths such as `lekhani_core::buffer::EditorBuffer`.
- `gui/build.rs` runs `winres` only for Windows targets and emits `GIT_BRANCH`, but no code consumes `GIT_BRANCH`.

## 5. Files that are easy to miss

- `document.log` is tracked at the repo root. It is a real LaTeX log from a local build, not used by the app. Do not treat it as a fixture.
- `assets/lekhani-latex.desktop` duplicates `packaging/debian/lekhani-latex.desktop`; packaging scripts use the latter.
- `website/src/App.css`, `website/src/assets/{hero.png,react.svg,vite.svg}` are leftover template assets and are not imported.
- `website/package-lock.json` and `website/pnpm-lock.yaml` are both committed, but CI and release use **npm** with `package-lock.json`.
- `packaging/apt/build-repo.py` also writes `index.html` and copies `install.sh`; the website later overwrites `index.html` in the Pages artifact.

## 6. What not to edit casually

| Area | Why |
|---|---|
| `Cargo.toml` workspace version | Release workflow derives every package name, tag documentation, and Windows version from the first `version =` line. |
| `eframe` features | Removing `wgpu` or `x11`/`wayland` breaks shipped Linux/Windows rendering. Re-enabling defaults restores `accesskit` and its advisory chain. |
| `packaging/debian/build-deb.sh` dependency list | The GUI libraries are `dlopen`ed, so `dpkg-shlibdeps`/`ldd` cannot discover them; the explicit list is intentional. |
| `.github/workflows/release.yml` expected artifact patterns | The release job fails if any expected filename is missing. Renaming an artifact requires updating both the script and the pattern list. |
| `website/vite.config.ts` `base` | Must stay `/lekhani-latex/` for the GitHub Pages project-site path. |
