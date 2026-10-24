//! The kit's `SkSL` compiles with the Skia backend's compiler, which logs a broken shader once
//! and draws nothing in its place.
#![cfg(all(feature = "skia", not(target_arch = "wasm32")))]

use fframes::Shader;
use fframes_kit::effects::{GRAIN_SKSL, grain_shader};
use fframes_skia_renderer::render::compile_shader;

#[test]
fn the_grain_shader_compiles() {
    let effect = compile_shader(grain_shader()).expect("the grain shader compiles");
    let names: Vec<String> = effect
        .uniforms()
        .iter()
        .map(|u| u.name().to_owned())
        .collect();
    for uniform in ["uSeed", "uScale", "uOpacity"] {
        assert!(names.iter().any(|n| n == uniform), "{uniform} in {names:?}");
    }
}

#[test]
fn a_broken_shader_is_reported() {
    let broken = Shader::sksl(GRAIN_SKSL.replace("fract(p.x * p.y)", "fract(p.x * q)"));
    assert!(compile_shader(&broken).is_err());
}
