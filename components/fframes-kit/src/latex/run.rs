//! Running TeX and dvisvgm in a temporary directory.

use std::{
    fs::File,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use super::{TexCompiler, TexError};

/// A directory of its own for one compilation, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, TexError> {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("fframes-kit-tex-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs `program args` in `dir` with stdout and stderr in `name.out`, killing it after
/// `timeout`. `Ok(true)` when it exited successfully.
fn run(
    program: &Path,
    args: &[&str],
    dir: &Path,
    name: &str,
    timeout: Duration,
) -> Result<bool, TexError> {
    let out = File::create(dir.join(format!("{name}.out")))?;
    let err = out.try_clone()?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.success());
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(TexError::Timeout {
                tool: name.to_owned(),
                seconds: timeout.as_secs(),
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The part of a TeX log that explains the error: from the first line starting with `!`,
/// at most 20 lines; the last 20 lines when there is none.
pub(crate) fn log_excerpt(log: &str) -> String {
    let lines: Vec<&str> = log.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with('!'))
        .unwrap_or(lines.len().saturating_sub(20));
    lines[start..lines.len().min(start + 20)].join("\n")
}

/// Typesets `document` with `compiler` at `tex` and converts the page with `dvisvgm`: the
/// SVG with every glyph as a path.
pub(crate) fn compile(
    tex: &Path,
    dvisvgm: &Path,
    compiler: TexCompiler,
    document: &str,
    timeout: Duration,
) -> Result<String, TexError> {
    let dir = TempDir::new()?;
    std::fs::write(dir.0.join("formula.tex"), document)?;
    let mut args = vec!["-interaction=nonstopmode", "-halt-on-error"];
    args.extend_from_slice(compiler.output_args());
    args.push("formula.tex");
    let ok = run(tex, &args, &dir.0, compiler.program(), timeout)?;
    let page = dir
        .0
        .join(format!("formula.{}", compiler.output_extension()));
    if !ok || !page.is_file() {
        let log = std::fs::read_to_string(dir.0.join("formula.log"))
            .or_else(|_| std::fs::read_to_string(dir.0.join(format!("{}.out", compiler.program()))))
            .unwrap_or_default();
        return Err(TexError::Compile {
            log: log_excerpt(&log),
            source: document.to_owned(),
        });
    }
    let page_name = format!("formula.{}", compiler.output_extension());
    let args = [
        "--no-fonts",
        "--exact-bbox",
        "--output=formula.svg",
        page_name.as_str(),
    ];
    let ok = run(dvisvgm, &args, &dir.0, "dvisvgm", timeout)?;
    if ok && let Ok(svg) = std::fs::read_to_string(dir.0.join("formula.svg")) {
        return Ok(svg);
    }
    let out = std::fs::read_to_string(dir.0.join("dvisvgm.out")).unwrap_or_default();
    Err(TexError::Convert(log_excerpt(&out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_excerpt_starts_at_the_error() {
        let log = "This is pdfTeX\n(./formula.tex\n! Undefined control sequence.\nl.4 \\foo\n\nNo pages of output.";
        assert_eq!(
            log_excerpt(log),
            "! Undefined control sequence.\nl.4 \\foo\n\nNo pages of output."
        );
        assert_eq!(log_excerpt("a\nb"), "a\nb");
    }
}
