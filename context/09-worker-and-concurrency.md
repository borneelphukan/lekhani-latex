# 09 — Workers, Channels, and Concurrency

Lekhani uses blocking threads and `std::sync::mpsc` channels. There is no async runtime, no task scheduler, and no cancellation token. This document records every background execution path and its lifetime.

## 1. Thread inventory

| Thread | Created by | Lifetime | Channel/result |
|---|---|---|---|
| Compile thread | `CompilerBridge::compile` | One per compile; joined when the next compile starts | `CompileEvent` on a per-`CompilerBridge` channel |
| Preview render thread | `PreviewViewer::ensure_page_rendered` | One per page render; detached | `PreviewEvent` on a per-`PreviewViewer` channel |
| LLM request thread | `App::do_llm_correction` | One per request; detached | `Result<LlmAction, String>` on the shared `llm_tx` |
| Update check thread | `App::check_for_updates` | Short-lived, detached | `UpdateMessage::CheckResult` |
| Update download thread | `App::download_update` | One per download; detached | `UpdateMessage::DownloadProgress/DownloadComplete` |
| TeX installer download thread | `App::start_compiler_download` | One per download; detached | `UpdateMessage::CompilerDownload*` |
| `tlmgr` install thread | `poll_events` when handling `InstallPackage` | Short-lived, detached | `UpdateMessage::PackageInstalled` |
| Linux theme poller | `App::new` | Process lifetime, `loop { gsettings; sleep(2s) }` | `UpdateMessage::OsThemeChanged` |

All threads are OS threads. None are named, so thread dumps show `lekhani-latex`/`rust` defaults.

## 2. Channel topology

```text
Tab[0].compiler.receiver  ← CompileEvent  ← compile thread 0
Tab[0].preview.receiver   ← PreviewEvent  ← render threads 0
Tab[1].compiler.receiver  ← CompileEvent  ← compile thread 1
Tab[1].preview.receiver   ← PreviewEvent  ← render threads 1
...
App.llm_rx                ← Result<LlmAction, String> ← every LLM request
App.update_rx             ← UpdateMessage ← update/download/install/theme threads
```

- Compile and preview channels are **per tab** and move with the tab when tabs are reordered.
- LLM and update channels are **app-wide singletons**, created in `App::new`.
- Channels are unbounded. A fast LLM/update producer can enqueue without backpressure.
- Receivers are drained with `try_recv()` loops in `poll_events`/`process_update_messages`; there is no blocking receive on the UI thread.

## 3. Compile concurrency

`CompilerBridge::compile` is synchronous only for setup:

1. load `active_pid`; if non-zero, run `kill -9 <pid>` (Unix) or `taskkill /F /PID <pid>` (Windows);
2. `join()` the previous `JoinHandle`;
3. spawn a new thread and store its handle;
4. return immediately.

Implications:

- **One compile per tab at a time.** Starting a second compile kills the first, even if it was nearly done.
- **`join()` can block the UI** while the killed process is reaped. In normal cases this is short; a stuck process can delay the frame.
- **Tabs compile independently.** Two tabs can run `latexmk` concurrently with no global lock; TeX's own file locks are the only protection.
- **No timeout.** A hung `latexmk` keeps its thread alive until killed by another compile or process exit.
- **Drop does nothing.** Closing the app with a compile running leaves the child to the OS; on Unix it may outlive the app briefly.
- **Status is event-driven.** `compile()` sets `CompileStatus::Running` immediately; `poll()` changes it to `Success`/`Failed`. A `Warnings` event does not alter status.
- **Disconnected channel** is treated as `Failed` in `poll()`.

The PID is stored with `Ordering::SeqCst`. The child sets it immediately after `spawn()`, and clears it after `wait_with_output()` returns (success or wait error). If `spawn()` fails, no PID is stored.

## 4. Preview concurrency

`ensure_page_rendered`:

- guards on `rendered_pages.contains_key(page)` and `active_renders.contains(page)`;
- resolves the renderer once per viewer;
- resolves the page count once per PDF;
- computes a temp path from `process::id()` and the page index;
- inserts the page into `active_renders`;
- spawns a detached thread that runs the external tool, decodes the PNG with `image`, and sends `NewImage` or `Error`.

The UI polls `PreviewViewer::poll` for every tab in `poll_events`, which stores images in `rendered_pages` and removes the page from `active_renders`.

Concurrency properties:

