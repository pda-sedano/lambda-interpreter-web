use std::str::Chars;

#[derive(Debug, PartialEq)]
pub enum Token {
    Variable(String),
    LeftParen,
    RightParen,
    Dot,
    Lambda,
    ScopeIndex(Option<usize>),
}

// Inspiration from https://dzfrias.dev/blog/automatic-semicolon-lexer/
pub struct Lex<'a> {
    buff_chars : Chars<'a>,
    current: char,
    update: bool
}

use Token::*;

impl <'a> Lex<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            buff_chars: input.chars(),
            current: char::default(),
            update: true
        }
    }

    // Get non-whitespace
    fn get_next_non_whitespace(&mut self) -> Option<char> {
        self.buff_chars.find(|c| !c.is_whitespace())
    }

    fn luthor(&mut self) -> Option<Token> {
        if self.update {
            if let Some ( c ) = self.get_next_non_whitespace() {
                self.current = c;
            } else {
                return None;
            }
        } else {
            self.update = true;
        }

        // Match the current char
        let token = match self.current {
            '(' => LeftParen,
            ')' => RightParen,
            '.' => Dot,
            'λ' => Lambda,
            '^' => self.get_scope_index(),
            other => self.get_var(other),
        };

        Some(token)
    }

    fn get_scope_index(&mut self) -> Token {
        // A scope index is a ^ followed by a nonnegative integer
        let mut num = String::new();

        while let Some(c) = self.buff_chars.next() {
            if c.is_ascii_digit() {
                num.push(c);
            } else if c.is_whitespace() {
                // If we hit whitespace, DO NOT set current to c and update = false.
                // Otherwise it will put whitespace into the next variable name. God damn this was annoying
                break;
            } else {
                self.current = c;
                self.update = false;
                break;
            }
        }

        let index = if num.is_empty() {
            None
        } else {
            Some(num.parse().unwrap())
        };

        ScopeIndex(index)
    }

    fn get_var(&mut self, initial_char: char) -> Token {
        let mut name = initial_char.to_string();

        while let Some ( c ) = self.buff_chars.next() {
            if "().λ^".contains(c) {
                self.current = c;
                self.update = false;
                break;
            } else if c.is_whitespace() {
                break;
            } else {
                name.push(c);
            }
        }
        
        Variable(name)
    }
}

impl Iterator for Lex<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.luthor()
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Lex, Token::{self, *}};

    fn test_lex(input: &str, output: &[Token]) {
        let mut lex = Lex::new(&input);

        for token in output {
            assert_eq!(lex.next().unwrap(), *token);
        }

        assert!(lex.next().is_none());
    }

    #[test]
    fn empty_string() {
        let mut lex = Lex::new("");
        assert!(lex.next().is_none());
    }

    #[test]
    fn whitespace_skipped() {
        let mut lex = Lex::new("\u{0020} \n\t\u{200a}");

        // Whitespace should be skipped
        assert!(lex.next().is_none());
    }

    #[test]
    fn single_char_tokens() {
        let inputs = ["(", ")", ".", "λ"];
        let outputs = [LeftParen, RightParen, Dot, Lambda];

        for (input, output) in inputs.iter().zip(outputs.iter()) {
            let mut lex = Lex::new(input);

            assert_eq!(lex.next().unwrap(), *output);
            assert!(lex.next().is_none());
        }
    }

    #[test]
    fn var() {
        let name = "theQuickBrown🦊JumpsOverTheLaZ犬";

        let mut lex = Lex::new(name);

        assert_eq!(lex.next().unwrap(), Variable(name.to_string()));
        assert!(lex.next().is_none());
    }

    #[test]
    fn scope_index() {
        test_lex(
            "^42",
            &[ScopeIndex(Some(42))]
        );
    }

    #[test]
    fn combined() {
        let input = ")..( maryhadalittleλ \t\tblaaaaahh\n^-1^^3";

        test_lex(
            ")..( maryhadalittleλ \t\tblaaaaahh\n^-1^^3",
            &[
                RightParen,
                Dot,
                Dot,
                LeftParen,
                Variable("maryhadalittle".to_string()),
                Lambda,
                Variable("blaaaaahh".to_string()),
                ScopeIndex(None),
                Variable("-1".to_string()),
                ScopeIndex(None),
                ScopeIndex(Some(3)),
            ]
        );
    }

    #[test]
    fn token_symbols_embedded_in_variable() {
        let input = "aie.o)bubbaλhotep.5";

        let mut lex = Lex::new(input);

        assert_eq!(lex.next().unwrap(), Variable("aie".to_string()));
        assert_eq!(lex.next().unwrap(), Dot);
        assert_eq!(lex.next().unwrap(), Variable("o".to_string()));
        assert_eq!(lex.next().unwrap(), RightParen);
        assert_eq!(lex.next().unwrap(), Variable("bubba".to_string()));
        assert_eq!(lex.next().unwrap(), Lambda);
        assert_eq!(lex.next().unwrap(), Variable("hotep".to_string()));
        assert_eq!(lex.next().unwrap(), Dot);
        assert_eq!(lex.next().unwrap(), Variable("5".to_string()));
        assert!(lex.next().is_none());
    }
}
