use std::process::Command;

fn main() {
    let result = Command::new("npm")
        .current_dir("./src/static/src")
        .args(&["run", "build"])
        .status();

    match result {
        Ok(status) => {
            if ! status.success() {
                panic!("Unable to build framework JavaScript");
            }
        },
        Err(err) => {
            panic!("Error building framework JavaScript: {}", err);
        }
    }

    println!("cargo::rerun-if-changed=src/static");
}
