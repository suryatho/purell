//! Source preprocessing: `#` comments, `:name body` macros, `@file.pl`
//! includes, and blank-line-separated expressions.
//!
//! The interpreter expands macros lazily during reduction. A compiler has no
//! reduction phase, so macros are expanded eagerly into the AST here, and a
//! macro that refers to itself is a compile error rather than a hang.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Expr;
use crate::lexer;

const COMMENT_CHAR: char = '#';
const MACRO_CHAR: char = ':';
const INCLUDE_CHAR: char = '@';

pub struct Unit {
    pub macros: HashMap<String, String>,
    pub exprs: Vec<SourceExpr>,
}

pub struct SourceExpr {
    /// The expression as written, used for the `Expr:` line at runtime.
    pub source: String,
    pub origin: PathBuf,
    pub line: usize,
}

pub struct Preprocessor {
    macros: HashMap<String, String>,
    exprs: Vec<SourceExpr>,
    include_paths: Vec<PathBuf>,
}

impl Preprocessor {
    pub fn new(extra_include_paths: Vec<PathBuf>) -> Self {
        let mut include_paths = extra_include_paths;

        if let Ok(cwd) = std::env::current_dir() {
            include_paths.push(cwd.join("std"));
            include_paths.push(cwd);
        }
        // Alongside the compiler binary, and its ../std for a cargo layout.
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            include_paths.push(dir.join("std"));
            include_paths.push(dir.to_path_buf());
        }

