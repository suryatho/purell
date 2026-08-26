//! impurell — an ahead-of-time compiler for purell.
//!
//! Pipeline: preprocess (macros, includes) -> parse -> expand macros ->
//! closure-convert -> emit LLVM IR -> llc -> link against the C runtime.

mod ast;
mod codegen;
mod core;
mod lexer;
mod link;
mod preprocessor;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

struct Options {
    input: PathBuf,
    output: Option<PathBuf>,
    work_dir: PathBuf,
    include_paths: Vec<PathBuf>,
    opt_level: u8,
    emit_llvm: bool,
    dump_ast: bool,
    verify: bool,
    run: bool,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }

    let options = match parse_args(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("impurell: {message}");
            return ExitCode::FAILURE;
        }
    };

    match compile(&options) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("impurell: {message}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!("impurell — compile purell to a native executable\n");
    println!("USAGE: impurell [options] <file.pl>\n");
    println!("OPTIONS:");
    println!("  -o <path>       Executable to write (default: build/<name>)");
    println!("  -I <dir>        Add a directory to the include search path");
    println!("  -O<0-3>         Optimization level passed to llc (default: 2)");
    println!("  --emit-llvm     Write the .ll file and stop");
    println!("  --dump-ast      Print each expression's AST after macro expansion");
    println!("  --verify        Run the LLVM verifier over the generated IR");
    println!("  --run           Run the executable after building it");
    println!("  --build-dir <d> Directory for intermediates (default: build)");
    println!("  -h, --help      Show this message\n");
    println!("LANGUAGE:");
    println!("  \\x.body         Lambda; the body extends as far right as it can");
    println!("  f x y           Application, left-associative");
    println!("  42, -7          63-bit integer literals");
    println!("  :name body      Macro definition (expanded at compile time)");
    println!("  @file.pl        Include");
    println!("  # ...           Comment\n");
    println!("PRIMITIVES:");
    println!("  + - * / %       Arithmetic on numbers");
    println!("  < > <= >= = /=  Comparison, returning Church booleans");
    println!("  true false      Church booleans");
    println!("  print           Print a value and return it\n");
    println!("Evaluation is call-by-value, so use the Z combinator (in std/prelude.pl)");
    println!("for recursion; Y diverges under a strict evaluator.");
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut input = None;
    let mut output = None;
    let mut work_dir = PathBuf::from("build");
    let mut include_paths = Vec::new();
    let mut opt_level = 2u8;
    let mut emit_llvm = false;
    let mut dump_ast = false;
    let mut verify = false;
    let mut run = false;

    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        match arg {
            "-o" => {
                index += 1;
                output = Some(PathBuf::from(
                    args.get(index).ok_or("-o needs a path")?,
                ));
            }
            "-I" => {
                index += 1;
                include_paths.push(PathBuf::from(
                    args.get(index).ok_or("-I needs a directory")?,
                ));
            }
            "--build-dir" => {
                index += 1;
                work_dir = PathBuf::from(args.get(index).ok_or("--build-dir needs a path")?);
            }
            "--emit-llvm" => emit_llvm = true,
            "--dump-ast" => dump_ast = true,
            "--verify" => verify = true,
            "--run" => run = true,
            _ if arg.starts_with("-O") => {
                opt_level = arg[2..]
                    .parse()
                    .map_err(|_| format!("bad optimization level '{arg}'"))?;
                if opt_level > 3 {
                    return Err(format!("bad optimization level '{arg}'"));
                }
            }
            _ if arg.starts_with('-') => return Err(format!("unknown option '{arg}'")),
            _ => {
                if input.is_some() {
                    return Err("more than one input file given".to_string());
                }
                input = Some(PathBuf::from(arg));
            }
        }
        index += 1;
    }

    Ok(Options {
        input: input.ok_or("no input file")?,
        output,
        work_dir,
        include_paths,
        opt_level,
        emit_llvm,
        dump_ast,
        verify,
        run,
    })
}

fn compile(options: &Options) -> Result<ExitCode, String> {
    let unit = preprocessor::Preprocessor::new(options.include_paths.clone())
        .load(&options.input)?;

    if unit.exprs.is_empty() {
        return Err(format!("no expressions in {}", options.input.display()));
    }

    let mut converter = core::Converter::new();
    let mut tops = Vec::new();

    for source_expr in &unit.exprs {
        let location = format!(
            "{}:{}",
            source_expr.origin.display(),
            source_expr.line
        );
        let parsed = lexer::parse_expr(&source_expr.source)
            .map_err(|e| format!("{location}: {e}"))?;
        let expanded = unit.expand(&parsed).map_err(|e| format!("{location}: {e}"))?;

        if options.dump_ast {
            eprintln!("{location}: {}", expanded.show());
        }

        tops.push(
            converter
                .convert_top(&source_expr.source, &expanded)
                .map_err(|e| format!("{location}: {e}"))?,
        );
    }

    let program = converter.finish(tops);
    let module_name = options.input.display().to_string();
    let ir = codegen::emit(&program, &module_name);

    std::fs::create_dir_all(&options.work_dir)
        .map_err(|e| format!("cannot create {}: {e}", options.work_dir.display()))?;

    let stem = options
        .input
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "program".to_string());
    let ir_path = options.work_dir.join(format!("{stem}.ll"));
    std::fs::write(&ir_path, &ir).map_err(|e| format!("cannot write {}: {e}", ir_path.display()))?;

    let toolchain = link::Toolchain {
        opt_level: options.opt_level,
        ..Default::default()
    };

    if options.verify || options.emit_llvm {
        toolchain.verify(&ir_path)?;
    }

    if options.emit_llvm {
        println!("wrote {}", ir_path.display());
        return Ok(ExitCode::SUCCESS);
    }

    let exe_path = options
        .output
        .clone()
        .unwrap_or_else(|| link::default_output(&options.input, &options.work_dir));
    toolchain.build(&ir_path, &exe_path, &options.work_dir)?;

    if !options.run {
        println!("wrote {}", exe_path.display());
        return Ok(ExitCode::SUCCESS);
    }

    run_executable(&exe_path)
}

fn run_executable(exe_path: &Path) -> Result<ExitCode, String> {
    // Resolve to an explicit path so this works whether or not `.` is on PATH.
    let absolute = std::fs::canonicalize(exe_path)
        .map_err(|e| format!("cannot find {}: {e}", exe_path.display()))?;
    let status = Command::new(&absolute)
        .status()
        .map_err(|e| format!("cannot run {}: {e}", absolute.display()))?;

    Ok(match status.code() {
        Some(0) => ExitCode::SUCCESS,
        Some(code) => ExitCode::from(code as u8),
        None => ExitCode::FAILURE,
    })
}
