use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let crate_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out_file = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bindings.rs");
    let allowlist = fs::read_to_string(crate_dir.join("allowlist.txt")).unwrap();

    let mut command = Command::new("bindgen");
    command
        .arg(crate_dir.join("wrapper.h"))
        .arg("--output")
        .arg(&out_file);

    for pattern in allowlist
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        command.arg("--allowlist-item").arg(pattern);
    }

    let status = command.status().expect("failed to invoke bindgen");
    assert!(status.success(), "bindgen failed");

    println!(
        "cargo:rerun-if-changed={}",
        crate_dir.join("wrapper.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        crate_dir.join("allowlist.txt").display()
    );
    println!("cargo:rustc-link-lib=c");
}
