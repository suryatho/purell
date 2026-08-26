use crate::lexer;
use crate::preprocessor;
use crate::Expr;
use std::fmt::Write;
use std::io::{self, Write as IoWrite};
use std::path::Path;

/// Interactive REPL for the lambda calculus interpreter
pub struct Repl {
    show_debug: bool,
    show_steps: bool,
    history: Vec<String>,
    preprocessor: preprocessor::Preprocessor,
}

impl Repl {
    /// Create a new REPL instance with stdlib macros loaded
    pub fn new() -> Self {
        let mut preprocessor_instance = preprocessor::Preprocessor::with_stdlib();

        // Load stdlib macros
        if let Err(e) = Self::load_stdlib_macros(&mut preprocessor_instance) {
            eprintln!("Warning: Could not load stdlib macros: {}", e);
        }

        Repl {
            show_debug: false,
            show_steps: false,
            history: Vec::new(),
            preprocessor: preprocessor_instance,
        }
    }

    /// Load macros from stdlib.pl
    fn load_stdlib_macros(preprocessor: &mut preprocessor::Preprocessor) -> Result<(), String> {
        for path_str in &["std/stdlib.pl", "stdlib.pl"] {
            if let Ok(contents) = std::fs::read_to_string(path_str) {
                let base_dir = Path::new(path_str).parent().unwrap_or(Path::new("."));
                let _ = preprocessor.split_expressions_with_base(&contents, base_dir);
                for stdmath in &["std/stdmath.pl", "stdmath.pl"] {
                    if let Ok(c) = std::fs::read_to_string(stdmath) {
                        let d = Path::new(stdmath).parent().unwrap_or(Path::new("."));
                        let _ = preprocessor.split_expressions_with_base(&c, d);
                    }
                }
                return Ok(());
            }
        }
        Err("stdlib.pl not found".to_string())
    }

    /// Run the interactive REPL
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Lambda Calculus REPL");
        println!("Type 'help' for commands, 'exit' to quit\n");

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            if !self.process_command(input)? {
                break;
            }
        }

        Ok(())
    }

    /// Process a REPL command
    fn process_command(&mut self, input: &str) -> Result<bool, Box<dyn std::error::Error>> {
        match input {
            "exit" | "quit" => Ok(false),
            "help" => {
                self.print_help();
                Ok(true)
            }
            "debug" => {
                self.show_debug = !self.show_debug;
                println!("Debug mode: {}", if self.show_debug { "ON" } else { "OFF" });
                Ok(true)
            }
            "steps" => {
                self.show_steps = !self.show_steps;
                println!(
                    "Step-by-step reduction: {}",
                    if self.show_steps { "ON" } else { "OFF" }
                );
                Ok(true)
            }
            "history" => {
                for (i, expr) in self.history.iter().enumerate() {
                    println!("{}: {}", i + 1, expr);
                }
                Ok(true)
            }
            "clear" => {
                self.history.clear();
                println!("History cleared");
                Ok(true)
            }
            _ => {
                self.evaluate_expression(input)?;
                self.history.push(input.to_string());
                Ok(true)
            }
        }
    }

    /// Evaluate a lambda calculus expression
    fn evaluate_expression(&mut self, expr_string: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Parse without expanding macros; macros are expanded lazily during reduction
        match lexer::parse_expr(expr_string.to_string()) {
            Ok(expr) => {
                let mut output = String::new();

                // Show original expression
                output.push_str("Input:      ");
                Self::show_expr(&expr, &mut output, false);
                output.push('\n');

                // Show normalization steps if enabled
                if self.show_steps {
                    output.push_str("Reduction:\n");
                    let mut current = expr.clone();
                    let mut step = 1;
                    while let Some(next) = current.beta_reduce_with_macros(&self.preprocessor) {
                        output.push_str(&format!("  Step {:2}: ", step));
                        Self::show_expr(&next, &mut output, false);
                        output.push('\n');
                        current = next;
                        step += 1;
                    }
                }

                // Show normalized expression
                let normalized = expr.normalize_with_macros(&self.preprocessor);
                output.push_str("Result:     ");
                Self::show_expr(&normalized, &mut output, false);
                output.push('\n');

                // Show debug info if enabled
                if self.show_debug {
                    output.push_str(&format!("AST:        {:#?}\n", expr));
                }

                print!("{}", output);
                Ok(())
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                Ok(())
            }
        }
    }

    /// Print help message
    fn print_help(&self) {
        println!("Commands:");
        println!("  help        - Show this help message");
        println!("  exit/quit   - Exit the REPL");
        println!("  debug       - Toggle debug mode (shows AST)");
        println!("  steps       - Toggle step-by-step reduction display");
        println!("  history     - Show expression history");
        println!("  clear       - Clear history");
        println!();
        println!("Examples:");
        println!("  \\x.x           - Identity function");
        println!("  (\\x.x) (\\y.y) - Identity applied to identity");
        println!("  \\x.\\y.x        - Constant function");
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

impl Repl {
    /// Show the expression in readable form
    fn show_expr(expr: &Expr, sv: &mut String, parentheses: bool) {
        match expr {
            Expr::Var(name) => _ = write!(sv, "{name}"),
            Expr::Fun { arg, body } => {
                if parentheses {
                    _ = write!(sv, "(");
                }
                _ = write!(sv, "\\{arg}.");
                Repl::show_expr(body, sv, false);
                if parentheses {
                    _ = write!(sv, ")");
                }
            }
            Expr::App {
                left,
                right,
                right_first,
            } => {
                if parentheses {
                    _ = write!(sv, "(");
                }
                Repl::show_expr(left, sv, matches!(&**left, Expr::Fun { .. }));
                _ = write!(sv, " ");
                Repl::show_expr(
                    right,
                    sv,
                    matches!(&**right, Expr::Fun { .. }) || *right_first,
                );
                if parentheses {
                    _ = write!(sv, ")");
                }
            }
        }
    }
}
