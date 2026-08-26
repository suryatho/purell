//! Drive the external toolchain: LLVM IR -> object -> linked executable.
//!
//! The C runtime is compiled once by build.rs and embedded here, so a built
//! `impurell` needs nothing from its source tree at run time.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const RUNTIME_OBJECT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/imprt.o"));

pub struct Toolchain {
    pub llc: String,
    pub linker: String,
    pub opt_level: u8,
}

impl Default for Toolchain {
    fn default() -> Self {
        Toolchain {
            llc: std::env::var("IMPURELL_LLC").unwrap_or_else(|_| "llc".to_string()),
            linker: std::env::var("IMPURELL_CC").unwrap_or_else(|_| "cc".to_string()),
            opt_level: 2,
        }
    }
}

impl Toolchain {
    /// Assemble `ir_path` and link it with the runtime into `exe_path`.
    pub fn build(&self, ir_path: &Path, exe_path: &Path, work_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(work_dir)
            .map_err(|e| format!("cannot create {}: {e}", work_dir.display()))?;

        let stem = ir_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "program".to_string());

        let program_obj = work_dir.join(format!("{stem}.o"));
        let runtime_obj = work_dir.join("imprt.o");

        fs::write(&runtime_obj, RUNTIME_OBJECT)
            .map_err(|e| format!("cannot write {}: {e}", runtime_obj.display()))?;

        run(
            &self.llc,
            &[
                format!("-O{}", self.opt_level),
                "--filetype=obj".to_string(),
                ir_path.display().to_string(),
                "-o".to_string(),
                program_obj.display().to_string(),
            ],
        )?;

        run(
            &self.linker,
            &[
                program_obj.display().to_string(),
                runtime_obj.display().to_string(),
                "-o".to_string(),
                exe_path.display().to_string(),
            ],
        )?;

        Ok(())
    }

    /// Run the LLVM verifier over the generated IR. Used by `--verify` and the
    /// test suite to catch malformed output before it reaches llc.
    pub fn verify(&self, ir_path: &Path) -> Result<(), String> {
        let opt = std::env::var("IMPURELL_OPT").unwrap_or_else(|_| "opt".to_string());
        run(
            &opt,
            &[
                "-passes=verify".to_string(),
                "-disable-output".to_string(),
                ir_path.display().to_string(),
            ],
        )
    }
}

fn run(program: &str, args: &[String]) -> Result<(), String> {
    let output = Command::new(program).args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("could not find '{program}' on PATH ({e})")
        } else {
            format!("could not run '{program}': {e}")
        }
    })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "{program} failed ({}):\n{}",
        output.status,
        stderr.trim()
    ))
}

/// Default executable path for an input file: the input with its extension
/// dropped, placed in `work_dir`.
pub fn default_output(input: &Path, work_dir: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "program".to_string());
    work_dir.join(stem)
}
