use std::{path::Path, process::Command};

fn main() {
    let node_modules = Path::new("./test-framework/node_modules");
    if !node_modules.exists() {
        let result_install = Command::new("npm")
            .current_dir("./test-framework")
            .args(["install"])
            .status();

        match result_install {
            Ok(status) => {
                if !status.success() {
                    panic!("Unable to initialize NodeJS");
                }
            }
            Err(err) => {
                panic!("Error initilalizing NodeJS: {}", err);
            }
        }
    }

    let result_build = Command::new("npm")
        .current_dir("./test-framework")
        .args(["run", "build"])
        .status();

    match result_build {
        Ok(status) => {
            if !status.success() {
                panic!("Unable to build test framework JavaScript");
            }
        }
        Err(err) => {
            panic!("Error building test framework JavaScript: {}", err);
        }
    }

    println!("cargo::rerun-if-changed=test-framework");
}
