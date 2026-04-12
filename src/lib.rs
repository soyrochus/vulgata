pub mod ast;
pub mod cli;
pub mod codegen;
pub mod diagnostics;
pub mod externs;
pub mod lexer;
pub mod parser;
pub mod repl;
pub mod resolver;
pub mod runtime;
pub mod standard_runtime;
pub mod tir;
pub mod types;

use std::path::Path;

use ast::{AstModule, Expr, Stmt};
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

pub fn parse_expression_source(path: &Path, source: &str) -> FrontendResult<Expr> {
    let tokens = lex_source(path, source)?;
    Parser::new(path, tokens).parse_expression()
}

pub fn parse_statement_source(path: &Path, source: &str) -> FrontendResult<Stmt> {
    let mut owned = source.to_string();
    if !owned.ends_with('\n') {
        owned.push('\n');
    }
    let tokens = lex_source(path, &owned)?;
    Parser::new(path, tokens).parse_statement()
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

pub fn eval_expression_source(
    path: &Path,
    module_source: &str,
    expression_source: &str,
) -> FrontendResult<runtime::Value> {
    let checked = check_source(path, module_source)?;
    let expression = parse_expression_source(path, expression_source)?;
    let checked_expr = types::check_expression(&checked, &expression)?;
    let typed_expr = tir::lower_expression(&checked, &expression, &checked_expr.expr_types)?;
    let lowered = tir::lower_module(&checked)?;
    let interpreter = runtime::Interpreter::new(&lowered)?;
    interpreter.eval_expression(&typed_expr)
}

pub fn eval_expression_source_with_bindings(
    path: &Path,
    module_source: &str,
    expression_source: &str,
    bindings: &std::collections::HashMap<String, types::ReplBinding>,
    values: &std::collections::HashMap<String, runtime::Value>,
) -> FrontendResult<runtime::Value> {
    let checked = check_source(path, module_source)?;
    let expression = parse_expression_source(path, expression_source)?;
    let checked_expr = types::check_expression_with_bindings(&checked, &expression, bindings)?;
    let typed_expr = tir::lower_expression(&checked, &expression, &checked_expr.expr_types)?;
    let lowered = tir::lower_module(&checked)?;
    let interpreter = runtime::Interpreter::new(&lowered)?;

    let env = values.clone();
    for (name, binding) in bindings {
        if !env.contains_key(name) {
            return Err(vec![Diagnostic::new(
                diagnostics::SourceSpan::for_path(path, 1, 1),
                diagnostics::Phase::Cli,
                format!(
                    "missing runtime value for repl binding `{name}` ({})",
                    binding.ty.describe()
                ),
            )]);
        }
    }
    interpreter.eval_expression_with_env(&typed_expr, &env)
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
