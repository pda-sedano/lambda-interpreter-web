// Inspiration from here: https://ruslanspivak.com/lsbasi-part7/
// and here: https://people.hsc.edu/faculty-staff/robbk/Coms480/Lectures/Spring%202015/Lecture%207%20-%20Recursive%20Descent%20Parsers.pdf
// and here: https://petermalmgren.com/three-rust-parsers/

use std::fmt::{Display, Formatter, self};
use std::collections::HashMap;

use super::lexer::Token::{self, *};
use super::expression::Expression::{self, *};
use super::expression::PrettifiedExpression;

#[derive(Debug, PartialEq)]
pub enum SyntaxError {
    WrongTokenError {
        expected: Vec<Token>,
        actual: Option<Token>,
    },
    IndexOutOfBoundsError {
        index: usize,
    },
    // The input nests deeper than the interpreter can safely handle (see crate::MAX_DEPTH).
    TooDeepError {
        limit: usize,
    },
}

impl Display for SyntaxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTokenError { expected, actual } => {
                write!(f, "Error! Expected ")?;

                for (i, token) in expected.iter().enumerate() {
                    let sep =
                        if i == 0 {
                            ""
                        } else if i < expected.len() - 1 {
                            ", "
                        } else {
                            " or "
                        };

                    write!(f, "{}{}", sep, display_token_err(token))?;
                }

                write!(f, ", recieved ")?;

                match actual {
                    Some(ref token) => write!(f, "{}", display_token_err(token)),
                    None => write!(f, "unexpected end of input")
                }
            },
            Self::IndexOutOfBoundsError { index } => write!(f, "Error! The scope index {} is out of bounds!", index),
            Self::TooDeepError { limit } => write!(f, "Error! The expression is nested more than {} levels deep, which is the interpreter's limit.", limit),
        }
    }     
}

fn display_token_err(token: &Token) -> String {
    match token {
        Dot => "'.'",
        LeftParen => "'('",
        RightParen => "')'",
        Token::Lambda => "'λ'",
        Token::Variable(_) => "variable",
        ScopeIndex(None) => "^ without a following numerical value",
        ScopeIndex(_) => "scope index",
    }.to_string()
}

use SyntaxError::*;

pub struct Parser<T: Iterator<Item = Token>> {
    iter: T,
    current: Option<Token>,
    free_vars: Vec<String>,
    bound_var_binding_levels: HashMap<String, Vec<i32>>, // name -> name-specific binding level -> absolute binding level
    free_var_binding_levels: HashMap<String, usize>,
    current_name_binding_levels: HashMap<String, usize>, // name -> name-specific binding level
    current_absolute_binding_level: i32,
    nesting: usize, // current recursion depth of term(), bounded to avoid stack overflow
}

impl<T: Iterator<Item = Token>> Parser<T> {
    pub fn new(iter: T) -> Self {
        Self {
            iter,
            current: None,
            free_vars: Vec::new(),
            bound_var_binding_levels: HashMap::new(),
            free_var_binding_levels: HashMap::new(),
            current_name_binding_levels: HashMap::new(),
            current_absolute_binding_level: 0,
            nesting: 0,
        }
    }

    fn get_next(&mut self) -> Option<Token> {
        if self.current.is_some() {
            self.current.take()
        } else {
            self.iter.next()
        }
    }

    fn var(&mut self, name: String) -> Result<Expression, SyntaxError> {
        let scope_index = self.scope_index()?;

        // if free, this will be 0. If one binder, 1, etc.
        let current_name_binding_level = self.bound_var_binding_levels.get(&name).unwrap_or(&Vec::new()).len();

        // If free var this will be 0
        let name_binding_level = match current_name_binding_level.checked_sub(scope_index) {
            Some(i) => i,
            None => return Err(
                IndexOutOfBoundsError {
                    index: scope_index,
                }
            ),
        };

        // 0 for first free var encountered, +1 for each binder entered, -1 for each subsequent free var
        let absolute_binding_level =
            match name_binding_level.checked_sub(1) {
                Some (i) => {
                    // bound var
                    let levels = self.bound_var_binding_levels
                        .entry(name)
                        .or_default();
                    *levels.get(i).unwrap()
                },
                None => {
                    // free var
                    let inverse_binding_level = self.free_var_binding_levels
                        .entry(name.clone())
                        .or_insert_with(|| {
                            // newly-encountered free var
                            self.free_vars.push(name);
                            self.free_vars.len() - 1
                        });

                    -(inverse_binding_level.clone() as i32)        
                },
            };

        let de_bruijn_index = self.current_absolute_binding_level - absolute_binding_level;

        Ok(Expression::Variable(de_bruijn_index))
    }

