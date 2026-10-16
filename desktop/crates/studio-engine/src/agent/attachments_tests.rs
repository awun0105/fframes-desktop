use super::*;

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join(".env"), "SECRET=1").unwrap();
    fs::write(dir.path().join("Server.PEM"), "key").unwrap();
    dir
}

fn file(path: &str) -> AttachmentRequest {
    AttachmentRequest::File {
        path: PathBuf::from(path),
    }
}

fn attach(
    dir: &tempfile::TempDir,
    request: AttachmentRequest,
) -> Result<PreparedAttachment, WorkflowError> {
    prepare(dir.path(), &StageStore::default(), request)
}

fn message(error: &WorkflowError) -> String {
    assert!(matches!(error, WorkflowError::Refused(_)), "{error:?}");
    error.to_string()
}

fn never_allowed(result: Result<PreparedAttachment, WorkflowError>, what: &str) {
    let error = result.expect_err(what);
    assert!(message(&error).contains("never allowed"), "{what}: {error}");
}

#[test]
fn a_project_file_is_attached_by_its_project_path() {
    let dir = project();
    let attached = attach(&dir, file("src/main.rs")).unwrap();
    assert_eq!(attached.label(), "src/main.rs");
    assert!(!attached.is_image());
    assert!(!attached.is_embedded());
    let absolute = dir.path().join("src").join("main.rs");
    let attached = attach(&dir, file(absolute.to_str().unwrap())).unwrap();
    assert_eq!(attached.label(), "src/main.rs");
}

#[test]
fn a_project_file_becomes_a_link_inside_the_working_copy_never_the_project_root() {
    let dir = project();
    let draft = tempfile::tempdir().unwrap();
    fs::create_dir_all(draft.path().join("src")).unwrap();
    fs::write(
        draft.path().join("src/main.rs"),
        "fn main() { /* draft */ }",
    )
    .unwrap();
    let attached = attach(&dir, file("src/main.rs")).unwrap();
    let (files, images) = PreparedAttachment::split(vec![attached], draft.path()).unwrap();
    assert!(images.is_empty());
    let path = files[0].path().unwrap().to_path_buf();
    let canonical_draft = fs::canonicalize(draft.path()).unwrap();
    assert!(path.starts_with(&canonical_draft), "{path:?}");
    assert!(!path.starts_with(fs::canonicalize(dir.path()).unwrap()));
    assert_eq!(files[0].name(), "src/main.rs");

    // A file the working copy does not hold is reported, not linked to the project.
    let attached = attach(&dir, file("src/main.rs")).unwrap();
    let empty = tempfile::tempdir().unwrap();
    let error = PreparedAttachment::split(vec![attached], empty.path()).unwrap_err();
    assert!(error.contains("working copy"), "{error}");
}

#[test]
fn denied_names_are_refused_inside_and_outside_and_cannot_be_confirmed() {
    let dir = project();
    for name in [".env", ".ENV", "Server.PEM", ".git/config", ".ssh/id"] {
        let error = attach(&dir, file(name)).unwrap_err();
        assert!(matches!(error, WorkflowError::Refused(_)), "{name}");
    }
    let outside = tempfile::tempdir().unwrap();
    let key = outside.path().join("deploy.key");
    fs::write(&key, "key").unwrap();
    let error = stage(dir.path(), &StageStore::default(), &key).unwrap_err();
    assert!(error.to_string().contains("never allowed"), "{error}");
}

#[test]
fn a_path_that_climbs_out_of_the_project_is_judged_by_where_it_lands() {
    let dir = project();
    let error = attach(&dir, file("../elsewhere.txt")).unwrap_err();
    assert!(matches!(error, WorkflowError::Refused(_)));
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("n.txt"), "x").unwrap();
    let climbing = dir
        .path()
        .join("..")
        .join(outside.path().file_name().unwrap())
        .join("n.txt");
    // Only a path that really is outside can be staged, and a file request for it is
    // refused as outside whatever its spelling.
    let error = attach(&dir, AttachmentRequest::File { path: climbing }).unwrap_err();
    assert!(
        message(&error).contains("outside the project")
            || message(&error).contains("cannot be read")
    );
}

