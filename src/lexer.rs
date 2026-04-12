use std::mem;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, Phase, SourceSpan};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    IntLiteral(i64),
    DecLiteral(String),
    StringLiteral(String),
    Module,
    Import,
    As,
    From,
    Const,
    Record,
    Enum,
    Extern,
    Pure,
    Impure,
    Action,
    Test,
    Let,
    Set,
    If,
    Elif,
    Else,
    While,
    For,
    Each,
    In,
    Return,
    Break,
    Continue,
    Expect,
    And,
    Or,
    Not,
    True,
    False,
    None,
    Colon,
    Comma,
    Dot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Arrow,
    Assign,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Newline,
    Indent,
    Dedent,
    Eof,
}

impl Token {
    pub fn same_variant(&self, other: &Token) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: SourceSpan,
}

pub struct Lexer<'a> {
    path: PathBuf,
    source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(path: &Path, source: &'a str) -> Self {
        Self {
            path: path.to_path_buf(),
            source,
        }
    }

    pub fn tokenize(&self) -> Result<Vec<SpannedToken>, Vec<Diagnostic>> {
        let mut tokens = Vec::new();
        let mut diagnostics = Vec::new();
        let mut indent_stack = vec![0usize];

        for (line_index, line) in self.source.lines().enumerate() {
            let line_number = line_index + 1;
            if line.contains('\t') {
                diagnostics.push(self.lex_error(
                    line_number,
                    1,
                    "tab indentation is not supported",
                ));
                continue;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let indent = line.chars().take_while(|ch| *ch == ' ').count();
            let top_indent = *indent_stack.last().expect("indent stack always has root");
            if indent > top_indent {
                indent_stack.push(indent);
                tokens.push(self.simple_token(Token::Indent, line_number, 1));
            } else if indent < top_indent {
                while indent < *indent_stack.last().expect("indent stack always has root") {
                    indent_stack.pop();
                    tokens.push(self.simple_token(Token::Dedent, line_number, 1));
                }
                if indent != *indent_stack.last().expect("indent stack always has root") {
                    diagnostics.push(self.lex_error(
                        line_number,
                        1,
                        "indentation does not match any prior block level",
                    ));
                    continue;
                }
            }

            self.lex_line(line_number, line, indent, &mut tokens, &mut diagnostics);
            tokens.push(self.simple_token(Token::Newline, line_number, line.len() + 1));
        }

        while indent_stack.len() > 1 {
            indent_stack.pop();
            let last_line = self.source.lines().count().max(1);
            tokens.push(self.simple_token(Token::Dedent, last_line, 1));
        }
        let eof_line = self.source.lines().count().max(1);
        tokens.push(self.simple_token(Token::Eof, eof_line, 1));

        if diagnostics.is_empty() {
            Ok(tokens)
        } else {
            Err(diagnostics)
        }
    }

    fn lex_line(
        &self,
        line_number: usize,
        line: &str,
        indent: usize,
        tokens: &mut Vec<SpannedToken>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let chars: Vec<char> = line.chars().collect();
        let mut index = indent;
        while index < chars.len() {
            let column = index + 1;
            let ch = chars[index];
            if ch == '#' {
                break;
            }
            if ch.is_whitespace() {
                index += 1;
                continue;
            }

            let remaining: String = chars[index..].iter().collect();
            let token = if remaining.starts_with("->") {
                index += 2;
                Some(Token::Arrow)
            } else if remaining.starts_with("==") {
                index += 2;
                Some(Token::EqualEqual)
            } else if remaining.starts_with("!=") {
                index += 2;
                Some(Token::NotEqual)
            } else if remaining.starts_with("<=") {
                index += 2;
                Some(Token::LessEqual)
            } else if remaining.starts_with(">=") {
                index += 2;
                Some(Token::GreaterEqual)
            } else {
                match ch {
                    ':' => {
                        index += 1;
                        Some(Token::Colon)
                    }
                    ',' => {
                        index += 1;
                        Some(Token::Comma)
                    }
                    '.' => {
                        index += 1;
                        Some(Token::Dot)
                    }
                    '(' => {
                        index += 1;
                        Some(Token::LParen)
                    }
                    ')' => {
                        index += 1;
                        Some(Token::RParen)
                    }
                    '[' => {
                        index += 1;
                        Some(Token::LBracket)
                    }
                    ']' => {
                        index += 1;
                        Some(Token::RBracket)
                    }
                    '{' => {
                        index += 1;
                        Some(Token::LBrace)
                    }
                    '}' => {
                        index += 1;
                        Some(Token::RBrace)
                    }
                    '=' => {
                        index += 1;
                        Some(Token::Assign)
                    }
                    '<' => {
                        index += 1;
                        Some(Token::Less)
                    }
                    '>' => {
                        index += 1;
                        Some(Token::Greater)
                    }
                    '+' => {
                        index += 1;
                        Some(Token::Plus)
                    }
                    '-' => {
                        index += 1;
                        Some(Token::Minus)
                    }
                    '*' => {
                        index += 1;
                        Some(Token::Star)
                    }
                    '/' => {
                        index += 1;
                        Some(Token::Slash)
                    }
                    '%' => {
                        index += 1;
                        Some(Token::Percent)
                    }
                    '"' => match self.lex_string(line_number, &chars, index) {
                        Ok((value, next_index)) => {
                            index = next_index;
                            Some(Token::StringLiteral(value))
                        }
                        Err(message) => {
                            diagnostics.push(self.lex_error(line_number, column, message));
                            return;
                        }
                    },
                    _ if ch.is_ascii_digit() => {
                        let (token, next_index) = self.lex_number(&chars, index);
                        index = next_index;
                        Some(token)
                    }
                    _ if is_ident_start(ch) => {
                        let (ident, next_index) = self.lex_identifier(&chars, index);
                        index = next_index;
                        Some(keyword_or_ident(&ident))
                    }
                    _ => {
                        diagnostics.push(self.lex_error(
                            line_number,
                            column,
                            format!("unexpected character `{ch}`"),
                        ));
                        return;
                    }
                }
            };

            if let Some(token) = token {
                tokens.push(self.simple_token(token, line_number, column));
            }
        }
    }

    fn lex_string(
        &self,
        line_number: usize,
        chars: &[char],
        start_index: usize,
    ) -> Result<(String, usize), String> {
        let mut value = String::new();
        let mut index = start_index + 1;
        while index < chars.len() {
            let ch = chars[index];
            match ch {
                '"' => return Ok((value, index + 1)),
                '\\' => {
                    index += 1;
                    let escaped = chars.get(index).ok_or_else(|| {
                        format!("unterminated escape sequence on line {line_number}")
                    })?;
                    match escaped {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        other => return Err(format!("unsupported escape `\\{other}`")),
                    }
                    index += 1;
                }
                other => {
                    value.push(other);
                    index += 1;
                }
            }
        }
        Err("unterminated string literal".to_string())
    }

    fn lex_number(&self, chars: &[char], start_index: usize) -> (Token, usize) {
        let mut index = start_index;
        let mut saw_dot = false;
        while index < chars.len() {
            let ch = chars[index];
            if ch.is_ascii_digit() {
                index += 1;
            } else if ch == '.'
                && !saw_dot
                && chars
                    .get(index + 1)
                    .is_some_and(|next| next.is_ascii_digit())
            {
                saw_dot = true;
                index += 1;
            } else {
                break;
            }
        }
        let lexeme: String = chars[start_index..index].iter().collect();
        if saw_dot {
            (Token::DecLiteral(lexeme), index)
        } else {
            let value = lexeme
                .parse::<i64>()
                .expect("lexer scanned only valid integer digits");
            (Token::IntLiteral(value), index)
        }
    }

    fn lex_identifier(&self, chars: &[char], start_index: usize) -> (String, usize) {
        let mut index = start_index;
        while index < chars.len() && is_ident_continue(chars[index]) {
            index += 1;
        }
        (chars[start_index..index].iter().collect(), index)
    }

    fn simple_token(&self, token: Token, line: usize, column: usize) -> SpannedToken {
        SpannedToken {
            token,
            span: SourceSpan::for_path(&self.path, line, column),
        }
    }

    fn lex_error(&self, line: usize, column: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(
            SourceSpan::for_path(&self.path, line, column),
            Phase::Lex,
            message,
        )
    }
}

