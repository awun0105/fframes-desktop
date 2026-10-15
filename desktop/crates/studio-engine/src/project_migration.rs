//! Migration of a project generated with the first instructions to the SDK's current
//! framework, which plays Studio's timing documents and exports with settings.
//!
//! The migration rewrites the framework versions in `Cargo.toml` (and nothing else in
//! it), regenerates the worker around the project's own video, opts the CLI in
//! `src/main.rs` into `timeline.json`, adds the timing section to `AGENTS.md`, and moves
//! `studio.json` to the current SDK pin and instruction version. `src/lib.rs`, the
//! user's scenes, is never touched. A file it does not recognise refuses the migration
//! with the reason instead of being overwritten.
//!
//! Planning only reads the inventoried bytes; the controller publishes the plan.
use std::{io::Read, path::Path};

use sha2::{Digest, Sha256};
use studio_project::{
    Manifest, ProjectPath, SourceInventory, manifest::CURRENT_INSTRUCTION_VERSION,
};
use studio_sdk::CompatibilityManifest;

use crate::{build_materialization::sdk_pin, preset_state::FileChange};

/// Why timing cannot be edited in a project that was not migrated.
pub const MIGRATE_TO_EDIT_TIMING: &str = "Migrate this project to edit timing";
/// The document the CLI and Studio's timeline share.
pub const TIMING_FILE: &str = "timeline.json";

const FRAMEWORK_CRATES: [&str; 3] = [
    "fframes",
    "fframes-studio-runtime",
    "fframes-studio-protocol",
];
const MAX_SOURCE_FILE: u64 = 1024 * 1024;
const TEMPLATE_WORKER: &str =
    include_str!("../../studio-project/templates/src/bin/studio_worker.rs");
const TEMPLATE_INSTRUCTIONS: &str = include_str!("../../studio-project/templates/AGENTS.md");
/// The video of the template's worker, replaced by the project's own.
const TEMPLATE_VIDEO: &str = "studio_video::StudioVideo::new()?";
const TEMPLATE_VIDEO_TYPE: &str = "studio_video::StudioVideo";

/// The raw `studio.json` a migration (or its undo) replaces after its file set, kept so
/// the original bytes come back exactly.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestSwap {
    pub before: String,
    pub after: String,
}

/// The complete change a migration makes.
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    /// The source files, published as one file-set transaction.
    pub changes: Vec<FileChange>,
    /// `studio.json` after the migration, written after the file set (a manifest that
    /// is briefly absent would make the project unopenable).
    pub manifest: Manifest,
    /// What the migration left for the user to do.
    pub notes: Vec<String>,
}

/// Whether `manifest` was generated before the current instructions.
pub fn needs_migration(manifest: &Manifest) -> bool {
    manifest.generated_instruction_version < CURRENT_INSTRUCTION_VERSION
}

/// Plans the migration of the project at `root` to the current framework of the SDK
/// described by `compatibility`, or explains why it cannot be migrated.
pub fn plan(
    root: &Path,
    inventory: &SourceInventory,
    manifest: &Manifest,
    compatibility: &CompatibilityManifest,
) -> Result<MigrationPlan, String> {
    if !needs_migration(manifest) {
        return Err("The project already uses the current framework".into());
    }
    if compatibility.runtime_version.is_none() {
        return Err("Install the current SDK to migrate this project".into());
    }
    let pinned_release = &manifest.sdk.release;
    if manifest.sdk != sdk_pin(compatibility)
        && !compatibility
            .legacy
            .as_ref()
            .is_some_and(|legacy| legacy.sdk_releases.contains(pinned_release))
    {
        return Err(format!(
            "The project is pinned to SDK {pinned_release}, which this SDK does not continue"
        ));
    }
    let read = |path: &str| read_source(root, inventory, path);
    let mut changes = Vec::new();
    let mut notes = Vec::new();

    let cargo_path = manifest.entry.manifest.as_str();
    let cargo = read(cargo_path)?.ok_or_else(|| format!("{cargo_path} is missing"))?;
    let package_dir = cargo_path.strip_suffix("Cargo.toml").unwrap_or("");
    let cargo = rewrite_framework_pins(
        &cargo,
        &compatibility.fframes_version,
        compatibility.studio_runtime_version(),
    )
    .map_err(|reason| format!("{cargo_path}: {reason}"))?;
    changes.push(change(cargo_path, cargo));

    let worker_path = format!("{package_dir}src/bin/{}.rs", manifest.entry.worker_target);
    let worker = read(&worker_path)?.ok_or_else(|| {
        format!("{worker_path} is missing; the migration regenerates the generated worker")
    })?;
    let worker = regenerate_worker(&worker).map_err(|reason| format!("{worker_path}: {reason}"))?;
    changes.push(change(&worker_path, worker));

    let main_path = format!("{package_dir}src/main.rs");
    match read(&main_path)? {
        Some(main) => match opt_into_timing(&main) {
            Ok(Some(main)) => changes.push(change(&main_path, main)),
            Ok(None) => (),
            Err(reason) => notes.push(format!(
                "{main_path}: {reason}; add .timing_file(\"{TIMING_FILE}\") to the fframes::cli runner so the CLI plays Studio's timeline"
            )),
        },
        None => notes.push(format!(
            "{main_path} is missing; the CLI does not play Studio's timeline"
        )),
    }

    if let Some(instructions) = read("AGENTS.md")?
        && let Some(instructions) = add_timing_instructions(&instructions)
    {
        changes.push(change("AGENTS.md", instructions));
    }

    let mut next = manifest.clone();
    next.sdk = sdk_pin(compatibility);
    next.generated_instruction_version = CURRENT_INSTRUCTION_VERSION;
    Ok(MigrationPlan {
        changes,
        manifest: next,
        notes,
    })
}

