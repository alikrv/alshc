use std::process::Command;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let stdlib_path = PathBuf::from(manifest_dir).join("alsh-std").join("impl").join("rust");

    // Rerun if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");
    // Rerun if any alsh-std source changes
    println!("cargo:rerun-if-changed=alsh-std/impl/rust/src");

    // Build the alsh-std library
    let status = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir(&stdlib_path)
        .status()
        .expect("Failed to build alsh-std");

    if !status.success() {
        panic!("Failed to build alsh-std library");
    }

    // Link the built library
    let stdlib_lib_path = stdlib_path.join("target").join("release");
    println!("cargo:rustc-link-search=native={}", stdlib_lib_path.display());
    println!("cargo:rustc-link-lib=static=alsh_std");

    // Add the preprocessor static lib directory to the linker search path
    println!("cargo:rustc-link-search=native={}/alshpp", manifest_dir);
    // Link statically with libalshpp.a
    println!("cargo:rustc-link-lib=static=alshpp");

    // Ensure alshpp static library exists by invoking its Makefile to build the library
    let alshpp_dir = PathBuf::from(manifest_dir).join("alshpp");
    let make_status = Command::new("make")
        .args(&["lib"])
        .current_dir(&alshpp_dir)
        .status()
        .expect("Failed to run make in alshpp directory");

    if !make_status.success() {
        panic!("Failed to build alshpp static library");
    }
}
