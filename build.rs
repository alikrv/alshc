fn main() {
    // Add the preprocessor static lib directory to the linker search path
    println!("cargo:rustc-link-search=native={}/alshpp", env!("CARGO_MANIFEST_DIR"));
    // Link statically with libalshpp.a
    println!("cargo:rustc-link-lib=static=alshpp");
}
