#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;

use crate::types::CompilerConfig;

#[derive(Debug, Clone)]
pub enum CompileEvent {
    Started,
    Warnings(Vec<String>),
    Success(PathBuf),
    Failure(Vec<String>),
}

pub struct CompilerBridge {
    config: CompilerConfig,
    receiver: mpsc::Receiver<CompileEvent>,
    sender: mpsc::Sender<CompileEvent>,
    status: CompileStatus,
    join_handle: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileStatus {
    Idle,
    Running,
    Success,
    Failed,
}

impl CompilerBridge {
    pub fn new(config: CompilerConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            config,
            receiver: rx,
            sender: tx,
            status: CompileStatus::Idle,
            join_handle: None,
        }
    }

    pub fn compile(&mut self, file_path: &Path) {
        // Join any previous compilation thread before starting a new one
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }

        let tx = self.sender.clone();
        let config = self.config.clone();
        let path = file_path.to_path_buf();
        self.status = CompileStatus::Running;

        let handle = thread::spawn(move || {
            let _ = tx.send(CompileEvent::Started);

            let metadata = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(e) => {
                    let _ = tx.send(CompileEvent::Failure(vec![format!(
                        "Cannot read file: {}",
                        e
                    )]));
                    return;
                }
            };
            if metadata.len() == 0 {
                let _ = tx.send(CompileEvent::Failure(vec![
                    "The document is empty. Add some LaTeX content first.".into(),
                ]));
                return;
            }

            // Quick content check – warn if the file doesn't look like LaTeX
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.len() < 10 {
                    let _ = tx.send(CompileEvent::Failure(vec![
                        "The document is too short to compile.\nAdd LaTeX content like:\n  \\documentclass{article}\n  \\begin{document}\n  Hello, world!\n  \\end{document}"
                            .into(),
                    ]));
                    return;
                }
                // Only skip if it has absolutely no LaTeX structure
                let has_tex_structure = content.contains("\\document")
                    || content.contains("\\begin{")
                    || content.contains("\\section")
                    || content.contains("\\chapter")
                    || content.contains("\\end{");
                if !has_tex_structure && content.len() < 100 {
                    let _ = tx.send(CompileEvent::Failure(vec![
                        "The document doesn't appear to contain valid LaTeX.\nAdd a preamble like:\n  \\documentclass{article}\n  \\begin{document}\n  Your content here\n  \\end{document}"
                            .into(),
                    ]));
                    return;
                }
            }

            let output_dir = path.parent().unwrap_or(Path::new("."));
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");

            let pdf_path = output_dir.join(format!("{}.pdf", file_stem));
            let log_path = output_dir.join(format!("{}.log", file_stem));

            // Remove stale files
            let _ = std::fs::remove_file(&pdf_path);
            let _ = std::fs::remove_file(&log_path);

            // Run pdflatex in the output directory so all auxiliary files
            // (.aux, .log, .pdf) are created next to the .tex source.
            let tex_arg = path.to_string_lossy().to_string();
            let mut cmd = Command::new(&config.command);
            cmd.args(&config.args)
                .arg(&tex_arg)
                .current_dir(output_dir);

