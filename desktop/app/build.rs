fn main() {
    println!("cargo:rerun-if-changed=../packaging/windows/fframes-studio.ico");
    if !std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        return;
    }

    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let icon = manifest.join("../packaging/windows/fframes-studio.ico");
    let resource =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("studio-icon.rc");
    std::fs::write(
        &resource,
        format!(
            "1 ICON \"{}\"\n",
            icon.display().to_string().replace('\\', "/")
        ),
    )
    .expect("write Studio icon resource");
    embed_resource::compile_for(&resource, ["fframes-studio"], embed_resource::NONE)
        .manifest_required()
        .expect("embed Studio application icon");
}