- **Multiple pages can render at once** for a single tab, and multiple tabs can render at once.
- **No global renderer lock.** Each viewer has its own renderer string.
- **Temp filename collisions** are unlikely (`pid + page`) but two tabs rendering page 0 would both use `lekhani_preview_<pid>_0.png` and race. The second overwrite can cause the first decode to read the other file. This is a real bug if two tabs compile and display page 0 simultaneously.
- **Detached threads** are not joined at exit; `PreviewViewer::drop` is empty.
- **`active_renders` leak on thread panic/decode crash** because nothing removes the page if the thread dies before sending an event.
- **Temp files are never deleted**, so a long session accumulates one PNG per rendered page per compile cycle.

## 5. LLM concurrency

- `llm_correction_in_progress` is the only guard; it is app-wide, so two tabs cannot issue concurrent requests.
- The worker is a detached thread using blocking `ureq`. No timeout is configured explicitly.
- The result handler always applies to `self.active_tab_mut()` at drain time, not the tab that initiated the request. Switching tabs while “Fixing…” therefore applies the correction to the wrong document.
- `poll_events` clears `llm_correction_in_progress` before dispatching. If the channel disconnects (impossible in normal operation), the flag would remain set.
- Package install runs another detached thread (`tlmgr`) after the LLM thread completes, so a correction and a package install cannot overlap for the same request, but a second AI request could start while `tlmgr` is still running.

## 6. Update/download concurrency

- `check_for_updates` spawns a thread that GETs the GitHub API and sleeps 1 s before sending `CheckResult`. The sleep is artificial.
- `download_update` streams the installer in 8 KiB chunks and sends progress only when `Content-Length` exists. Cancel in the UI does not stop the thread; it only hides the window and sets `update_state = None`. The thread continues to write the file and may later send `DownloadComplete`, which is ignored visually but still processed (it would spawn the installer and exit the app if successful).
- `start_compiler_download` similarly streams to the downloads directory, sends progress, and on completion launches an OS-specific installer. No cancel path exists while `Downloading`.
- The Linux path spawns a terminal and runs `tar` + `sudo ./install-tl`; that child is detached and its output is not captured.
- If `dirs::download_dir()` is `None`, the fallback is `/tmp` for the TeX installer and `.` for the update downloader.

## 7. Linux theme poller

`App::new` spawns a never-ending thread on Linux only:

```rust
loop {
    gsettings get org.gnome.desktop.interface color-scheme
    // map prefer-dark / prefer-light to egui::Theme
    if changed { send OsThemeChanged; ctx.request_repaint(); }
    sleep(2s);
}
```

- It runs even when the selected theme is Light/Dark (not System).
- It assumes a GNOME-style `gsettings`; on KDE/other desktops it fails silently or reports nothing.
- It keeps running after the window closes until the process exits.
- It is the only background thread that requests a repaint itself.

## 8. UI-thread blocking

These operations run synchronously inside a frame and can stall it:

| Operation | Where |
|---|---|
| Reading a file | `Tab::load` during open/startup |
| Saving a file | `EditorBuffer::save/save_as` |
| Writing the source before compile | `trigger_compile`, auto-compile |
| Joining the previous compile thread | `CompilerBridge::compile` |
| `kill`/`taskkill` process spawn | `CompilerBridge::compile` |
| Native file dialogs | `rfd` calls from menu actions |
| Unsaved-change dialogs | `rfd` on close/tab close |
| Loading a texture | `ctx.load_texture` (GPU upload) |
| `gsettings` is **not** on the UI thread | Linux poller |

Saving a large document or joining a slow compile can visibly freeze the window. There is no progress indicator for saves.

## 9. Repaint behaviour

- `App::ui` itself does not request continuous repaints.
- `process_update_messages` receives `request_repaint()` from update/download threads (they call `ctx.request_repaint()` after sends).
- Compile, preview, and LLM workers do **not** request repaints. Their events are only observed on the next frame produced by user input or OS events. In practice, egui redraws often enough while a user is interacting, but an idle window may not update immediately when a compile finishes.
- The Linux theme poller requests a repaint only when the theme changes.

## 10. Rules for new background work

If adding a thread:

1. Send a typed message through an existing channel or add a new `UpdateMessage` variant; do not mutate `App` from the thread.
2. Request a repaint if the result should appear while the user is idle.
3. Keep the UI-side handler in `poll_events` or `process_update_messages`, and make it tolerant of zero tabs.
4. Do not create `egui` textures off the UI thread.
5. Add cancellation or a generation token if the result can become stale (tab closed, new request started).
6. Prefer `Command::new(...).spawn()` over shell strings on Windows; use `CREATE_NO_WINDOW` for console tools.
7. Do not assume a tab still exists or is active when the result arrives.
