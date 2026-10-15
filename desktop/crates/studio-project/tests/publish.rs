//! No-clobber publication primitives and exact transaction-name exclusion, on Linux, macOS and
//! Windows. Tests keep one name on every OS; where the platforms differ by design (Windows
//! renames the opened file, has junctions and a read-only attribute instead of mode
//! bits, refuses to rename an ancestor of a held directory) the test asserts each
//! platform's own guarantee.
#![cfg(any(target_os = "linux", target_os = "macos", windows))]

use std::{fs, io::ErrorKind, path::Path};
use studio_project::{
    SourceInventory,
    paths::{TxRole, is_transaction_internal_name, transaction_file_name},
    publish::{Dir, EntryKind, NoClobber, probe_no_clobber},
};

fn tx() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn symlink_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_dir(target, link);
    result.unwrap_or_else(|e| {
        panic!("creating a directory symlink failed ({e}); on Windows this needs SeCreateSymbolicLinkPrivilege (Administrator or Developer Mode)")
    });
}

fn symlink_file(target: &Path, link: &Path) {
    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let result = std::os::windows::fs::symlink_file(target, link);
    result.unwrap_or_else(|e| {
        panic!("creating a file symlink failed ({e}); on Windows this needs SeCreateSymbolicLinkPrivilege (Administrator or Developer Mode)")
    });
}

/// A directory junction (`mklink /J`), which needs no privilege.
#[cfg(windows)]
fn junction(target: &Path, link: &Path) {
    let status = std::process::Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(link)
        .arg(target)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "mklink /J failed");
}

#[test]
fn the_probe_proves_no_clobber_publication_on_this_filesystem_and_cleans_up() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("user.txt"), "mine").unwrap();
    let dir = Dir::open_root(temp.path()).unwrap();
    let mechanism = probe_no_clobber(&dir).unwrap();
    // The probe offers exactly one mechanism for live names.
    assert_eq!(
        mechanism,
        NoClobber::Renameat2,
        "only an atomic no-replace rename qualifies; a link fallback is never offered"
    );
    // Recorded in the hand-off: which primitive this machine's filesystem proved.
    eprintln!("proved no-clobber mechanism: {}", mechanism.label());
    let left: Vec<_> = fs::read_dir(temp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["user.txt"]);
}

#[test]
fn both_mechanisms_refuse_to_replace_and_leave_both_files_intact() {
    let mechanisms: &[NoClobber] = if cfg!(windows) {
        // Windows has no link-then-unlink pair: it is refused before anything moves.
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("staged"), "new bytes").unwrap();
        let dir = Dir::open_root(temp.path()).unwrap();
        let error = dir
            .rename_noreplace(NoClobber::Link, "staged", &dir, "free")
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Unsupported, "{error}");
        assert!(dir.stat("staged").unwrap().is_some());
        &[NoClobber::Renameat2]
    } else {
        &[NoClobber::Renameat2, NoClobber::Link]
    };
    for &mechanism in mechanisms {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("sub")).unwrap();
        fs::write(temp.path().join("staged"), "new bytes").unwrap();
        fs::write(temp.path().join("destination"), "editor bytes").unwrap();
        fs::write(temp.path().join("sub").join("destination"), "other bytes").unwrap();
        let dir = Dir::open_root(temp.path()).unwrap();
        let sub = dir.open_child("sub").unwrap();
        for target in [&dir, &sub] {
            let error = dir
                .rename_noreplace(mechanism, "staged", target, "destination")
                .unwrap_err();
            assert_eq!(error.kind(), ErrorKind::AlreadyExists, "{mechanism:?}");
        }
        assert_eq!(
            fs::read_to_string(temp.path().join("destination")).unwrap(),
            "editor bytes"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("sub").join("destination")).unwrap(),
            "other bytes"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("staged")).unwrap(),
            "new bytes"
        );
        // A free name works and moves the file itself.
        let before = dir.stat("staged").unwrap().unwrap().id;
        dir.rename_noreplace(mechanism, "staged", &dir, "free")
            .unwrap();
        assert_eq!(dir.stat("free").unwrap().unwrap().id, before);
        assert!(dir.stat("staged").unwrap().is_none());
        // Across directories too.
        dir.rename_noreplace(mechanism, "free", &sub, "moved")
            .unwrap();
        assert_eq!(sub.stat("moved").unwrap().unwrap().id, before);
    }
}

