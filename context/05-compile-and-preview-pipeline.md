# 05 — Compile and Preview Pipeline

This is the core loop of the application: take the text in the active `EditorBuffer`, run the local TeX toolchain, parse the result, and render the PDF one page at a time.

## 1. End-to-end compile flow

```text
Compile button / Ctrl+S with auto-compile / auto-compile timer
        │
        ▼
App::trigger_compile()  (gui/src/app/layout/menubar.rs)
        │
        ├─ run `pdflatex --version`
        │      └─ missing? → show_compiler_dialog = true; return
        │
        ├─ tab.show_preview = true
        ├─ path = tab.buffer.path()
        ├─ no path? → error "Please save the file before compiling."
        └─ std::fs::write(path, tab.buffer.text)      ← direct write, not atomic
                │
                ▼
CompilerBridge::compile(path)  (compiler/src/compiler/bridge.rs)
        │
        ├─ kill previous active PID (kill -9 / taskkill /F)
        ├─ join previous compile thread
        ├─ spawn thread, status = Running
        └─ send CompileEvent::Started
                │
                ▼
   validate source (non-empty, length ≥ 10, contains LaTeX structure)
        │
        ├─ output_dir = parent/<file_stem>
        ├─ create_dir_all(output_dir)
        ├─ remove stale <stem>.pdf and <stem>.log
        ├─ Command::new("latexmk")
        │     .args(["-pdf", "-interaction=nonstopmode"])
        │     .arg("-output-directory=<output_dir>")
        │     .arg(<absolute .tex path>)
        │     .current_dir(parent)
        └─ capture stdout/stderr
                │
                ▼
   success = exit status success && !log.contains("Fatal error occurred")
        ├─ success && <stem>.pdf exists → CompileEvent::Success(pdf)
        └─ otherwise → extract_errors(...) → CompileEvent::Failure(errors)
```

### 1.1 Preflight check is inconsistent with the command

`trigger_compile` and the first-frame startup check both execute `pdflatex --version`. The actual compile command comes from `CompilerConfig::default()` and is `latexmk`. Consequences:

- `pdflatex` present + `latexmk` absent → the dialog does not appear, compile fails with “'latexmk' was not found…”;
- `latexmk` present + `pdflatex` absent is not a supported configuration (the preflight rejects it).

Any change to the compiler command must update the preflight check too. See [15-task-recipes.md](15-task-recipes.md).

### 1.2 Output layout

For a source at `/home/me/paper/main.tex`:

```text
/home/me/paper/main/          ← output directory
├── main.pdf                  ← success artifact
├── main.log                  ← parsed for fatal errors/warnings
├── main.aux
└── ...                       ← whatever latexmk/TeX writes
```

The trailing directory is `<parent>/<file_stem>`, so compiling `main.tex` twice reuses the same output directory. Stale `main.pdf` and `main.log` are deleted before every run; other auxiliary files are left to `latexmk`.

`new_document` and `save_as_file` use `project_tex_path` to create `<chosen parent>/<chosen stem>/<chosen filename>`. The file dialog asks for a path, but the actual file is moved one directory deeper, which keeps per-project output isolated.

### 1.3 Source validation

Before spawning, the compile thread rejects:

- unreadable files;
- zero-length files (“The document is empty…”);
- files shorter than 10 bytes;
- files shorter than 100 bytes that contain none of `\document`, `\begin{`, `\section`, `\chapter`, or `\end{`.

These checks are heuristic; a legitimate very short fragment can be rejected, and a malformed 200-byte file passes.

### 1.4 Process lifecycle

- `active_pid: Arc<AtomicU32>` stores the child PID (`0` = none).
- Starting a new compile kills the old child with `kill -9` (Unix) or `taskkill /F /PID` (Windows), then joins the old thread.
- On Windows the child is spawned with `CREATE_NO_WINDOW = 0x08000000`.
- The bridge does not implement cancellation/timeout. A hung `latexmk` blocks its thread until the process exits or a new compile is requested.
- `Drop for CompilerBridge` is empty, so an app exit does not explicitly kill a running child.

### 1.5 `CompileEvent` handling in the UI

`App::poll_events()` drains each tab’s events:

