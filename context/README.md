# lekhani-latex — AI Agent Context Pack

This directory is the **authoritative, current context** for anyone (human or AI) working on `lekhani-latex`. It describes the application as it exists at:

| Field | Value |
|---|---|
| Repository | `github.com/borneelphukan/lekhani-latex` |
| Branch verified | `main` |
| Commit verified | `7eba198` (`fix(ci): set GH_REPO in release job so gh works without a checkout`) |
| Version | `0.0.7` (`Cargo.toml` workspace package) |
| Rust edition | `2021` |
| UI stack | `eframe` / `egui` `0.34` with the `wgpu` renderer |
| PDF preview | external CLI rasterizer (`mutool`, `mudraw`, Ghostscript, or `pdftoppm`) |
| Platforms | Windows (x64/x86/ARM64), Linux (x86_64/ARM64), macOS (universal) |
| License | MIT (declared in `Cargo.toml`; **no `LICENSE` file is committed**) |
| Website | React 19 + TypeScript + Vite 8 + Tailwind 4, GitHub Pages |

> There is no `AGENTS.md` in this repository. Treat this `context/` directory as the source of truth. The root `README.md` is user-facing and is sometimes stale: it says compilation needs `pdflatex`, while `CompilerConfig::default()` actually invokes `latexmk`, and it does not describe the AI features, update checker, or apt repository.

---

## What to read first

| Question | Read |
|---|---|
| What does the app do and what are its boundaries? | [01-project-overview.md](01-project-overview.md) |
| Which file owns which behaviour? | [02-project-structure.md](02-project-structure.md) |
| How do the process, threads, UI, and state fit together? | [03-architecture.md](03-architecture.md) |
| What are the Rust types and message contracts? | [04-data-model.md](04-data-model.md) |
| How does a `.tex` file become a compiled PDF and a preview page? | [05-compile-and-preview-pipeline.md](05-compile-and-preview-pipeline.md) |
| How is the egui UI assembled and how do shortcuts behave? | [06-ui-and-interactions.md](06-ui-and-interactions.md) |
| How does each user-facing feature work end to end? | [07-features-and-flows.md](07-features-and-flows.md) |
| How do AI correction and LLM configuration work? | [08-ai-and-llm.md](08-ai-and-llm.md) |
| How does background work and messaging work? | [09-worker-and-concurrency.md](09-worker-and-concurrency.md) |
| How do I build, run, and configure it locally? | [10-build-run-and-config.md](10-build-run-and-config.md) |
| How do CI, audits, and workflows work? | [11-ci-cd.md](11-ci-cd.md) |
| How are Linux/Windows/macOS packages and the apt repo produced? | [12-packaging-and-release.md](12-packaging-and-release.md) |
| How does the GitHub Pages website work? | [13-website.md](13-website.md) |
| What patterns, bugs, and platform traps should I avoid? | [14-conventions-and-gotchas.md](14-conventions-and-gotchas.md) |
| I need a concrete step-by-step change recipe | [15-task-recipes.md](15-task-recipes.md) |

---

## Critical facts at a glance

1. **Three crates, one application.** `core` is the pure editor model/lexer/completions, `compiler` wraps `latexmk` and log parsing, and `gui` is the `eframe` binary `lekhani-latex`.
2. **The UI is immediate mode.** `App` owns `Vec<Tab>` and an `active_tab` index. Each `Tab` owns its own `EditorBuffer`, `CompilerBridge`, `PreviewViewer`, textures, and logs. Panels are `impl App` methods in separate modules; there is no retained widget tree.
3. **Background work is channel-based.** Every tab has its own compile channel and preview channel. One shared `mpsc` channel carries app-wide update/download/package-install results, and another carries LLM results. All channels are drained with `try_recv()` once per frame.
4. **Compilation is `latexmk`, not `pdflatex`.** `CompilerConfig::default()` is `latexmk -pdf -interaction=nonstopmode`; however `trigger_compile()` and the startup check only test `pdflatex --version`. A machine with `pdflatex` but no `latexmk` passes the check and then fails at compile time.
5. **The preview is an external-process pipeline.** The app searches for `mutool`, `mudraw`, `gswin64c`, `gswin32c`, `gs`, then `pdftoppm`, renders one page at 150 DPI into a PNG in the system temp dir, and uploads it as an egui texture. `pdfinfo` is required for page counts. Temp PNGs are never deleted.
6. **Saving is only partly atomic.** `EditorBuffer::write_atomically` writes `<name>.tex.tmp`, then **removes the destination**, then renames. The explicit remove makes it a non-crash-safe replace. The compile button and auto-compile bypass this entirely and call `std::fs::write` on the source file.
7. **Configuration is one JSON file.** `dirs::config_dir()/lekhani-latex/config.json` holds the theme and all LLM keys. The API key is stored in plaintext. A legacy `llm_config.json` is migrated once. `LLM_API_KEY` is only a startup fallback.
8. **AI features are optional and error-driven.** “Fix with AI” is enabled only when an LLM is configured and the active tab has a compile error. The model must return one JSON object with `action: "correct"` or `action: "install"`; installs run `tlmgr install <package>`.
9. **CI is strict but there are no tests.** The gate is `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --locked`; the workspace currently contains zero `#[test]` functions, so tests always pass trivially.
10. **Releases are tag-driven and multi-platform.** `release.yml` reads the version from `Cargo.toml`, builds Debian/RPM/tarball/AppImage/Windows installer+zip/universal DMG, verifies the expected filenames, writes `SHA256SUMS`, and attaches everything to a `v*` GitHub Release. Non-tag runs publish the same bundle as a workflow artifact.
11. **GitHub Pages is both the landing page and the apt repository.** The Debian amd64 package is indexed under `/apt`, `install.sh` is served at the site root, and the React site is copied over the same Pages artifact at the root.
12. **Repository ↔ code URL drift.** The real repository is `borneelphukan/lekhani-latex`, but the in-app update checker, About links, and several website download links still use `example/lekhani-latex` or non-existent release tags. See [14-conventions-and-gotchas.md](14-conventions-and-gotchas.md) before touching those areas.

---

## Repository entry points

- Application entry: `gui/src/main.rs` → `app::App::new` → `impl eframe::App for App::ui` in `gui/src/app/mod.rs`.
- Tab model: `gui/src/app/tab.rs`.
- Layout/panels: `gui/src/app/layout/{menubar,toolbar,statusbar,preview_panel}.rs`.
- Editor text area, syntax layout, completion, context menu: `gui/src/app/editor.rs`.
- Compiler bridge and log parsing: `compiler/src/compiler/{bridge,parser}.rs`.
- Editor model, lexer, completions: `core/src/{buffer/*,lexer.rs,completions.rs,types.rs}`.
- PDF rendering: `gui/src/preview.rs`.
- Packaging: `packaging/**`, `installer/lekhani-latex.iss`.
- Release automation: `.github/workflows/{ci,audit,release,website}.yml`.
- Website: `website/src/**`, `website/vite.config.ts`.

## Keeping this pack accurate

When behaviour changes, update the relevant file here **in the same PR**. Prefer function/module names and contracts over large code blocks; line numbers drift quickly. If `Cargo.toml`, `Cargo.lock`, `installer/lekhani-latex.iss`, `packaging/**`, `website/**`, or the workflows change, re-check [11-ci-cd.md](11-ci-cd.md), [12-packaging-and-release.md](12-packaging-and-release.md), and [13-website.md](13-website.md).
