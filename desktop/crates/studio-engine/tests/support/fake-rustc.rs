/// Stands in for an SDK's bundled compiler during SDK verification: prints `rustc-version.txt`
/// from its own folder (the version line and `host: <triple>`), whatever it is asked.
fn main() {
    let text = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("rustc-version.txt")))
        .and_then(|path| std::fs::read_to_string(path).ok());
    match text {
        Some(text) => print!("{text}"),
        None => {
            eprintln!("fake rustc: rustc-version.txt is missing");
            std::process::exit(1);
        }
    }
}
