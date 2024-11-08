use std::{
    fs::{copy, create_dir},
    path::Path,
};

fn main() {
    
    let source_path = Path::new("test-framework").join("dist").join("framework.min.js");
    let dest_path = Path::new(std::env::var("OUT_DIR").unwrap().as_str()).join("test-framework");
    
    println!("Copy framework file {} to {}", source_path.to_str().unwrap(), dest_path.to_str().unwrap());
    if !Path::exists(&dest_path) {
        create_dir(&dest_path).unwrap();
    }
    copy(
        source_path,
        Path::new(&dest_path).join("framework.min.js")
    )
    .unwrap();
}