#[test]
fn an_outside_file_is_staged_then_confirmed_once_and_sent_as_content() {
    let dir = project();
    let store = StageStore::default();
    let outside = tempfile::tempdir().unwrap();
    let notes = outside.path().join("notes.txt");
    fs::write(&notes, "hello").unwrap();

    let error = prepare(dir.path(), &store, file(notes.to_str().unwrap())).unwrap_err();
    assert!(message(&error).contains("outside the project"), "{error}");

    let staged = stage(dir.path(), &store, &notes).unwrap();
    assert_eq!(staged.label, "notes.txt");
    assert_eq!(staged.size, 5);
    assert_eq!(staged.path, fs::canonicalize(&notes).unwrap());
    let token = AttachmentRequest::Staged {
        token: staged.token.clone(),
    };
    let attached = prepare(dir.path(), &store, token.clone()).unwrap();
    assert!(attached.is_embedded());
    assert_eq!(attached.label(), "notes.txt");
    let draft = tempfile::tempdir().unwrap();
    let (files, _) = PreparedAttachment::split(vec![attached], draft.path()).unwrap();
    assert!(files[0].is_embedded());
    assert_eq!(files[0].content_len(), 5);

    // Single use.
    let error = prepare(dir.path(), &store, token).unwrap_err();
    assert!(
        message(&error).contains("expired or was already used"),
        "{error}"
    );
}

#[test]
fn a_staged_token_expires_and_unknown_tokens_are_refused() {
    let dir = project();
    let outside = tempfile::tempdir().unwrap();
    let notes = outside.path().join("notes.txt");
    fs::write(&notes, "hello").unwrap();
    let store = StageStore::with_ttl(Duration::ZERO);
    let staged = stage(dir.path(), &store, &notes).unwrap();
    let error = prepare(
        dir.path(),
        &store,
        AttachmentRequest::Staged {
            token: staged.token,
        },
    )
    .unwrap_err();
    assert!(message(&error).contains("expired"), "{error}");
    let forged = AttachmentRequest::Staged {
        token: StageToken("0".repeat(32)),
    };
    assert!(prepare(dir.path(), &StageStore::default(), forged).is_err());
}

#[test]
fn a_file_replaced_after_staging_is_not_the_confirmed_file() {
    let dir = project();
    let store = StageStore::default();
    let outside = tempfile::tempdir().unwrap();
    let notes = outside.path().join("notes.txt");
    let other = outside.path().join("other.txt");
    fs::write(&notes, "hello").unwrap();
    fs::write(&other, "something else").unwrap();
    let staged = stage(dir.path(), &store, &notes).unwrap();
    fs::rename(&other, &notes).unwrap();
    let error = prepare(
        dir.path(),
        &store,
        AttachmentRequest::Staged {
            token: staged.token,
        },
    )
    .unwrap_err();
    assert!(
        message(&error).contains("not the file that was confirmed"),
        "{error}"
    );
}

#[test]
fn staging_refuses_project_files_relative_paths_and_oversized_files() {
    let dir = project();
    let store = StageStore::default();
    let inside = dir.path().join("src/main.rs");
    assert!(
        message(&stage(dir.path(), &store, &inside).unwrap_err()).contains("inside the project")
    );
    assert!(stage(dir.path(), &store, Path::new("src/main.rs")).is_err());
    let outside = tempfile::tempdir().unwrap();
    let big = outside.path().join("big.bin");
    fs::write(&big, vec![0u8; MAX_EMBEDDED_FILE_BYTES as usize + 1]).unwrap();
    assert!(message(&stage(dir.path(), &store, &big).unwrap_err()).contains("larger than"));

    // A file that grows after it was confirmed is refused when it is read.
    let small = outside.path().join("small.txt");
    fs::write(&small, "x").unwrap();
    let staged = stage(dir.path(), &store, &small).unwrap();
    fs::write(&small, vec![b'y'; MAX_EMBEDDED_FILE_BYTES as usize + 1]).unwrap();
    let error = prepare(
        dir.path(),
        &store,
        AttachmentRequest::Staged {
            token: staged.token,
        },
    )
    .unwrap_err();
    assert!(message(&error).contains("larger than"), "{error}");
}