    // Obtain the scope index or die trying!
    fn scope_index(&mut self) -> Result<usize, SyntaxError> {
        match self.get_next() {
            Some(ScopeIndex(index)) => {
                match index {
                    Some(i) => Ok(i),
                    None => Err(WrongTokenError {
                        expected: vec![ScopeIndex(Some(usize::default()))], // Represents a proper (non-null) scope index
                        actual: Some(ScopeIndex(None)),
                    }), // Error
                }
            },
            other => {
                self.current = other;
                Ok(0)
            },
        }
    }

    fn lambda(&mut self) -> Result<Expression, SyntaxError> {
        let mut vars = Vec::new();
        let original_binding_level = self.current_absolute_binding_level;

        loop {
            match self.get_next() {
                Some(Token::Variable(name)) => {
                    // each time binding variable name is encountered, add to the binding levels
                    self.current_absolute_binding_level += 1;
                    let name_binding_level = self.current_name_binding_levels
                        .entry(name.clone())
                        .or_insert(0);
                    *name_binding_level += 1;

                    self.bound_var_binding_levels.entry(name.clone()).or_default().push(self.current_absolute_binding_level);
                    vars.push(name);
                },
                Some(Dot) if !vars.is_empty() => {
                    let mut expr = self.term()?;

                    while let Some(name) = vars.pop() {
                        // after end of lambda scope, pop
                        self.bound_var_binding_levels.get_mut(&name).unwrap().pop();
                        expr = Expression::Lambda(Box::new(expr), name);
                    }

                    self.current_absolute_binding_level = original_binding_level;
                    break Ok(expr);
                },
                unexpected => {
                    let mut expected = vec![Token::Variable(String::new())];

                    if !vars.is_empty() {
                        expected.push(Dot);
                    }

                    break Err(WrongTokenError {
                        expected,
                        actual: unexpected
                    });
                }
            }
        }
    }

     fn term(&mut self) -> Result<Expression, SyntaxError> {
        if self.nesting >= crate::MAX_DEPTH {
            return Err(TooDeepError { limit: crate::MAX_DEPTH });
        }

        self.nesting += 1;
        let result = self.term_inner();
        self.nesting -= 1;
        result
     }

