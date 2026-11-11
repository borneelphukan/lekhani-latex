# 04 — Data Model and Contracts

This document describes the types that carry state between the editor, compiler, preview, and UI. Prefer these names when searching the code.

## 1. `App` (gui/src/app/mod.rs)

`App` is the single top-level state object. Public methods on `App` are used by the layout modules; most fields are private to the `app` module tree.

Important fields (grouped):

```rust
pub struct App {
    tabs: Vec<tab::Tab>,
    active_tab: usize,
    theme: Theme,
    file_dialog_action: Option<FileDialogAction>,
    // completion popup state
    completion_visible: bool,
    completion_matches: Vec<String>,
    completion_byte_range: Option<(usize, usize)>,
    completion_selected: usize,
    completion_prefix: String,
    completion_block_trigger: bool,
    // output panel
    show_outputs: bool,
    show_outputs_requested: bool,
    about_open: bool,
    // updates
    update_state: UpdateState,
    update_rx: Receiver<UpdateMessage>,
    update_tx: Sender<UpdateMessage>,
    // formatting / compile
    heading_numbered: bool,
    auto_compile: bool,
    last_auto_compile: Option<Instant>,
    last_content_change: Option<Instant>,
    tab_drag: Option<(usize, f32)>,
    // LLM
    llm_correction_in_progress: bool,
    llm_tx: Sender<Result<LlmAction, String>>,
    llm_rx: Receiver<Result<LlmAction, String>>,
    llm_api_key: String,
    llm_settings_tab_index: usize,   // 0 = local, 1 = API key
    llm_selected_preset: usize,
    llm_is_custom_model: bool,
    llm_configured: bool,
    llm_model: String,
    llm_custom_model: String,
    llm_endpoint_url: String,
    show_llm_settings: bool,
    // OS/theme/compiler prompt
    os_theme: Option<egui::Theme>,
    startup_compiler_checked: bool,
    show_compiler_dialog: bool,
    compiler_download_state: CompilerDownloadState,
    output_panel_tab: OutputPanelTab,
}
```

There is no `Default` impl; `App::new` constructs everything explicitly.

## 2. `Tab` (gui/src/app/tab.rs)

```rust
pub struct Tab {
    pub title: String,
    pub buffer: EditorBuffer,
    pub compiler: CompilerBridge,
    pub preview: PreviewViewer,
    pub show_preview: bool,
    pub preview_textures: HashMap<usize, egui::TextureHandle>,
    pub status_message: String,
    pub error_message: Option<String>,
    pub error_lines: Vec<usize>,
    pub output_log: Vec<(String, Color32)>,
    pub ai_output_log: Vec<(String, Color32)>,
    pub scroll_offset: egui::Vec2,
    pub scroll_request: Option<egui::Vec2>,
    pub compile_start_time: Option<Instant>,
}
```

Constructors:

- `Tab::load(path)` — reads the file; on read failure falls back to `EditorBuffer::new()` (the template document) with that path, so a failed open still yields a tab.
- `Tab::new_empty(path)` — default template buffer, path set, `dirty = false`.
- `title` is the file stem, not the full filename.

Every tab gets `CompilerBridge::new(CompilerConfig::default())` and `PreviewViewer::new()`; the config is not per-tab configurable from the UI.

## 3. `EditorBuffer` (core/src/buffer/mod.rs)

```rust
pub struct EditorBuffer {
    pub text: String,
    path: Option<PathBuf>,
    pub dirty: bool,
    pub cursor: usize,          // byte offset, not char index
    undo_stack: Vec<EditToken>,
    redo_stack: Vec<EditToken>,
    line_starts: Vec<usize>,    // byte offset of each line start
}
```

Important methods:

| Method | Contract |
|---|---|
| `new()` | Returns a template article document and `dirty = false`. |
| `load(path)` | Reads UTF-8, sets path, `dirty = false`. |
| `save()` / `save_as(path)` | Both call `write_atomically`; `save()` errors if no path. |
| `sync_after_edit()` | Sets `dirty = true` and rebuilds `line_starts`. |
| `insert_str`, `backspace`, `delete` | Record `EditToken`, mutate text/cursor, mark dirty, rebuild lines. |
| `undo`, `redo` | Pop/push `EditToken`s; cursor semantics are operation-specific. |
| `replace_all(new)` | Pushes a delete-old + insert-new pair, sets cursor to end. |
| `cursor_line_col()` | Returns 1-based line and 0-based char column. |
| `move_*`, `line_*`, `indentation_at_cursor()` | Byte/char-aware cursor helpers. |

Limits and invariants:

