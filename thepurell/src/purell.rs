mod lexer;
mod preprocessor;
mod repl;

use std::collections::HashSet;
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum Expr {
    Var(String),
    Fun {
        arg: String,
        body: Box<Expr>,
    },
    App {
        left: Box<Expr>,
        right: Box<Expr>,
        right_first: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        // Start REPL if no arguments
        let mut repl_instance = repl::Repl::new();
        repl_instance.run()?;
        return Ok(());
    }

    if args.contains(&"-h".to_string()) || args.contains(&"--help".to_string()) {
        print_help();
        return Ok(());
    }

    let debug = args.contains(&"-g".to_string());
    let print_normalization = args.contains(&"-pn".to_string());
    let repl_mode = args.contains(&"-r".to_string()) || args.contains(&"--repl".to_string());

    let filename = args
        .iter()
        .skip(1)
        .find(|arg| !arg.starts_with("-"))
        .ok_or("ERROR: No file specified")?;

    if debug {
        println!("INFO: Debug mode enabled");
    }

    let contents = fs::read_to_string(filename)
        .map_err(|e| format!("ERROR: Couldn't read from file: {}\n{}", filename, e))?;

    // Parse expressions from file with standard library paths
    let mut preprocessor_instance = preprocessor::Preprocessor::with_stdlib();

    let file_path = Path::new(filename);
    let base_dir = file_path.parent().unwrap_or(Path::new("."));
    let expressions = preprocessor_instance.split_expressions_with_base(&contents, base_dir)?;

    if expressions.is_empty() {
        println!("No expressions found in file");
        return Ok(());
    }

    // Process expressions
    for processed_expr in expressions {
        // Parse without expanding macros; macros are expanded lazily during reduction
        match lexer::parse_expr(processed_expr.original.clone()) {
            Ok(expr) => {
                process_expression(
                    &processed_expr.original,
                    &expr,
                    &preprocessor_instance,
                    debug,
                    print_normalization,
                );
            }
            Err(e) => eprintln!("Parse Error: {}", e),
        }
    }

    // If REPL mode requested, enter interactive mode
    if repl_mode {
        let mut repl_instance = repl::Repl::new();
        repl_instance.run()?;
    }

    Ok(())
}

fn print_help() {
    println!("Lambda Calculus Interpreter");
    println!();
    println!("USAGE: lambda [options] [file]");
    println!();
    println!("MODES:");
    println!("  (no args)          Start interactive REPL");
    println!("  <file.pl>          Process lambda expressions from file");
    println!();
    println!("OPTIONS:");
    println!("  -g                 Show debug view of AST for each expression");
    println!("  -pn                Print reduction steps for each expression");
    println!("  -r, --repl         After processing file, enter interactive REPL");
    println!("  -h, --help         Show this help message");
    println!();
    println!("FILE FORMAT (.pl):");
    println!("  - Comments start with '#'");
    println!("  - Macro definitions: :name body");
    println!("  - Include files: @filename.pl");
    println!("  - Expressions separated by blank lines");
    println!("  - Multi-line expressions are joined with spaces");
    println!();
    println!("EXAMPLES:");
    println!("  lambda                    # Start REPL");
    println!("  lambda program.pl         # Process file");
    println!("  lambda -pn program.pl     # Show reduction steps");
    println!("  lambda -r program.pl      # Process file then enter REPL");
    println!();
    println!("STANDARD LIBRARY:");
    println!("  Include stdlib.pl at the top of your file with: @stdlib.pl");
}

fn process_expression(
    original: &str,
    expr: &Expr,
    preprocessor: &preprocessor::Preprocessor,
    debug: bool,
    print_normalization: bool,
) {
    let mut sv = String::new();
    sv.push_str("Expr: ");
    sv.push_str(original);
    sv.push('\n');

    if print_normalization {
        sv.push_str("Reduction:\n");
        let mut current = expr.clone();
        let mut step = 1;
        while let Some(next) = current.beta_reduce_with_macros(preprocessor) {
            _ = write!(&mut sv, "  Step {step}: ");
            next.show(&mut sv, false);
            sv.push('\n');
            current = next;
            step += 1;
        }
    }

    let normalized = expr.normalize_with_macros(preprocessor);
    sv.push_str("Result: ");
    normalized.show(&mut sv, false);
    sv.push_str("\n\n");
    print!("{sv}");

    if debug {
        dbg!(&expr);
        println!();
    }
}

impl Expr {
    fn expand_macro(name: &str, preprocessor: &preprocessor::Preprocessor) -> Option<Expr> {
        preprocessor
            .get_macro_expansion(name)
            .and_then(|body| lexer::parse_expr(body).ok())
    }

