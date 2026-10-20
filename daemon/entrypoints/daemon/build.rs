use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=PXNA_BUILD_VERSION");
    let version = env::var("PXNA_BUILD_VERSION")
        .ok()
        .filter(|version| !version.trim().is_empty())
        .or_else(|| {
            let output = Command::new("git").args(["describe", "--tags", "--always"]).output().ok()?;
            output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        })
        .unwrap_or_else(|| env::var("CARGO_PKG_VERSION").expect("Cargo provides package version"));
    println!("cargo:rustc-env=GIT_TAG={version}");
}
