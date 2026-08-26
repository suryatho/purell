// Compile the C runtime to an object file at build time. The object is
// embedded in the compiler binary (see src/link.rs) so a built `impurell` is
// self-contained and can link programs from any directory.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Build runtime object file on change
    println!("cargo:rerun-if-changed=runtime/imprt.c");
    println!("cargo:rerun-if-changed=runtime/imprt.h");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let obj = out_dir.join("imprt.o");
    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());

    let status = Command::new(&cc)
        .args(["-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", "-c"])
        .arg("runtime/imprt.c")
        .arg("-o")
        .arg(&obj)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {cc}: {e}"));

    if !status.success() {
        panic!("failed to compile runtime/imprt.c with {cc}");
    }
}
