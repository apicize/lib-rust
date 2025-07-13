use std::{
    fs::{copy, create_dir},
    path::Path,
};

fn main() {
    let out = std::env::var("OUT_DIR").unwrap();
    let source_path = Path::new("test-framework").join("dist").join("framework.min.js");
    let dest_path = Path::new(&out);
    let dest_file_name = Path::new(&dest_path).join("framework.min.js");
    // println!("Copy framework file {} to {}", source_path.to_str().unwrap(), dest_file_name.to_str().unwrap());
    if !Path::exists(dest_path) {
        create_dir(dest_path).unwrap();
    }
    copy(&source_path, &dest_file_name).unwrap();
}