#[test]
fn names_never_follow_links_or_open_special_files() {
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "outside").unwrap();
    symlink_dir(outside.path(), &temp.path().join("link-dir"));
    symlink_file(
        &outside.path().join("secret"),
        &temp.path().join("link-file"),
    );
    fs::create_dir(temp.path().join("real")).unwrap();
    fs::write(temp.path().join("plain"), "plain").unwrap();
    let root = Dir::open_root(temp.path()).unwrap();
    assert!(root.open_relative("link-dir").is_err());
    assert_eq!(
        root.stat("link-dir").unwrap().unwrap().kind,
        EntryKind::Symlink
    );
    assert!(root.open_relative("real").is_ok());
    assert!(root.open_regular("link-file").is_err());
    assert!(root.open_regular("real").is_err());
    #[cfg(unix)]
    {
        let fifo = std::process::Command::new("mkfifo")
            .arg(temp.path().join("pipe"))
            .status()
            .unwrap();
        assert!(fifo.success());
        assert!(root.open_regular("pipe").is_err());
    }
    #[cfg(windows)]
    {
        // A junction is refused because it is a link, not because the open failed.
        junction(outside.path(), &temp.path().join("junction"));
        let error = root.open_child("junction").unwrap_err();
        assert!(error.to_string().contains("is a link"), "{error}");
        assert_eq!(
            root.stat("junction").unwrap().unwrap().kind,
            EntryKind::Symlink
        );
        // Alternate data streams are not single components.
        assert!(root.open_regular("plain:stream").is_err());
        assert!(root.create_new("plain:stream", 0o644).is_err());
        assert!(root.stat("plain.").is_err());
    }
    // Creation refuses an existing name, including a dangling or live link.
    assert_eq!(
        root.create_new("link-file", 0o600).unwrap_err().kind(),
        ErrorKind::AlreadyExists
    );
    symlink_file(&temp.path().join("nowhere"), &temp.path().join("dangling"));
    assert!(root.create_new("dangling", 0o600).is_err());
    assert!(!temp.path().join("nowhere").exists());
    // Removing a link removes the link, never its target.
    root.unlink("link-file").unwrap();
    // Components must be single names.
    assert!(root.stat("real/inner").is_err());
    assert!(root.stat("..").is_err());
    assert_eq!(
        fs::read_to_string(outside.path().join("secret")).unwrap(),
        "outside"
    );
}

#[test]
fn a_swapped_directory_is_detected_by_identity() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sub")).unwrap();
    let root = Dir::open_root(temp.path()).unwrap();
    let sub = root.open_relative("sub").unwrap();
    assert!(sub.is_still_at(&root, "sub"));
    fs::rename(temp.path().join("sub"), temp.path().join("moved")).unwrap();
    fs::create_dir(temp.path().join("sub")).unwrap();
    assert!(
        !sub.is_still_at(&root, "sub"),
        "a replacement directory has another identity"
    );
    fs::remove_dir(temp.path().join("sub")).unwrap();
    symlink_dir(&temp.path().join("moved"), &temp.path().join("sub"));
    assert!(!sub.is_still_at(&root, "sub"), "a symlink never matches");
    #[cfg(windows)]
    {
        fs::remove_dir(temp.path().join("sub")).unwrap();
        junction(&temp.path().join("moved"), &temp.path().join("sub"));
        assert!(!sub.is_still_at(&root, "sub"), "a junction never matches");
    }
    // The held handle still reaches the moved directory itself.
    assert!(sub.is_still_at(&root, "moved"));
}