     fn term_inner(&mut self) -> Result<Expression, SyntaxError> {
        match self.get_next() {
            Some(LeftParen) => self.expr_possibly_wrapped(true),
            Some(Token::Lambda) => self.lambda(),
            Some(Token::Variable(name)) => self.var(name), // Get var or var + ScopeIndex
            unexpected => Err(WrongTokenError {
                expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new())],
                actual: unexpected
            })
        }
     }

    // wrapped: indicates whether this expression is wrapped in a pair of parentheses
    fn expr_possibly_wrapped(&mut self, wrapped: bool) -> Result<Expression, SyntaxError> {
        let mut node = self.term()?;

        loop {
            match self.term() {
                Ok(expr) => {
                    node = Compound(Box::new(node), Box::new(expr))
                },
                Err(WrongTokenError {mut expected, actual}) => {
                    if expected == vec![LeftParen, Token::Lambda, Token::Variable(String::new())] {
                        // If we expected any valid term, then we can expect a right paren if wrapped
                        // and None otherwise
                        if wrapped {
                            if actual == Some(RightParen) {
                                break Ok(node);
                            }

                            expected.push(RightParen);
                        } else if actual.is_none() {
                            break Ok(node);
                        }
                    }
                    
                    break Err(WrongTokenError {
                        expected,
                        actual
                    });
                },
                Err(e) => break Err(e),
            }
        }
    }

    pub fn expr(&mut self) -> Result<PrettifiedExpression, SyntaxError> {
        Ok(PrettifiedExpression::new(
            self.expr_possibly_wrapped(false)?,
            self.free_vars.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::expression::{Expression::{self, *}, PrettifiedExpression};
    use crate::lexer::Token::{self, *};
    use crate::parser::{Parser, SyntaxError::{self, *}};


    fn test_parse_output(tokens: Vec<Token>, expected_expr: Expression, expected_free_vars: Vec<String>) {
        let mut parse = Parser::new(tokens.into_iter());
        assert_eq!(parse.expr().unwrap(), PrettifiedExpression::new(expected_expr, expected_free_vars));
    }

    fn test_parse_err(tokens: Vec<Token>, expected_err: SyntaxError) {
        let mut parse = Parser::new(tokens.into_iter());
        assert_eq!(parse.expr().unwrap_err(), expected_err);
    }

    // Proper syntax
    #[test]
    fn var() {
        test_parse_output(
            vec![Token::Variable("theQuickBrown🦊JumpsOverTheLaZ犬".to_string())],
            Expression::Variable(0),
            vec!["theQuickBrown🦊JumpsOverTheLaZ犬".to_string()]
        );
    }

    #[test]
    fn scoped_var() {
        test_parse_output(
            vec![Token::Variable("x".to_string()), ScopeIndex(Some(0))],
            Expression::Variable(0),
            vec!["x".to_string()],
        );
    }

    #[test]
    fn compound() {
        test_parse_output(
            vec![Token::Variable("💀".to_string()), Token::Variable("🌸".to_string())],
            Compound(Box::new(Expression::Variable(0)), Box::new(Expression::Variable(1))),
            vec!["💀", "🌸"].into_iter().map(String::from).collect()
        );

        test_parse_output(
            vec![
                Token::Variable("💀".to_string()), 
                Token::Variable("🌸".to_string()),
                Token::Variable("🫀".to_string()),
                Token::Variable("🦖".to_string()),
            ],
            Compound(
                Box::new(Compound(
                    Box::new(Compound(
                        Box::new(Expression::Variable(0)),
                        Box::new(Expression::Variable(1))
                    )), 
                    Box::new(Expression::Variable(2)),
                )),
                Box::new(Expression::Variable(3))
            ),
            vec!["💀", "🌸", "🫀", "🦖"].into_iter().map(String::from).collect()
        );
    }

    #[test]
    fn lambda() {
        test_parse_output(
            vec![
                Token::Lambda,
                Token::Variable("𝑥".to_string()),
                Dot,
                Token::Variable("𝑥".to_string()),
            ],
            Expression::Lambda(Box::new(Expression::Variable(0)), "𝑥".to_string()),
            vec![]
        );

        test_parse_output(
            vec![
                Token::Lambda,
                Token::Variable("𝑥".to_string()),
                Token::Variable("𝑦".to_string()),
                Dot,
                Token::Variable("𝑥".to_string()),
            ],
            Expression::Lambda(
                Box::new(Expression::Lambda(
                    Box::new(Expression::Variable(1)),
                    "𝑦".to_string()
                )),
                "𝑥".to_string()
            ),
            vec![]
        );

        test_parse_output(
            vec![
                Token::Lambda,
                Token::Variable("𝑥".to_string()),
                Token::Variable("𝑦".to_string()),
                Token::Variable("𝑧".to_string()),
                Dot,
                Token::Variable("𝑥".to_string()),
            ],
            Expression::Lambda(
                Box::new(Expression::Lambda(
                    Box::new(Expression::Lambda(
                        Box::new(Expression::Variable(2)),
                        "𝑧".to_string()
                    )),
                    "𝑦".to_string()
                )),
                "𝑥".to_string()
            ),
            vec![]
        );
    }

    #[test]
    fn successor() {
        test_parse_output(
            vec![
                Token::Lambda,
                Token::Variable("n".to_string()),
                Token::Variable("f".to_string()),
                Token::Variable("x".to_string()),
                Dot,
                LeftParen,
                Token::Variable("f".to_string()),
                LeftParen,
                Token::Variable("n".to_string()),
                Token::Variable("f".to_string()),
                Token::Variable("x".to_string()),
                RightParen,
                RightParen,
            ],
            Expression::Lambda(Box::new(
                Expression::Lambda(Box::new(
                    Expression::Lambda(Box::new(
                        Compound(
                            Box::new(Expression::Variable(1)),
                            Box::new(Compound(
                                Box::new(Compound(
                                    Box::new(Expression::Variable(2)),
                                    Box::new(Expression::Variable(1))
                                )),
                                Box::new(Expression::Variable(0))
                            ))
                        )
                    ), "x".to_string())
                ), "f".to_string())
            ), "n".to_string()),
            vec![]
        );
    }

    #[test]
    fn mix_free_bound() {
        test_parse_output(
            vec![
                Token::Lambda,
                Token::Variable("y".to_string()),
                Token::Variable("z".to_string()),
                Dot,
                LeftParen,
                Token::Variable("x".to_string()),
                Token::Variable("w".to_string()),
                Token::Variable("y".to_string()),
                Token::Variable("z".to_string()),
                RightParen,
                Token::Variable("w".to_string()),
                Token::Variable("u".to_string()),
            ],
            Compound(
                Box::new(Compound(
                    Box::new(Expression::Lambda(Box::new(
                        Expression::Lambda(Box::new(
                            Compound(
                                Box::new(Compound(
                                    Box::new(Compound(
                                        Box::new(Expression::Variable(2)),
                                        Box::new(Expression::Variable(3))
                                    )),
                                    Box::new(Expression::Variable(1))
                                )),
                                Box::new(Expression::Variable(0))
                            )
                        ), "z".to_string())
                    ), "y".to_string())),
                    Box::new(Expression::Variable(1))
                )),
                Box::new(Expression::Variable(2))
            ),
            vec!["x", "w", "u"].into_iter().map(String::from).collect()
        );
    }

    #[test]
    fn name_shadowing() {
        test_parse_output(
            vec![
                Token::Lambda,
                Token::Variable("x".to_string()),
                Token::Variable("x".to_string()),
                Dot,
                Token::Variable("x".to_string()),
            ],
            Expression::Lambda(Box::new(
                Expression::Lambda(Box::new(
                    Expression::Variable(0)
                ), "x".to_string())
            ), "x".to_string()),
            vec![]
        );
    }

    #[test]
    fn free_var_name_shadowing() {
        test_parse_output(
            vec![
                Token::Variable("x".to_string()),
                Token::Variable("y".to_string()),
                Token::Lambda,
                Token::Variable("y".to_string()),
                Dot,
                Token::Variable("y".to_string()),
                Token::Variable("y".to_string()),
            ],
            Compound(
                Box::new(Compound(
                    Box::new(Compound(
                        Box::new(Expression::Variable(0)),
                        Box::new(Expression::Variable(1))
                    )),
                    Box::new(Expression::Lambda(
                        Box::new(Expression::Variable(0)),
                        "y".to_string()
                    ))
                )),
                Box::new(Expression::Variable(1))
            ),
            vec!["x", "y"].into_iter().map(String::from).collect()
        );

        test_parse_output(
            vec![
                Token::Lambda,
                Token::Variable("y".to_string()),
                Dot,
                Token::Variable("y".to_string()),
                Token::Variable("x".to_string()),
                LeftParen,
                Token::Variable("x".to_string()),
                Token::Variable("y".to_string()),
                RightParen,
            ],
            Compound(
                Box::new(Compound(
                    Box::new(Expression::Lambda(
                        Box::new(
                            Expression::Variable(0)
                        ), "y".to_string())
                    ),
                    Box::new(Expression::Variable(0))
                )),
                Box::new(Compound(
                    Box::new(Expression::Variable(0)),
                    Box::new(Expression::Variable(1))
                ))
            ),
            vec!["x", "y"].into_iter().map(String::from).collect()
        );
    }

    #[test]
    fn free_and_bound_vars_scope_index() {
        test_parse_output(
            vec![
                Token::Lambda,
                Token::Variable("x".to_string()),
                Token::Variable("x".to_string()),
                Dot,
                Token::Variable("x".to_string()),
                ScopeIndex(Some(1)),
            ],
            Expression::Lambda(
                Box::new(
                    Expression::Lambda(
                        Box::new(Expression::Variable(1)),
                    "x".to_string())),
                "x".to_string(),
            ),
            vec![],
        );

        test_parse_output(
            vec![
                Token::Lambda,
                Token::Variable("x".to_string()),
                Token::Variable("y".to_string()),
                Token::Variable("x".to_string()),
                Token::Variable("z".to_string()),
                Dot,
                LeftParen,
                Token::Lambda,
                Token::Variable("x".to_string()),
                Dot,
                Token::Variable("x".to_string()),
                ScopeIndex(Some(2)),
                Token::Lambda,
                Token::Variable("y".to_string()),
                Dot,
                LeftParen,
                Token::Variable("y".to_string()),
                Token::Variable("y".to_string()),
                ScopeIndex(Some(1)),
                Token::Variable("y".to_string()),
                ScopeIndex(Some(2)),
                RightParen,
                RightParen,
            ],
            Expression::Lambda(
                Box::new(
                    Expression::Lambda(Box::new(
                        Expression::Lambda(Box::new(
                            Expression::Lambda(Box::new(
                                Compound(
                                    Box::new(
                                        Expression::Lambda(Box::new(
                                            Expression::Variable(4),
                                        ), "x".to_string()),
                                    ),
                                    Box::new(Expression::Lambda(Box::new(
                                        Compound(
                                            Box::new(Compound(
                                                Box::new(Expression::Variable(0)),
                                                Box::new(Expression::Variable(3)),
                                            )),
                                            Box::new(Expression::Variable(5))
                                        )),
                                    "y".to_string()))
                                )), "z".to_string())
                        ), "x".to_string())
                    ), "y".to_string())
                ), "x".to_string()),
            vec!["y".to_string()],
        );
    }

    // Improper syntax
    #[test]
    fn empty() {
        let expr = [].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!((parse.expr().unwrap_err()), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new())],
            actual: None
        });
    }

    #[test]
    fn empty_parens() {
        let expr = [LeftParen, RightParen].into_iter();
        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new())],
            actual: Some(RightParen)
        });
    }

    #[test]
    fn left_paren_heavy() {
        let expr = [
            LeftParen,
            LeftParen,
            Token::Variable("blah".to_string()),
            Token::Variable("bleh".to_string()),
            RightParen
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new()), RightParen],
            actual: None
        });
    }

    #[test]
    fn right_paren_heavy() {
        let expr = [
            Token::Variable("blah".to_string()),
            Token::Variable("bleh".to_string()),
            Token::Variable("blih".to_string()),
            RightParen
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new())],
            actual: Some(RightParen)
        });
    }

    #[test]
    fn no_closing_paren_valid_terms() {
        let expr = [
            LeftParen,
            Token::Variable("blah".to_string()),
            Token::Variable("bleh".to_string()),
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new()), RightParen],
            actual: None
        });
    }

    #[test]
    // In the case where there's no closing parenthesis but there is a malformed term, the parser should not return an
    // error indicating that a right parenthesis would have been valid.
    fn no_closing_paren_malformed_lambda() {
        let expr = [
            LeftParen,
            Token::Variable("blah".to_string()),
            Token::Variable("bleh".to_string()),
            Token::Lambda,
            Dot,
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![Token::Variable(String::new())],
            actual: Some(Dot)
        });

        let expr = [
            LeftParen,
            Token::Variable("blah".to_string()),
            Token::Variable("bleh".to_string()),
            Token::Lambda,
        ].into_iter();
        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![Token::Variable(String::new())],
            actual: None
        });
    }

    #[test]
    fn no_closing_paren_invalid_final_term_no_previous_term() {
         let expr = [
            LeftParen,
            Dot
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new())],
            actual: Some(Dot)
        });
    }

    #[test]
    fn no_closing_paren_invalid_final_term_with_previous_terms() {
        let expr = [
            LeftParen,
            Token::Variable("blah".to_string()),
            Token::Variable("bleh".to_string()),
            Dot
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new()), RightParen],
            actual: Some(Dot)
        });
    }

    #[test]
    fn correctly_wrapped_invalid_lambda_final_term() {
        let expr = [
            LeftParen,
            Token::Variable("blah".to_string()),
            Token::Variable("bleh".to_string()),
            Token::Lambda,
            RightParen
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![Token::Variable(String::new())],
            actual: Some(RightParen)
        });
    }

    #[test]
    fn abs_double_lambda() {
        let expr = [
            Token::Lambda,
            Token::Lambda,
            Token::Variable("🫀".to_string()),
            Dot,
            Token::Variable("🦖".to_string())
        ].into_iter();

        let mut parse = Parser::new(expr);

        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![Token::Variable(String::new())],
            actual: Some(Token::Lambda)
        });
    }

    #[test]
    fn abs_missing_lambda_symbol() {
        let expr = [
            Token::Variable("🫀".to_string()),
            Dot,
            Token::Variable("🦖".to_string())
        ].into_iter();

        let mut parse = Parser::new(expr);

        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new())],
            actual: Some(Dot)
        });
    }

    #[test]
    fn abs_missing_var_bind() {
        let expr = [
            Token::Lambda,
            Dot,
            Token::Variable("🦖".to_string())
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![Token::Variable(String::new())],
            actual: Some(Dot)
        });
    }

    #[test]
    fn abs_double_dot() {
        let expr = [
            Token::Lambda,
            Token::Variable("🫀".to_string()),
            Dot,
            Dot,
            Token::Variable("🦖".to_string())
        ].into_iter();

        let mut parse = Parser::new(expr);

        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new())],
            actual: Some(Dot)
        });
    }

    #[test]
    fn abs_missing_dot() {
        let expr = [
            Token::Lambda,
            Token::Variable("🫀".to_string()),
            LeftParen,
            Token::Variable("🦖".to_string()),
            RightParen
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![Token::Variable(String::new()), Dot],
            actual: Some(LeftParen)
        });        
    }

    #[test]
    fn abs_missing_body() {
        let expr = [
            Token::Lambda,
            Token::Variable("🫀".to_string()),
            Token::Variable("🦖".to_string()),
            Dot
        ].into_iter();

        let mut parse = Parser::new(expr);
        assert_eq!(parse.expr().unwrap_err(), WrongTokenError {
            expected: vec![LeftParen, Token::Lambda, Token::Variable(String::new())],
            actual: None
        });
    }

    #[test]
    fn out_of_bounds_index() {
        test_parse_err(vec![
            Token::Variable("x".to_string()),
            ScopeIndex(Some(1)),
        ], IndexOutOfBoundsError { index: 1 });

        test_parse_err(vec![
            Token::Lambda,
            Token::Variable("x".to_string()),
            Dot,
            Token::Variable("x".to_string()),
            ScopeIndex(Some(2)),
        ], IndexOutOfBoundsError { index: 2 });

        test_parse_err(vec![
            Token::Lambda,
            Token::Variable("x".to_string()),
            Token::Variable("y".to_string()),
            Dot,
            LeftParen,
            Token::Lambda,
            Token::Variable("x".to_string()),
            Dot,
            Token::Variable("x".to_string()),
            ScopeIndex(Some(3)),
            Token::Variable("y".to_string()),
            RightParen,
        ], IndexOutOfBoundsError { index: 3} );
    }

    // Error display format
    fn test_wrong_token_error_display(expected: Vec<Token>, actual: Option<Token>, expected_str: &str, actual_str: &str) {
        let err = WrongTokenError {
            expected,
            actual
        };

        assert_eq!(format!("{}", err), format!("Error! Expected {}, recieved {}", expected_str, actual_str));
    }

    fn test_index_out_of_bounds_error_display(index: usize, expected_str: &str) {
        let err = IndexOutOfBoundsError { index };

        assert_eq!(format!("{}", err), expected_str);
    }

    #[test]
    fn single_expected() {
        test_wrong_token_error_display(vec![Dot], Some(Token::Lambda), "'.'", "'λ'");
        test_wrong_token_error_display(vec![Token::Lambda], Some(Dot), "'λ'", "'.'");
        test_wrong_token_error_display(vec![LeftParen], Some(RightParen), "'('", "')'");
        test_wrong_token_error_display(vec![RightParen], Some(Token::Variable(String::new())), "')'", "variable");
        test_wrong_token_error_display(vec![Token::Variable(String::new())], Some(LeftParen), "variable", "'('");
    }

    #[test]
    fn multiple_expected() {
        test_wrong_token_error_display(vec![Dot, Token::Lambda], Some(Token::Variable(String::new())), "'.' or 'λ'", "variable");
        test_wrong_token_error_display(vec![LeftParen, Token::Lambda], Some(RightParen), "'(' or 'λ'", "')'");
        test_wrong_token_error_display(vec![Token::Lambda, RightParen], Some(Token::Variable(String::new())), "'λ' or ')'", "variable");

        test_wrong_token_error_display(vec![
                LeftParen,
                Token::Lambda,
                Token::Variable(String::new())
            ], 
            Some(RightParen), 
            "'(', 'λ' or variable", 
            "')'"
        );

        test_wrong_token_error_display(
        vec![
                LeftParen,
                Token::Lambda,
                Token::Variable(String::new()),
                Dot
            ], 
            Some(Token::Lambda), 
            "'(', 'λ', variable or '.'", 
            "'λ'"
        );

    }

    #[test]
    fn none_recieved() {
        test_wrong_token_error_display(vec![Dot], None, "'.'", "unexpected end of input");
        test_wrong_token_error_display(vec![LeftParen, Token::Lambda], None, "'(' or 'λ'", "unexpected end of input");

        test_wrong_token_error_display(vec![
            LeftParen,
            Token::Lambda,
            Token::Variable(String::new())
        ], 
        None, 
        "'(', 'λ' or variable", 
        "unexpected end of input"
        );
    }

    // TODO: Test for null scope index

    #[test]
    fn improper_index() {
        test_index_out_of_bounds_error_display(42, "Error! The scope index 42 is out of bounds!");
    }
}
