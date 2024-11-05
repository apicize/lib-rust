use std::{path::Path, process::Command};

fn main() {
    let node_modules = Path::new("./src/static/src/node_modules");
    if !node_modules.exists() {
        let result_install = Command::new("npm")
            .current_dir("./src/static/src")
            .args(&["install"])
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
        .current_dir("./src/static/src")
        .args(&["run", "build"])
        .status();

    match result_build {
        Ok(status) => {
            if !status.success() {
                panic!("Unable to build framework JavaScript");
            }
        }
        Err(err) => {
            panic!("Error building framework JavaScript: {}", err);
        }
    }

    println!("cargo::rerun-if-changed=src/static");
}
