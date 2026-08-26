//! End-to-end tests: compile a program, link it, run it, check its output.
//!
//! These need `llc` and a C compiler on PATH. Override with IMPURELL_LLC /
//! IMPURELL_CC if they live somewhere unusual (Homebrew LLVM, for instance,
//! installs to /opt/homebrew/opt/llvm/bin).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const COMPILER: &str = env!("CARGO_BIN_EXE_impurell");

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A private build directory per test, so parallel tests do not race on
/// intermediate object files.
fn scratch(name: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = repo_root()
        .join("target")
        .join("test-build")
        .join(format!("{name}-{id}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Compile `source` and return everything it printed to stdout.
fn run_source(name: &str, source: &str) -> String {
    let dir = scratch(name);
    let file = dir.join(format!("{name}.pl"));
    std::fs::write(&file, source).expect("write source");
    compile_and_run(&file, &dir)
}

fn compile_and_run(file: &Path, dir: &Path) -> String {
    let output = Command::new(COMPILER)
        .arg(file)
        .arg("--run")
        .arg("--verify")
        .args(["-I", repo_root().join("std").to_str().unwrap()])
        .args(["--build-dir", dir.to_str().unwrap()])
        .output()
        .expect("run impurell");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        output.status.success(),
        "compiling {} failed ({})\nstdout:\n{stdout}\nstderr:\n{stderr}",
        file.display(),
        output.status
    );
    stdout
}

/// Compile a program expected to fail, and return the diagnostic.
fn expect_failure(name: &str, source: &str) -> String {
    let dir = scratch(name);
    let file = dir.join(format!("{name}.pl"));
    std::fs::write(&file, source).expect("write source");

    let output = Command::new(COMPILER)
        .arg(&file)
        .arg("--run")
        .args(["-I", repo_root().join("std").to_str().unwrap()])
        .args(["--build-dir", dir.to_str().unwrap()])
        .output()
        .expect("run impurell");

    assert!(
        !output.status.success(),
        "expected {} to fail, but it succeeded",
        file.display()
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Pull the `Result:` lines out of a program's output, in order.
fn results(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix("Result: "))
        .map(str::to_string)
        .collect()
}

#[test]
fn arithmetic_and_application() {
    let out = run_source(
        "arith",
        "(\\x.\\y.+ x y) 3 5\n\n(\\x.x) 42\n\n(\\x.\\y.\\z.+ x (+ y z)) 1 2 3\n",
    );
    assert_eq!(results(&out), ["8", "42", "6"]);
}

#[test]
fn all_arithmetic_primitives() {
    let out = run_source(
        "prims",
        "+ 7 3\n\n- 7 3\n\n* 7 3\n\n/ 7 3\n\n% 7 3\n\n- 0 7\n\n+ 1 -5\n",
    );
    assert_eq!(results(&out), ["10", "4", "21", "2", "1", "-7", "-4"]);
}

#[test]
fn comparisons_return_church_booleans() {
    // A Church boolean applied to two values selects one of them, which is how
    // we observe the result as a number.
    let out = run_source(
        "cmp",
        "< 1 2 10 20\n\n< 2 1 10 20\n\n= 3 3 10 20\n\n/= 3 3 10 20\n\n>= 3 3 10 20\n",
    );
    assert_eq!(results(&out), ["10", "20", "10", "20", "10"]);
}

#[test]
fn closures_outlive_the_frame_that_built_them() {
    // The whole point of heap closures: `add3` and `add100` hold different
    // captures at the same time, after `makeAdder` has returned.
    let out = run_source(
        "closures",
        ":makeAdder \\x.\\y.+ x y\n\n\
         (\\a.\\b.+ (b 1) (a 1)) (makeAdder 3) (makeAdder 100)\n\n\
         (\\f.+ (f 1) (f 2)) (makeAdder 1000)\n",
    );
    assert_eq!(results(&out), ["105", "2003"]);
}

#[test]
fn deeply_nested_captures_are_threaded_through() {
    let out = run_source("nested", "(\\a.(\\b.(\\c.(\\d.+ a (+ b (+ c d))) 4) 3) 2) 1\n");
    assert_eq!(results(&out), ["10"]);
}

#[test]
fn recursion_through_the_z_combinator() {
    let out = run_source(
        "recursion",
        "@prelude.pl\n\n\
         :fact Z (\\rec.\\n.if (<= n 1) (\\_.1) (\\_.* n (rec (- n 1))))\n\n\
         fact 20\n\n\
         :fib Z (\\rec.\\n.if (< n 2) (\\_.n) (\\_.+ (rec (- n 1)) (rec (- n 2))))\n\n\
         fib 20\n\n\
         :gcd Z (\\rec.\\a.\\b.if (= b 0) (\\_.a) (\\_.rec b (% a b)))\n\n\
         gcd 462 1071\n",
    );
    assert_eq!(results(&out), ["2432902008176640000", "6765", "21"]);
}

#[test]
fn tail_recursion_runs_in_constant_stack() {
    // Five million tail calls. Without musttail this blows the stack; the
    // point of this test is that it does not.
    let out = run_source(
        "tailcalls",
        "@prelude.pl\n\n\
         Z (\\rec.\\k.\\acc.if (= k 0) (\\_.acc) (\\_.rec (- k 1) (+ acc k))) 5000000 0\n",
    );
    assert_eq!(results(&out), ["12500002500000"]);
}

#[test]
fn church_numerals_match_native_arithmetic() {
    // Pure lambda calculus with no primitives except the final conversion,
    // cross-checking the compiler against the interpreter's semantics.
    let out = run_source(
        "church",
        ":zero \\f.\\x.x\n\
         :succ \\n.\\f.\\x.f (n f x)\n\
         :one succ zero\n\
         :two succ one\n\
         :three succ two\n\
         :four succ three\n\
         :add \\m.\\n.\\f.\\x.m f (n f x)\n\
         :mul \\m.\\n.\\f.m (n f)\n\
         :exp \\m.\\n.n m\n\
         :pred \\n.\\f.\\x.n (\\g.\\h.h (g f)) (\\u.x) (\\u.u)\n\
         :sub \\m.\\n.n pred m\n\
         :toInt \\n.n (\\x.+ x 1) 0\n\n\
         toInt (add two three)\n\n\
         toInt (mul two three)\n\n\
         toInt (exp two three)\n\n\
         toInt (sub (mul three three) four)\n\n\
         toInt (pred (pred four))\n",
    );
    assert_eq!(results(&out), ["5", "6", "8", "5", "2"]);
}

#[test]
fn sixty_three_bit_range_round_trips() {
    let out = run_source(
        "range",
        "4611686018427387903\n\n-4611686018427387904\n\n+ 4611686018427387902 1\n",
    );
    assert_eq!(
        results(&out),
        [
            "4611686018427387903",
            "-4611686018427387904",
            "4611686018427387903"
        ]
    );
}

#[test]
fn functions_print_as_functions() {
    let out = run_source("showfn", "\\x.x\n\n+ 1\n");
    assert_eq!(results(&out), ["<function>", "<function>"]);
}

#[test]
fn shipped_examples_all_run() {
    for name in ["basic", "closures", "recursion"] {
        let path = repo_root().join("examples").join(format!("{name}.pl"));
        let dir = scratch(name);
        let out = compile_and_run(&path, &dir);
        assert!(
            out.contains("Result: "),
            "example {name} produced no results:\n{out}"
        );
    }
}

// --- Diagnostics ------------------------------------------------------------

#[test]
fn applying_a_number_is_a_runtime_error() {
    let err = expect_failure("applynum", "5 3\n");
    assert!(err.contains("applied a non-function: 5"), "{err}");
}

#[test]
fn arithmetic_on_a_function_is_a_runtime_error() {
    let err = expect_failure("typeerr", "+ 1 (\\x.x)\n");
    assert!(err.contains("+ expected a number, got a function"), "{err}");
}

#[test]
fn division_by_zero_is_reported() {
    assert!(expect_failure("divzero", "/ 7 0\n").contains("/ by zero"));
    assert!(expect_failure("remzero", "% 7 0\n").contains("% by zero"));
}

#[test]
fn unbound_variables_are_caught_at_compile_time() {
    let err = expect_failure("unbound", "\\x.y\n");
    assert!(err.contains("unbound variable 'y'"), "{err}");
}

#[test]
fn self_referential_macros_are_caught_at_compile_time() {
    let err = expect_failure("recmacro", ":loop \\x.loop x\n\nloop 1\n");
    assert!(err.contains("expands into itself"), "{err}");
}

#[test]
fn oversized_literals_are_rejected() {
    let err = expect_failure("toobig", "4611686018427387904\n");
    assert!(err.contains("does not fit in 63 bits"), "{err}");
}
