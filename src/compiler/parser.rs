use std::path::Path;

pub fn read_log_fatal(log_path: &Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(log_path) {
        content.contains("Fatal error occurred")
    } else {
        false
    }
}

pub fn extract_warnings(stderr: &str, stdout: &str, log_path: &Path) -> Vec<String> {
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

pub fn extract_errors(stderr: &str, stdout: &str, log_path: &Path) -> Vec<String> {
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
