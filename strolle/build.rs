use std::error::Error;
use std::path::PathBuf;
use std::{env, process};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../strolle-shader-builder/Cargo.toml");
    println!("cargo:rerun-if-changed=../strolle-shader-builder/src/main.rs");

    let profile = env::var("PROFILE").unwrap();
    println!("cargo:rustc-env=PROFILE={profile}");

    let mut dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Strip `$profile/build/*/out`.
    let ok = dir.ends_with("out")
        && dir.pop()
        && dir.pop()
        && dir.ends_with("build")
        && dir.pop()
        && dir.ends_with(profile)
        && dir.pop();

    assert!(ok);

    let dir = dir.join("shader-builder");

    let status = process::Command::new("cargo")
        .args([
            "run",
            "--release",
            "-p",
            "strolle-shader-builder",
            "--target-dir",
        ])
        .arg(dir)
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .stderr(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .status()?;

    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
