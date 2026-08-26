use crate::Expr;

// Lex text into Expr
#[derive(Debug)]
enum Token {
    Lambda,
    Dot,
    OpenParen,
    CloseParen,
    Ident(String),
}

struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl ExprParser {
    fn new(expr: Vec<Token>) -> Self {
        ExprParser {
            tokens: expr,
            pos: 0,
        }
    }

    fn parse(mut self) -> Result<Expr, String> {
        let expr = self.parse_expr()?;

        // All tokens should be consumed
        if self.pos < self.tokens.len() {
            Err(format!(
                "Unexpected token(s) {:?}",
                &self.tokens[self.pos..self.tokens.len()]
            ))
        } else {
            Ok(expr)
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let (mut left, mut left_parens) = self.parse_atom_with_parens()?;

        while self.pos < self.tokens.len() && !matches!(&self.tokens[self.pos], Token::CloseParen) {
            let (right, right_first) = self.parse_atom_with_parens()?;

            left = Expr::App {
                left: Box::new(left),
                right: Box::new(right),
                right_first: right_first && !left_parens,
            };
            left_parens = right_first;
        }
        Ok(left)
    }

    fn parse_atom_with_parens(&mut self) -> Result<(Expr, bool), String> {
        if self.pos >= self.tokens.len() {
            return Err("Unexpected end of input".to_string());
        }

        match &self.tokens[self.pos] {
            Token::Lambda => Ok((self.parse_lambda()?, false)),
            Token::Ident(name) => {
                let name = name.clone();
                self.pos += 1;
                Ok((Expr::Var(name), false))
            }
            Token::OpenParen => {
                self.pos += 1;
                let expr = self.parse_expr()?;
                self.expect(Token::CloseParen)?;
                Ok((expr, true))
            }
            token => Err(format!("Unexpected token {:?}", token)),
        }
    }

    fn parse_lambda(&mut self) -> Result<Expr, String> {
        self.expect(Token::Lambda)?;

        let arg = match &self.tokens[self.pos] {
            Token::Ident(name) => {
                let name = name.clone();
                self.pos += 1;
                name
            }
            token => return Err(format!("Expected Identifier got {:?}", token)),
        };

        self.expect(Token::Dot)?;
        let body = self.parse_expr()?;
        Ok(Expr::Fun {
            arg,
            body: Box::new(body),
        })
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.pos >= self.tokens.len() {
            return Err(format!("Expected {:?}, got EOF", expected));
        }

        if std::mem::discriminant(&self.tokens[self.pos]) == std::mem::discriminant(&expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, got {:?}",
                expected, self.tokens[self.pos]
            ))
        }
    }
}

fn lex_expr(expr: String) -> Result<Vec<Token>, String> {
    let mut expr = expr.chars().peekable();
    // Collect from iterable of token generator function
    std::iter::from_fn(move || {
        // Skip white space first
        while let Some(&c) = expr.peek() {
            if c.is_whitespace() {
                expr.next();
            } else {
                break;
            }
        }

        let c = expr.next()?;
        match c {
            '\\' => Some(Ok(Token::Lambda)),
            '.' => Some(Ok(Token::Dot)),
            '(' => Some(Ok(Token::OpenParen)),
            ')' => Some(Ok(Token::CloseParen)),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => {
                let mut name = String::from(c);
                while let Some(&c) = expr.peek() {
                    if c.is_alphabetic() || c.is_numeric() || c == '_' {
                        name.push(c);
                        expr.next();
                    } else {
                        break;
                    }
                }
                Some(Ok(Token::Ident(name)))
            }
            _ => Some(Err(format!("Unexpected character {c}"))),
        }
    })
    .collect()
}

pub fn parse_expr(expr_string: String) -> Result<Expr, String> {
    let tokens = lex_expr(expr_string)?;
    ExprParser::new(tokens).parse()
}