fn change(path: &str, text: String) -> FileChange {
    FileChange {
        path: ProjectPath::try_from(path.to_owned()).expect("migration paths are portable"),
        bytes: Some(text.into_bytes()),
    }
}

/// Reads one source file of the inventory, verifying the bytes are the inventoried ones.
fn read_source(
    root: &Path,
    inventory: &SourceInventory,
    path: &str,
) -> Result<Option<String>, String> {
    let Some(file) = inventory.files.iter().find(|f| f.path.as_str() == path) else {
        return Ok(None);
    };
    if file.size > MAX_SOURCE_FILE {
        return Err(format!("{path} is too large to migrate"));
    }
    let mut bytes = Vec::with_capacity(file.size as usize);
    file.path
        .open_file(root)
        .map_err(|error| error.to_string())?
        .take(MAX_SOURCE_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{path}: {error}"))?;
    if format!("{:x}", Sha256::digest(&bytes)) != file.sha256 {
        return Err(format!(
            "{path} changed while it was being read; refresh and retry"
        ));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| format!("{path} is not UTF-8"))
}

/// Sets the exact version of every framework crate in `[dependencies]` and leaves every
/// other byte of the manifest as it was.
fn rewrite_framework_pins(text: &str, fframes: &str, runtime: &str) -> Result<String, String> {
    let version = |name: &str| if name == "fframes" { fframes } else { runtime };
    let mut section = String::new();
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            section = trimmed
                .trim_end()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_owned();
            out.push_str(line);
            continue;
        }
        let key = trimmed
            .split('=')
            .next()
            .map(|key| key.trim().trim_matches('"'));
        let rewritten = if section == "dependencies" {
            key.and_then(|key| FRAMEWORK_CRATES.into_iter().find(|name| *name == key))
                .map(|name| set_version(line, version(name), true))
        } else if let Some(table) = section.strip_prefix("dependencies.") {
            FRAMEWORK_CRATES
                .into_iter()
                .find(|name| *name == table.trim().trim_matches('"'))
                .filter(|_| key == Some("version"))
                .map(|name| set_version(line, version(name), false))
        } else {
            None
        };
        match rewritten {
            Some(line) => out.push_str(&line.ok_or("a framework version is not a quoted string")?),
            None => out.push_str(line),
        }
    }
    // The result must pin every framework crate exactly, as the SDK binding requires.
    let cargo: toml::Value = out
        .parse()
        .map_err(|error| format!("is not valid TOML: {error}"))?;
    for name in FRAMEWORK_CRATES {
        let dependency = cargo
            .get("dependencies")
            .and_then(|dependencies| dependencies.get(name))
            .ok_or_else(|| format!("does not depend on {name}"))?;
        let pinned = dependency
            .as_str()
            .or_else(|| dependency.get("version").and_then(toml::Value::as_str));
        if pinned != Some(format!("={}", version(name)).as_str())
            || dependency.get("package").is_some()
            || dependency.get("workspace").is_some()
        {
            return Err(format!(
                "the {name} dependency is not a plain version; set it to ={} by hand",
                version(name)
            ));
        }
    }
    Ok(out)
}

