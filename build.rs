use std::{fs::{copy, create_dir, remove_dir_all}, path::Path, process::Command};

fn main() {
    let framework_dir = Path::new("./test-framework");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_framework_dir = Path::join(Path::new(&out_dir), "test-framework");
    let out_node_modules_dir = Path::join(Path::new(&out_framework_dir), "node_modules");

    if ! Path::exists(&out_framework_dir) {
        create_dir(&out_framework_dir).unwrap();
    }

    copy(Path::join(framework_dir, "index.js"), Path::join(&out_framework_dir, "index.js")).unwrap();
    copy(Path::join(framework_dir, "package.json"), Path::join(&out_framework_dir, "package.json")).unwrap();
    copy(Path::join(framework_dir, "package-lock.json"), Path::join(&out_framework_dir, "package-lock.json")).unwrap();
    copy(Path::join(framework_dir, "webpack.config.js"), Path::join(&out_framework_dir, "webpack.config.js")).unwrap();

    if !out_node_modules_dir.exists() {
        let result_install = Command::new("yarn")
            .current_dir(&out_framework_dir)
            .args(["install"])
            .status();

        match result_install {
            Ok(status) => {
                if !status.success() {
                    panic!("yarn install failed");
                }
            }
            Err(err) => {
                panic!("unable to execute yarn install: {}", err);
            }
        }
    }

    let result_build = Command::new("yarn")
        .current_dir(&out_framework_dir)
        .args(["build"])
        .status();

    match result_build {
        Ok(status) => {
            if !status.success() {
                panic!("yarn build failed");
            }
        }
        Err(err) => {
            panic!("unable to execute yarn build: {}", err);
        }
    }

    remove_dir_all(&out_framework_dir).unwrap();

    println!("cargo::rerun-if-changed=test-framework");
}
