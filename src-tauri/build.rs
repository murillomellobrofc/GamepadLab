fn main() {
    tauri_build::build();

    // tauri-build embeds the Common Controls v6 manifest only into the app binary. Integration
    // tests (tests/ipc.rs) link the same dialog code, and without the manifest Windows loads
    // comctl32 v5, which lacks TaskDialogIndirect: the test exe dies with
    // STATUS_ENTRYPOINT_NOT_FOUND before running. Give test targets the same manifest.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-test-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
