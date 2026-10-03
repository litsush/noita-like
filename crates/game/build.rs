//! Copies the Steamworks redistributable (built by steamworks-sys) next to
//! the game binary and points the binary's rpath at its own directory, so
//! `target/<profile>/sbct` runs without `cargo run` or LD_LIBRARY_PATH.

use std::path::{Path, PathBuf};
use std::{env, fs};

const LIBS: [&str; 3] = ["libsteam_api.so", "libsteam_api.dylib", "steam_api64.dll"];

fn main() {
    let target = env::var("TARGET").unwrap();
    if target.contains("linux") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN");
    } else if target.contains("darwin") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,@executable_path");
    }

    // OUT_DIR is target/<profile>/build/sbct-<hash>/out.
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out.ancestors().nth(2).unwrap();
    let profile_dir = out.ancestors().nth(3).unwrap();

    let Some(lib) = newest_steam_lib(build_dir) else {
        println!("cargo:warning=libsteam_api not found; run via `cargo run` or copy it next to the binary");
        return;
    };
    let dest = profile_dir.join(lib.file_name().unwrap());
    if let Err(e) = fs::copy(&lib, &dest) {
        println!("cargo:warning=failed to copy {}: {e}", lib.display());
    }
    println!("cargo:rerun-if-changed={}", lib.display());
}

fn newest_steam_lib(build_dir: &Path) -> Option<PathBuf> {
    fs::read_dir(build_dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("steamworks-sys-"))
        .flat_map(|e| LIBS.map(|l| e.path().join("out").join(l)))
        .filter(|p| p.exists())
        .max_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
}
