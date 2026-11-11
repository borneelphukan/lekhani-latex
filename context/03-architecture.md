# 03 — Architecture

## 1. Process and thread model

```text
┌─────────────────────────────────────────────────────────────────────┐
│ Native process (lekhani-latex)                                      │
│                                                                     │
│  ┌──────────────────────────────────┐                               │
│  │ Main / UI thread                 │                               │
│  │ eframe + egui (wgpu)             │                               │
│  │ App { tabs: Vec<Tab>, ... }      │                               │
│  │ textures created here            │                               │
│  └───────┬───────────────┬──────────┘                               │
│          │               │                                          │
│          │ per tab       │ per tab                                  │
│          ▼               ▼                                          │
│  ┌───────────────┐  ┌──────────────────┐                            │
│  │ Compile thread│  │ Preview threads  │  one short-lived thread    │
│  │ (join handle  │  │ (spawned per     │  per requested page        │
│  │  per tab)     │  │  page render)    │                            │
│  └───────┬───────┘  └────────┬─────────┘                            │
│          │ CompileEvent      │ PreviewEvent                         │
│          └──────────┬────────┘                                     │
│                     ▼                                              │
│            Tab.poll() / try_recv() once per frame                  │
│                                                                     │
│  Shared app-wide mpsc: UpdateMessage (update check/download,        │
│  TeX installer download, tlmgr result, OS theme)                    │
│  Shared LLM mpsc: Result<LlmAction, String>                         │
│  Linux-only theme poller: std::thread, gsettings every 2 s forever  │
└─────────────────────────────────────────────────────────────────────┘
```

Properties:

- **No async runtime.** All background work uses `std::thread` + `std::sync::mpsc`, plus `ureq` for blocking HTTP.
- **Per-tab compile concurrency.** Each `Tab` owns a `CompilerBridge`; two tabs can compile simultaneously. Within a tab, starting a compile kills the previous child and joins the previous thread.
- **Per-page preview concurrency.** `PreviewViewer::ensure_page_rendered` spawns a thread per page not already rendered/in-flight. `active_renders` prevents duplicates.
- **One shared LLM channel.** Only one correction can be in flight globally (`llm_correction_in_progress`), and its result is applied to whichever tab is active when the message is drained.
- **Polling, not wake-ups.** Compile/preview/LLM threads do not call `ctx.request_repaint()` (only update/installer threads do). Results are observed on the next frame that egui happens to draw. During a compile the UI still redraws on input; there is no forced animation.
- **Textures are created only on the UI thread** in `App::update_preview_textures` via `ctx.load_texture`.
- **Saving and file writes run on the UI thread** except the AI package install and downloads.

## 2. Startup sequence

1. `main()` sets `MESA_LOG_FILE=/dev/null` on Linux if unset (suppresses Mesa/EGL warnings).
2. `load_icon()` decodes `assets/logo.png` with the `image` crate and builds an `egui::IconData`.
3. `eframe::NativeOptions` uses a 1280×860 viewport titled `Lekhani Latex`, with the icon if decoding succeeded. No minimum size is set.
4. The `eframe` creation closure installs a hard-coded dark style (selection grey, translucent dark panels, 8–12 px corner radii).
5. `App::new(cc)`:
   - creates the update and LLM `mpsc` channels;
   - loads LLM settings (config file, then `LLM_API_KEY`, then defaults);
   - initializes the preset/custom model selector;
   - loads the saved theme;
   - opens the first CLI argument as a tab if it exists;
   - applies the saved theme;
   - on Linux, spawns the `gsettings` theme poller;
   - installs a font fallback: `Hack` appended to the `Proportional` family so `▲▼` glyphs exist on Ubuntu/NotoEmoji setups.
6. The first frame runs with no tabs and shows the empty state.

There is no splash screen, no single-instance lock, no file-association handling beyond the first positional argument, and no restore-session feature.

## 3. Frame lifecycle

`impl eframe::App for App` implements `fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame)` (eframe 0.34 supports a `ui` entry point; there is no `update` method here).

Order per frame:

1. **Startup compiler check**, once: run `pdflatex --version`; if missing, set `show_compiler_dialog = true`.
2. **Apply theme** from `self.theme`/OS theme every frame.
3. **Drain app-wide `UpdateMessage`s** (`process_update_messages`).
4. **Fullscreen keys**: `F12` toggles; `Esc` exits when fullscreen.
5. **Close request**: if any tab is dirty, show an `rfd` Yes/No dialog; “No” sends `ViewportCommand::CancelClose`.
6. **Global shortcuts**: `Cmd/Ctrl+O`, `Cmd/Ctrl+S`, `Cmd/Ctrl+Z`, `Cmd/Ctrl+Y` (last three only when tabs exist). These set `file_dialog_action` or mutate the active buffer.
7. **Execute one file-dialog action**, if any.
8. **`poll_events()`**: drain every tab’s compile and preview channels, then the shared LLM channel. This is where output logs, error lines, preview state, and AI edits are updated.
9. **Flush output-panel requests** and **upload rendered preview textures**.
10. **Auto-compile**: if enabled, the active tab is dirty, ≥0.8 s since the last content change, and ≥1.2 s since the last auto-compile, write the file directly and start a compile.
11. **Top panels** (menu bar, tab bar, toolbar) unless fullscreen (macOS keeps panels in fullscreen).
12. **Bottom panels** (status bar, output bar, output panel).
13. **Central panel**: empty state or formatting toolbar + optional resizable right preview panel + editor area.
14. **Fullscreen toast** area.
15. **Modal-ish windows**: About, update dialog/banner, AI settings viewport, missing-compiler dialog.

