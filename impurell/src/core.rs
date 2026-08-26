//! Closure conversion: surface `Expr` -> lifted `Program`.
//!
//! Every lambda becomes a top-level function taking `(self, arg)`. The
//! variables it uses from enclosing scopes are collected into an environment
//! stored alongside the function pointer in a heap closure, so a closure stays
//! correct after the frame that built it has returned. (The earlier prototype
//! used a global capture stack, which cannot express that.)

use std::collections::BTreeSet;

use crate::ast::Expr;

/// Surface name -> runtime symbol for the primitives the runtime provides.
pub const PRIMITIVES: &[(&str, &str)] = &[
    ("+", "imp_prim_add"),
    ("-", "imp_prim_sub"),
    ("*", "imp_prim_mul"),
    ("/", "imp_prim_div"),
    ("%", "imp_prim_rem"),
    ("<", "imp_prim_lt"),
    (">", "imp_prim_gt"),
    ("<=", "imp_prim_le"),
    (">=", "imp_prim_ge"),
    ("=", "imp_prim_eq"),
    ("/=", "imp_prim_ne"),
    ("true", "imp_prim_true"),
    ("false", "imp_prim_false"),
    ("print", "imp_prim_print"),
];

fn primitive_symbol(name: &str) -> Option<&'static str> {
    PRIMITIVES
        .iter()
        .find(|(surface, _)| *surface == name)
        .map(|(_, symbol)| *symbol)
}

/// How to fetch a captured value from the frame that is building a closure.
#[derive(Debug, Clone, Copy)]
pub enum Capture {
    Param,
    Env(usize),
}

#[derive(Debug, Clone)]
pub enum Term {
    /// Untagged literal; codegen applies the `(n << 1) | 1` tag.
    Num(i64),
    /// Address of a runtime primitive global.
    Prim(&'static str),
    /// The enclosing function's argument.
    Param,
    /// Slot `i` of the enclosing function's captured environment.
    Env(usize),
    /// Allocate a closure over lifted function `lam`, filling its environment
    /// from the current frame.
    MakeClosure { lam: usize, captures: Vec<Capture> },
    App(Box<Term>, Box<Term>),
}

#[derive(Debug)]
pub struct Lam {
    pub id: usize,
    pub param: String,
    /// Names of the captured slots, in environment order. Kept for readable
    /// comments in the generated IR.
    pub env: Vec<String>,
    pub body: Term,
}

#[derive(Debug)]
pub struct TopLevel {
    pub source: String,
    pub term: Term,
}

#[derive(Debug, Default)]
pub struct Program {
    pub lams: Vec<Lam>,
    pub tops: Vec<TopLevel>,
}

/// The variables reachable from the function currently being compiled.
struct Frame<'a> {
    param: Option<&'a str>,
    env: &'a [String],
}

impl Frame<'_> {
    fn lookup(&self, name: &str) -> Option<Capture> {
        if self.param == Some(name) {
            return Some(Capture::Param);
        }
        self.env
            .iter()
            .position(|slot| slot == name)
            .map(Capture::Env)
    }
}

#[derive(Default)]
pub struct Converter {
    lams: Vec<Lam>,
}

impl Converter {
    pub fn new() -> Self {
        Converter::default()
    }

    /// Convert one closed top-level expression.
    pub fn convert_top(&mut self, source: &str, expr: &Expr) -> Result<TopLevel, String> {
        let frame = Frame {
            param: None,
            env: &[],
        };
        let term = self.convert(expr, &frame)?;
        Ok(TopLevel {
            source: source.to_string(),
            term,
        })
    }

    pub fn finish(self, tops: Vec<TopLevel>) -> Program {
        Program {
            lams: self.lams,
            tops,
        }
    }

    fn convert(&mut self, expr: &Expr, frame: &Frame) -> Result<Term, String> {
        match expr {
            Expr::Num(n) => Ok(Term::Num(*n)),

            Expr::Var(name) => match frame.lookup(name) {
                Some(Capture::Param) => Ok(Term::Param),
                Some(Capture::Env(i)) => Ok(Term::Env(i)),
                // Primitives are globals, so they are referenced directly
                // rather than captured.
                None => match primitive_symbol(name) {
                    Some(symbol) => Ok(Term::Prim(symbol)),
                    None => Err(format!("unbound variable '{name}'")),
                },
            },

            Expr::App(left, right) => Ok(Term::App(
                Box::new(self.convert(left, frame)?),
                Box::new(self.convert(right, frame)?),
            )),

            Expr::Fun { arg, body } => {
                // Environment layout is the body's free variables minus the
                // parameter and minus anything that resolves to a global.
                // BTreeSet keeps the layout stable across runs.
                let mut free: BTreeSet<String> = body.free_vars();
                free.remove(arg);
                free.retain(|name| primitive_symbol(name).is_none());
                let env: Vec<String> = free.into_iter().collect();

                let captures = env
                    .iter()
                    .map(|name| {
                        frame
                            .lookup(name)
                            .ok_or_else(|| format!("unbound variable '{name}'"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                // Reserve the id before converting the body so nested lambdas
                // get larger ids than their parent.
                let id = self.lams.len();
                self.lams.push(Lam {
                    id,
                    param: arg.clone(),
                    env: env.clone(),
                    body: Term::Num(0), // placeholder, replaced below
                });

                let inner = Frame {
                    param: Some(arg),
                    env: &env,
                };
                let converted = self.convert(body, &inner)?;
                self.lams[id].body = converted;

                Ok(Term::MakeClosure { lam: id, captures })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    fn convert(source: &str) -> Result<Program, String> {
        let expr = lexer::parse_expr(source)?;
        let mut converter = Converter::new();
        let top = converter.convert_top(source, &expr)?;
        Ok(converter.finish(vec![top]))
    }

    #[test]
    fn identity_captures_nothing() {
        let program = convert("\\x.x").unwrap();
        assert_eq!(program.lams.len(), 1);
        assert!(program.lams[0].env.is_empty());
        assert!(matches!(program.lams[0].body, Term::Param));
    }

    #[test]
    fn inner_lambda_captures_outer_param() {
        let program = convert("\\x.\\y.x").unwrap();
        assert_eq!(program.lams.len(), 2);
        // The inner lambda captures `x`...
        assert_eq!(program.lams[1].env, vec!["x".to_string()]);
        assert!(matches!(program.lams[1].body, Term::Env(0)));
        // ...taken from the outer lambda's parameter.
        match &program.lams[0].body {
            Term::MakeClosure { captures, .. } => {
                assert_eq!(captures.len(), 1);
                assert!(matches!(captures[0], Capture::Param));
            }
            other => panic!("expected a closure allocation, got {other:?}"),
        }
    }

    #[test]
    fn capture_is_threaded_through_two_levels() {
        // `z` must be copied into the middle closure to reach the innermost.
        let program = convert("\\z.\\y.\\x.z").unwrap();
        assert_eq!(program.lams[1].env, vec!["z".to_string()]);
        assert_eq!(program.lams[2].env, vec!["z".to_string()]);
    }

    #[test]
    fn primitives_are_not_captured() {
        let program = convert("\\x.+ x 1").unwrap();
        assert!(program.lams[0].env.is_empty());
    }

    #[test]
    fn unbound_variables_are_rejected() {
        let err = convert("\\x.y").unwrap_err();
        assert!(err.contains("unbound variable 'y'"), "{err}");
    }
}
