//! Protected trees (`.git`, `target`, `.fframes/context`, `.fframes/cache`) cannot be
//! reached through a name the filesystem treats as the same one: case and Unicode
//! variants are recognised from the text, and a directory opened under any name other
//! than the one it is stored under (an 8.3 short name included) is refused.
use std::fs;

use studio_project::paths::{PROTECTED_TREES, protected_tree};

#[test]
fn case_unicode_and_ignorable_variants_of_a_protected_tree_are_recognised() {
    for (path, tree) in [
        (".git", ".git"),
        (".git/hooks/pre-commit", ".git"),
        (".Git/hooks/x", ".git"),
        (".GIT", ".git"),
        ("TARGET/x", "target"),
        ("Target/debug/app", "target"),
        // Dotless i and long s only meet `i` and `s` through upper case.
        (".g\u{131}t/x", ".git"),
        (".fframe\u{17F}/cache/x", ".fframes/cache"),
        // Code points HFS+ ignores in names.
        (".g\u{200C}it/x", ".git"),
        ("\u{FEFF}.git/x", ".git"),
        // Compatibility forms (fullwidth letters).
        ("\u{FF54}arget/x", "target"),
        (".fframes/CONTEXT/a", ".fframes/context"),
        (".FFrames/Cache", ".fframes/cache"),
    ] {
        assert_eq!(protected_tree(path), Some(tree), "{path:?}");
    }
    for path in [
        "src/.git/x",
        "my-target/x",
        "targets/x",
        ".gitignore",
        ".github/workflows/ci.yml",
        ".fframes/presets/x",
        ".fframes",
        "media/target.png",
    ] {
        assert_eq!(protected_tree(path), None, "{path:?}");
    }
    assert_eq!(PROTECTED_TREES.len(), 4);
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
mod open {
    use std::{fs, io::ErrorKind};

    use studio_project::publish::Dir;

    #[test]
    fn a_directory_opens_only_under_the_name_it_is_stored_under() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join(".git/hooks")).unwrap();
        let root = Dir::open_root(temp.path()).unwrap();
        root.open_child(".git").unwrap();
        root.open_relative(".git/hooks").unwrap();
        for alias in [".Git", ".GIT"] {
            let error = root.open_child(alias).unwrap_err();
            if fs::metadata(temp.path().join(alias)).is_ok() {
                // A case-insensitive volume (Windows, macOS by default).
                assert_eq!(error.kind(), ErrorKind::InvalidInput, "{alias}: {error}");
                assert!(error.to_string().contains("alias"), "{error}");
            } else {
                assert_eq!(error.kind(), ErrorKind::NotFound, "{alias}: {error}");
            }
            assert!(root.open_relative(&format!("{alias}/hooks")).is_err());
        }
    }

    /// The 8.3 name Windows generated for `.git`, if the volume generates them.
    #[cfg(windows)]
    fn short_name(path: &std::path::Path) -> Option<String> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
        let long: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let mut short = vec![0u16; 1024];
        // SAFETY: NUL-terminated input and a buffer of the given length.
        let len = unsafe { GetShortPathNameW(long.as_ptr(), short.as_mut_ptr(), 1024) } as usize;
        assert!(len > 0 && len < short.len(), "GetShortPathNameW failed");
        let short = String::from_utf16(&short[..len]).unwrap();
        let name = short.rsplit('\\').next().unwrap().to_owned();
        (name != ".git").then_some(name)
    }

    #[cfg(windows)]
    #[test]
    fn an_8_3_short_name_of_a_protected_tree_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join(".git/hooks")).unwrap();
        let Some(short) = short_name(&temp.path().join(".git")) else {
            eprintln!("8.3 name generation is off on this volume; nothing to alias");
            return;
        };
        // The short name really opens `.git` through the path API...
        assert!(fs::metadata(temp.path().join(&short).join("hooks")).is_ok());
        // ...but never through a handle-relative open.
        let root = Dir::open_root(temp.path()).unwrap();
        let error = root.open_child(&short).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput, "{short}: {error}");
        assert!(error.to_string().contains(".git"), "{error}");
        assert!(root.open_relative(&format!("{short}/hooks")).is_err());
    }
}

#[test]
fn the_inventory_keeps_excluding_exactly_the_protected_names() {
    let temp = tempfile::tempdir().unwrap();
    for dir in [
        ".git",
        "target",
        ".fframes/context",
        ".fframes/cache",
        "src",
    ] {
        fs::create_dir_all(temp.path().join(dir)).unwrap();
        fs::write(temp.path().join(dir).join("f.txt"), "x").unwrap();
    }
    let inventory = studio_project::SourceInventory::scan(temp.path()).unwrap();
    let paths: Vec<&str> = inventory.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["src/f.txt"]);
}
