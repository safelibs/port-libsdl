use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let shim = manifest_dir.join("src/variadic_shims.c");
    println!("cargo:rerun-if-changed={}", shim.display());

    cc::Build::new()
        .file(&shim)
        .flag_if_supported("-std=c11")
        .warnings(false)
        .compile("safe_sdl2_test_variadics");
}