/// Unix: exactly the requested bits whatever the umask. Windows: the one permission bit
/// it has, read-only, follows the write bits, and the synthesised mode reads back.
#[test]
fn created_files_carry_exactly_the_requested_mode_whatever_the_umask() {
    let temp = tempfile::tempdir().unwrap();
    let root = Dir::open_root(temp.path()).unwrap();
    #[cfg(unix)]
    let modes = [("a", 0o755), ("b", 0o600), ("c", 0o644)];
    #[cfg(windows)]
    let modes = [("a", 0o644), ("b", 0o444)];
    for (name, mode) in modes {
        drop(root.create_new(name, mode).unwrap());
        let permissions = fs::metadata(temp.path().join(name)).unwrap().permissions();
        #[cfg(unix)]
        assert_eq!(
            std::os::unix::fs::PermissionsExt::mode(&permissions) & 0o7777,
            mode
        );
        #[cfg(windows)]
        assert_eq!(permissions.readonly(), mode & 0o222 == 0, "{name}");
        assert_eq!(root.stat(name).unwrap().unwrap().mode, mode);
        assert!(!root.stat(name).unwrap().unwrap().executable() || cfg!(unix));
    }
    // A read-only file is still removed (rollback and cleanup collect our own files).
    root.unlink("b").unwrap();
    assert!(root.stat("b").unwrap().is_none());
}

#[test]
fn hashing_reports_identity_mode_and_detects_a_concurrent_in_place_change() {
    #[cfg(unix)]
    use std::os::unix::fs::FileExt;
    let write_at = |file: &fs::File, bytes: &[u8], offset: u64| {
        #[cfg(unix)]
        file.write_all_at(bytes, offset).unwrap();
        #[cfg(windows)]
        assert_eq!(
            std::os::windows::fs::FileExt::seek_write(file, bytes, offset).unwrap(),
            bytes.len()
        );
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("f");
    fs::write(&path, "abc").unwrap();
    let root = Dir::open_root(temp.path()).unwrap();
    let (stat, hash) = root.hash_regular("f").unwrap();
    assert_eq!(stat.size, 3);
    assert_eq!(
        hash,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(stat.id, root.stat("f").unwrap().unwrap().id);
    assert!(!stat.executable());
    // An uninterrupted second read agrees, so the check below is not a false positive.
    assert_eq!(root.hash_regular("f").unwrap().1, hash);

    // Backdate the modification time: a write that lands within the filesystem's
    // timestamp granularity must still be told apart from the original state.
    let backdate = || {
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000))
            .unwrap();
    };

    // A same-length write between the last byte read and the final stat.
    backdate();
    let error = root
        .hash_regular_observed("f", || {
            let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
            write_at(&file, b"xyz", 0);
        })
        .unwrap_err();
    assert!(error.to_string().contains("changed while"), "{error}");

    // A write that changes the length.
    fs::write(&path, "abc").unwrap();
    backdate();
    let error = root
        .hash_regular_observed("f", || {
            let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
            write_at(&file, b"d", 3);
        })
        .unwrap_err();
    assert!(error.to_string().contains("changed while"), "{error}");

    // A mode change with identical bytes (the read-only attribute on Windows).
    fs::write(&path, "abc").unwrap();
    #[cfg(unix)]
    let (plain, changed) = (0o644, 0o755);
    #[cfg(windows)]
    let (plain, changed) = (0o644, 0o444);
    let set_mode = |mode: u32| {
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        #[cfg(unix)]
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, mode);
        #[cfg(windows)]
        permissions.set_readonly(mode & 0o222 == 0);
        fs::set_permissions(&path, permissions).unwrap();
    };
    set_mode(plain);
    let error = root
        .hash_regular_observed("f", || set_mode(changed))
        .unwrap_err();
    assert!(error.to_string().contains("changed while"), "{error}");

    // After the interleavings stop, the file hashes again and reports its new mode.
    let (stat, _) = root.hash_regular("f").unwrap();
    assert_eq!(stat.mode, changed);
    assert_eq!(stat.executable(), cfg!(unix));
    set_mode(plain);
}

