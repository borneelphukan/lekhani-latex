# 15 — Task Recipes

Step-by-step recipes for the changes that come up most often. Read the related context document first; each recipe names the files to touch and the checks to run.

## 0. Before any change

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

Keep changes narrow and update `context/` in the same PR. Clippy runs with `-D warnings` across the workspace, so new code must be warning-free.

## 1. Add a LaTeX command to autocompletion

1. Edit `core/src/completions.rs`: add a `"\\command{}"` entry in the appropriate section of `LATEX_COMMANDS` (alphabetical/sectional convention, not enforced).
2. If it is an environment, add both the `\begin{…}` form and any needed `\caption`/helper entries.
3. Check for typos — the current list contains the malformed `"\\{figure}[h]"` entry documented in [14-conventions-and-gotchas.md](14-conventions-and-gotchas.md).
4. Run the quality commands and manually type `\` + prefix in the editor.
5. Note: the popup is **not** scroll-synced and there are no tests; verify with several prefixes including a case-sensitive mismatch.

## 2. Change the compiler command (e.g. switch to `pdflatex` or add flags)

1. Update `CompilerConfig::default()` in `core/src/types.rs` (`command` and `args`).
2. Update the preflight check in **both** places so the right tool is detected:
   - `App::ui` first-frame startup check in `gui/src/app/mod.rs`;
   - `App::trigger_compile` in `gui/src/app/layout/menubar.rs`.
   Both currently run `pdflatex --version` and set `show_compiler_dialog` on failure.
3. Review the error-matching string in `poll_events` (`"was not found. Is a LaTeX distribution (MiKTeX/TeX Live) installed?"`) and the `Command`/`Stdio` handling in `compiler/src/compiler/bridge.rs` (flags order, `-output-directory`, cwd).
4. Verify the log parser still matches the new tool's output (`compiler/src/compiler/parser.rs`).
5. Update [05-compile-and-preview-pipeline.md](05-compile-and-preview-pipeline.md), [14](14-conventions-and-gotchas.md), and the root `README.md`.

## 3. Add or change a PDF preview renderer

1. In `gui/src/preview.rs`, add the binary to the `find_renderer` list **in discovery order** and add a match arm in `run_renderer`.
2. Use `(page + 1)` where the tool uses 1-based pages and keep the DPI at 150 unless there is a reason.
3. On Windows, keep `CREATE_NO_WINDOW` via `#[cfg(windows)] cmd.creation_flags(...)`.
4. Return a `String` error on nonzero exit; it becomes `PreviewEvent::Error`.
5. Consider the temp-file collision in [14](14-conventions-and-gotchas.md) — if you add cleanup, include the PDF identity in the filename, not just `pid + page`.
6. Document the new order and any extra system package in [05](05-compile-and-preview-pipeline.md) and [10-build-run-and-config.md](10-build-run-and-config.md).

## 4. Add a setting persisted in `config.json`

1. Add a field to `App` and initialise it in `App::new`.
2. Add a read function using `Self::read_app_config()` with a safe default (pattern: `load_theme`).
3. Add a write function that mutates the JSON and calls `Self::write_app_config(&config)` (pattern: `save_theme`). Write-on-change is the existing convention; there is no “Apply” step for theme.
4. Wire the UI control (menu checkbox, toolbar toggle, or a new dialog) and call the writer when it changes.
5. If the value is per-document rather than per-app, store it on `Tab`, not in the global config.
6. Update the schema in [04-data-model.md](04-data-model.md) and [10](10-build-run-and-config.md).
7. Consider whether an old, partially written config must be tolerated: the reader must never panic on missing/invalid keys.

## 5. Add a new feature panel or toolbar button

1. Add an `impl App` method in the closest layout file (`gui/src/app/layout/…`) or create a new module under `app/layout` and add it to `layout/mod.rs`.
2. Read state through `active_tab()` / `active_tab_mut()` and guard `tabs.is_empty()` first (indexing panics otherwise).
3. Use the `components::button::*` factories for a consistent look; hard-coded colours are the current style.
4. Call the method from `App::ui` at the right position in the panel order documented in [06-ui-and-interactions.md](06-ui-and-interactions.md).
5. If the button starts work, follow the deferred-flag pattern (`do_ai_fix`, `close_requested`) and update [03-architecture.md](03-architecture.md) and [06](06-ui-and-interactions.md).

## 6. Add a keyboard shortcut

1. Handle it in `App::ui`, alongside the existing `Cmd/Ctrl+O/S/Z/Y` checks, using `modifiers.command` so macOS `Cmd` works.
2. Guard on `!self.tabs.is_empty()` for document actions.
3. Beware that the completion popup consumes ↑/↓/Enter/Esc while visible ([14](14-conventions-and-gotchas.md)).
4. Add a hover hint on the matching toolbar/menu button (`on_hover_text`), since there is no shortcut list UI.
5. Update the shortcut tables in [06](06-ui-and-interactions.md) and `README.md`.

## 7. Change the AI prompt or response contract

1. Edit `do_llm_correction` in `gui/src/app/layout/menubar.rs` (prompt construction and the JSON request body).
2. Keep the single-object contract: `{"action":"correct"|"install", …}`. The tolerant parser lives in the same thread closure; the consumer is the `llm_rx` branch of `poll_events` in `gui/src/app/mod.rs`.
3. If you add an action, extend `LlmAction` in `gui/src/app/mod.rs`, handle it in `poll_events`, and cover the “no active tab” case without panicking.
4. Preserve the two prompt variants (`target_line.is_some()` and whole document). The explanation is shown in the AI output tab and is user-facing.
5. Update [08-ai-and-llm.md](08-ai-and-llm.md), including the JSON schema and any new `tlmgr` behaviour.

