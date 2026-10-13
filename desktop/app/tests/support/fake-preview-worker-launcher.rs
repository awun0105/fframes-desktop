use std::{
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

/// Runs `fake-preview-worker.py` with the interpreter the test resolved, recorded next to
/// this launcher in `python-executable.path` (never a guessed `python` on PATH).
fn main() {
    let Some(python) = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("python-executable.path")))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(PathBuf::from)
    else {
        eprintln!("cannot start fake preview worker: python-executable.path is missing");
        std::process::exit(127);
    };
    let mut worker = Command::new(python);
    let script = env::current_dir()
        .map(|directory| directory.join("fake-preview-worker.py"))
        .unwrap_or_else(|_| PathBuf::from("fake-preview-worker.py"));
    worker
        .arg(script)
        .args(env::args_os().skip(1))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = worker.status().unwrap_or_else(|error| {
        eprintln!("cannot start fake preview worker: {error}");
        std::process::exit(127);
    });
    std::process::exit(status.code().unwrap_or(1));
}
