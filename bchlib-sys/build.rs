extern crate bindgen;
extern crate cc;

use std::env;
use std::path::PathBuf;

fn main() {
    let use_std = std::env::var("CARGO_FEATURE_STD").is_ok();
    let use_malloc = std::env::var("CARGO_FEATURE_MALLOC").is_ok();
    let use_decode = std::env::var("CARGO_FEATURE_DECODE").is_ok();

    let mut build = cc::Build::new();
    build
        .file("src/bch/bch.c")
        .flag("-Wno-sign-compare")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-stringop-overflow");
    if use_malloc {
        // use the system allocator instead of the small fixed-size static pool
        // meant for no_std/embedded targets.
        build.define("BCH_USE_MALLOC", None);
    }
    if use_decode {
        // compile in decode_bch/correct_bch and their supporting tables;
        // disable for encode-only, memory-constrained no_std targets.
        build.define("BCH_DECODE", None);
    }
    build.compile("bch");

    let target = std::env::var("TARGET").unwrap();
    let mut bindings = bindgen::Builder::default()
        .header("src/bch/bch.h")
        .clang_arg(format!("--target={}", target));

    if use_decode {
        // must match the define passed to the C build above, or bindgen
        // and the compiled C object disagree on which declarations in
        // bch.h exist (decode_bch et al. are #ifdef BCH_DECODE there too).
        bindings = bindings.clang_arg("-DBCH_DECODE");
    }

    // When LIBCLANG_PATH is set (e.g. nix environments where libclang is
    // split out from the rest of the clang installation), libclang.so may
    // not locate its own resource directory correctly. Ask an actual clang
    // binary directly via `-print-resource-dir` (the officially supported
    // way to query this) rather than guessing the layout of whatever
    // packaged libclang.so we happen to be linked against.
    if std::env::var("LIBCLANG_PATH").is_ok() {
        let clang_bin = std::env::var("CLANG_PATH")
            .or_else(|_| std::env::var("CC"))
            .unwrap_or_else(|_| "clang".to_string());
        if let Ok(output) = std::process::Command::new(&clang_bin)
            .arg("-print-resource-dir")
            .output()
        {
            if output.status.success() {
                let resource_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !resource_dir.is_empty() {
                    bindings = bindings.clang_arg("-resource-dir").clang_arg(resource_dir);
                }
            }
        }
    }

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
