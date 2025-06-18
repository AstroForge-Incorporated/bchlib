extern crate bindgen;
extern crate cc;

use std::env;
use std::path::PathBuf;

fn main() {
    cc::Build::new().file("src/bch/bch.c").compile("bch");

    let mut bindings = bindgen::Builder::default()
        .header("src/bch/bch.h");

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