| Event | Effect |
|---|---|
| `Started` | status “Compiling…”, records `compile_start_time`, forces preview on. |
| `Warnings(lines)` | appends yellow warning lines to `output_log`; status unchanged. |
| `Success(pdf)` | status “Compilation succeeded”; clears errors/error lines; replaces the log with a green “√ Compilation succeeded in …”; resets preview page/textures/page count; starts rendering page 0; opens the output panel. |
| `Failure(errors)` | status “Compilation failed”; stores joined errors; clears the log; detects the “compiler not found” message and opens the compiler dialog; parses `!` + `l.<n>` pairs into red log lines and `error_lines`; opens the output panel. |

Failure parsing details (in `poll_events`):

- a `! ...` line starts an error record; a following `l.<n>` line supplies the line number;
- the source line is looked up in the buffer for the display text;
- the display format is `× Error[line N]: <source line> <error code>`;
- line numbers are sorted and deduplicated for editor highlighting.

`error_line_regex()` (`l\.(\d+)`) is compiled once in a `OnceLock`.

## 2. Error and warning extraction (compiler/src/compiler/parser.rs)

| Priority | Errors | Warnings |
|---|---|---|
| 1 | Lines from stdout/stderr that start with `! ` (excluding `Emergency stop`, `Fatal error`, `==>`) or `l.` | Lines containing `Warning:`, `Overfull`, `Underfull` |
| 2 | If none, scan the `.log` for `! ` and include following `l.`/indented continuation lines | Add matching log lines not already present |
| 3 | If still none, take the last 15 log lines, filter noise (`Transcript written`, `Output written`, parenthesis lines), keep the last 5 | — |
| 4 | Fallback string “Compilation failed. Check your LaTeX syntax.” | — |
| Cap | 8 errors | 10 warnings |

`read_log_fatal` treats the presence of `Fatal error occurred` anywhere in the log as failure even when the process exit status is zero.

## 3. Preview pipeline

### 3.1 Renderer discovery

`PreviewViewer::find_renderer` probes, in order:

1. `mutool`
2. `mudraw`
3. `gswin64c`
4. `gswin32c`
5. `gs`
6. `pdftoppm`

It runs `<tool> --version` and accepts the **first tool that can be spawned**, regardless of exit status. The chosen name is cached in `PreviewViewer::renderer` for the tab’s lifetime.

`pdfinfo` is probed separately (`pdfinfo --version`) and used only to obtain the page count.

### 3.2 Page-count resolution

`get_pdf_page_count`:

- returns `None` if `pdfinfo` cannot be spawned;
- otherwise parses the `Pages:` line from `pdfinfo <pdf>`;
- `num_pages` is cached on the viewer and reset to `None` on every successful compile.

If `pdfinfo` is missing, navigation has no upper bound; requesting a non-existent page produces a renderer error.

### 3.3 Render-to-texture flow

```text
preview_content() (ui)
  └─ for visible pages p:
       if !preview_textures.contains_key(p) && pdf_path.is_some():
            preview.ensure_page_rendered(pdf_path, p)
                 │
                 ├─ guard rendered_pages / active_renders
                 ├─ resolve renderer (once)
                 ├─ resolve page count (once)
                 ├─ output = <temp>/lekhani_preview_<pid>_<page>.png
                 ├─ active_renders.insert(page)
                 └─ std::thread::spawn:
                      run_renderer(tool, 150 dpi, output, pdf, page)
                      image::open(output) → RgbaImage → ColorImage
                      send PreviewEvent::NewImage(page, image)
  │
  └─ next frame: App::update_preview_textures
       for every rendered_pages entry not yet textured:
            ctx.load_texture("preview_<title>_<page>", image, LINEAR)
```

The UI draws only pages near the viewport; far-away pages get an exact-size placeholder without rendering.

### 3.4 Renderer command lines

| Tool | Command |
|---|---|
| `mutool` / `mudraw` | `mutool draw -r 150 -o <out.png> <in.pdf> <page+1>` |
| `gswin64c` / `gswin32c` / `gs` | `gs -dNOPAUSE -dBATCH -sDEVICE=png16m -r150 -dFirstPage=N -dLastPage=N -sOutputFile=<out.png> <in.pdf>` |
| `pdftoppm` | `pdftoppm -f N -l N -r 150 -png -singlefile <in.pdf> <stem>` (poppler appends `.png`) |

On Windows every renderer invocation uses `CREATE_NO_WINDOW`.

### 3.5 Caching and invalidation

