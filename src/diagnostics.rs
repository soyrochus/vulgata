use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub path: String,
    pub line: usize,
    pub column: usize,
}

impl SourceSpan {
    pub fn new(path: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            path: path.into(),
            line,
            column,
        }
    }

    pub fn for_path(path: &Path, line: usize, column: usize) -> Self {
        Self::new(path.display().to_string(), line, column)
    }
}

impl fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.path, self.line, self.column)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Lex,
    Parse,
    Resolve,
    TypeCheck,
    Lower,
    Runtime,
    Extern,
    Codegen,
    Cli,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Phase::Lex => "lex",
            Phase::Parse => "parse",
            Phase::Resolve => "resolve",
            Phase::TypeCheck => "type-check",
            Phase::Lower => "lower",
            Phase::Runtime => "runtime",
            Phase::Extern => "extern",
            Phase::Codegen => "codegen",
            Phase::Cli => "cli",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: SourceSpan,
    pub phase: Phase,
    pub message: String,
}

impl Diagnostic {
    pub fn new(span: SourceSpan, phase: Phase, message: impl Into<String>) -> Self {
        Self {
            span,
            phase,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}: {}", self.span, self.phase, self.message)
    }
}