#[test]
fn a_root_renamed_away_and_replaced_by_a_link_fails_the_pathname_binding() {
    #[cfg(unix)]
    let links: [fn(&Path, &Path); 1] = [symlink_dir];
    #[cfg(windows)]
    let links: [fn(&Path, &Path); 2] = [symlink_dir, junction];
    for link in links {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();
        let root = Dir::open_root(&project).unwrap();
        root.check_root_binding().unwrap();
        // A clone and a relative open keep (or drop) the binding consistently.
        root.try_clone().unwrap().check_root_binding().unwrap();
        fs::rename(&project, temp.path().join("moved")).unwrap();
        assert!(
            root.check_root_binding().is_err(),
            "a missing root pathname is not the opened directory"
        );
        link(&temp.path().join("moved"), &project);
        let error = root.check_root_binding().unwrap_err();
        assert!(
            error.contains("link") || error.contains("replaced"),
            "{error}"
        );
        // The held handle still reaches the real directory: the swap is only visible
        // through the pathname, which is exactly what the binding re-checks.
        assert!(root.is_still_at(&root, ""));
        // A look-alike directory under the old name is another directory.
        fs::remove_dir(&project).unwrap();
        fs::create_dir(&project).unwrap();
        assert!(root.check_root_binding().is_err());
    }
}

/// Unix: the walk from `/` catches an ancestor swapped for a look-alike tree. Windows:
/// NTFS refuses to rename a directory while a handle below it is open, so the swap
/// cannot happen while the root is held.
#[test]
fn a_swapped_ancestor_of_the_root_fails_the_pathname_binding() {
    let temp = tempfile::tempdir().unwrap();
    let outer = temp.path().join("outer");
    fs::create_dir_all(outer.join("project")).unwrap();
    let root = Dir::open_root(&outer.join("project")).unwrap();
    root.check_root_binding().unwrap();
    let moved = fs::rename(&outer, temp.path().join("outer-moved"));
    if cfg!(windows) {
        let error = moved.unwrap_err();
        assert_eq!(error.raw_os_error(), Some(5), "{error}");
        root.check_root_binding().unwrap();
        return;
    }
    moved.unwrap();
    // A look-alike tree appears under the old ancestor name.
    fs::create_dir_all(outer.join("project")).unwrap();
    assert!(
        root.check_root_binding().is_err(),
        "same names through a different ancestor are another directory"
    );
}

