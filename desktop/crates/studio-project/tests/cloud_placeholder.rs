//! A project inside a real Cloud Files API sync root (what OneDrive Files On-Demand
//! creates) is refused: the sync root and its placeholders are reparse points of a cloud
//! kind, so neither the root nor anything below it is ever opened for mutation.
#![cfg(windows)]

use std::{fs, os::windows::ffi::OsStrExt, path::Path};

use studio_project::publish::{Dir, EntryKind, is_cloud_reparse_tag};
use windows_sys::{
    Win32::Storage::CloudFilters::{
        CF_CREATE_FLAG_NONE, CF_HARDLINK_POLICY_NONE, CF_HYDRATION_POLICY,
        CF_HYDRATION_POLICY_FULL, CF_HYDRATION_POLICY_MODIFIER_NONE, CF_INSYNC_POLICY_NONE,
        CF_PLACEHOLDER_CREATE_FLAG_MARK_IN_SYNC, CF_PLACEHOLDER_CREATE_INFO,
        CF_PLACEHOLDER_MANAGEMENT_POLICY_DEFAULT, CF_POPULATION_POLICY,
        CF_POPULATION_POLICY_ALWAYS_FULL, CF_POPULATION_POLICY_MODIFIER_NONE,
        CF_REGISTER_FLAG_NONE, CF_SYNC_POLICIES, CF_SYNC_REGISTRATION, CfCreatePlaceholders,
        CfRegisterSyncRoot, CfUnregisterSyncRoot,
    },
    core::GUID,
};

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain([0]).collect()
}

/// Unregisters the sync root when the test ends, so no entry is left in the registry.
struct SyncRoot(Vec<u16>);

impl Drop for SyncRoot {
    fn drop(&mut self) {
        // SAFETY: NUL-terminated path registered below.
        unsafe { CfUnregisterSyncRoot(self.0.as_ptr()) };
    }
}

fn register_sync_root(path: &Path) -> SyncRoot {
    let path = wide(path);
    let name: Vec<u16> = "fframes publish test\0".encode_utf16().collect();
    let version: Vec<u16> = "1.0\0".encode_utf16().collect();
    let identity = b"fframes-publish-test";
    let registration = CF_SYNC_REGISTRATION {
        StructSize: size_of::<CF_SYNC_REGISTRATION>() as u32,
        ProviderName: name.as_ptr(),
        ProviderVersion: version.as_ptr(),
        SyncRootIdentity: identity.as_ptr().cast(),
        SyncRootIdentityLength: identity.len() as u32,
        FileIdentity: identity.as_ptr().cast(),
        FileIdentityLength: identity.len() as u32,
        ProviderId: GUID::from_u128(0x6f1d_2a3c_51b4_4c1e_9a57_0c2e_8d3f_a001),
    };
    let policies = CF_SYNC_POLICIES {
        StructSize: size_of::<CF_SYNC_POLICIES>() as u32,
        Hydration: CF_HYDRATION_POLICY {
            Primary: CF_HYDRATION_POLICY_FULL,
            Modifier: CF_HYDRATION_POLICY_MODIFIER_NONE,
        },
        Population: CF_POPULATION_POLICY {
            Primary: CF_POPULATION_POLICY_ALWAYS_FULL,
            Modifier: CF_POPULATION_POLICY_MODIFIER_NONE,
        },
        InSync: CF_INSYNC_POLICY_NONE,
        HardLink: CF_HARDLINK_POLICY_NONE,
        PlaceholderManagement: CF_PLACEHOLDER_MANAGEMENT_POLICY_DEFAULT,
    };
    // SAFETY: all pointers are valid for the call.
    let hr = unsafe {
        CfRegisterSyncRoot(
            path.as_ptr(),
            &registration,
            &policies,
            CF_REGISTER_FLAG_NONE,
        )
    };
    assert!(hr >= 0, "CfRegisterSyncRoot failed: {hr:#x}");
    SyncRoot(path)
}

fn create_placeholder(dir: &Path, name: &str, size: i64) {
    let base = wide(dir);
    let relative: Vec<u16> = name.encode_utf16().chain([0]).collect();
    let identity = name.as_bytes();
    let mut info = CF_PLACEHOLDER_CREATE_INFO {
        RelativeFileName: relative.as_ptr(),
        FileIdentity: identity.as_ptr().cast(),
        FileIdentityLength: identity.len() as u32,
        Flags: CF_PLACEHOLDER_CREATE_FLAG_MARK_IN_SYNC,
        ..Default::default()
    };
    info.FsMetadata.FileSize = size;
    info.FsMetadata.BasicInfo.FileAttributes = 0x80; // FILE_ATTRIBUTE_NORMAL
    let mut processed = 0u32;
    // SAFETY: all pointers are valid for the call.
    let hr = unsafe {
        CfCreatePlaceholders(
            base.as_ptr(),
            &mut info,
            1,
            CF_CREATE_FLAG_NONE,
            &mut processed,
        )
    };
    assert!(
        hr >= 0 && info.Result >= 0,
        "CfCreatePlaceholders({name}) failed: {hr:#x} / {:#x}",
        info.Result
    );
}

#[test]
fn a_project_in_a_cloud_synced_folder_is_refused_and_never_opened() {
    let temp = tempfile::tempdir().unwrap();
    let sync = temp.path().join("cloud");
    fs::create_dir(&sync).unwrap();
    let _registration = register_sync_root(&sync);
    create_placeholder(&sync, "scene.rs", 1234);
    // A project folder somewhere below the sync root.
    fs::create_dir(sync.join("project")).unwrap();

    // The sync root is itself a cloud reparse point (seen once the process opted in to
    // PHCM_EXPOSE_PLACEHOLDERS, which `Dir::open_root` does).
    let parent = Dir::open_root(temp.path()).unwrap();
    let root_stat = parent.stat("cloud").unwrap().unwrap();
    assert_eq!(root_stat.kind, EntryKind::CloudPlaceholder);
    assert!(is_cloud_reparse_tag(root_stat.reparse_tag));
    assert!(parent.open_child("cloud").is_err());

    // Neither the sync root nor a project folder below it opens as a project root, and
    // the reason asks the user to move the project to a local folder.
    for root in [sync.clone(), sync.join("project")] {
        let error = Dir::open_root(&root).unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains("cloud-files placeholder") && text.contains("local folder"),
            "{}: {text}",
            root.display()
        );
    }
    // Nothing was hydrated or moved.
    assert_eq!(fs::read_dir(&sync).unwrap().count(), 2);
}