## 8. Fix the update checker / About links / website download links

1. Replace `example/lekhani-latex` in `check_for_updates` and `download_update` (`gui/src/app/mod.rs`) with `borneelphukan/lekhani-latex`.
2. Do **semver-aware** comparison instead of string inequality; compare the parsed tag with `env!("CARGO_PKG_VERSION")`.
3. Build the asset URL per platform from the real release assets (or read `assets[].browser_download_url`) rather than hard-coding `lekhani-latexSetup.exe`.
4. Make the download path cancellable (add a generation/cancel flag checked by the download thread) so “Cancel” really stops it.
5. Update the three About hyperlinks and `website/src/pages/Download.tsx` hrefs to the actual tag/filename scheme from [12-packaging-and-release.md](12-packaging-and-release.md).
6. Update [07-features-and-flows.md](07-features-and-flows.md), [13-website.md](13-website.md), and remove the gotcha entry in [14](14-conventions-and-gotchas.md) only when fixed.

## 9. Add a Rust test

There is currently no test infrastructure, so start small:

1. Add `#[cfg(test)] mod tests` in the crate under test (`core` is the easiest — pure logic, no UI).
2. Good first targets: `EditorBuffer::insert_str`/`undo`/`redo`/`replace_all`, `cursor_line_col`, `lexer::tokenize` token ranges, `completions::find_completions`, `parser::extract_errors`/`extract_warnings` against captured TeX log text.
3. Keep `compiler` tests free of subprocesses; test the pure parser functions.
4. Run `cargo test --workspace --locked`; CI already gates on it.
5. Note in [14](14-conventions-and-gotchas.md) once real tests exist.

## 10. Add a platform package or change artifact names

1. Add/modify the script under `packaging/`. Scripts must take a prebuilt binary and not run `cargo`.
2. Update `.github/workflows/release.yml`:
   - matrix entry and/or job steps;
   - `upload-artifact` name (it becomes the artifact name consumed by `release` and `publish-pages`);
   - the `expected=()` filename pattern list in the `release` job.
3. Update `.github/workflows/ci.yml` → `linux-assets` shell-syntax checks if you add a script.
4. Update [11-ci-cd.md](11-ci-cd.md), [12-packaging-and-release.md](12-packaging-and-release.md), and the website download table.
5. Missing expected files **fail** the release job, so run one full workflow_dispatch before tagging.

## 11. Bump the version

1. Edit `version` in `[workspace.package]` in the root `Cargo.toml` (the **first** `version =` line: `release.yml` parses it with `sed`).
2. `cargo update --workspace` is not required unless dependencies changed; commit `Cargo.lock` if it changed.
3. Run the full quality commands and a release build.
4. Optional metadata review:
   - `packaging/debian/lekhani-latex.metainfo.xml` — the `<release version>` in git lags behind; `build-deb.sh` rewrites it at build time, `build-rpm.sh` does not;
   - the website does not display a version (`VITE_APP_VERSION` is unused);
   - the Windows four-part version is generated as `major.minor.patch.0` by the workflow.
5. Commit `chore: bump version to X.Y.Z`, tag `vX.Y.Z`, and push both. Ensure the tag matches `Cargo.toml` — nothing enforces it.

## 12. Make a release

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --release -p lekhani-latex
git commit -m "chore: bump version to <v>"
git push origin main            # runs the packaging matrix + Pages
git tag v<v>
git push origin v<v>            # creates the GitHub Release with artifacts
```

Then verify:

- the `release` job passed the 14 expected filename patterns and produced `SHA256SUMS`;
- `publish-pages` succeeded (Debian + website jobs green) and the site root serves the React app;
- `curl -fsSL <site>/install.sh | sudo bash -n` parses and the apt `Release` lists `Architectures: amd64`;
- `SHA256SUMS` matches a downloaded asset;
- the GitHub Release title/notes look right.

## 13. Debug a compile/preview problem

- Confirm what is on `PATH`: `which pdflatex latexmk mutool gs pdftoppm pdfinfo tlmgr`.
- Watch the output panel colours: yellow warnings, red `× Error[line N] …`, green `√ Compilation succeeded in …`.
- Check the output directory `<parent>/<stem>/` for the `.log` and the PDF.
- On Linux/macOS, the “no renderer” external fallback cannot work; install a rasterizer rather than relying on it.
- If the preview is stale, remember only a successful compile clears it.
- If the UI seems frozen, suspect a UI-thread join or a file dialog, not a background worker ([09-worker-and-concurrency.md](09-worker-and-concurrency.md)).

## 14. Debug an AI problem

- The AI button is gated on `llm_configured && active_tab.error_message.is_some()`; compile first.
- Local mode posts to the stored endpoint verbatim: use `.../v1/chat/completions`.
- API mode ignores the stored endpoint and hard-codes `https://api.openai.com/v1/chat/completions`.
- Check the AI Output tab (it auto-opens on results); invalid JSON surfaces the raw response.
- The API key is stored in `config.json`; `LLM_API_KEY` is only a startup fallback.

## 15. Update this pack

- Prefer module/function names and contracts over line numbers.
- When behaviour changes, edit the relevant document in the same PR; if a bug is fixed, delete or rewrite its gotcha entry.
- After packaging, workflow, or website changes, re-check [11](11-ci-cd.md), [12](12-packaging-and-release.md), and [13](13-website.md).
- Update the verified commit and version in `README.md` when the table drifts.