/// Unix: the link-based pair has a window between its two syscalls in which an
/// editor's save is lost; the atomic rename has none. Windows has no link pair; its
/// rename has a window between opening the source and renaming it, but the source
/// handle excludes `FILE_SHARE_DELETE`, so an editor's rename-over inside the window
/// fails with its bytes intact and its retry afterwards lands.
#[test]
fn link_then_unlink_loses_an_editor_save_between_the_two_syscalls() {
    use std::cell::Cell;
    let temp = tempfile::tempdir().unwrap();
    let dir = Dir::open_root(temp.path()).unwrap();
    let tag = tx();
    let orig = transaction_file_name(&tag, 0, TxRole::Original);
    let editor_temp = temp.path().join(".editor.tmp");
    let editor_save = |content: &str| {
        // An editor saves by writing a temporary file and renaming it over the name.
        fs::write(&editor_temp, content).unwrap();
        fs::rename(&editor_temp, temp.path().join("old.txt"))
    };

    if cfg!(windows) {
        fs::write(temp.path().join("old.txt"), "original A").unwrap();
        let in_window = Cell::new(None);
        dir.rename_noreplace_observed(NoClobber::Renameat2, "old.txt", &dir, &orig, || {
            in_window.set(Some(editor_save("editor B")));
        })
        .unwrap();
        let error = in_window.take().expect("window opened").unwrap_err();
        assert_eq!(
            error.raw_os_error(),
            Some(5),
            "ERROR_ACCESS_DENIED: {error}"
        );
        assert_eq!(fs::read(&editor_temp).unwrap(), b"editor B");
        assert_eq!(fs::read(temp.path().join(&orig)).unwrap(), b"original A");
        // The editor's retry after the move lands; nothing was lost or overwritten.
        fs::rename(&editor_temp, temp.path().join("old.txt")).unwrap();
        assert_eq!(fs::read(temp.path().join("old.txt")).unwrap(), b"editor B");
        return;
    }

    // Link based: the editor's rename lands after `linkat` and before `unlinkat`.
    fs::write(temp.path().join("old.txt"), "original A").unwrap();
    let window_opened = Cell::new(false);
    dir.rename_noreplace_observed(NoClobber::Link, "old.txt", &dir, &orig, || {
        window_opened.set(true);
        editor_save("editor B").unwrap();
    })
    .unwrap();
    assert!(window_opened.get());
    // The slot verifies perfectly (it holds A), yet the editor's file is gone: the
    // `unlinkat` removed B. This is why links never move a live source name.
    assert_eq!(fs::read(temp.path().join(&orig)).unwrap(), b"original A");
    assert!(
        !temp.path().join("old.txt").exists(),
        "editor B was silently unlinked"
    );

    // Atomic rename: there is no window. The editor saving after the move simply
    // creates its file; nothing of it can be lost.
    fs::remove_file(temp.path().join(&orig)).unwrap();
    fs::write(temp.path().join("old.txt"), "original A").unwrap();
    let window_opened = Cell::new(false);
    dir.rename_noreplace_observed(NoClobber::Renameat2, "old.txt", &dir, &orig, || {
        window_opened.set(true);
    })
    .unwrap();
    assert!(
        !window_opened.get(),
        "the atomic rename has no two-syscall window"
    );
    editor_save("editor B").unwrap();
    assert_eq!(fs::read(temp.path().join(&orig)).unwrap(), b"original A");
    assert_eq!(fs::read(temp.path().join("old.txt")).unwrap(), b"editor B");
}

/// The link pair exists only on Unix.
#[cfg(unix)]
#[test]
fn a_crash_inside_the_link_pair_leaves_both_names_and_never_loses_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let dir = Dir::open_root(temp.path()).unwrap();
    let tag = tx();
    let orig = transaction_file_name(&tag, 0, TxRole::Original);
    fs::write(temp.path().join("old.txt"), "original A").unwrap();
    // Partial success: only the first syscall ran before the process died.
    dir.link_noreplace("old.txt", &dir, &orig).unwrap();
    let source = dir.stat("old.txt").unwrap().unwrap();
    let slot = dir.stat(&orig).unwrap().unwrap();
    assert_eq!(source.id, slot.id);
    assert_eq!(source.nlink, 2);
    assert_eq!(
        fs::read(temp.path().join("old.txt")).unwrap(),
        b"original A"
    );
    // Linking onto an existing name refuses and changes nothing.
    let error = dir.link_noreplace("old.txt", &dir, &orig).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
}

#[test]
fn hard_linked_files_report_their_link_count_and_are_refused_for_publication() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("a.txt"), "shared").unwrap();
    fs::hard_link(temp.path().join("a.txt"), temp.path().join("b.txt")).unwrap();
    fs::write(temp.path().join("single.txt"), "alone").unwrap();
    let dir = Dir::open_root(temp.path()).unwrap();
    let (stat, _) = dir.hash_regular("a.txt").unwrap();
    assert_eq!(stat.nlink, 2);
    let reason = studio_project::publish::hard_link_refusal(&stat).unwrap();
    assert!(reason.contains("2 hard links"), "{reason}");
    let (single, _) = dir.hash_regular("single.txt").unwrap();
    assert_eq!(single.nlink, 1);
    assert!(studio_project::publish::hard_link_refusal(&single).is_none());
}

