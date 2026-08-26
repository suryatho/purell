# purell

**purell** is a small untyped lambda calculus. This repository contains two
independent implementations of it:

| | | |
|---|---|---|
| [`thepurell/`](thepurell) | **interpreter** | Reduces terms to normal form and prints the resulting term. Has a REPL. |
| [`impurell/`](impurell) | **compiler** | Compiles to LLVM IR, assembles with `llc`, links against a C runtime, produces a native executable. Adds 63-bit machine integers. |

They implement the same lambda calculus and agree on pure-lambda programs, but
they make deliberately different trade-offs — see
[Interpreter vs. compiler](#interpreter-vs-compiler).

```
purell/
├── thepurell/       the interpreter (Rust, no dependencies)
├── impurell/        the compiler    (Rust, no dependencies)
│   ├── src/         compiler passes
│   ├── runtime/     the C runtime linked into every compiled program
│   ├── std/         prelude
│   ├── examples/
│   ├── tests/       end-to-end tests: compile, link, run, check output
│   └── sketch/      the original hand-written .ll and C prototypes
└── README.md
```

---

## The language

Untyped lambda calculus, plus a preprocessor.

```
\x.body           lambda ("λx.body"); the body extends as far right as it can
f x y             application, left-associative — this is ((f x) y)
(...)             grouping
:name body        macro definition
@file.pl          include another file
# ...             comment, to end of line
```

Expressions are separated by **blank lines**; consecutive non-blank lines are
joined with spaces, so a long expression can be wrapped freely.

```purell
@prelude.pl

# The body extends right, so this is \x.(\y.(+ x y))
:add \x.\y.+ x y

add 3 5
```

`impurell` additionally has **63-bit integer literals** (`42`, `-7`) and these
primitives:

```
+  -  *  /  %             arithmetic
<  >  <=  >=  =  /=       comparison, returning Church booleans
true  false               Church booleans
print                     print a value, return it
```

Comparisons returning Church booleans is what lets native numbers and pure
lambda terms mix: `< 1 2` evaluates to `\x.\y.x`, so `< 1 2 10 20` is `10`.

An unbound variable is a **compile error** in `impurell`. In `thepurell` it is
simply a free variable, which is the mathematically correct reading — normal
order reduction leaves it alone.

---

## impurell — the compiler

### Pipeline

```
  file.pl
    │
    ├─ preprocessor.rs   strip comments, resolve @includes, collect :macros,
    │                    split into blank-line-separated expressions
    ├─ lexer.rs          tokenize + parse  ────────────────►  ast.rs (Expr)
    ├─ preprocessor.rs   expand macros into the AST (eagerly, cycle-checked)
    ├─ core.rs           closure conversion  ──────────────►  core.rs (Program)
    ├─ codegen.rs        emit textual LLVM IR             ►  build/file.ll
    └─ link.rs           llc → file.o,  cc file.o imprt.o → executable
```

### Value representation — pointer tagging

This is the core design decision, and everything else follows from it.

A purell value is **exactly one 64-bit word**. Bit 0 says which kind it is:

```
  bit 0 = 1     immediate integer, payload in the upper 63 bits:  (n << 1) | 1
  bit 0 = 0     pointer to a heap closure
```

Bit 0 is available because closures are `malloc`-backed and therefore 8-byte
aligned — their low three bits are always zero. Numbers deliberately set the
one bit a pointer can never have set.

Consequences:

- **Numbers never allocate.** `42` compiles to the literal constant `85`. It
  lives in a register; there is no box and no dereference.
- **Integers are 63-bit**, range `-2^62 .. 2^62-1`
  (`-4611686018427387904 .. 4611686018427387903`). The parser rejects literals
  outside it rather than letting them corrupt the tag. Arithmetic wraps inside
  the payload; there is no overflow trap.
- **Every application must check the tag**, because applying `5` to something
  is a type error only discoverable at runtime in an untyped language.

The tag is read in exactly three places: at every application, in the
arithmetic primitives (before untagging their operands), and in `imp_show`
(to decide between printing `42` and `<function>`).

### Closures

A closure is three or more consecutive words:

```
  word 0    function pointer     i64 (ptr self, i64 arg)
  word 1    capture count
  word 2..  captured values
```

Every compiled lambda has the same signature, `(self, arg) -> value`, where
`self` is the closure it was reached through. A captured variable is a load
from `self` at a fixed offset.

> **Why not a global capture stack?** The original prototype in
> [`sketch/test.ll`](impurell/sketch/test.ll) pushed captures onto a global
> stack and returned a bare function pointer. That works only if closures are
> consumed immediately, in order. It breaks as soon as one *escapes* — two
> partial applications alive at once read each other's captures. Heap closures
> are what make `(\f.+ (f 1) (f 2)) (makeAdder 1000)` produce `2003`.

### Closure conversion (`core.rs`)

Lambda lifting: each `\x.body` becomes a top-level LLVM function plus an
allocation site.

1. Compute the body's free variables, minus the parameter, minus anything that
   resolves to a primitive (primitives are globals — reference them directly,
   never capture them).
2. That set, sorted, *is* the environment layout. Sorting makes codegen
   deterministic across runs.
3. Resolve each captured name in the **enclosing** frame — it is either that
   frame's parameter or a slot in *its* environment. This is what threads a
   variable down through several levels: in `\z.\y.\x.z`, `z` is copied into
   the middle closure so the innermost one can reach it.
4. A name that resolves nowhere and is not a primitive is an unbound-variable
   error.

The result is a `Term` tree where every variable reference is already either
`Param` or `Env(i)` — no name lookup survives into codegen.

### Code generation (`codegen.rs`)

Emits **textual LLVM IR**, then shells out to `llc`. No `llvm-sys`/`inkwell`
dependency: this repo's LLVM is 22.x, far ahead of what those bindings
support, and the generated `.ll` is worth reading anyway. Inspect it with
`--emit-llvm`.

An application compiles to a tag check, a function-pointer load, and an
indirect call:

```llvm
  %v4 = and i64 %v1, 1                              ; is it a number?
  %v5 = icmp eq i64 %v4, 0
  br i1 %v5, label %apply.1, label %notfn.1
notfn.1:
  call void @imp_not_a_function(i64 %v1)            ; applied 5 to something
  unreachable
apply.1:
  %v6  = inttoptr i64 %v1 to ptr
  %v7  = load ptr, ptr %v6                          ; word 0: the code pointer
  %v13 = musttail call i64 %v7(ptr %v6, i64 %arg)
  ret i64 %v13
```

**`musttail` is a guarantee, not a hint.** Because it is a hard requirement
rather than an optimization, combinator-driven recursion runs in constant
stack space *even at `-O0`* — verified with 10,000,000 tail calls with the
optimizer switched off. Applications in tail position get `musttail`;
everything else gets an ordinary `call`.

### The runtime (`runtime/imprt.c`, ~220 lines)

Linked into every compiled program. It provides:

- **A chunked bump arena.** `imp_alloc` bumps a pointer; chunks are linked and
  **never `realloc`'d**. This matters: growing by `realloc` would move live
  closures and invalidate every pointer already handed to compiled code — a
  latent bug in the original prototype's arena. Everything is released in one
  `imp_arena_release` at exit.
- **The primitives**, curried exactly the way a compiled lambda is: stage one
  takes the first argument and returns a heap closure capturing it, stage two
  does the work. The compiler treats `+` as an ordinary value, with no special
  case. `false`'s second stage captures nothing, so it is a single static
  object rather than an allocation.
- **Diagnostics** — applying a non-function, arithmetic on a function, and
  division by zero all print a message and exit non-zero.
- **`main`**, which calls the compiler-generated `imp_start`.

The runtime is compiled once by [`build.rs`](impurell/build.rs) and embedded
in the compiler binary, so a built `impurell` is self-contained and can link
programs from any directory.

### Evaluation: call-by-value

Compiled purell is **strict**. Arguments are evaluated before the call, which
maps directly onto machine calls with no thunks. Two consequences you will
meet immediately:

**The Y combinator diverges.** Use `Z` from the prelude, which eta-expands the
self-application so it is not evaluated until applied:

```
Y = \f.(\x.f (x x)) (\x.f (x x))            -- loops forever under CBV
Z = \f.(\x.f (\v.x x v)) (\x.f (\v.x x v))  -- works
```

**Both branches of a conditional are evaluated.** `cond a b` computes `a` and
`b` before selecting. When a branch must not run — a recursive call, a
division by zero — wrap both in `\_.` and apply the result. That is all
`prelude.pl`'s `if` is:

```purell
:if \c.\t.\e.c t e 0

:fact Z (\rec.\n.if (<= n 1) (\_.1) (\_.* n (rec (- n 1))))
```

### Macros are expanded at compile time

The interpreter expands macros lazily during reduction. A compiler has no
reduction phase, so `impurell` substitutes macro bodies into the AST eagerly,
tracking which macros are currently being expanded. A macro that refers to
itself is therefore a clear compile error instead of a hang:

```
macro 'loop' expands into itself (loop -> loop); recursion must go through
a fixpoint combinator such as Z
```

A lambda parameter shadows a macro of the same name. A macro body is closed
over its definition site, so nothing bound at the use site leaks into it.

### Source files

| File | Lines | What it does |
|---|---|---|
| `src/main.rs` | 237 | CLI, drives the pipeline |
| `src/ast.rs` | 74 | `Expr`, free variables, pretty-printing |
| `src/lexer.rs` | 286 | Tokenizer and parser |
| `src/preprocessor.rs` | 293 | Comments, includes, macros, expression splitting, macro expansion |
| `src/core.rs` | 248 | Closure conversion / lambda lifting |
| `src/codegen.rs` | 368 | LLVM IR emission |
| `src/link.rs` | 113 | Invokes `llc` and the linker; embeds the runtime object |
| `runtime/imprt.h` | 71 | Value representation and ABI |
| `runtime/imprt.c` | 219 | Arena, primitives, diagnostics, `main` |
| `tests/programs.rs` | 265 | End-to-end tests |

---

## thepurell — the interpreter

Normal-order reduction of pure lambda terms to normal form, printing the
resulting *term* rather than a value. Because it reduces under lambdas and
leaves free variables alone, it can evaluate open terms — `S id id one`
reduces to `one one` with `one` never defined.

| File | Lines | What it does |
|---|---|---|
| `src/purell.rs` | 346 | `Expr`, capture-avoiding substitution, beta reduction, CLI |
| `src/lexer.rs` | 155 | Tokenizer and parser |
| `src/preprocessor.rs` | 225 | Comments, includes, macros |
| `src/repl.rs` | 223 | Interactive REPL |

Substitution is capture-avoiding: when substituting into a lambda whose
parameter occurs free in the replacement, it alpha-converts by appending `'`
to the parameter until the name is fresh.

Macros are expanded **lazily**, one step at a time during reduction, so a
macro is only unfolded when reduction actually reaches it.

---

## Interpreter vs. compiler

| | `thepurell` | `impurell` |
|---|---|---|
| Strategy | Normal order, reduces under lambdas | Call-by-value, closures are opaque |
| Result of a program | A normal-form **term** (`\f.\x.f (f x)`) | A **value** (`42` or `<function>`) |
| Unbound variable | A free variable, left alone | Compile error |
| Recursion | `Y` works | `Y` diverges; use `Z` |
| Numbers | None — Church numerals only | Native 63-bit, plus Church numerals |
| Macros | Expanded lazily during reduction | Expanded eagerly at compile time |
| Termination | Finds a normal form if one exists | May loop where the interpreter would not |

They agree on pure-lambda programs. The `church_numerals_match_native_arithmetic`
test pins this down: it runs `add`, `mul`, `exp`, `pred` and `sub` on Church
numerals — no primitives — and checks the compiler's answers against the
interpreter's.

---

## Building and running

**Requirements:** Rust, `llc` (LLVM), and a C compiler. Neither crate has any
Cargo dependencies. With Homebrew LLVM you need it on `PATH`:

```bash
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
```

Or point the compiler at the tools directly with `IMPURELL_LLC`, `IMPURELL_CC`
and `IMPURELL_OPT`.

### The compiler

```bash
cd impurell
cargo run -- examples/recursion.pl -I std --run
```

```
impurell [options] <file.pl>

  -o <path>       Executable to write (default: build/<name>)
  -I <dir>        Add a directory to the include search path
  -O<0-3>         Optimization level passed to llc (default: 2)
  --emit-llvm     Write the .ll file and stop
  --dump-ast      Print each expression's AST after macro expansion
  --verify        Run the LLVM verifier over the generated IR
  --run           Run the executable after building it
  --build-dir <d> Directory for intermediates (default: build)
```

### The interpreter

```bash
cd thepurell
cargo run -- examples/basic.pl      # run a file
cargo run                           # REPL
cargo run -- -pn examples/basic.pl  # print every reduction step
```

### Tests

```bash
cd impurell && cargo test
```

19 unit tests plus 17 end-to-end tests that compile, link and run real
binaries and check their output.

---

## Known limitations

- **Memory is never reclaimed.** The arena bumps and frees only at exit, so a
  long-running loop grows without bound: 3,000,000 tail-recursive iterations
  peak at ~1.08 GB, roughly 360 bytes per iteration (each `if` allocates two
  thunk closures). Fine for batch programs; this is the first wall you will
  hit. Fixing it means a tracing GC, which means teaching codegen to emit a
  shadow stack or stack maps so roots are findable.
- Arithmetic wraps silently at 63 bits rather than trapping.
- No strings, no I/O beyond `print` and the per-expression result line.
- Every application emits a tag check, including calls to primitives and to
  closures the compiler just allocated, where the check is provably dead.
  Skipping it for known-closure callees would tighten the hot path.
- `--emit-llvm` output is unoptimized IR straight from codegen; `llc -O2` does
  the cleanup.
