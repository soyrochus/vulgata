pub mod ast;
pub mod cli;
pub mod codegen;
pub mod diagnostics;
pub mod externs;
pub mod lexer;
pub mod parser;
pub mod resolver;
pub mod runtime;
pub mod tir;
pub mod types;

use std::path::Path;

use ast::AstModule;
use diagnostics::Diagnostic;
use parser::Parser;
use resolver::Resolver;
use tir::TypedIrModule;
use types::{CheckedModule, TypeChecker};

pub type FrontendResult<T> = Result<T, Vec<Diagnostic>>;

pub fn lex_source(path: &Path, source: &str) -> FrontendResult<Vec<lexer::SpannedToken>> {
    lexer::Lexer::new(path, source).tokenize()
}

pub fn parse_source(path: &Path, source: &str) -> FrontendResult<AstModule> {
    let tokens = lex_source(path, source)?;
    Parser::new(path, tokens).parse_module()
}

pub fn check_source(path: &Path, source: &str) -> FrontendResult<CheckedModule> {
    let module = parse_source(path, source)?;
    let resolution = Resolver::new(&module).resolve()?;
    TypeChecker::new(&module, &resolution).check()
}

pub fn lower_source(path: &Path, source: &str) -> FrontendResult<TypedIrModule> {
    let checked = check_source(path, source)?;
    tir::lower_module(&checked)
}

pub fn run_source(path: &Path, source: &str) -> FrontendResult<runtime::RunResult> {
    let lowered = lower_source(path, source)?;
    let interpreter = runtime::Interpreter::new(&lowered)?;
    interpreter.run_main()
}

pub fn test_source(path: &Path, source: &str) -> FrontendResult<Vec<runtime::TestResult>> {
    let lowered = lower_source(path, source)?;
    let interpreter = runtime::Interpreter::new(&lowered)?;
    interpreter.run_tests()
}

pub fn compile_source(path: &Path, source: &str) -> FrontendResult<String> {
    let lowered = lower_source(path, source)?;
    let rust_module = codegen::lower_module(&lowered)?;
    Ok(codegen::emit_module(&rust_module))
}