- Undo stack is capped at `MAX_UNDO = 1024`; the oldest token is removed. Redo is cleared on every new edit.
- `line_starts` always begins with 0 and contains the byte after every `\n`.
- The cursor is a byte offset that is expected to be on a UTF-8 boundary. Most callers derive it from `char_indices()`.
- `write_atomically` creates parent directories, writes `<path>.tex.tmp` (so `foo.tex` → `foo.tex.tmp` because `with_extension` replaces `tex`), removes the destination, then renames.

## 4. Edit tokens (core/src/types.rs)

```rust
pub enum EditKind { Insert(String), Delete(String) }
pub struct EditToken { pub kind: EditKind, pub position: usize }
```

An insert token at `position` is undone by draining the inserted bytes. A delete token is undone by re-inserting the deleted bytes at the same position. `redo` reverses that logic. There is no grouping/coalescing; each helper call is one undo step.

## 5. `Theme` and syntax colors (core/src/types.rs)

```rust
pub enum Theme { System, Dark, Light }

pub struct SyntaxColors {
    pub text: Color32,
    pub cmd: Color32,
    pub math: Color32,
    pub brace: Color32,
    pub comment: Color32,
}
```

- `Theme::resolve(ctx)` turns `System` into `Dark`/`Light` using `ctx.global_style().visuals.dark_mode`.
- `Theme::syntax_colors(ctx)` returns fixed palettes for dark (blue commands, salmon math, gold braces, green comments) and light (navy commands, orange math, ochre braces, green comments).
- `Theme::System` is `unreachable!()` in `syntax_colors`; callers must resolve first.
- Persisted config stores `"System"`, `"Light"`, or `"Dark"`.

## 6. Compiler types

### `CompilerConfig` (core/src/types.rs)

```rust
pub struct CompilerConfig { pub command: String, pub args: Vec<String> }

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            command: "latexmk".into(),
            args: vec!["-pdf".into(), "-interaction=nonstopmode".into()],
        }
    }
}
```

`CompilerBridge` appends `-output-directory=<parent>/<stem>` and the absolute `.tex` path, and runs with `current_dir(parent)`.

### `CompileEvent` / `CompileStatus` (compiler/src/compiler/bridge.rs)

```rust
pub enum CompileEvent {
    Started,
    Warnings(Vec<String>),
    Success(PathBuf),      // path to the produced PDF
    Failure(Vec<String>),  // user-facing error lines
}

pub enum CompileStatus { Idle, Running, Success, Failed }
```

`CompilerBridge`:

```rust
pub struct CompilerBridge {
    config: CompilerConfig,
    receiver: Receiver<CompileEvent>,
    sender: Sender<CompileEvent>,
    status: CompileStatus,
    join_handle: Option<JoinHandle<()>>,
    active_pid: Arc<AtomicU32>,
}
```

- `compile(path)` kills `active_pid` if non-zero, joins the previous handle, then spawns a new thread. It never blocks waiting for the new compile.
- `poll()` maps events to status and returns one event per call; `Warnings` does **not** change status and is returned immediately.
- `reset_status()` exists but is `#[allow(dead_code)]` and unused.
- `Drop` is empty.

### Parser contracts (compiler/src/compiler/parser.rs)

| Function | Behaviour |
|---|---|
| `read_log_fatal(path)` | `true` if the `.log` contains `Fatal error occurred`. |
| `extract_warnings(stderr, stdout, log)` | Lines containing `Warning:`, `Overfull`, or `Underfull`, deduplicated against the log, truncated to 10. |
| `extract_errors(stderr, stdout, log)` | Prefer `! ...` and `l.<n>` lines from streams; otherwise scan the log for `!` blocks; otherwise take the cleaned tail (5 lines); always non-empty fallback; truncate to 8. |

## 7. Preview types (gui/src/preview.rs)

```rust
pub enum PreviewEvent {
    NewImage(usize, egui::ColorImage),
    Error(usize, String),
    Unsupported,
}

pub struct PreviewViewer {
    receiver: Receiver<PreviewEvent>,
    sender: Sender<PreviewEvent>,
    pub rendered_pages: HashMap<usize, ColorImage>,
    pub active_renders: HashSet<usize>,
    pub zoom: f32,
    pub render_error: Option<String>,
    pub last_pdf_path: Option<PathBuf>,
    pub page: usize,              // 0-based
    pub num_pages: Option<usize>,
    pub image_size: Option<[usize; 2]>,
    pub pan_mode: bool,
    renderer: Option<String>,
}
```

Contracts:

