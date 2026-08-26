//! Surface syntax: untyped lambda calculus plus 63-bit integer literals.

use std::collections::BTreeSet;
use std::fmt::Write;

#[derive(Debug, Clone)]
pub enum Expr {
    Var(String),
    Num(i64),
    Fun { arg: String, body: Box<Expr> },
    App(Box<Expr>, Box<Expr>),
}

impl Expr {
    /// Variables that are not bound by an enclosing lambda.
    pub fn free_vars(&self) -> BTreeSet<String> {
        let mut acc = BTreeSet::new();
        self.collect_free(&mut acc);
        acc
    }

    fn collect_free(&self, acc: &mut BTreeSet<String>) {
        match self {
            Expr::Num(_) => {}
            Expr::Var(name) => {
                acc.insert(name.clone());
            }
            Expr::Fun { arg, body } => {
                let mut inner = BTreeSet::new();
                body.collect_free(&mut inner);
                inner.remove(arg);
                acc.extend(inner);
            }
            Expr::App(left, right) => {
                left.collect_free(acc);
                right.collect_free(acc);
            }
        }
    }

    pub fn show(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, false);
        out
    }

    fn write(&self, out: &mut String, parens: bool) {
        match self {
            Expr::Var(name) => _ = write!(out, "{name}"),
            Expr::Num(n) => _ = write!(out, "{n}"),
            Expr::Fun { arg, body } => {
                if parens {
                    out.push('(');
                }
                _ = write!(out, "\\{arg}.");
                body.write(out, false);
                if parens {
                    out.push(')');
                }
            }
            Expr::App(left, right) => {
                if parens {
                    out.push('(');
                }
                left.write(out, matches!(&**left, Expr::Fun { .. }));
                out.push(' ');
                right.write(out, !matches!(&**right, Expr::Var(_) | Expr::Num(_)));
                if parens {
                    out.push(')');
                }
            }
        }
    }
}