#[test]
fn a_file_with_a_second_name_is_refused_inside_and_outside() {
    let dir = project();
    fs::hard_link(dir.path().join("src/main.rs"), dir.path().join("second.rs")).unwrap();
    for name in ["src/main.rs", "second.rs"] {
        let error = attach(&dir, file(name)).unwrap_err();
        assert!(message(&error).contains("hard link"), "{name}: {error}");
    }
    let outside = tempfile::tempdir().unwrap();
    let first = outside.path().join("a.txt");
    fs::write(&first, "x").unwrap();
    fs::hard_link(&first, outside.path().join("b.txt")).unwrap();
    let error = stage(dir.path(), &StageStore::default(), &first).unwrap_err();
    assert!(message(&error).contains("hard link"), "{error}");
}

#[test]
fn the_stage_store_keeps_a_bounded_number_of_confirmations() {
    let store = StageStore::default();
    let identity = FileIdentity {
        volume: 1,
        index: 1,
    };
    let first = store.insert(PathBuf::from("first"), identity);
    for index in 0..MAX_STAGED {
        store.insert(PathBuf::from(format!("n{index}")), identity);
    }
    assert!(store.redeem(&first).is_none(), "the oldest is dropped");
}

#[test]
fn labels_lose_control_and_format_characters_and_stay_bounded() {
    assert_eq!(
        clean_label("a\u{202E}txt.exe\u{7}\u{200B}", "x"),
        "atxt.exe"
    );
    assert_eq!(clean_label("  \u{202E}\u{FEFF} ", "image"), "image");
    assert_eq!(
        clean_label(&"a".repeat(500), "x").chars().count(),
        MAX_LABEL_CHARS
    );
    let attached = prepare_image("sho\u{202E}t\n", &encoded(ImageFormat::Png)).unwrap();
    assert_eq!(attached.label(), "shot");
}

#[cfg(unix)]
#[test]
fn links_are_refused_inside_and_outside() {
    let dir = project();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("target.txt");
    fs::write(&target, "x").unwrap();
    std::os::unix::fs::symlink(&target, dir.path().join("link.txt")).unwrap();
    assert!(attach(&dir, file("link.txt")).is_err());
    let alias = outside.path().join("alias.txt");
    std::os::unix::fs::symlink(&target, &alias).unwrap();
    assert!(stage(dir.path(), &StageStore::default(), &alias).is_err());
}

#[cfg(unix)]
#[test]
fn a_linked_directory_into_ssh_is_refused() {
    let dir = project();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir_all(outside.path().join(".ssh")).unwrap();
    fs::write(outside.path().join(".ssh/id"), "k").unwrap();
    std::os::unix::fs::symlink(outside.path().join(".ssh"), dir.path().join("docs")).unwrap();
    never_allowed(attach(&dir, file("docs/id")), "linked directory");
}

// ---- Windows path spellings, run for real on Windows ---------------------------------------------------

#[cfg(windows)]
mod windows_spellings {
    use super::*;
    use std::process::Command;

    fn junction(link: &Path, target: &Path) {
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(status.status.success(), "mklink /J: {status:?}");
    }