- `ensure_page_rendered(pdf_path, page)` is idempotent for `rendered_pages` and `active_renders`.
- After the idempotence guard passes, it records `last_pdf_path`; a page already rendered from the previous PDF is not refreshed merely because a new PDF path is offered.
- `image_size` is taken from the **first** decoded page and reused for all pages.
- `page` is 0-based in state; the UI displays `page + 1`.
- `renderer` is discovered once and cached; changing installed tools at runtime requires restarting the app.
- `poll()` moves a `NewImage` into `rendered_pages` and clears `render_error`.

## 8. LLM contract

```rust
pub enum LlmAction {
    Correction { text: String, line: Option<usize>, explanation: Option<String> },
    InstallPackage { package: String, explanation: Option<String> },
}
```

The channel type is `Result<LlmAction, String>`.

The model is instructed to return a single JSON object:

```json
{"action":"correct","text":"...","explanation":"..."}
{"action":"install","package":"booktabs","explanation":"..."}
```

The parser also tolerates a JSON array and uses the first element, and it strips a leading ```` ```json ```` fence and trailing ```` ``` ````. Any other shape becomes an error string shown in the AI output panel.

## 9. UI message enums (gui/src/app/mod.rs)

```rust
enum UpdateState { None, Checking, Prompt(String), Downloading(f32) }

enum CompilerDownloadState { None, Downloading(f32), Complete, Failed(String) }

enum UpdateMessage {
    CheckResult(bool, Option<String>),
    DownloadProgress(f32),
    DownloadComplete(Result<String, String>),
    CompilerDownloadProgress(f32),
    CompilerDownloadComplete(Result<PathBuf, String>),
    OsThemeChanged(egui::Theme),
    PackageInstalled(String, Result<(), String>),
}

enum FileDialogAction { Open, Save, SaveAs, NewDocument }

pub enum OutputPanelTab { Compiler, AI }
```

`OsThemeChanged` is constructed by the Linux poller but has no visible effect beyond assigning `App::os_theme`; `apply_theme` reads it on the next frame.

## 10. Persisted config schema

Path: `dirs::config_dir()/lekhani-latex/config.json`.

```json
{
  "llm_api_key": "sk-...",
  "llm_settings_tab_index": 0,
  "llm_configured": true,
  "llm_model": "llama-3.2-3b",
  "llm_custom_model": "",
  "llm_endpoint_url": "http://localhost:1234/v1",
  "theme": "System"
}
```

- All keys are optional on read; missing values fall back to defaults:
  - `tab_index = 0`, `configured = false`, `model = "llama-3.2-3b"`, `custom = ""`,
    `endpoint = "http://localhost:1234/v1"`, `theme = System`.
- `llm_api_key` is filtered out if empty.
- `LLM_API_KEY` environment variable is used only when no stored key exists, and it is treated as configured.
- Legacy path: `dirs::config_dir()/lekhani-latex/llm_config.json`. If `config.json` does not exist and the legacy file parses, it is copied to `config.json` and the legacy file is removed.
- Writes are pretty JSON via `std::fs::write`; they are not atomic and there is no file locking.

## 11. Defaults and constants

| Constant/value | Location | Value |
|---|---|---|
| Window size | `gui/src/main.rs` | 1280×860 |
| Editor font | `gui/src/app/editor.rs` | monospace 14.0 |
| Completion popup width/height | `gui/src/app/editor.rs` | min 180, max 300 high |
| Preview DPI | `gui/src/preview.rs` | 150 |
| Preview zoom clamp | `gui/src/preview.rs` | 0.25–4.0 |
| Preview page gap | `gui/src/preview.rs` / `app/layout/preview_panel.rs` | 16 px |
| Auto-compile idle delay | `gui/src/app/mod.rs` | 0.8 s since change |
| Auto-compile minimum interval | `gui/src/app/mod.rs` | 1.2 s |
| Undo cap | `core/src/buffer/mod.rs` | 1024 tokens |
| LLM presets | `gui/src/app/mod.rs` | 8 entries (Llama, Mistral, Gemma, Phi, Qwen, DeepSeek) |
| Default local endpoint | `gui/src/app/mod.rs` | `http://localhost:1234/v1` |
| Default API base | `gui/src/app/mod.rs` | `https://api.openai.com/v1` |
| Default model | `gui/src/app/mod.rs` | `llama-3.2-3b` |
| Fallback request model | `gui/src/app/layout/menubar.rs` | `gpt-4o-mini` if model string is empty |
| Max compile warnings | `compiler/src/compiler/parser.rs` | 10 |
| Max compile errors | `compiler/src/compiler/parser.rs` | 8 |