fn keyword_or_ident(ident: &str) -> Token {
    match ident {
        "module" => Token::Module,
        "import" => Token::Import,
        "as" => Token::As,
        "from" => Token::From,
        "const" => Token::Const,
        "record" => Token::Record,
        "enum" => Token::Enum,
        "extern" => Token::Extern,
        "pure" => Token::Pure,
        "impure" => Token::Impure,
        "action" => Token::Action,
        "test" => Token::Test,
        "let" => Token::Let,
        "set" => Token::Set,
        "if" => Token::If,
        "elif" => Token::Elif,
        "else" => Token::Else,
        "while" => Token::While,
        "for" => Token::For,
        "each" => Token::Each,
        "in" => Token::In,
        "return" => Token::Return,
        "break" => Token::Break,
        "continue" => Token::Continue,
        "expect" => Token::Expect,
        "and" => Token::And,
        "or" => Token::Or,
        "not" => Token::Not,
        "true" => Token::True,
        "false" => Token::False,
        "none" => Token::None,
        _ => Token::Identifier(ident.to_string()),
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Lexer, Token};

    fn kinds(tokens: &[super::SpannedToken]) -> Vec<Token> {
        tokens.iter().map(|token| token.token.clone()).collect()
    }

    #[test]
    fn lexes_keywords_operators_and_literals() {
        let source = "let value = 42 + 1.5\nexpect value != 0\n";
        let tokens = Lexer::new(Path::new("test.vg"), source)
            .tokenize()
            .expect("tokenize");
        assert!(kinds(&tokens).contains(&Token::Let));
        assert!(kinds(&tokens).contains(&Token::Expect));
        assert!(kinds(&tokens).contains(&Token::IntLiteral(42)));
        assert!(kinds(&tokens).contains(&Token::DecLiteral("1.5".to_string())));
        assert!(kinds(&tokens).contains(&Token::Plus));
        assert!(kinds(&tokens).contains(&Token::NotEqual));
    }

    #[test]
    fn lexes_strings() {
        let source = "const APP: Text = \"vulgata\"\n";
        let tokens = Lexer::new(Path::new("test.vg"), source)
            .tokenize()
            .expect("tokenize");
        assert!(kinds(&tokens).contains(&Token::StringLiteral("vulgata".to_string())));
    }

    #[test]
    fn emits_indent_and_dedent_transitions() {
        let source = "action main() -> None:\n  let x = 1\n  if true:\n    let y = 2\n  return\n";
        let tokens = Lexer::new(Path::new("test.vg"), source)
            .tokenize()
            .expect("tokenize");
        let kinds = kinds(&tokens);
        assert_eq!(
            kinds
                .iter()
                .filter(|token| **token == Token::Indent)
                .count(),
            2
        );
        assert_eq!(
            kinds
                .iter()
                .filter(|token| **token == Token::Dedent)
                .count(),
            2
        );
        assert!(matches!(kinds.last(), Some(Token::Eof)));
    }
}