        Preprocessor {
            macros: HashMap::new(),
            exprs: Vec::new(),
            include_paths,
        }
    }

    pub fn load(mut self, path: &Path) -> Result<Unit, String> {
        let mut open = Vec::new();
        self.load_file(path, &mut open)?;
        Ok(Unit {
            macros: self.macros,
            exprs: self.exprs,
        })
    }

    fn load_file(&mut self, path: &Path, open: &mut Vec<PathBuf>) -> Result<(), String> {
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if open.contains(&canonical) {
            return Err(format!("circular include: {}", path.display()));
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();

        open.push(canonical);
        let result = self.load_contents(&contents, path, &base_dir, open);
        open.pop();
        result
    }

    fn load_contents(
        &mut self,
        contents: &str,
        origin: &Path,
        base_dir: &Path,
        open: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        let mut current = String::new();
        let mut current_line = 0usize;

        for (index, raw_line) in contents.lines().enumerate() {
            let line_no = index + 1;
            let line = strip_comment(raw_line);
            let line = line.trim();

            if line.is_empty() {
                self.flush(&mut current, current_line, origin);
                continue;
            }

            if let Some(rest) = line.strip_prefix(INCLUDE_CHAR) {
                self.flush(&mut current, current_line, origin);
                let include = self.resolve_include(rest.trim(), base_dir).map_err(|e| {
                    format!("{}:{line_no}: {e}", origin.display())
                })?;
                self.load_file(&include, open)?;
                continue;
            }

            if let Some(rest) = line.strip_prefix(MACRO_CHAR) {
                self.flush(&mut current, current_line, origin);
                self.define_macro(rest)
                    .map_err(|e| format!("{}:{line_no}: {e}", origin.display()))?;
                continue;
            }

            if current.is_empty() {
                current_line = line_no;
            } else {
                current.push(' ');
            }
            current.push_str(line);
        }

        self.flush(&mut current, current_line, origin);
        Ok(())
    }

    fn flush(&mut self, current: &mut String, line: usize, origin: &Path) {
        if current.trim().is_empty() {
            current.clear();
            return;
        }
        self.exprs.push(SourceExpr {
            source: std::mem::take(current),
            origin: origin.to_path_buf(),
            line,
        });
    }

    fn define_macro(&mut self, rest: &str) -> Result<(), String> {
        let rest = rest.trim();
        let (name, body) = rest
            .split_once(char::is_whitespace)
            .ok_or("macro definition must be ':name body'")?;

        let name = name.trim();
        let body = body.trim();
        if name.is_empty() || body.is_empty() {
            return Err("macro name and body must both be non-empty".to_string());
        }

        // Reject up front rather than at expansion time, where the error would
        // point at a use site instead of the definition.
        lexer::parse_expr(body).map_err(|e| format!("in macro '{name}': {e}"))?;

        self.macros.insert(name.to_string(), body.to_string());
        Ok(())
    }

    fn resolve_include(&self, name: &str, base_dir: &Path) -> Result<PathBuf, String> {
        if !name.ends_with(".pl") {
            return Err(format!("include file must end in .pl: {name}"));
        }

        let direct = base_dir.join(name);
        if direct.exists() {
            return Ok(direct);
        }
        for search in &self.include_paths {
            let candidate = search.join(name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(format!("cannot find include file: {name}"))
    }
}

/// Strip everything from an unescaped `#` onward.
fn strip_comment(line: &str) -> &str {
    match line.find(COMMENT_CHAR) {
        Some(index) => &line[..index],
        None => line,
    }
}

impl Unit {
    /// Substitute macro bodies into `expr` until no macro names remain.
    ///
    /// `active` holds the macros currently being expanded on this path, which
    /// is what turns `:loop loop` from an infinite expansion into an error.
    pub fn expand(&self, expr: &Expr) -> Result<Expr, String> {
        self.expand_inner(expr, &mut Vec::new(), &mut Vec::new())
    }

    fn expand_inner(
        &self,
        expr: &Expr,
        active: &mut Vec<String>,
        bound: &mut Vec<String>,
    ) -> Result<Expr, String> {
        match expr {
            Expr::Num(n) => Ok(Expr::Num(*n)),
            Expr::Var(name) => {
                // A lambda parameter shadows a macro of the same name.
                if bound.contains(name) {
                    return Ok(Expr::Var(name.clone()));
                }
                let Some(body) = self.macros.get(name) else {
                    return Ok(Expr::Var(name.clone()));
                };
                if active.contains(name) {
                    active.push(name.clone());
                    return Err(format!(
                        "macro '{name}' expands into itself ({}); \
                         recursion must go through a fixpoint combinator such as Z",
                        active.join(" -> ")
                    ));
                }
                let parsed =
                    lexer::parse_expr(body).map_err(|e| format!("in macro '{name}': {e}"))?;

                // A macro body is closed over the definition site, not the use
                // site, so nothing bound out here shadows names inside it.
                let mut inner_bound = Vec::new();
                active.push(name.clone());
                let expanded = self.expand_inner(&parsed, active, &mut inner_bound);
                active.pop();
                expanded
            }
            Expr::Fun { arg, body } => {
                bound.push(arg.clone());
                let body = self.expand_inner(body, active, bound);
                bound.pop();
                Ok(Expr::Fun {
                    arg: arg.clone(),
                    body: Box::new(body?),
                })
            }
            Expr::App(left, right) => Ok(Expr::App(
                Box::new(self.expand_inner(left, active, bound)?),
                Box::new(self.expand_inner(right, active, bound)?),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(macros: &[(&str, &str)]) -> Unit {
        Unit {
            macros: macros
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            exprs: Vec::new(),
        }
    }

    #[test]
    fn expands_nested_macros() {
        let unit = unit(&[("id", "\\x.x"), ("twice", "\\f.\\x.f (f x)")]);
        let expr = lexer::parse_expr("twice id").unwrap();
        assert_eq!(unit.expand(&expr).unwrap().show(), "(\\f.\\x.f (f x)) (\\x.x)");
    }

    #[test]
    fn self_referential_macro_is_an_error() {
        let unit = unit(&[("loop", "\\x.loop x")]);
        let expr = lexer::parse_expr("loop").unwrap();
        let err = unit.expand(&expr).unwrap_err();
        assert!(err.contains("expands into itself"), "{err}");
    }

    #[test]
    fn parameter_shadows_macro() {
        let unit = unit(&[("x", "42")]);
        let expr = lexer::parse_expr("\\x.x").unwrap();
        assert_eq!(unit.expand(&expr).unwrap().show(), "\\x.x");
    }

    #[test]
    fn comments_are_stripped_mid_line() {
        assert_eq!(strip_comment("\\x.x # identity").trim(), "\\x.x");
    }
}
