extern crate bindgen;
extern crate cc;

use std::env;
use std::path::PathBuf;

fn main() {
    cc::Build::new()
        .file("src/bch/bch.c")
        .flag("-Wno-sign-compare")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-stringop-overflow")
        .compile("bch");

    let target = std::env::var("TARGET").unwrap();
    let mut bindings = bindgen::Builder::default()
        .header("src/bch/bch.h")
        .clang_arg(format!("--target={}", target));

    // When LIBCLANG_PATH is set (e.g. nix environments), libclang.so may not locate
    // its own resource directory correctly. Pass -resource-dir explicitly so that
    // libclang can find its builtin headers (stdint.h, etc.).
    if let Ok(libclang_path) = std::env::var("LIBCLANG_PATH") {
        let clang_dir = std::path::Path::new(&libclang_path).join("clang");
        if let Ok(entries) = std::fs::read_dir(&clang_dir) {
            for entry in entries.flatten() {
                let resource_dir = entry.path();
                if resource_dir.join("include").join("stdint.h").exists() {
                    bindings = bindings
                        .clang_arg("-resource-dir")
                        .clang_arg(resource_dir.display().to_string());
                    break;
                }
            }
        }
    }

    let use_std = std::env::var("CARGO_FEATURE_STD").is_ok();
    if !use_std {
        bindings = bindings.use_core();
    }

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