    /// The 8.3 name of `path`, when the volume has one that differs from the name.
    fn short_name(path: &Path) -> Option<PathBuf> {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: `wide` is NUL-terminated; the output buffer length is passed along.
        let mut buffer = vec![0u16; 1024];
        let length = unsafe { GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), 1024) };
        if length == 0 || length as usize >= buffer.len() {
            return None;
        }
        let short = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize]));
        (short != path && short.file_name() != path.file_name()).then_some(short)
    }

    #[test]
    fn trailing_dots_spaces_and_streams_cannot_reach_a_denied_name() {
        let dir = project();
        fs::write(dir.path().join("X.key"), "k").unwrap();
        fs::write(dir.path().join("id.key"), "k").unwrap();
        fs::write(dir.path().join("a.pem"), "k").unwrap();
        for name in [
            "X.key ",
            "X.key.",
            ".env ",
            ".env.",
            "Server.PEM.",
            ".ENV  ",
        ] {
            never_allowed(attach(&dir, file(name)), name);
        }
        for name in [
            ".env::$DATA",
            ".env:s",
            "id.key:s",
            "a.pem::$DATA",
            "src/main.rs:hidden",
            "src/main.rs::$DATA",
        ] {
            let error = attach(&dir, file(name)).expect_err(name);
            assert!(
                message(&error).contains("alternate data stream"),
                "{name}: {error}"
            );
        }
    }

    #[test]
    fn outside_spellings_with_streams_or_trailing_characters_are_refused_too() {
        let dir = project();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("k.key"), "k").unwrap();
        fs::write(outside.path().join("n.txt"), "x").unwrap();
        let store = StageStore::default();
        let key = outside.path().join("k.key ");
        assert!(message(&stage(dir.path(), &store, &key).unwrap_err()).contains("never allowed"));
        let stream = outside.path().join("n.txt:s");
        assert!(message(&stage(dir.path(), &store, &stream).unwrap_err()).contains("data stream"));
        // Device and drive-relative spellings are not file paths.
        assert!(stage(dir.path(), &store, Path::new(r"\\.\C:\Windows\win.ini")).is_err());
        assert!(stage(dir.path(), &store, Path::new(r"C:win.ini")).is_err());
    }

    #[test]
    fn an_8_3_short_name_of_a_denied_file_is_refused_by_where_it_really_is() {
        let dir = project();
        fs::create_dir_all(dir.path().join(".ssh")).unwrap();
        fs::write(dir.path().join(".ssh/id"), "k").unwrap();
        // Long names are the ones that get an 8.3 alias (`ENV~1.PRO`, `SERVER~1.KEY`).
        fs::write(dir.path().join(".env.production"), "k").unwrap();
        fs::write(dir.path().join("server-private.key"), "k").unwrap();
        fs::write(dir.path().join("certificate-bundle.pem"), "k").unwrap();
        let mut checked = 0;
        for target in [
            dir.path().join(".env"),
            dir.path().join(".env.production"),
            dir.path().join("server-private.key"),
            dir.path().join("certificate-bundle.pem"),
            dir.path().join(".ssh").join("id"),
        ] {
            let Some(short) = short_name(&target) else {
                eprintln!(
                    "skipped: this volume has no distinct 8.3 name for {}",
                    target.display()
                );
                continue;
            };
            checked += 1;
            never_allowed(
                attach(
                    &dir,
                    AttachmentRequest::File {
                        path: short.clone(),
                    },
                ),
                &short.display().to_string(),
            );
        }
        if checked == 0 {
            eprintln!("skipped: 8.3 short names are not enabled on this volume");
        }
    }

    #[test]
    fn a_junction_into_ssh_is_refused_whichever_side_of_the_project_it_is_on() {
        let dir = project();
        // A junction inside the project to a denied folder inside the project.
        fs::create_dir_all(dir.path().join(".ssh")).unwrap();
        fs::write(dir.path().join(".ssh/id"), "k").unwrap();
        junction(&dir.path().join("alias"), &dir.path().join(".ssh"));
        never_allowed(
            attach(&dir, file("alias/id")),
            "junction to the project's .ssh",
        );

        // A junction inside the project to a denied folder elsewhere.
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(outside.path().join(".ssh")).unwrap();
        fs::write(outside.path().join(".ssh/id"), "k").unwrap();
        junction(&dir.path().join("docs"), &outside.path().join(".ssh"));
        never_allowed(attach(&dir, file("docs/id")), "junction to an outside .ssh");
    }

    #[test]
    fn a_junction_to_an_ordinary_outside_folder_makes_an_outside_file_that_needs_staging() {
        let dir = project();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("n.txt"), "x").unwrap();
        junction(&dir.path().join("shared"), outside.path());
        let error = attach(&dir, file("shared/n.txt")).unwrap_err();
        assert!(message(&error).contains("outside the project"), "{error}");
    }

    #[test]
    fn a_file_symlink_is_refused_when_the_account_may_create_one() {
        let dir = project();
        let target = dir.path().join("src/main.rs");
        let link = dir.path().join("link.rs");
        if std::os::windows::fs::symlink_file(&target, &link).is_err() {
            eprintln!("skipped: this account may not create symbolic links");
            return;
        }
        let error = attach(&dir, file("link.rs")).unwrap_err();
        assert!(message(&error).contains("link"), "{error}");
    }

    #[test]
    fn file_identity_comes_from_the_volume_serial_and_file_index() {
        let dir = project();
        let first = File::open(dir.path().join("src/main.rs")).unwrap();
        let second = File::open(dir.path().join("src/main.rs")).unwrap();
        let other = File::open(dir.path().join(".env")).unwrap();
        let (a, links) = handle_facts(&first).unwrap();
        assert_eq!(links, 1);
        assert_eq!(a, handle_facts(&second).unwrap().0);
        assert_ne!(a, handle_facts(&other).unwrap().0);
    }
}