            match cmd.output() {
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let warnings = extract_warnings(&stderr, &stdout, &log_path);
                    if !warnings.is_empty() {
                        let _ = tx.send(CompileEvent::Warnings(warnings));
                    }

                    let success = output.status.success();
                    // Also check the log for the presence of fatal errors
                    let log_has_fatal = read_log_fatal(&log_path);

                    if success && !log_has_fatal {
                        if pdf_path.exists() {
                            let _ = tx.send(CompileEvent::Success(pdf_path));
                        } else {
                            let _ = tx.send(CompileEvent::Failure(vec![
                                "Compilation finished but no PDF was produced.".into(),
                            ]));
                        }
                    } else {
                        let errors = extract_errors(&stderr, &stdout, &log_path);
                        let _ = tx.send(CompileEvent::Failure(errors));
                    }
                }
                Err(e) => {
                    let msg = if e.kind() == std::io::ErrorKind::NotFound {
                        format!(
                            "'{}' was not found. Is a LaTeX distribution (MiKTeX/TeX Live) installed?",
                            config.command
                        )
                    } else {
                        format!("Failed to run '{}': {}", config.command, e)
                    };
                    let _ = tx.send(CompileEvent::Failure(vec![msg]));
                }
            }
        });

        self.join_handle = Some(handle);
    }

    pub fn poll(&mut self) -> Option<CompileEvent> {
        match self.receiver.try_recv() {
            Ok(event) => {
                self.status = match &event {
                    CompileEvent::Started => CompileStatus::Running,
                    CompileEvent::Warnings(_) => return Some(event),
                    CompileEvent::Success(_) => CompileStatus::Success,
                    CompileEvent::Failure(_) => CompileStatus::Failed,
                };
                Some(event)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status = CompileStatus::Failed;
                None
            }
        }
    }

    pub fn status(&self) -> CompileStatus {
        self.status
    }

    pub fn reset_status(&mut self) {
        self.status = CompileStatus::Idle;
    }
}

impl Drop for CompilerBridge {
    fn drop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

fn read_log_fatal(log_path: &Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(log_path) {
        content.contains("Fatal error occurred")
    } else {
        false
    }
}

fn extract_warnings(stderr: &str, stdout: &str, log_path: &Path) -> Vec<String> {
    let combined = format!("{}\n{}", stderr, stdout);
    let mut warnings: Vec<String> = combined
        .lines()
        .filter(|l| {
            l.contains("Warning:")
                || l.contains("Overfull")
                || l.contains("Underfull")
        })
        .map(|l| l.trim().to_string())
        .collect();

    // Also check the log file for warnings
    if let Ok(log) = std::fs::read_to_string(log_path) {
        for line in log.lines() {
            let trimmed = line.trim();
            if (trimmed.contains("Warning:")
                || trimmed.contains("Overfull")
                || trimmed.contains("Underfull"))
                && !warnings.contains(&trimmed.to_string())
            {
                warnings.push(trimmed.to_string());
            }
        }
    }

    warnings.truncate(10);
    warnings
}

fn extract_errors(stderr: &str, stdout: &str, log_path: &Path) -> Vec<String> {
    let combined = format!("{}\n{}", stderr, stdout);

    // 1. Try extracting real LaTeX errors from stdout/stderr
    let mut errors: Vec<String> = combined
        .lines()
        .filter(|l| {
            // Real LaTeX errors start with "! " and aren't the boilerplate tail lines
            (l.starts_with("! ")
                && !l.contains("Emergency stop")
                && !l.contains("Fatal error")
                && !l.contains("==>"))
                || l.starts_with("l.") // line-number pointer, e.g. "l.6 \begin{document}"
        })
        .map(|l| l.trim().to_string())
        .collect();

    // 2. If no structured errors found, read the .log file for details
    if errors.is_empty() {
        if let Ok(log) = std::fs::read_to_string(log_path) {
            let mut in_error = false;
            for line in log.lines() {
                if line.starts_with("! ") {
                    errors.push(line.trim().to_string());
                    in_error = true;
                } else if in_error {
                    // Include continuation lines (indented)
                    if line.starts_with("l.") || line.starts_with(' ') {
                        errors.push(line.trim().to_string());
                    } else {
                        in_error = false;
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        // 3. Last resort – show the tail of the log
        if let Ok(log) = std::fs::read_to_string(log_path) {
            let tail: Vec<&str> = log.lines().rev().take(15).collect();
            let tail: Vec<&str> = tail.into_iter().rev().collect();
            errors = tail
                .iter()
                .filter(|l| {
                    !l.is_empty()
                        && !l.contains("Transcript written")
                        && !l.contains("Output written")
                        && !l.starts_with('(')
                        && !l.starts_with(')')
                })
                .take(5)
                .map(|l| l.to_string())
                .collect();
        }
    }

    if errors.is_empty() {
        errors.push("Compilation failed. Check your LaTeX syntax.".into());
    }

    errors.truncate(8);
    errors
}