/// Replaces the quoted version on `line`: the value of an inline dependency, the
/// `version` of an inline table, or the value of a table's `version` key.
fn set_version(line: &str, version: &str, inline: bool) -> Option<String> {
    let value = line.find('=')? + 1;
    let anchor = if inline && line[value..].trim_start().starts_with('{') {
        let key = value + line[value..].find("version")?;
        key + line[key..].find('=')? + 1
    } else {
        value
    };
    let open = anchor + line[anchor..].find('"')?;
    let close = open + 1 + line[open + 1..].find('"')?;
    Some(format!(
        "{}\"={version}\"{}",
        &line[..open],
        &line[close + 1..]
    ))
}

/// The template's worker around the video the project's own worker serves.
fn regenerate_worker(worker: &str) -> Result<String, String> {
    let unrecognised =
        || "is not a generated Studio worker; update it to the current template by hand";
    let served = worker
        .split_once("serve_preview_worker(")
        .and_then(|(_, rest)| rest.split_once(','))
        .map(|(video, _)| video.trim().trim_start_matches('&').trim())
        .ok_or_else(unrecognised)?;
    let construction = if is_identifier(served) {
        let binding = format!("let {served} = ");
        let mut constructions = worker.match_indices(&binding).map(|(at, _)| {
            let rest = &worker[at + binding.len()..];
            rest[..rest.find(';').unwrap_or(rest.len())].trim()
        });
        let first = constructions.next().ok_or_else(unrecognised)?;
        if constructions.any(|other| other != first) {
            return Err(unrecognised().into());
        }
        first
    } else {
        served
    };
    let video_type = construction
        .strip_suffix("::new()?")
        .or_else(|| construction.strip_suffix("::new()"))
        .unwrap_or(construction);
    if !is_path(video_type) || !worker.contains("PreviewWorkerConfig::new(") {
        return Err(unrecognised().into());
    }
    Ok(TEMPLATE_WORKER
        .replace(TEMPLATE_VIDEO, construction)
        .replace(TEMPLATE_VIDEO_TYPE, video_type))
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_path(text: &str) -> bool {
    !text.is_empty() && text.split("::").all(is_identifier)
}

/// `main.rs` with the CLI reading `timeline.json`; `None` when it already does.
fn opt_into_timing(main: &str) -> Result<Option<String>, String> {
    if main.contains(".timing_file(") {
        return Ok(None);
    }
    let mut runners = main.match_indices("cli::new(");
    let (start, _) = runners.next().ok_or("no fframes::cli runner was found")?;
    if runners.next().is_some() {
        return Err("it has more than one fframes::cli runner".into());
    }
    let run = start
        + main[start..]
            .find(".run()")
            .ok_or("the fframes::cli runner does not end with .run()")?;
    Ok(Some(format!(
        "{}.timing_file(\"{TIMING_FILE}\"){}",
        &main[..run],
        &main[run..]
    )))
}

/// `AGENTS.md` with the template's timing section and the current version heading;
/// `None` when it already describes `timeline.json`.
fn add_timing_instructions(instructions: &str) -> Option<String> {
    if instructions.contains(TIMING_FILE) {
        return None;
    }
    let start = TEMPLATE_INSTRUCTIONS.find("Timing:")?;
    let end = start + TEMPLATE_INSTRUCTIONS[start..].find("Studio validation tools:")?;
    let section = &TEMPLATE_INSTRUCTIONS[start..end];
    let current_heading = TEMPLATE_INSTRUCTIONS.lines().next()?;
    let mut text = match instructions.lines().next() {
        Some(heading) if heading.starts_with("# Studio project instructions") => {
            format!("{current_heading}{}", &instructions[heading.len()..])
        }
        _ => instructions.to_owned(),
    };
    match text.find("Studio validation tools:") {
        Some(at) => text.insert_str(at, section),
        None => {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(section);
        }
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framework_pins_change_and_nothing_else_does() {
        let cargo = "[package]\nname = \"video\" # fframes = \"=1.2.0\"\n\n[dependencies]\n# pinned by Studio\nfframes = { version = \"=1.2.0\", features = [\"cli\"] }\nfframes-studio-runtime = \"0.1.0\"\nsha2 = \"0.10\"\n\n[dependencies.fframes-studio-protocol]\nversion = \"=0.1.0\" # exact\n";
        assert_eq!(
            rewrite_framework_pins(cargo, "1.3.0", "0.2.0").unwrap(),
            "[package]\nname = \"video\" # fframes = \"=1.2.0\"\n\n[dependencies]\n# pinned by Studio\nfframes = { version = \"=1.3.0\", features = [\"cli\"] }\nfframes-studio-runtime = \"=0.2.0\"\nsha2 = \"0.10\"\n\n[dependencies.fframes-studio-protocol]\nversion = \"=0.2.0\" # exact\n"
        );
        for refused in [
            "[dependencies]\nfframes = \"=1.2.0\"\nfframes-studio-runtime = \"=0.1.0\"\n",
            "[dependencies]\nfframes = { path = \"../fframes\" }\nfframes-studio-runtime = \"=0.1.0\"\nfframes-studio-protocol = \"=0.1.0\"\n",
            "[dependencies]\nfframes = { workspace = true }\nfframes-studio-runtime = \"=0.1.0\"\nfframes-studio-protocol = \"=0.1.0\"\n",
        ] {
            assert!(
                rewrite_framework_pins(refused, "1.3.0", "0.2.0").is_err(),
                "{refused}"
            );
        }
    }

    #[test]
    fn the_worker_is_regenerated_around_the_projects_video() {
        let constructed = "fn main() {\n    let video = my_video::Film::new()?;\n    let config = PreviewWorkerConfig::new(identity, sdk, \"1.2.0\");\n    serve_preview_worker(&video, &options, transport, config)?;\n    serve_worker(&video, &options, &[], transport, revision, generation)?;\n}\n";
        let worker = regenerate_worker(constructed).unwrap();
        assert!(worker.contains("let video = my_video::Film::new()?;"));
        assert!(worker.contains("print_export_capabilities::<my_video::Film>()"));
        assert!(!worker.contains("studio_video"));
        let unit = "let config = PreviewWorkerConfig::new(identity, sdk, \"1.1.0\");\nserve_preview_worker(&studio_video::StudioVideo, &options, transport, config)?;\n";
        let worker = regenerate_worker(unit).unwrap();
        assert!(worker.contains("let video = studio_video::StudioVideo;"));
        assert!(worker.contains("print_export_capabilities::<studio_video::StudioVideo>()"));
        assert!(TEMPLATE_WORKER.contains(TEMPLATE_VIDEO));
        for refused in [
            "fn main() {}\n",
            "let config = PreviewWorkerConfig::new(i, s, v);\nserve_preview_worker(&build(), &o, t, c)?;\n",
            "let video = a::A::new()?;\nlet video = b::B::new()?;\nlet config = PreviewWorkerConfig::new(i, s, v);\nserve_preview_worker(&video, &o, t, c)?;\n",
        ] {
            assert!(regenerate_worker(refused).is_err(), "{refused}");
        }
    }

    #[test]
    fn the_cli_opts_into_the_timing_document_once() {
        let main = "fn main() {\n    fframes::cli::new(&video, options).run()\n}\n";
        let timed = opt_into_timing(main).unwrap().unwrap();
        assert_eq!(
            timed,
            "fn main() {\n    fframes::cli::new(&video, options).timing_file(\"timeline.json\").run()\n}\n"
        );
        assert_eq!(opt_into_timing(&timed).unwrap(), None);
        assert!(opt_into_timing("fn main() {}\n").is_err());
    }

    #[test]
    fn instructions_gain_the_timing_section_once() {
        let first = "# Studio project instructions — version 1\n\nKeep media inside media/.\nStudio validation tools: run frame.\n";
        let migrated = add_timing_instructions(first).unwrap();
        assert!(migrated.starts_with(TEMPLATE_INSTRUCTIONS.lines().next().unwrap()));
        assert!(migrated.contains("Keep media inside media/.\nTiming:"));
        assert!(migrated.ends_with("Studio validation tools: run frame.\n"));
        assert_eq!(add_timing_instructions(&migrated), None);
        let custom = add_timing_instructions("My own notes").unwrap();
        assert!(custom.starts_with("My own notes\nTiming:"));
    }
}