// ---- images --------------------------------------------------------------------------------------------

fn encoded(format: ImageFormat) -> Vec<u8> {
    let image = DynamicImage::new_rgb8(8, 4);
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, format).unwrap();
    out.into_inner()
}

fn png_of(attached: &PreparedAttachment) -> &[u8] {
    let Inner::Image { png, .. } = &attached.0 else {
        panic!("not an image");
    };
    png
}

#[test]
fn images_become_png_and_bad_images_are_refused() {
    for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Gif] {
        let attached = prepare_image("shot", &encoded(format)).unwrap();
        assert_eq!(
            image::guess_format(png_of(&attached)).unwrap(),
            ImageFormat::Png
        );
    }
    assert!(prepare_image("x", b"not an image").is_err());
    assert!(prepare_image("x", b"").is_err());
    assert!(
        format!(
            "{:?}",
            prepare_image("shot", &encoded(ImageFormat::Png)).unwrap()
        )
        .contains("shot")
    );
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[test]
fn a_png_is_re_encoded_so_metadata_and_trailing_bytes_are_dropped() {
    let plain = encoded(ImageFormat::Png);
    // A text chunk right after IHDR (8 signature + 25 IHDR bytes), with a valid CRC.
    let mut chunk_data = b"Comment".to_vec();
    chunk_data.push(0);
    chunk_data.extend_from_slice(b"META-SECRET");
    let mut chunk = (chunk_data.len() as u32).to_be_bytes().to_vec();
    let mut typed = b"tEXt".to_vec();
    typed.extend_from_slice(&chunk_data);
    chunk.extend_from_slice(&typed);
    chunk.extend_from_slice(&crc32(&typed).to_be_bytes());
    let mut dirty = plain[..33].to_vec();
    dirty.extend_from_slice(&chunk);
    dirty.extend_from_slice(&plain[33..]);
    dirty.extend_from_slice(b"TRAILING-SECRET");
    assert!(dirty.windows(11).any(|w| w == b"META-SECRET"));

    let attached = prepare_image("shot", &dirty).unwrap();
    let png = png_of(&attached);
    assert!(!png.windows(11).any(|w| w == b"META-SECRET"));
    assert!(!png.windows(15).any(|w| w == b"TRAILING-SECRET"));
    assert!(!png.windows(4).any(|w| w == b"tEXt"));
    assert_eq!(image::load_from_memory(png).unwrap().width(), 8);
}

#[test]
fn a_jpeg_orientation_is_applied_before_the_metadata_is_dropped() {
    let jpeg = encoded(ImageFormat::Jpeg);
    assert_eq!(&jpeg[..2], &[0xFF, 0xD8]);
    // APP1 "Exif" with one IFD entry: orientation 6 (rotate 90 degrees clockwise).
    let mut exif = vec![0xFF, 0xE1, 0x00, 0x22];
    exif.extend_from_slice(b"Exif\0\0");
    exif.extend_from_slice(b"MM\0*\0\0\0\x08");
    exif.extend_from_slice(&[
        0x00, 0x01, 0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x06, 0x00, 0x00,
    ]);
    exif.extend_from_slice(&[0, 0, 0, 0]);
    let mut tagged = jpeg[..2].to_vec();
    tagged.extend_from_slice(&exif);
    tagged.extend_from_slice(&jpeg[2..]);
    let attached = prepare_image("photo", &tagged).unwrap();
    let decoded = image::load_from_memory(png_of(&attached)).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (4, 8));
}
