//! Correct the pinned adapter's ignored advisor result, before publishing its install.
//! Both input and output hashes are pinned: a changed npm artifact fails closed.
use std::path::Path;

use sha2::{Digest, Sha256};

use super::{AdapterPackage, InstallError, io};

pub(super) const REVISION: &str = "advisor-result-v1";
const ORIGINAL: &str = "f00fea811d7144360a6e70bc87e38e36d00a807d15825a8179e2458520b71784";
const PATCHED: &str = "8d90e02099d7d1968225896341271406da20c935e63973af1d874af2bbe877ad";
const HANDLER: &str = include_str!("../../adapters/claude/advisor-result.js");

pub(super) fn applies(package: &AdapterPackage) -> bool {
    package.name == "@agentclientprotocol/claude-agent-acp" && package.version == "0.89.1"
}

fn path(root: &Path) -> std::path::PathBuf {
    root.join("node_modules/@agentclientprotocol/claude-agent-acp/dist/acp-agent.js")
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn installed(root: &Path, package: &AdapterPackage) -> bool {
    !applies(package) || std::fs::read(path(root)).is_ok_and(|bytes| hash(&bytes) == PATCHED)
}

pub(super) fn apply(root: &Path, package: &AdapterPackage) -> Result<(), InstallError> {
    if !applies(package) {
        return Ok(());
    }
    let path = path(root);
    let source = std::fs::read_to_string(&path).map_err(io)?;
    let unexpected = || {
        InstallError::Invalid(
            "The pinned Claude ACP advisor patch does not match the installed artifact.".into(),
        )
    };
    if hash(source.as_bytes()) != ORIGINAL {
        return Err(unexpected());
    }
    let patched = source
        .replace("            case \"advisor_tool_result\":\n", "")
        .replace(
            "            case \"document\":\n",
            &format!(
                "{}            case \"document\":\n",
                HANDLER.replace("\r\n", "\n")
            ),
        );
    if hash(patched.as_bytes()) != PATCHED {
        return Err(unexpected());
    }
    std::fs::write(path, patched).map_err(io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_adapter_artifacts_are_refused_without_writing() {
        let temp = tempfile::tempdir().unwrap();
        let path = path(temp.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = b"unexpected upstream build";
        std::fs::write(&path, bytes).unwrap();
        assert!(apply(temp.path(), &super::super::CLAUDE_ADAPTER).is_err());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        assert!(!installed(temp.path(), &super::super::CLAUDE_ADAPTER));
    }
}
