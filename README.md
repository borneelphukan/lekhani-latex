# Lekhani Latex

A cross-platform desktop LaTeX editor with live PDF preview, syntax highlighting, and autocompletion. Built with [egui](https://github.com/emilk/egui) (immediate-mode GUI) and [eframe](https://github.com/emilk/egui/tree/master/crates/eframe).

## Features

- **Syntax-highlighted editor** — Commands, math delimiters, braces, and comments are color-coded with dark/light theme support
- **Live PDF preview** — Compile and view the resulting PDF inline (requires a PDF rasterizer: `mutool`, Ghostscript, or `pdftoppm`)
- **Tabbed interface** — Open multiple `.tex` files simultaneously
- **Autocompletion** — Type `\` followed by a partial command name to see matching LaTeX commands
- **Undo/redo** — Full undo history per document
- **Auto-compile** — Automatically recompile on save
- **Zoom & page navigation** — Pinch-to-zoom, Ctrl+scroll zoom, and page-by-page navigation in the preview panel

## Requirements

- **Rust** (edition 2021)
- A LaTeX distribution (e.g., [MiKTeX](https://miktex.org/), [TeX Live](https://tug.org/texlive/)) providing `pdflatex`
- An optional PDF rasterizer for the inline preview:
  - [mupdf-tools](https://mupdf.com/) (`mutool` / `mudraw`)
  - [Ghostscript](https://ghostscript.com/) (`gs` / `gswin64c`)
  - [poppler](https://poppler.freedesktop.org/) (`pdftoppm`)
  - If none are found, the PDF opens in the system's default viewer

### Key design decisions

- **Module-per-feature** — UI panels are split into separate files under `app/`, each implementing a method on `App`. Rust's child-module privacy allows them to access `App`'s private fields without `pub(crate)` leakage.
- **Channel-based concurrency** — Both compilation and PDF rendering run on background threads. Events are delivered to the main UI loop via `mpsc` channels, keeping the UI responsive.
- **Atomic file saves** — Files are written to a `.tex.tmp` temporary file and then renamed atomically to prevent data loss.
- **Multiplicative zoom** — Uses egui's unified `zoom_delta()` input, supporting both trackpad pinch gestures and Ctrl+scroll in a single code path.

## Usage

| Shortcut / Action | Description |
|---|---|
| File > New Document | Create a new `.tex` file (creates a project folder) |
| File > Open… | Open an existing `.tex`, `.sty`, or `.cls` file |
| File > Save / Save As… | Save the current document |
| Compile button | Run `pdflatex` on the current file |
| Auto-compile toggle | Recompile automatically after each save |
| Ctrl+Scroll / Pinch | Zoom the PDF preview in/out |
| − / + buttons | Zoom the PDF preview |
| ◀ / ▶ buttons | Navigate preview pages |
| View > Toggle Preview | Show/hide the preview panel |