- Cache key is `(tab, page)` in `Tab::preview_textures` (GPU) and `PreviewViewer::rendered_pages` (CPU `ColorImage`).
- A successful compile clears both caches, resets `page = 0`, `num_pages = None`, `rendered_pages`, `active_renders`, and textures, then renders page 0.
- A failed compile does not invalidate the preview; the last good PDF/texture stays visible.
- Temp PNGs under the system temp directory are **never removed**, not even on success or app exit.
- `image_size` is the size of the first successfully decoded page and is reused for every page in that compile cycle. Mixed page sizes will be displayed at the first page's dimensions.

### 3.6 Zoom, fit, pan, and page tracking

In `preview_content`:

- `zoom` is multiplicative and clamped to `0.25..=4.0`.
- Ctrl/Cmd+scroll is converted to a 1.1/0.9 factor and all `MouseWheel` events are removed while the modifier is down, so the scroll area does not also scroll.
- egui’s `zoom_delta()` handles trackpad pinch.
- If `zoom == 1.0` on entry, the code computes `avail_width / image_width` and, if that is < 1.0, sets zoom to fit. This only happens while zoom is exactly 1.0, so it is effectively an initial fit, not a persistent “fit width” mode.
- Each page occupies `image_height * zoom + 16` px; the current page is inferred as `round(scroll_offset.y / page_height)`.
- Page navigation buttons set `scroll_request.y = page * (display_height + 16)`.
- Panning uses the middle mouse button or, in pan mode, the primary button; it accumulates `scroll_offset + pointer_delta` and stores it as a `scroll_request` for the next frame.
- If there is no image, the pane shows `render_error` in red or “No preview available” in grey.

### 3.7 External fallback

`PreviewEvent::Unsupported` sets an error and calls `PreviewViewer::open_externally()`. That function always runs:

```rust
Command::new("cmd").args(["/c", "start", "", path])
```

There is no `#[cfg(windows)]` around the call, so on Linux/macOS it tries to launch a nonexistent `cmd` and silently fails. The user sees the “No PDF renderer found” message but no external viewer. See [14-conventions-and-gotchas.md](14-conventions-and-gotchas.md).

## 4. Auto-compile pipeline

Auto-compile is a toolbar checkbox (`App::auto_compile`, default `false`, not persisted).

Per frame, when enabled and tabs exist:

1. Compute idle time since `last_content_change` (set whenever the `TextEdit` reports a change, including toolbar/heading/AI edits routed through the editor).
2. Compute time since `last_auto_compile`.
3. If idle ≥ 0.8 s and interval ≥ 1.2 s:
   - take the active tab;
   - if it has a path and `dirty`, `std::fs::write(path, text)` (direct, not atomic) and `compiler.compile(path)`;
   - update `last_auto_compile` even if the tab was not dirty or had no path.

The `Ctrl/Cmd+S` path is different: `save_file()` uses `EditorBuffer::save()` (temp-file flow), then compiles if auto-compile is on.

## 5. Manual save pipeline

```text
Ctrl/Cmd+S / File > Save
  └─ App::save_file()
       ├─ no path → save_as_file()
       └─ buffer.save()
            └─ write_atomically(text, path)
                 ├─ create_dir_all(parent)
                 ├─ write <path>.tex.tmp
                 ├─ remove_file(path)        ← destination removed first
                 └─ rename(tmp, path)
```

`save_as_file` opens an `rfd` save dialog, applies `project_tex_path`, writes through `save_as`, updates the tab title from the **chosen** path stem (not the nested final path), and optionally compiles.

## 6. Interaction summary

| Trigger | Writes source first? | Uses atomic save? | Starts compile? | Preview |
|---|---|---|---|---|
| Compile button | yes, direct write | no | yes | forced on |
| Ctrl/Cmd+S | yes, via `save()` | temp+remove+rename | only if auto-compile is checked | unchanged unless compile succeeds |
| Auto-compile timer | yes, direct write | no | yes | forced on by `Started` |
| AI correction | no | — | no | unchanged |
| On exit | yes, via `save_as()` | temp+remove+rename | no | textures cleared |

## 7. Known pipeline failure modes

- `latexmk` writes a PDF but exits non-zero (or the log contains a fatal marker): treated as failure, even if a usable PDF exists.
- A renderer is installed but broken: `--version` succeeds, rendering fails, `render_error` is shown; there is no fallback to the next renderer.
- `pdfinfo` returns a page count for a stale PDF if the renderer reads a different file; the app always passes `last_pdf_path`.
- Two tabs compiling at once can both invoke external tools; there is no global TeX lock.
- Deleting the output directory while a compile is running is not guarded.
