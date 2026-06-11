#![allow(dead_code)]

use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Command,
    MathDollar,
    MathDoubleDollar,
    OpenBrace,
    CloseBrace,
    Comment,
    Text,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
}

fn tokenize_with(text: &str, re: &Regex) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut last_end = 0;
    for m in re.find_iter(text) {
        if m.start() > last_end {
            tokens.push(Token {
                token_type: TokenType::Text,
                start: last_end,
                end: m.start(),
            });
        }

        let token_type = match m.as_str() {
            "$$" => TokenType::MathDoubleDollar,
            "$" => TokenType::MathDollar,
            "{" => TokenType::OpenBrace,
            "}" => TokenType::CloseBrace,
            s if s.starts_with('\\') => TokenType::Command,
            s if s.starts_with('%') => TokenType::Comment,
            _ => TokenType::Text,
        };

        tokens.push(Token {
            token_type,
            start: m.start(),
            end: m.end(),
        });

        last_end = m.end();
    }

    if last_end < text.len() {
        tokens.push(Token {
            token_type: TokenType::Text,
            start: last_end,
            end: text.len(),
        });
    }

    tokens
}

pub fn tokenize(text: &str) -> Vec<Token> {
    let re = Regex::new(r"(?m)\\([a-zA-Z]+|[^a-zA-Z])|%.*$|\$\$|\$|[{}]").unwrap();
    tokenize_with(text, &re)
}

pub fn tokenize_line(text: &str) -> Vec<Token> {
    let re = Regex::new(r"\\([a-zA-Z]+|[^a-zA-Z])|%.*$|\$\$|\$|[{}]").unwrap();
    tokenize_with(text, &re)
}