    fn show(&self, sv: &mut String, parantheses: bool) {
        match self {
            Expr::Var(name) => _ = write!(sv, "{name}"),
            Expr::Fun { arg, body } => {
                if parantheses {
                    _ = write!(sv, "(");
                }
                _ = write!(sv, "\\{arg}.");
                body.show(sv, false);
                if parantheses {
                    _ = write!(sv, ")");
                }
            }
            Expr::App {
                left,
                right,
                right_first,
            } => {
                if parantheses {
                    _ = write!(sv, "(");
                }
                left.show(sv, matches!(&**left, Expr::Fun { .. }));
                _ = write!(sv, " ");
                right.show(sv, matches!(&**right, Expr::Fun { .. }) || *right_first);
                if parantheses {
                    _ = write!(sv, ")");
                }
            }
        }
    }

    /// Check if var is free within expression
    fn is_free(&self, var: &str) -> bool {
        match self {
            Expr::Var(name) => name == var,
            Expr::Fun { arg, body } => arg != var && body.is_free(var),
            Expr::App { left, right, .. } => left.is_free(var) || right.is_free(var),
        }
    }

    /// Get free variables in expression
    fn free_vars(&self) -> HashSet<String> {
        match self {
            Expr::Var(name) => {
                let mut set = HashSet::new();
                set.insert(name.clone());
                set
            }
            Expr::Fun { arg, body } => {
                let mut set = body.free_vars();
                set.remove(arg);
                set
            }
            Expr::App { left, right, .. } => {
                let mut set = left.free_vars();
                set.extend(right.free_vars());
                set
            }
        }
    }

    /// Get a variable name that is free
    fn fresh_var(avoid: &HashSet<String>, var: &str) -> String {
        let mut name = var.to_string();
        while avoid.contains(&name) {
            name.push('\'');
        }
        name
    }

    fn substitute(&self, var: &str, replacement: &Expr) -> Expr {
        match self {
            Expr::Var(name) if name == var => replacement.clone(),
            Expr::Var(_) => self.clone(),
            // There is an inner function with the same argument name
            Expr::Fun { arg, body: _ } if arg == var => self.clone(),
            Expr::Fun { arg, body } => {
                // Do alpha conversion here
                if replacement.is_free(arg) {
                    let mut avoid = replacement.free_vars();
                    avoid.extend(body.free_vars());
                    let new_arg = Self::fresh_var(&avoid, arg);
                    let new_body = body.substitute(arg, &Expr::Var(new_arg.clone()));
                    Expr::Fun {
                        arg: new_arg,
                        body: Box::new(new_body.substitute(var, replacement)),
                    }
                } else {
                    Expr::Fun {
                        arg: arg.clone(),
                        body: Box::new(body.substitute(var, replacement)),
                    }
                }
            }
            Expr::App {
                left,
                right,
                right_first,
            } => Expr::App {
                left: Box::new(left.substitute(var, replacement)),
                right: Box::new(right.substitute(var, replacement)),
                right_first: *right_first,
            },
        }
    }


    fn beta_reduce_with_macros(
        &self,
        preprocessor: &preprocessor::Preprocessor,
    ) -> Option<Expr> {
        match self {
            Expr::Var(name) => Self::expand_macro(name, preprocessor),
            Expr::Fun { arg, body } => body
                .beta_reduce_with_macros(preprocessor)
                .map(|new_body| Expr::Fun {
                    arg: arg.clone(),
                    body: Box::new(new_body),
                }),
            Expr::App {
                left,
                right,
                right_first,
            } => {
                macro_rules! beta_reduce {
                    (right) => {
                        right
                            .beta_reduce_with_macros(preprocessor)
                            .map(|new_right| Expr::App {
                                left: left.clone(),
                                right: Box::new(new_right),
                                right_first: false,
                            })
                    };
                    (left) => {
                        left
                            .beta_reduce_with_macros(preprocessor)
                            .map(|new_left| Expr::App {
                                left: Box::new(new_left),
                                right: right.clone(),
                                right_first: false,
                            })
                    };
                }

                if let Expr::Fun { arg, body } = &**left {
                    Some(body.substitute(arg, right))
                } else if let Expr::Var(name) = &**left {
                    if let Some(expanded) = Self::expand_macro(name, preprocessor) {
                        Some(Expr::App {
                            left: Box::new(expanded),
                            right: right.clone(),
                            right_first: false,
                        })
                    } else if *right_first {
                        beta_reduce!(right).or_else(|| beta_reduce!(left))
                    } else {
                        beta_reduce!(left).or_else(|| beta_reduce!(right))
                    }
                } else if *right_first {
                    beta_reduce!(right).or_else(|| beta_reduce!(left))
                } else {
                    beta_reduce!(left).or_else(|| beta_reduce!(right))
                }
            }
        }
    }

    fn normalize_with_macros(&self, preprocessor: &preprocessor::Preprocessor) -> Expr {
        let mut current = self.clone();
        while let Some(next) = current.beta_reduce_with_macros(preprocessor) {
            current = next;
        }
        current
    }
}