#[test]
fn directories_are_removed_only_when_empty_and_never_through_unlink() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("full").join("inner")).unwrap();
    fs::create_dir(temp.path().join("empty")).unwrap();
    fs::write(temp.path().join("file"), "f").unwrap();
    let dir = Dir::open_root(temp.path()).unwrap();
    assert_eq!(
        dir.rmdir("full").unwrap_err().kind(),
        ErrorKind::DirectoryNotEmpty
    );
    assert_eq!(
        dir.open_child("file").unwrap_err().kind(),
        ErrorKind::NotADirectory
    );
    assert!(dir.rmdir("file").is_err());
    assert!(dir.unlink("empty").is_err());
    dir.rmdir("empty").unwrap();
    assert!(dir.stat("empty").unwrap().is_none());
    assert_eq!(
        dir.mkdir("full", 0o755).unwrap_err().kind(),
        ErrorKind::AlreadyExists
    );
    dir.mkdir("new", 0o755).unwrap();
    assert_eq!(dir.stat("new").unwrap().unwrap().kind, EntryKind::Directory);
    // A held directory handle can be flushed.
    dir.open_child("new").unwrap().sync().unwrap();
    dir.sync().unwrap();
}

#[test]
fn only_exact_app_generated_names_are_internal() {
    let id = tx();
    for role in [
        TxRole::Stage,
        TxRole::Original,
        TxRole::Rollback,
        TxRole::Variant(2),
    ] {
        let name = transaction_file_name(&id, 17, role);
        assert!(is_transaction_internal_name(&name), "{name}");
    }
    for name in [
        ".fframes-tx-".to_owned(),
        format!(".fframes-tx-{id}-0"),
        format!(".fframes-tx-{id}-0.stage.bak"),
        format!(".fframes-tx-{id}-01.stage"),
        format!(".fframes-tx-{id}--1.stage"),
        format!(".fframes-tx-{id}-0.other"),
        format!(".fframes-tx-{id}-0.var"),
        format!(".fframes-tx-{}-0.stage", id.to_uppercase()),
        format!(".fframes-tx-{}-0.stage", &id[..31]),
        format!("x.fframes-tx-{id}-0.stage"),
        "notes.stage".to_owned(),
    ] {
        assert!(!is_transaction_internal_name(&name), "{name}");
    }
}

#[test]
fn the_source_inventory_excludes_exact_internal_files_but_never_user_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("a.txt"), "a").unwrap();
    let plain = SourceInventory::scan(root).unwrap();
    let id = tx();
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(
        root.join(transaction_file_name(&id, 0, TxRole::Stage)),
        "staged",
    )
    .unwrap();
    fs::write(
        root.join("sub")
            .join(transaction_file_name(&id, 3, TxRole::Original)),
        "orig",
    )
    .unwrap();
    fs::write(
        root.join(transaction_file_name(&id, 1, TxRole::Variant(1))),
        "variant",
    )
    .unwrap();
    let with_internal = SourceInventory::scan(root).unwrap();
    assert_eq!(with_internal.revision, plain.revision);
    assert_eq!(with_internal.files, plain.files);
    // Similar-looking filenames are user content.
    fs::write(root.join(".fframes-tx-mine.stage"), "user").unwrap();
    fs::write(root.join(format!(".fframes-tx-{id}-0.stage.bak")), "user").unwrap();
    let with_user = SourceInventory::scan(root).unwrap();
    assert_ne!(with_user.revision, plain.revision);
    let names: Vec<_> = with_user.files.iter().map(|f| f.path.as_str()).collect();
    assert!(names.contains(&".fframes-tx-mine.stage"));
    assert!(names.iter().any(|n| n.ends_with(".stage.bak")));
    // An internal-looking name that is not a regular file is still refused.
    let other = tempfile::tempdir().unwrap();
    fs::write(other.path().join("a.txt"), "a").unwrap();
    symlink_file(
        Path::new("a.txt"),
        &other
            .path()
            .join(transaction_file_name(&id, 9, TxRole::Stage)),
    );
    assert!(SourceInventory::scan(other.path()).is_err());
}
