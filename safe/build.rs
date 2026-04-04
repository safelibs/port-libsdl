use std::env;
use std::fs;
use std::path::PathBuf;

use cc::Build;
use serde::Deserialize;

#[derive(Deserialize)]
struct LinuxSymbolManifest {
    soname: String,
    symbols: Vec<LinuxSymbolEntry>,
}

#[derive(Deserialize)]
struct LinuxSymbolEntry {
    name: String,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let symbol_manifest_path = manifest_dir.join("generated/linux_symbol_manifest.json");
    let variadic_shims_path = manifest_dir.join("src/core/phase2_variadic_shims.c");
    println!("cargo:rerun-if-changed={}", symbol_manifest_path.display());
    println!("cargo:rerun-if-changed={}", variadic_shims_path.display());

    let symbol_manifest: LinuxSymbolManifest = serde_json::from_slice(
        &fs::read(&symbol_manifest_path).expect("read linux_symbol_manifest.json"),
    )
    .expect("parse linux_symbol_manifest.json");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let version_script_path = out_dir.join("libSDL2.version.map");
    fs::write(
        &version_script_path,
        render_version_script(&symbol_manifest),
    )
    .expect("write version script");

    link_cdylib_arg(format!(
        "-Wl,--version-script={}",
        version_script_path.display()
    ));
    link_cdylib_arg(format!("-Wl,--soname,{}", symbol_manifest.soname));
    println!("cargo:rustc-link-lib=dylib=dl");

    Build::new()
        .file(&variadic_shims_path)
        .flag_if_supported("-std=c11")
        .warnings(false)
        .compile("safe_sdl_phase2_variadics");
}

fn render_version_script(manifest: &LinuxSymbolManifest) -> String {
    let mut script = String::from("Base {\n  global:\n");
    for symbol in &manifest.symbols {
        script.push_str("    ");
        script.push_str(&symbol.name);
        script.push_str(";\n");
    }
    script.push_str("  local:\n    *;\n};\n");
    script
}

fn link_cdylib_arg(arg: String) {
    println!("cargo:rustc-link-arg-cdylib={arg}");
}
