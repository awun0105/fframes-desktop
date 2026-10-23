//! The SVG cache: one `.svg` file per typeset document, named by a hash of the input.

use std::path::{Path, PathBuf};

use super::TexTemplate;

/// Changes when the kit changes how it builds documents or reads SVGs, so old files are
/// not used.
const CACHE_VERSION: &str = "fframes-kit-tex-1";

/// 64-bit FNV-1a: the same value on every platform and Rust version, unlike `std`'s hasher.
fn fnv1a(parts: &[&str]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for part in parts {
        for byte in part.bytes().chain(std::iter::once(0)) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

/// The cache file for `document` typeset with `template`'s compiler.
pub(crate) fn cache_file(dir: &Path, template: &TexTemplate, document: &str) -> PathBuf {
    let key = fnv1a(&[CACHE_VERSION, template.compiler.program(), document]);
    dir.join(format!("{key:016x}.svg"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_depends_on_every_input() {
        let dir = Path::new("cache");
        let t = TexTemplate::default();
        let doc = t.document("x");
        let a = cache_file(dir, &t, &doc);
        assert_eq!(a, cache_file(dir, &t, &doc));
        assert_ne!(a, cache_file(dir, &t, &t.document("y")));
        let xe = t.clone().with_compiler(super::super::TexCompiler::XeLatex);
        assert_ne!(a, cache_file(dir, &xe, &doc));
        // fixed: a cache committed on one machine is found on every other
        assert_eq!(fnv1a(&["a"]), 0x089b_e207_b544_f1e4);
    }
}
