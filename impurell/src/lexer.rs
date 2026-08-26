//! Tokenizer and parser for purell source.
//!
//! Differences from the interpreter's lexer:
//!   * a token made entirely of digits is an integer literal, not an identifier
//!   * `-` immediately followed by a digit (no space) is a negative literal
//!   * runs of operator characters lex as identifiers, so `+`, `<=`, `/=` are
//!     ordinary names that happen to resolve to primitives

use crate::ast::Expr;

const SYMBOL_CHARS: &str = "+-*/%<>=";

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Lambda,
    Dot,
    OpenParen,
    CloseParen,
    Ident(String),
    Num(i64),
}

impl Token {
    fn describe(&self) -> String {
        match self {
            Token::Lambda => "'\\'".to_string(),
            Token::Dot => "'.'".to_string(),
            Token::OpenParen => "'('".to_string(),
            Token::CloseParen => "')'".to_string(),
            Token::Ident(name) => format!("identifier '{name}'"),
            Token::Num(n) => format!("number {n}"),
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '\''
}

fn lex(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        match c {
            '\\' | 'λ' => {
                tokens.push(Token::Lambda);
                i += 1;
            }
            '.' => {
                tokens.push(Token::Dot);
                i += 1;
            }
            '(' => {
                tokens.push(Token::OpenParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::CloseParen);
                i += 1;
            }
            _ if c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                // Reject `12abc` rather than silently splitting it in two.
                if i < chars.len() && is_ident_continue(chars[i]) {
                    return Err(format!(
                        "invalid number literal '{}'",
                        chars[start..].iter().take_while(|c| is_ident_continue(**c)).collect::<String>()
                    ));
                }
                tokens.push(Token::Num(parse_int(&chars[start..i], false)?));
            }
            _ if is_ident_start(c) => {
                let start = i;
                while i < chars.len() && is_ident_continue(chars[i]) {
                    i += 1;
                }
                tokens.push(Token::Ident(chars[start..i].iter().collect()));
            }
            _ if SYMBOL_CHARS.contains(c) => {
                // `-5` is a literal; `- 5` is the subtraction primitive applied
                // to 5. The space is the whole difference.
                if c == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                    i += 1;
                    let start = i;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    tokens.push(Token::Num(parse_int(&chars[start..i], true)?));
                } else {
                    let start = i;
                    while i < chars.len() && SYMBOL_CHARS.contains(chars[i]) {
                        i += 1;
                    }
                    tokens.push(Token::Ident(chars[start..i].iter().collect()));
                }
            }
            _ => return Err(format!("unexpected character '{c}'")),
        }
    }

    Ok(tokens)
}

/// Parse digits into the 63-bit payload range, rejecting anything that would
/// not survive tagging.
fn parse_int(digits: &[char], negative: bool) -> Result<i64, String> {
    let text: String = digits.iter().collect();
    let value: i128 = text
        .parse()
        .map_err(|_| format!("number literal out of range: {text}"))?;
    let value = if negative { -value } else { value };

    const MIN: i128 = -(1i128 << 62);
    const MAX: i128 = (1i128 << 62) - 1;
    if value < MIN || value > MAX {
        return Err(format!(
            "number literal {value} does not fit in 63 bits (range {MIN}..={MAX})"
        ));
    }
    Ok(value as i64)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn parse(mut self) -> Result<Expr, String> {
        let expr = self.parse_expr()?;
        if self.pos < self.tokens.len() {
            return Err(format!(
                "unexpected {}",
                self.tokens[self.pos].describe()
            ));
        }
        Ok(expr)
    }

    /// Application is left-associative and binds tighter than anything else.
    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_atom()?;
        while self.pos < self.tokens.len() && self.tokens[self.pos] != Token::CloseParen {
            let right = self.parse_atom()?;
            left = Expr::App(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_atom(&mut self) -> Result<Expr, String> {
        let Some(token) = self.tokens.get(self.pos) else {
            return Err("unexpected end of expression".to_string());
        };

        match token.clone() {
            Token::Lambda => self.parse_lambda(),
            Token::Num(n) => {
                self.pos += 1;
                Ok(Expr::Num(n))
            }
            Token::Ident(name) => {
                self.pos += 1;
                Ok(Expr::Var(name))
            }
            Token::OpenParen => {
                self.pos += 1;
                let inner = self.parse_expr()?;
                self.expect(&Token::CloseParen)?;
                Ok(inner)
            }
            other => Err(format!("unexpected {}", other.describe())),
        }
    }

    /// A lambda body extends as far right as it can, so `\x.f x y` is
    /// `\x.((f x) y)`.
    fn parse_lambda(&mut self) -> Result<Expr, String> {
        self.expect(&Token::Lambda)?;

        let arg = match self.tokens.get(self.pos) {
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.pos += 1;
                name
            }
            Some(other) => {
                return Err(format!(
                    "expected a parameter name after '\\', got {}",
                    other.describe()
                ));
            }
            None => return Err("expected a parameter name after '\\'".to_string()),
        };

        self.expect(&Token::Dot)?;
        let body = self.parse_expr()?;
        Ok(Expr::Fun {
            arg,
            body: Box::new(body),
        })
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        match self.tokens.get(self.pos) {
            Some(token) if std::mem::discriminant(token) == std::mem::discriminant(expected) => {
                self.pos += 1;
                Ok(())
            }
            Some(token) => Err(format!(
                "expected {}, got {}",
                expected.describe(),
                token.describe()
            )),
            None => Err(format!("expected {}, got end of input", expected.describe())),
        }
    }
}

pub fn parse_expr(source: &str) -> Result<Expr, String> {
    let tokens = lex(source)?;
    if tokens.is_empty() {
        return Err("empty expression".to_string());
    }
    Parser { tokens, pos: 0 }.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(source: &str) -> String {
        parse_expr(source).expect("should parse").show()
    }

    #[test]
    fn application_is_left_associative() {
        assert_eq!(show("f x y"), "f x y");
        assert_eq!(show("f (x y)"), "f (x y)");
    }

    #[test]
    fn lambda_body_extends_right() {
        assert_eq!(show("\\x.f x y"), "\\x.f x y");
        assert_eq!(show("(\\x.x) y"), "(\\x.x) y");
    }

    #[test]
    fn operators_are_identifiers() {
        assert_eq!(show("+ 1 2"), "+ 1 2");
        assert_eq!(show("<= a b"), "<= a b");
    }

    #[test]
    fn negative_literals_need_no_space() {
        assert_eq!(show("+ 1 -5"), "+ 1 -5");
        // With a space, `-` is the subtraction primitive.
        assert_eq!(show("- 1 5"), "- 1 5");
    }

    #[test]
    fn oversized_literals_are_rejected() {
        assert!(parse_expr("4611686018427387904").is_err());
        assert!(parse_expr("4611686018427387903").is_ok());
    }

    #[test]
    fn digits_glued_to_letters_are_rejected() {
        assert!(parse_expr("12abc").is_err());
    }
}