Because `poll_events()` runs before drawing, a result sent during frame *N* is normally visible in frame *N+1*.

## 4. State ownership

`App` owns:

| Field group | Purpose |
|---|---|
| `tabs: Vec<Tab>`, `active_tab: usize` | All open documents and which is shown. |
| `theme`, `os_theme` | Config-selected theme and last OS theme reported. |
| `file_dialog_action` | Deferred native file dialog request. |
| completion fields | Popup visibility, matches, byte range, selection, prefix, trigger suppression. |
| `show_outputs*`, `output_panel_tab` | Output panel visibility and Compiler/AI tab. |
| `about_open`, `update_state`, `update_rx/tx` | About and update UI + shared channel. |
| `heading_numbered`, `auto_compile`, `last_*` | Formatting state and auto-compile timers. |
| `tab_drag` | Drag-reorder state `(index, start_x)`. |
| LLM fields | In-flight flag, channel, API key, tab index, preset/custom model, endpoint, configured flag, dialog visibility. |
| compiler-dialog fields | Startup check latch, dialog visibility, download progress state. |

`Tab` owns:

| Field | Purpose |
|---|---|
| `title` | File stem shown in the tab. |
| `buffer: EditorBuffer` | Text, path, dirty flag, cursor, undo/redo, line index. |
| `compiler: CompilerBridge` | Per-tab compile status/thread/channel/active PID. |
| `preview: PreviewViewer` | Per-tab renderer, page cache, zoom, page number, textures source. |
| `show_preview` | Whether the right panel is visible. |
| `preview_textures: HashMap<usize, TextureHandle>` | GPU textures for rendered pages. |
| `status_message`, `error_message`, `error_lines` | Status line, full compiler error text, highlighted lines. |
| `output_log`, `ai_output_log` | Colored `(String, Color32)` log lines. |
| `scroll_offset`, `scroll_request` | Preview scroll state and one-shot scroll target. |
| `compile_start_time` | Used to display compile duration. |

## 5. Borrow and mutation patterns

- UI methods take `&mut self` and frequently clone small values (paths, error lines, log vectors, page numbers) before entering closures to avoid borrowing all of `self`.
- `editor_area` takes the buffer text out with `std::mem::take`, feeds it to `TextEdit`, then puts it back, because the layouter and the completion popup need `&mut self` while the text is being edited.
- `preview_content` clones `image_size`, `zoom`, `page`, `num_pages`, and `pdf_path` before the scroll closure, then writes them back afterward.
- The active tab is always accessed through `active_tab()` / `active_tab_mut()`. With no tabs these panic, so every entry point checks `tabs.is_empty()` first. Any new call site must preserve that guard.
- Deferred actions are represented by local flags (`do_cut`, `do_copy`, `do_ai_fix`, `close_requested`, `remove_tab`) and applied after the relevant UI closure.

## 6. Failure behaviour

| Failure | User-visible result |
|---|---|
| `pdflatex` missing at startup/compile | “Missing LaTeX Compiler” download dialog. |
| `latexmk` missing but `pdflatex` present | Compile `Failure` with “'latexmk' was not found…”. |
| Empty/too-short/non-LaTeX source | Early `CompileEvent::Failure` before spawning. |
| Compile error | Red lines in the output panel, error lines highlighted in the editor, `error_message` stored, preview keeps the last image. |
| No PDF produced | Failure message “Compilation finished but no PDF was produced.” |
| No PDF renderer | `PreviewEvent::Unsupported`; sets `render_error` and calls `open_externally()` (which only works on Windows). |
| Renderer decode failure | `render_error` shown in the preview pane. |
| No `pdfinfo` | `num_pages = None`; “next page” stays enabled and out-of-range pages produce render errors. |
| Save failure | `tab.error_message` set; the window stays open. |
| LLM/network/JSON failure | `error_message` plus a red “LLM Error” line; AI output tab opened. |
| `tlmgr` install failure | AI log gets a red failure line; no retry. |
| Update check failure | Reported as `CheckResult(false, None)`, so the user gets a “No Updates” dialog even if the request/parse actually failed. |

## 7. Shutdown

`App::on_exit`:

- clears each tab’s `preview_textures` before the renderer is destroyed;
- saves every dirty tab by calling `buffer.save_as(&path)` if it has a path.

`CompilerBridge::drop` and `PreviewViewer::drop` are **empty**; a running compile child is not explicitly killed when the window closes, and preview threads are allowed to die with the process. The app also does not flush undo history or ask per-tab save questions at exit; it relies on the close-request confirmation and then auto-saves dirty tabs.
