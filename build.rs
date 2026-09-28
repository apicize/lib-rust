use std::{
    fs::{copy, create_dir},
    path::Path,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out);
    if !Path::exists(dest_path) {
        create_dir(dest_path).unwrap();
    }

    // Copy JS frameworks (test and setup) to be embedded in V8 snapshots
    for (dir, file_name) in [
        ("script-frameworks/test", "framework.min.js"),
        ("script-frameworks/setup", "setup.min.js"),
    ] {
        let source_path = Path::new(dir).join("dist").join(file_name);
        let dest_file_name = dest_path.join(file_name);
        println!("cargo:rerun-if-changed={}", source_path.to_str().unwrap());
        copy(&source_path, &dest_file_name).unwrap();
    }
}
