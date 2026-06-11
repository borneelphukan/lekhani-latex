use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;

use egui::ColorImage;

#[derive(Debug)]
pub enum PreviewEvent {
    NewImage(ColorImage),
    Error(String),
    Unsupported,
}

#[derive(Debug)]
    pub struct PreviewViewer {
    receiver: mpsc::Receiver<PreviewEvent>,
    sender: mpsc::Sender<PreviewEvent>,
    pub current_image: Option<ColorImage>,
    pub base_image: Option<ColorImage>,
    pub zoom: f32,
    pub render_error: Option<String>,
    pub last_pdf_path: Option<PathBuf>,
    pub page: usize,
    pub num_pages: Option<usize>,
    renderer: Option<&'static str>,
}

impl PreviewViewer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            receiver: rx,
            sender: tx,
            current_image: None,
            base_image: None,
            zoom: 1.0,
            render_error: None,
            last_pdf_path: None,
            page: 0,
            num_pages: None,
            renderer: None,
        }
    }

    pub fn open_externally(&self) {
        if let Some(path) = &self.last_pdf_path {
            let path_str = path.to_string_lossy();
            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("cmd")
                    .args(["/c", "start", "", path_str.as_ref()])
                    .spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = Command::new("open")
                    .arg(path_str.as_ref())
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = Command::new("xdg-open")
                    .arg(path_str.as_ref())
                    .spawn();
            }
        }
    }

    pub fn render_pdf(&mut self, pdf_path: &Path, page: usize) {
        self.last_pdf_path = Some(pdf_path.to_path_buf());

        if self.renderer.is_none() {
            self.renderer = Self::find_renderer();
        }
        let renderer = match self.renderer {
            Some(r) => r,
            None => {
                let _ = self.sender.send(PreviewEvent::Unsupported);
                return;
            }
        };

        if self.num_pages.is_none() {
            self.num_pages = Self::get_pdf_page_count(pdf_path);
        }

        if let Some(num_pages) = self.num_pages {
            if page >= num_pages {
                let _ = self.sender.send(PreviewEvent::Error(format!(
                    "Page {} does not exist (PDF has {} pages)",
                    page + 1,
                    num_pages
                )));
                return;
            }
        }

        let tx = self.sender.clone();
        let path = pdf_path.to_path_buf();
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let output_stem = format!("latex_writer_preview_{}_{}", pid, page);
        let output_path = temp_dir.join(format!("{}.png", output_stem));
        let dpi = 600u32;
        let tool = renderer;

        thread::spawn(move || {
            let result = Self::run_renderer(tool, dpi, &output_path, &path, page);
            match result {
                Ok(()) => match image::open(&output_path) {
                    Ok(img) => {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();
                        let color_image =
                            ColorImage::from_rgba_unmultiplied(size, &pixels);
                        let _ = tx.send(PreviewEvent::NewImage(color_image));
                    }
                    Err(e) => {
                        let _ = tx.send(PreviewEvent::Error(format!(
                            "Failed to decode rendered image: {}",
                            e
                        )));
                    }
                },
                Err(e) => {
                    let _ = tx.send(PreviewEvent::Error(e));
                }
            }
        });
    }

    fn find_renderer() -> Option<&'static str> {
        for tool in &[
            "mutool",
            "mudraw",
            "gswin64c",
            "gswin32c",
            "gs",
            "pdftoppm",
        ] {
            if Command::new(tool).arg("--version").output().is_ok() {
                return Some(tool);
            }
        }
        None
    }

    fn get_pdf_page_count(input: &Path) -> Option<usize> {
        let output = Command::new("pdfinfo")
            .arg(input)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(count_str) = line.strip_prefix("Pages:") {
                return count_str.trim().parse().ok();
            }
        }
        None
    }

    fn run_renderer(
        tool: &str,
        dpi: u32,
        output: &Path,
        input: &Path,
        page: usize,
    ) -> Result<(), String> {
        let page_str = (page + 1).to_string();
        let dpi_str = dpi.to_string();

        match tool {
            "mutool" | "mudraw" => {
                let out = Command::new(tool)
                    .args(["draw", "-r", &dpi_str, "-o"])
                    .arg(output)
                    .arg(input)
                    .arg(&page_str)
                    .output()
                    .map_err(|e| format!("Failed to run {}: {}", tool, e))?;
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr);
                    return Err(format!("{} draw failed: {}", tool, err.trim()));
                }
                Ok(())
            }
            "gswin64c" | "gswin32c" | "gs" => {
                let output_str = output.to_string_lossy();
                let out = Command::new(tool)
                    .args([
                        "-dNOPAUSE",
                        "-dBATCH",
                        "-sDEVICE=png16m",
                        &format!("-r{}", dpi),
                        &format!("-dFirstPage={}", page_str),
                        &format!("-dLastPage={}", page_str),
                        &format!("-sOutputFile={}", output_str),
                    ])
                    .arg(input)
                    .output()
                    .map_err(|e| format!("Failed to run {}: {}", tool, e))?;
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr);
                    return Err(format!("{} failed: {}", tool, err.trim()));
                }
                Ok(())
            }
            "pdftoppm" => {
                let stem = output.with_extension("");
                let stem_str = stem.to_string_lossy();
                let out = Command::new("pdftoppm")
                    .args([
                        "-f",
                        &page_str,
                        "-l",
                        &page_str,
                        "-r",
                        &dpi_str,
                        "-png",
                        "-singlefile",
                    ])
                    .arg(input)
                    .arg(stem_str.as_ref())
                    .output()
                    .map_err(|e| format!("Failed to run pdftoppm: {}", e))?;
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr);
                    return Err(format!("pdftoppm failed: {}", err.trim()));
                }
                Ok(())
            }
            _ => Err(format!("Unknown renderer: {}", tool)),
        }
    }

    pub fn poll(&mut self) -> Option<PreviewEvent> {
        match self.receiver.try_recv() {
            Ok(event) => {
                match &event {
                    PreviewEvent::NewImage(img) => {
                        self.base_image = Some(img.clone());
                        self.current_image = Some(img.clone());
                        self.render_error = None;
                    }
                    PreviewEvent::Error(e) => {
                        self.render_error = Some(e.clone());
                    }
                    PreviewEvent::Unsupported => {
                        self.render_error = Some(
                            "No PDF renderer found.\nInstall mupdf-tools (mutool), Ghostscript (gs), or poppler (pdftoppm)\nfor an embedded preview.".into(),
                        );
                    }
                }
                Some(event)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => None,
        }
    }
}
