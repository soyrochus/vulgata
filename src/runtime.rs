use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::rc::Rc;

use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::externs::ExternRegistry;
use crate::resolver::SymbolKind;
use crate::standard_runtime::{self, StandardRuntimeAction};
use crate::tir::{
    TypedCallArg, TypedDecl, TypedExpr, TypedExprKind, TypedIrModule, TypedLiteral, TypedStmt,
    TypedStmtKind, TypedSymbol, TypedTarget,
};
use crate::types::Type;

#[derive(Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Dec(String),
    Text(String),
    None,
    ResultOk(Box<Value>),
    ResultErr(Box<Value>),
    OptionSome(Box<Value>),
    OptionNone,
    Record(Rc<RefCell<BTreeMap<String, Value>>>),
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<Vec<(Value, Value)>>>),
    Callable(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(value) => write!(f, "{value}"),
            Value::Int(value) => write!(f, "{value}"),
            Value::Dec(value) => write!(f, "{value}"),
            Value::Text(value) => write!(f, "{value:?}"),
            Value::None => write!(f, "None"),
            Value::ResultOk(value) => write!(f, "Ok({})", format_result_value(value)),
            Value::ResultErr(value) => write!(f, "Err({})", format_result_value(value)),
            Value::OptionSome(value) => write!(f, "Some({})", format_result_value(value)),
            Value::OptionNone => write!(f, "None"),
            Value::Record(fields) => write!(f, "Record({:?})", fields.borrow()),
            Value::List(items) => write!(f, "List({:?})", items.borrow()),
            Value::Map(entries) => write!(f, "Map({:?})", entries.borrow()),
            Value::Callable(name) => write!(f, "Callable({name})"),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(value) => write!(f, "Bool({value})"),
            Value::Int(value) => write!(f, "Int({value})"),
            Value::Dec(value) => write!(f, "Dec({value})"),
            Value::Text(value) => write!(f, "Text({value:?})"),
            Value::None => write!(f, "None"),
            Value::ResultOk(value) => write!(f, "Ok({})", format_result_value(value)),
            Value::ResultErr(value) => write!(f, "Err({})", format_result_value(value)),
            Value::OptionSome(value) => write!(f, "Some({})", format_result_value(value)),
            Value::OptionNone => write!(f, "None"),
            Value::Record(fields) => f.debug_tuple("Record").field(&fields.borrow()).finish(),
            Value::List(items) => f.debug_tuple("List").field(&items.borrow()).finish(),
            Value::Map(entries) => f.debug_tuple("Map").field(&entries.borrow()).finish(),
            Value::Callable(name) => write!(f, "Callable({name})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestFailure {
    pub span: SourceSpan,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Release,
    Checked,
    Debug,
    Tooling,
}

impl ExecutionMode {
    pub fn parse_cli(value: &str) -> Option<Self> {
        match value {
            "release" => Some(Self::Release),
            "checked" => Some(Self::Checked),
            "debug" => Some(Self::Debug),
            "tooling" => Some(Self::Tooling),
            _ => None,
        }
    }

    pub fn as_cli_str(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Checked => "checked",
            Self::Debug => "debug",
            Self::Tooling => "tooling",
        }
    }

    fn enforces_checkable(self) -> bool {
        matches!(self, Self::Checked | Self::Debug)
    }
}

pub struct Interpreter<'a> {
    module: &'a TypedIrModule,
    actions: HashMap<&'a str, &'a crate::tir::TypedActionDecl>,
    tests: Vec<&'a crate::tir::TypedTestDecl>,
    globals: HashMap<String, Value>,
    externs: ExternRegistry,
    pub mode: ExecutionMode,
}

impl<'a> Interpreter<'a> {
    pub fn new(module: &'a TypedIrModule) -> Result<Self, Vec<Diagnostic>> {
        let externs = ExternRegistry::from_module(module)?;
        Self::with_externs_and_mode(module, externs, ExecutionMode::Checked)
    }

    pub fn new_with_mode(
        module: &'a TypedIrModule,
        mode: ExecutionMode,
    ) -> Result<Self, Vec<Diagnostic>> {
        let externs = ExternRegistry::from_module(module)?;
        Self::with_externs_and_mode(module, externs, mode)
    }

    pub fn from_path(
        module: &'a TypedIrModule,
        externs_path: &Path,
    ) -> Result<Self, Vec<Diagnostic>> {
        let externs = ExternRegistry::from_path(module, externs_path)?;
        Self::with_externs_and_mode(module, externs, ExecutionMode::Checked)
    }

    pub fn from_path_with_mode(
        module: &'a TypedIrModule,
        externs_path: &Path,
        mode: ExecutionMode,
    ) -> Result<Self, Vec<Diagnostic>> {
        let externs = ExternRegistry::from_path(module, externs_path)?;
        Self::with_externs_and_mode(module, externs, mode)
    }

    pub fn with_externs(
        module: &'a TypedIrModule,
        externs: ExternRegistry,
    ) -> Result<Self, Vec<Diagnostic>> {
        Self::with_externs_and_mode(module, externs, ExecutionMode::Checked)
    }

    pub fn with_externs_and_mode(
        module: &'a TypedIrModule,
        externs: ExternRegistry,
        mode: ExecutionMode,
    ) -> Result<Self, Vec<Diagnostic>> {
        let mut actions = HashMap::new();
        let mut tests = Vec::new();
        let mut globals = HashMap::new();

        for decl in &module.declarations {
            match decl {
                TypedDecl::Action(action) => {
                    actions.insert(action.name.as_str(), action);
                }
                TypedDecl::Test(test_decl) => tests.push(test_decl),
                TypedDecl::Const(const_decl) => {
                    let value = eval_const_expr(&const_decl.value)?;
                    globals.insert(const_decl.name.clone(), value);
                }
                TypedDecl::Record(_) | TypedDecl::Enum(_) | TypedDecl::Extern(_) => {}
            }
        }

        Ok(Self {
            module,
            actions,
            tests,
            globals,
            externs,
            mode,
        })
    }

    pub fn run_main(&self) -> Result<RunResult, Vec<Diagnostic>> {
        let action = self.actions.get("main").ok_or_else(|| {
            vec![runtime_error(
                &self.module.span,
                "run requires an action named `main`",
            )]
        })?;
        if !action.params.is_empty() {
            return Err(vec![runtime_error(
                &action.span,
                "run only supports `main()` with no parameters",
            )]);
        }

        let value = self.call_action("main", Vec::new())?;
        Ok(RunResult { value })
    }

    pub fn run_tests(&self) -> Result<Vec<TestResult>, Vec<Diagnostic>> {
        let mut results = Vec::new();
        for test_decl in &self.tests {
            let mut env = self.globals.clone();
            let mut failures = Vec::new();
            let flow = self.execute_block(&test_decl.body, &mut env, &mut failures, true)?;
            if !matches!(flow, ExecFlow::Continue) {
                return Err(vec![runtime_error(
                    &test_decl.span,
                    "test blocks may not use return, break, or continue",
                )]);
            }
            results.push(TestResult {
                name: test_decl.name.clone(),
                passed: failures.is_empty(),
                failures,
            });
        }
        Ok(results)
    }

    pub fn eval_expression(&self, expr: &TypedExpr) -> Result<Value, Vec<Diagnostic>> {
        let mut env = self.globals.clone();
        self.eval_expr(expr, &mut env)
    }

    pub fn eval_expression_with_env(
        &self,
        expr: &TypedExpr,
        env: &HashMap<String, Value>,
    ) -> Result<Value, Vec<Diagnostic>> {
        let mut env = env.clone();
        self.eval_expr(expr, &mut env)
    }

    pub fn execute_repl_statement(
        &self,
        stmt: &TypedStmt,
        env: &mut HashMap<String, Value>,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut failures = Vec::new();
        match self.execute_stmt(stmt, env, &mut failures, false)? {
            ExecFlow::Continue => Ok(()),
            ExecFlow::Return(_) | ExecFlow::Break | ExecFlow::ContinueLoop => {
                Err(vec![runtime_error(
                    &stmt.span,
                    "repl statement may not use return, break, or continue",
                )])
            }
        }
    }

    fn call_action(&self, name: &str, args: Vec<Value>) -> Result<Value, Vec<Diagnostic>> {
        self.call_action_with_options(
            name,
            args,
            CallOptions {
                run_examples: true,
            },
        )
    }

    fn call_action_with_options(
        &self,
        name: &str,
        args: Vec<Value>,
        options: CallOptions,
    ) -> Result<Value, Vec<Diagnostic>> {
        if let Some(action) = standard_runtime::lookup_standard_runtime_name(name) {
            return self.call_standard_runtime(action, args, &self.module.span);
        }
        if self.externs.contains(name) {
            return self.externs.call(name, args, &self.module.span);
        }

        let action = self.actions.get(name).ok_or_else(|| {
            vec![runtime_error(
                &self.module.span,
                format!("unknown action `{name}`"),
            )]
        })?;

        if action.params.len() != args.len() {
            return Err(vec![runtime_error(
                &action.span,
                format!(
                    "action `{name}` expected {} arguments, found {}",
                    action.params.len(),
                    args.len()
                ),
            )]);
        }

        let mut env = self.globals.clone();
        for ((param_name, param_ty), value) in action.params.iter().zip(args) {
            env.insert(param_name.clone(), coerce_value_to_type(value, param_ty));
        }

        if self.mode.enforces_checkable() {
            self.evaluate_requires(action, &mut env)?;
        }

        let result = match self.execute_block(&action.body, &mut env, &mut Vec::new(), false)? {
            ExecFlow::Continue => Value::None,
            ExecFlow::Return(value) => value,
            ExecFlow::Break | ExecFlow::ContinueLoop => Err(vec![runtime_error(
                &action.span,
                "break/continue escaped its loop boundary",
            )])?,
        };

        if self.mode.enforces_checkable() {
            self.evaluate_ensures(action, &env, &result)?;
            if options.run_examples {
                self.evaluate_examples(action)?;
            }
        }

        Ok(coerce_value_to_type(result, action.ty.action_result()))
    }

    fn execute_block(
        &self,
        block: &[TypedStmt],
        env: &mut HashMap<String, Value>,
        failures: &mut Vec<TestFailure>,
        in_test: bool,
    ) -> Result<ExecFlow, Vec<Diagnostic>> {
        for stmt in block {
            match self.execute_stmt(stmt, env, failures, in_test)? {
                ExecFlow::Continue => {}
                flow => return Ok(flow),
            }
        }
        Ok(ExecFlow::Continue)
    }

    fn execute_stmt(
        &self,
        stmt: &TypedStmt,
        env: &mut HashMap<String, Value>,
        failures: &mut Vec<TestFailure>,
        in_test: bool,
    ) -> Result<ExecFlow, Vec<Diagnostic>> {
        match &stmt.kind {
            TypedStmtKind::IntentBlock { .. } | TypedStmtKind::ExplainBlock { .. } => {
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::StepBlock { label, body } => {
                if self.mode == ExecutionMode::Debug {
                    eprintln!("{label}");
                }
                self.execute_block(body, env, failures, in_test)
            }
            TypedStmtKind::RequiresClause(_)
            | TypedStmtKind::EnsuresClause(_)
            | TypedStmtKind::ExampleBlock { .. } => Ok(ExecFlow::Continue),
            TypedStmtKind::Let { name, ty, value } => {
                let evaluated = coerce_value_to_type(self.eval_expr(value, env)?, ty);
                env.insert(name.clone(), evaluated);
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::Var { name, ty, value } => {
                let evaluated = coerce_value_to_type(self.eval_expr(value, env)?, ty);
                env.insert(name.clone(), evaluated);
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::Assign { target, value } => {
                let new_value = coerce_value_to_type(self.eval_expr(value, env)?, target.ty());
                self.assign_target(target, new_value, env)?;
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::If {
                branches,
                else_branch,
            } => {
                for (condition, body) in branches {
                    if self.eval_expr(condition, env)?.as_bool(&condition.span)? {
                        return self.execute_block(body, env, failures, in_test);
                    }
                }
                self.execute_block(else_branch, env, failures, in_test)
            }
            TypedStmtKind::While { condition, body } => {
                while self.eval_expr(condition, env)?.as_bool(&condition.span)? {
                    match self.execute_block(body, env, failures, in_test)? {
                        ExecFlow::Continue => {}
                        ExecFlow::ContinueLoop => continue,
                        ExecFlow::Break => break,
                        flow @ ExecFlow::Return(_) => return Ok(flow),
                    }
                }
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::ForEach {
                binding,
                iterable,
                body,
            } => {
                let iterable = self.eval_expr(iterable, env)?;
                let items = match iterable {
                    Value::List(items) => items.borrow().clone(),
                    other => {
                        return Err(vec![runtime_error(
                            &stmt.span,
                            format!("for-each expects a list, found {other:?}"),
                        )]);
                    }
                };
                for item in items {
                    env.insert(binding.clone(), item);
                    match self.execute_block(body, env, failures, in_test)? {
                        ExecFlow::Continue => {}
                        ExecFlow::ContinueLoop => continue,
                        ExecFlow::Break => break,
                        flow @ ExecFlow::Return(_) => return Ok(flow),
                    }
                }
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::Return(expr) => Ok(ExecFlow::Return(
                expr.as_ref()
                    .map(|expr| self.eval_expr(expr, env))
                    .transpose()?
                    .unwrap_or(Value::None),
            )),
            TypedStmtKind::Break => Ok(ExecFlow::Break),
            TypedStmtKind::Continue => Ok(ExecFlow::ContinueLoop),
            TypedStmtKind::Expect(expr) => {
                if !self.mode.enforces_checkable() {
                    return Ok(ExecFlow::Continue);
                }
                let value = self.eval_expr(expr, env)?;
                if !value.as_bool(&expr.span)? {
                    if in_test {
                        failures.push(TestFailure {
                            span: expr.span.clone(),
                            message: "expect expression evaluated to false".to_string(),
                        });
                    } else {
                        return Err(vec![runtime_error(
                            &expr.span,
                            "expect expression evaluated to false",
                        )]);
                    }
                }
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::Expr(expr) => {
                let _ = self.eval_expr(expr, env)?;
                Ok(ExecFlow::Continue)
            }
        }
    }

    fn evaluate_requires(
        &self,
        action: &crate::tir::TypedActionDecl,
        env: &mut HashMap<String, Value>,
    ) -> Result<(), Vec<Diagnostic>> {
        for stmt in &action.body {
            if let TypedStmtKind::RequiresClause(condition) = &stmt.kind {
                let value = self.eval_expr(condition, env)?;
                if !value.as_bool(&condition.span)? {
                    return Err(vec![runtime_error(
                        &condition.span,
                        format!(
                            "requires clause failed: {}",
                            render_typed_expr(condition)
                        ),
                    )]);
                }
            }
        }
        Ok(())
    }

    fn evaluate_ensures(
        &self,
        action: &crate::tir::TypedActionDecl,
        env: &HashMap<String, Value>,
        result: &Value,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut ensures_env = env.clone();
        ensures_env.insert("result".to_string(), result.snapshot());
        for stmt in &action.body {
            if let TypedStmtKind::EnsuresClause(condition) = &stmt.kind {
                let value = self.eval_expr(condition, &mut ensures_env)?;
                if !value.as_bool(&condition.span)? {
                    return Err(vec![runtime_error(
                        &condition.span,
                        format!("ensures clause failed: {}", render_typed_expr(condition)),
                    )]);
                }
            }
        }
        Ok(())
    }

    fn evaluate_examples(
        &self,
        action: &crate::tir::TypedActionDecl,
    ) -> Result<(), Vec<Diagnostic>> {
        for stmt in &action.body {
            let TypedStmtKind::ExampleBlock {
                name,
                inputs,
                outputs,
            } = &stmt.kind
            else {
                continue;
            };

            let mut input_values = HashMap::new();
            for (input_name, expr) in inputs {
                input_values.insert(input_name.clone(), self.eval_example_expr(expr)?);
            }

            let mut args = Vec::new();
            for (param_name, _) in &action.params {
                let value = input_values.remove(param_name).ok_or_else(|| {
                    vec![runtime_error(
                        &stmt.span,
                        format!("example `{name}` is missing input `{param_name}`"),
                    )]
                })?;
                args.push(value);
            }

            if let Some(extra) = input_values.keys().next() {
                return Err(vec![runtime_error(
                    &stmt.span,
                    format!("example `{name}` provides unknown input `{extra}`"),
                )]);
            }

            let actual = self.call_action_with_options(
                &action.name,
                args,
                CallOptions {
                    run_examples: false,
                },
            )?;

            for (output_name, expr) in outputs {
                let mut expected = self.eval_example_expr(expr)?;
                if output_name == "result" {
                    expected = coerce_value_to_type(expected, action.ty.action_result());
                }
                if actual != expected {
                    return Err(vec![runtime_error(
                        &stmt.span,
                        format!(
                            "example `{name}` failed for output `{output_name}`: expected {expected:?}, found {actual:?}",
                        ),
                    )]);
                }
            }
        }
        Ok(())
    }

    fn eval_example_expr(&self, expr: &TypedExpr) -> Result<Value, Vec<Diagnostic>> {
        let mut env = self.globals.clone();
        self.eval_expr(expr, &mut env)
    }

    fn eval_expr(
        &self,
        expr: &TypedExpr,
        env: &mut HashMap<String, Value>,
    ) -> Result<Value, Vec<Diagnostic>> {
        let value = match &expr.kind {
            TypedExprKind::Literal(literal) => Ok(match literal {
                TypedLiteral::Int(value) => Value::Int(*value),
                TypedLiteral::Dec(value) => Value::Dec(value.clone()),
                TypedLiteral::String(value) => Value::Text(value.clone()),
                TypedLiteral::Bool(value) => Value::Bool(*value),
                TypedLiteral::None => Value::None,
            }),
            TypedExprKind::Symbol(symbol) => self.resolve_symbol(symbol, env, &expr.span),
            TypedExprKind::Call { callee, args } => self.eval_call(callee, args, env, &expr.span),
            TypedExprKind::FieldAccess { base, field } => {
                let base_value = self.eval_expr(base, env)?;
                match base_value {
                    Value::Record(fields) => fields.borrow().get(field).cloned().ok_or_else(|| {
                        vec![runtime_error(
                            &expr.span,
                            format!("record field `{field}` is missing"),
                        )]
                    }),
                    other => Err(vec![runtime_error(
                        &expr.span,
                        format!("field access requires a record, found {other:?}"),
                    )]),
                }
            }
            TypedExprKind::ResultIsOk { target } => {
                let target_value = self.eval_expr(target, env)?;
                match target_value {
                    Value::ResultOk(_) => Ok(Value::Bool(true)),
                    Value::ResultErr(_) => Ok(Value::Bool(false)),
                    other => Err(vec![runtime_error(
                        &expr.span,
                        format!(
                            "built-in operation `is_ok()` expected Result runtime value, found {}",
                            runtime_variant_name(&other)
                        ),
                    )]),
                }
            }
            TypedExprKind::ResultIsErr { target } => {
                let target_value = self.eval_expr(target, env)?;
                match target_value {
                    Value::ResultOk(_) => Ok(Value::Bool(false)),
                    Value::ResultErr(_) => Ok(Value::Bool(true)),
                    other => Err(vec![runtime_error(
                        &expr.span,
                        format!(
                            "built-in operation `is_err()` expected Result runtime value, found {}",
                            runtime_variant_name(&other)
                        ),
                    )]),
                }
            }
            TypedExprKind::ResultValue { target } => {
                let target_value = self.eval_expr(target, env)?;
                match target_value {
                    Value::ResultOk(value) => Ok(*value),
                    Value::ResultErr(_) => Err(vec![invalid_access_error(
                        &expr.span,
                        "InvalidResultValueAccess",
                        "value()",
                        "Err",
                    )]),
                    other => Err(vec![runtime_error(
                        &expr.span,
                        format!(
                            "built-in operation `value()` expected Result runtime value, found {}",
                            runtime_variant_name(&other)
                        ),
                    )]),
                }
            }
            TypedExprKind::ResultError { target } => {
                let target_value = self.eval_expr(target, env)?;
                match target_value {
                    Value::ResultErr(value) => Ok(*value),
                    Value::ResultOk(_) => Err(vec![invalid_access_error(
                        &expr.span,
                        "InvalidResultErrorAccess",
                        "error()",
                        "Ok",
                    )]),
                    other => Err(vec![runtime_error(
                        &expr.span,
                        format!(
                            "built-in operation `error()` expected Result runtime value, found {}",
                            runtime_variant_name(&other)
                        ),
                    )]),
                }
            }
            TypedExprKind::OptionIsSome { target } => {
                let target_value = self.eval_expr(target, env)?;
                match target_value {
                    Value::OptionSome(_) => Ok(Value::Bool(true)),
                    Value::OptionNone => Ok(Value::Bool(false)),
                    other => Err(vec![runtime_error(
                        &expr.span,
                        format!(
                            "built-in operation `is_some()` expected Option runtime value, found {}",
                            runtime_variant_name(&other)
                        ),
                    )]),
                }
            }
            TypedExprKind::OptionIsNone { target } => {
                let target_value = self.eval_expr(target, env)?;
                match target_value {
                    Value::OptionSome(_) => Ok(Value::Bool(false)),
                    Value::OptionNone => Ok(Value::Bool(true)),
                    other => Err(vec![runtime_error(
                        &expr.span,
                        format!(
                            "built-in operation `is_none()` expected Option runtime value, found {}",
                            runtime_variant_name(&other)
                        ),
                    )]),
                }
            }
            TypedExprKind::OptionValue { target } => {
                let target_value = self.eval_expr(target, env)?;
                match target_value {
                    Value::OptionSome(value) => Ok(*value),
                    Value::OptionNone => Err(vec![invalid_access_error(
                        &expr.span,
                        "InvalidOptionValueAccess",
                        "value()",
                        "None",
                    )]),
                    other => Err(vec![runtime_error(
                        &expr.span,
                        format!(
                            "built-in operation `value()` expected Option runtime value, found {}",
                            runtime_variant_name(&other)
                        ),
                    )]),
                }
            }
            TypedExprKind::Index { base, index } => {
                let base_value = self.eval_expr(base, env)?;
                let index_value = self.eval_expr(index, env)?;
                self.index_value(base_value, index_value, &expr.span)
            }
            TypedExprKind::Unary { op, expr: inner } => {
                let value = self.eval_expr(inner, env)?;
                match op {
                    crate::ast::UnaryOp::Negate => match value {
                        Value::Int(value) => Ok(Value::Int(-value)),
                        Value::Dec(value) => {
                            let parsed = value.parse::<f64>().map_err(|error| {
                                vec![runtime_error(
                                    &expr.span,
                                    format!("invalid decimal literal `{value}`: {error}"),
                                )]
                            })?;
                            Ok(Value::Dec((-parsed).to_string()))
                        }
                        other => Err(vec![runtime_error(
                            &expr.span,
                            format!("cannot negate value {other:?}"),
                        )]),
                    },
                    crate::ast::UnaryOp::Not => Ok(Value::Bool(!value.as_bool(&expr.span)?)),
                }
            }
            TypedExprKind::Binary { left, op, right } => {
                let left_value = self.eval_expr(left, env)?;
                let right_value = self.eval_expr(right, env)?;
                eval_binary(op, left_value, right_value, &expr.span)
            }
            TypedExprKind::List(items) => Ok(Value::List(Rc::new(RefCell::new(
                items
                    .iter()
                    .map(|item| self.eval_expr(item, env))
                    .collect::<Result<Vec<_>, _>>()?,
            )))),
            TypedExprKind::Map(entries) => Ok(Value::Map(Rc::new(RefCell::new(
                entries
                    .iter()
                    .map(|(key, value)| {
                        Ok((self.eval_expr(key, env)?, self.eval_expr(value, env)?))
                    })
                    .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
            )))),
            TypedExprKind::Tuple(items) => Ok(Value::List(Rc::new(RefCell::new(
                items
                    .iter()
                    .map(|item| self.eval_expr(item, env))
                    .collect::<Result<Vec<_>, _>>()?,
            )))),
        }?;

        Ok(coerce_value_to_type(value, &expr.ty))
    }

    fn eval_call(
        &self,
        callee: &TypedExpr,
        args: &[TypedCallArg],
        env: &mut HashMap<String, Value>,
        span: &SourceSpan,
    ) -> Result<Value, Vec<Diagnostic>> {
        if let TypedExprKind::Symbol(symbol) = &callee.kind {
            if symbol.kind.is_none() && symbol.name == "Some" {
                if args.len() != 1 {
                    return Err(vec![runtime_error(
                        span,
                        "`Some(...)` expects exactly one argument",
                    )]);
                }
                if args[0].name.is_some() {
                    return Err(vec![runtime_error(
                        &args[0].expr.span,
                        "`Some(...)` does not support named arguments",
                    )]);
                }
                let value = self.eval_expr(&args[0].expr, env)?;
                return Ok(Value::OptionSome(Box::new(value)));
            }
            if symbol.kind == Some(SymbolKind::Record) {
                let mut fields = BTreeMap::new();
                let record_name = match &callee.ty {
                    Type::Record(name) => Some(name.as_str()),
                    _ => None,
                };
                for arg in args {
                    let name = arg.name.clone().ok_or_else(|| {
                        vec![runtime_error(
                            &arg.expr.span,
                            "record construction requires named arguments",
                        )]
                    })?;
                    let mut value = self.eval_expr(&arg.expr, env)?;
                    if let Some(record_name) = record_name {
                        if let Some(field_ty) = self.lookup_record_field_type(record_name, &name) {
                            value = coerce_value_to_type(value, field_ty);
                        }
                    }
                    fields.insert(name, value);
                }
                return Ok(Value::Record(Rc::new(RefCell::new(fields))));
            }
        }

        let callee_value = self.eval_expr(callee, env)?;
        let evaluated_args = args
            .iter()
            .map(|arg| self.eval_expr(&arg.expr, env))
            .collect::<Result<Vec<_>, _>>()?;
        match callee_value {
            Value::Callable(name) => self.call_action(&name, evaluated_args),
            other => Err(vec![runtime_error(
                span,
                format!("call target is not callable: {other:?}"),
            )]),
        }
    }

    fn resolve_symbol(
        &self,
        symbol: &TypedSymbol,
        env: &HashMap<String, Value>,
        span: &SourceSpan,
    ) -> Result<Value, Vec<Diagnostic>> {
        if let Some(value) = env.get(&symbol.name) {
            return Ok(value.snapshot());
        }
        if self.actions.contains_key(symbol.name.as_str()) {
            return Ok(Value::Callable(symbol.name.clone()));
        }
        if symbol.kind == Some(SymbolKind::Runtime)
            || standard_runtime::lookup_standard_runtime_name(&symbol.name).is_some()
        {
            return Ok(Value::Callable(symbol.name.clone()));
        }
        if symbol.kind == Some(SymbolKind::Extern) || self.externs.contains(symbol.name.as_str()) {
            return Ok(Value::Callable(symbol.name.clone()));
        }
        Err(vec![runtime_error(
            span,
            format!("unknown runtime symbol `{}`", symbol.name),
        )])
    }

    fn lookup_record_field_type(&self, record_name: &str, field_name: &str) -> Option<&Type> {
        self.module.declarations.iter().find_map(|decl| match decl {
            TypedDecl::Record(record) if record.name == record_name => record
                .fields
                .iter()
                .find(|field| field.name == field_name)
                .map(|field| &field.ty),
            _ => None,
        })
    }

    fn assign_target(
        &self,
        target: &TypedTarget,
        value: Value,
        env: &mut HashMap<String, Value>,
    ) -> Result<(), Vec<Diagnostic>> {
        match self.resolve_target(target, env)? {
            TargetRef::Local(name) => {
                env.insert(name, value);
                Ok(())
            }
            TargetRef::RecordField(fields, field) => {
                fields.borrow_mut().insert(field, value);
                Ok(())
            }
            TargetRef::ListIndex(items, index) => {
                let mut items = items.borrow_mut();
                if index >= items.len() {
                    return Err(vec![runtime_error(
                        target.span(),
                        format!("list index {index} is out of bounds"),
                    )]);
                }
                items[index] = value;
                Ok(())
            }
            TargetRef::MapEntry(entries, key) => {
                let mut entries = entries.borrow_mut();
                if let Some((_, existing_value)) = entries
                    .iter_mut()
                    .find(|(existing_key, _)| *existing_key == key)
                {
                    *existing_value = value;
                    return Ok(());
                }
                entries.push((key, value));
                Ok(())
            }
        }
    }

    fn resolve_target(
        &self,
        target: &TypedTarget,
        env: &mut HashMap<String, Value>,
    ) -> Result<TargetRef, Vec<Diagnostic>> {
        match target {
            TypedTarget::Name { symbol, .. } => Ok(TargetRef::Local(symbol.name.clone())),
            TypedTarget::Field { base, field, .. } => match self.read_target(base, env)? {
                Value::Record(fields) => Ok(TargetRef::RecordField(fields, field.clone())),
                other => Err(vec![runtime_error(
                    target.span(),
                    format!("field mutation requires a record, found {other:?}"),
                )]),
            },
            TypedTarget::Index { base, index, .. } => {
                let key = self.eval_expr(index, env)?;
                match self.read_target(base, env)? {
                    Value::List(items) => {
                        let index = match key {
                            Value::Int(index) if index >= 0 => index as usize,
                            other => {
                                return Err(vec![runtime_error(
                                    target.span(),
                                    format!(
                                        "list index must be a non-negative Int, found {other:?}"
                                    ),
                                )]);
                            }
                        };
                        Ok(TargetRef::ListIndex(items, index))
                    }
                    Value::Map(entries) => Ok(TargetRef::MapEntry(entries, key)),
                    other => Err(vec![runtime_error(
                        target.span(),
                        format!("index mutation requires a list or map, found {other:?}"),
                    )]),
                }
            }
        }
    }

    fn read_target(
        &self,
        target: &TypedTarget,
        env: &mut HashMap<String, Value>,
    ) -> Result<Value, Vec<Diagnostic>> {
        match target {
            TypedTarget::Name { symbol, span, .. } => {
                env.get(&symbol.name).cloned().ok_or_else(|| {
                    vec![runtime_error(
                        span,
                        format!("unknown runtime symbol `{}`", symbol.name),
                    )]
                })
            }
            TypedTarget::Field {
                base, field, span, ..
            } => match self.read_target(base, env)? {
                Value::Record(fields) => fields.borrow().get(field).cloned().ok_or_else(|| {
                    vec![runtime_error(
                        span,
                        format!("record field `{field}` is missing"),
                    )]
                }),
                other => Err(vec![runtime_error(
                    span,
                    format!("field access requires a record, found {other:?}"),
                )]),
            },
            TypedTarget::Index {
                base, index, span, ..
            } => {
                let base_value = self.read_target(base, env)?;
                let index_value = self.eval_expr(index, env)?;
                self.index_value(base_value, index_value, span)
            }
        }
    }

    fn index_value(
        &self,
        base_value: Value,
        index_value: Value,
        span: &SourceSpan,
    ) -> Result<Value, Vec<Diagnostic>> {
        match base_value {
            Value::List(items) => {
                let index = match index_value {
                    Value::Int(index) if index >= 0 => index as usize,
                    other => {
                        return Err(vec![runtime_error(
                            span,
                            format!("list index must be a non-negative Int, found {other:?}"),
                        )]);
                    }
                };
                items.borrow().get(index).cloned().ok_or_else(|| {
                    vec![runtime_error(
                        span,
                        format!("list index {index} is out of bounds"),
                    )]
                })
            }
            Value::Map(entries) => entries
                .borrow()
                .iter()
                .find(|(existing_key, _)| *existing_key == index_value)
                .map(|(_, value)| value.clone())
                .ok_or_else(|| vec![runtime_error(span, "map key not found")]),
            other => Err(vec![runtime_error(
                span,
                format!("indexing requires a list or map, found {other:?}"),
            )]),
        }
    }

    fn call_standard_runtime(
        &self,
        action: StandardRuntimeAction,
        args: Vec<Value>,
        span: &SourceSpan,
    ) -> Result<Value, Vec<Diagnostic>> {
        match action {
            StandardRuntimeAction::ConsolePrint => {
                let value = expect_text_arg(&args, span, action.qualified_name(), 0)?;
                write_console(&value, false, false, span)
            }
            StandardRuntimeAction::ConsolePrintln => {
                let value = expect_text_arg(&args, span, action.qualified_name(), 0)?;
                write_console(&value, true, false, span)
            }
            StandardRuntimeAction::ConsoleEprint => {
                let value = expect_text_arg(&args, span, action.qualified_name(), 0)?;
                write_console(&value, false, true, span)
            }
            StandardRuntimeAction::ConsoleEprintln => {
                let value = expect_text_arg(&args, span, action.qualified_name(), 0)?;
                write_console(&value, true, true, span)
            }
            StandardRuntimeAction::ConsoleReadLine => read_console_line(span),
            StandardRuntimeAction::FileReadText => {
                let path = expect_text_arg(&args, span, action.qualified_name(), 0)?;
                match fs::read_to_string(&path) {
                    Ok(content) => Ok(Value::ok(Value::Text(content))),
                    Err(error) => Ok(Value::err_text(error.to_string())),
                }
            }
            StandardRuntimeAction::FileWriteText => {
                let path = expect_text_arg(&args, span, action.qualified_name(), 0)?;
                let content = expect_text_arg(&args, span, action.qualified_name(), 1)?;
                match fs::write(&path, content) {
                    Ok(()) => Ok(Value::ok(Value::None)),
                    Err(error) => Ok(Value::err_text(error.to_string())),
                }
            }
            StandardRuntimeAction::FileAppendText => {
                let path = expect_text_arg(&args, span, action.qualified_name(), 0)?;
                let content = expect_text_arg(&args, span, action.qualified_name(), 1)?;
                match fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .and_then(|mut file| file.write_all(content.as_bytes()))
                {
                    Ok(()) => Ok(Value::ok(Value::None)),
                    Err(error) => Ok(Value::err_text(error.to_string())),
                }
            }
            StandardRuntimeAction::FileExists => {
                let path = expect_text_arg(&args, span, action.qualified_name(), 0)?;
                Ok(Value::Bool(Path::new(&path).exists()))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ExecFlow {
    Continue,
    ContinueLoop,
    Break,
    Return(Value),
}

#[derive(Debug, Clone, Copy)]
struct CallOptions {
    run_examples: bool,
}

enum TargetRef {
    Local(String),
    RecordField(Rc<RefCell<BTreeMap<String, Value>>>, String),
    ListIndex(Rc<RefCell<Vec<Value>>>, usize),
    MapEntry(Rc<RefCell<Vec<(Value, Value)>>>, Value),
}

fn eval_const_expr(expr: &TypedExpr) -> Result<Value, Vec<Diagnostic>> {
    let value = match &expr.kind {
        TypedExprKind::Literal(literal) => Ok(match literal {
            TypedLiteral::Int(value) => Value::Int(*value),
            TypedLiteral::Dec(value) => Value::Dec(value.clone()),
            TypedLiteral::String(value) => Value::Text(value.clone()),
            TypedLiteral::Bool(value) => Value::Bool(*value),
            TypedLiteral::None => Value::None,
        }),
        _ => Err(vec![runtime_error(
            &expr.span,
            "top-level constants currently support literal values only",
        )]),
    }?;
    Ok(coerce_value_to_type(value, &expr.ty))
}

fn eval_binary(
    op: &crate::ast::BinaryOp,
    left: Value,
    right: Value,
    span: &SourceSpan,
) -> Result<Value, Vec<Diagnostic>> {
    use crate::ast::BinaryOp;

    match op {
        BinaryOp::Add => match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left + right)),
            (Value::Text(left), Value::Text(right)) => Ok(Value::Text(left + &right)),
            (left, right) => Err(vec![runtime_error(
                span,
                format!("unsupported `+` operands {left:?} and {right:?}"),
            )]),
        },
        BinaryOp::Subtract => match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left - right)),
            (left, right) => Err(vec![runtime_error(
                span,
                format!("unsupported `-` operands {left:?} and {right:?}"),
            )]),
        },
        BinaryOp::Multiply => match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left * right)),
            (left, right) => Err(vec![runtime_error(
                span,
                format!("unsupported `*` operands {left:?} and {right:?}"),
            )]),
        },
        BinaryOp::Divide => match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left / right)),
            (left, right) => Err(vec![runtime_error(
                span,
                format!("unsupported `/` operands {left:?} and {right:?}"),
            )]),
        },
        BinaryOp::Modulo => match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left % right)),
            (left, right) => Err(vec![runtime_error(
                span,
                format!("unsupported `%` operands {left:?} and {right:?}"),
            )]),
        },
        BinaryOp::Equal => Ok(Value::Bool(left == right)),
        BinaryOp::NotEqual => Ok(Value::Bool(left != right)),
        BinaryOp::Less => compare_ints(left, right, span, |left, right| left < right),
        BinaryOp::LessEqual => compare_ints(left, right, span, |left, right| left <= right),
        BinaryOp::Greater => compare_ints(left, right, span, |left, right| left > right),
        BinaryOp::GreaterEqual => compare_ints(left, right, span, |left, right| left >= right),
        BinaryOp::And => Ok(Value::Bool(left.as_bool(span)? && right.as_bool(span)?)),
        BinaryOp::Or => Ok(Value::Bool(left.as_bool(span)? || right.as_bool(span)?)),
    }
}

fn compare_ints(
    left: Value,
    right: Value,
    span: &SourceSpan,
    comparison: impl FnOnce(i64, i64) -> bool,
) -> Result<Value, Vec<Diagnostic>> {
    match (left, right) {
        (Value::Int(left), Value::Int(right)) => Ok(Value::Bool(comparison(left, right))),
        (left, right) => Err(vec![runtime_error(
            span,
            format!("comparison expects Int operands, found {left:?} and {right:?}"),
        )]),
    }
}

fn render_typed_expr(expr: &TypedExpr) -> String {
    match &expr.kind {
        TypedExprKind::Literal(TypedLiteral::Int(value)) => value.to_string(),
        TypedExprKind::Literal(TypedLiteral::Dec(value)) => value.clone(),
        TypedExprKind::Literal(TypedLiteral::String(value)) => format!("{value:?}"),
        TypedExprKind::Literal(TypedLiteral::Bool(value)) => value.to_string(),
        TypedExprKind::Literal(TypedLiteral::None) => "none".to_string(),
        TypedExprKind::Symbol(symbol) => symbol.name.clone(),
        TypedExprKind::Call { callee, args } => format!(
            "{}({})",
            render_typed_expr(callee),
            args.iter()
                .map(|arg| render_typed_expr(&arg.expr))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TypedExprKind::FieldAccess { base, field } => {
            format!("{}.{}", render_typed_expr(base), field)
        }
        TypedExprKind::ResultIsOk { target } => format!("{}.is_ok()", render_typed_expr(target)),
        TypedExprKind::ResultIsErr { target } => format!("{}.is_err()", render_typed_expr(target)),
        TypedExprKind::ResultValue { target } => format!("{}.value()", render_typed_expr(target)),
        TypedExprKind::ResultError { target } => format!("{}.error()", render_typed_expr(target)),
        TypedExprKind::OptionIsSome { target } => {
            format!("{}.is_some()", render_typed_expr(target))
        }
        TypedExprKind::OptionIsNone { target } => {
            format!("{}.is_none()", render_typed_expr(target))
        }
        TypedExprKind::OptionValue { target } => format!("{}.value()", render_typed_expr(target)),
        TypedExprKind::Index { base, index } => {
            format!("{}[{}]", render_typed_expr(base), render_typed_expr(index))
        }
        TypedExprKind::Unary { op, expr } => {
            let op = match op {
                crate::ast::UnaryOp::Negate => "-",
                crate::ast::UnaryOp::Not => "not ",
            };
            format!("{op}{}", render_typed_expr(expr))
        }
        TypedExprKind::Binary { left, op, right } => {
            format!(
                "{} {} {}",
                render_typed_expr(left),
                render_binary_op(*op),
                render_typed_expr(right)
            )
        }
        TypedExprKind::List(items) => format!(
            "[{}]",
            items
                .iter()
                .map(render_typed_expr)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TypedExprKind::Map(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|(key, value)| {
                    format!("{}: {}", render_typed_expr(key), render_typed_expr(value))
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TypedExprKind::Tuple(items) => format!(
            "({})",
            items
                .iter()
                .map(render_typed_expr)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn render_binary_op(op: crate::ast::BinaryOp) -> &'static str {
    match op {
        crate::ast::BinaryOp::Add => "+",
        crate::ast::BinaryOp::Subtract => "-",
        crate::ast::BinaryOp::Multiply => "*",
        crate::ast::BinaryOp::Divide => "/",
        crate::ast::BinaryOp::Modulo => "%",
        crate::ast::BinaryOp::Equal => "==",
        crate::ast::BinaryOp::NotEqual => "!=",
        crate::ast::BinaryOp::Less => "<",
        crate::ast::BinaryOp::LessEqual => "<=",
        crate::ast::BinaryOp::Greater => ">",
        crate::ast::BinaryOp::GreaterEqual => ">=",
        crate::ast::BinaryOp::And => "and",
        crate::ast::BinaryOp::Or => "or",
    }
}

fn runtime_error(span: &SourceSpan, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(span.clone(), Phase::Runtime, message)
}

fn invalid_access_error(
    span: &SourceSpan,
    error_name: &str,
    operation: &str,
    actual_variant: &str,
) -> Diagnostic {
    runtime_error(
        span,
        format!("{error_name}: cannot call `{operation}` on {actual_variant}"),
    )
}

fn runtime_variant_name(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "Bool",
        Value::Int(_) => "Int",
        Value::Dec(_) => "Dec",
        Value::Text(_) => "Text",
        Value::None => "None",
        Value::ResultOk(_) => "Ok",
        Value::ResultErr(_) => "Err",
        Value::OptionSome(_) => "Some",
        Value::OptionNone => "None",
        Value::Record(_) => "Record",
        Value::List(_) => "List",
        Value::Map(_) => "Map",
        Value::Callable(_) => "Callable",
    }
}

fn coerce_value_to_type(value: Value, expected: &Type) -> Value {
    match expected {
        Type::Option(inner) => match value {
            Value::OptionSome(value) => {
                Value::OptionSome(Box::new(coerce_value_to_type(*value, inner)))
            }
            Value::OptionNone | Value::None => Value::OptionNone,
            other => Value::OptionSome(Box::new(coerce_value_to_type(other, inner))),
        },
        _ => value,
    }
}

impl Value {
    fn ok(value: Value) -> Self {
        Self::ResultOk(Box::new(value))
    }

    fn err(value: Value) -> Self {
        Self::ResultErr(Box::new(value))
    }

    fn err_text(message: impl Into<String>) -> Self {
        Self::err(Value::Text(message.into()))
    }

    fn snapshot(&self) -> Self {
        match self {
            Value::Bool(value) => Value::Bool(*value),
            Value::Int(value) => Value::Int(*value),
            Value::Dec(value) => Value::Dec(value.clone()),
            Value::Text(value) => Value::Text(value.clone()),
            Value::None => Value::None,
            Value::ResultOk(value) => Value::ResultOk(Box::new(value.snapshot())),
            Value::ResultErr(value) => Value::ResultErr(Box::new(value.snapshot())),
            Value::OptionSome(value) => Value::OptionSome(Box::new(value.snapshot())),
            Value::OptionNone => Value::OptionNone,
            Value::Record(fields) => Value::Record(Rc::new(RefCell::new(
                fields
                    .borrow()
                    .iter()
                    .map(|(name, value)| (name.clone(), value.snapshot()))
                    .collect(),
            ))),
            Value::List(items) => Value::List(Rc::new(RefCell::new(
                items.borrow().iter().map(Value::snapshot).collect(),
            ))),
            Value::Map(entries) => Value::Map(Rc::new(RefCell::new(
                entries
                    .borrow()
                    .iter()
                    .map(|(key, value)| (key.snapshot(), value.snapshot()))
                    .collect(),
            ))),
            Value::Callable(name) => Value::Callable(name.clone()),
        }
    }

    fn as_bool(&self, span: &SourceSpan) -> Result<bool, Vec<Diagnostic>> {
        match self {
            Value::Bool(value) => Ok(*value),
            other => Err(vec![runtime_error(
                span,
                format!("expected Bool, found {other:?}"),
            )]),
        }
    }
}

impl TypedTarget {
    pub fn span(&self) -> &SourceSpan {
        match self {
            TypedTarget::Name { span, .. } => span,
            TypedTarget::Field { span, .. } => span,
            TypedTarget::Index { span, .. } => span,
        }
    }

    fn ty(&self) -> &Type {
        match self {
            TypedTarget::Name { ty, .. } => ty,
            TypedTarget::Field { ty, .. } => ty,
            TypedTarget::Index { ty, .. } => ty,
        }
    }
}

trait ActionTypeExt {
    fn action_result(&self) -> &Type;
}

impl ActionTypeExt for Type {
    fn action_result(&self) -> &Type {
        match self {
            Type::Action(action) => &action.result,
            other => panic!("expected action type, found {}", other.describe()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::diagnostics::Diagnostic;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolver::Resolver;
    use crate::tir::lower_module;
    use crate::types::TypeChecker;

    use super::{ExecutionMode, Interpreter, Value};

    fn lower(source: &str) -> crate::tir::TypedIrModule {
        let path = Path::new("test.vg");
        let tokens = Lexer::new(path, source).tokenize().expect("tokenize");
        let module = Parser::new(path, tokens).parse_module().expect("parse");
        let resolution = Resolver::new(&module).resolve().expect("resolve");
        let checked = TypeChecker::new(&module, &resolution)
            .check()
            .expect("check");
        lower_module(&checked).expect("lower")
    }

    fn run_with_mode(source: &str, mode: ExecutionMode) -> Result<Value, Vec<Diagnostic>> {
        let module = lower(source);
        let interpreter = Interpreter::new_with_mode(&module, mode).expect("interpreter");
        interpreter.run_main().map(|result| result.value)
    }

    #[test]
    fn executes_arithmetic_and_control_flow() {
        let module = lower(
            r#"
action main() -> Int:
  var x = 4
  var y = 2
  while y > 0:
    x := x + 1
    y := y - 1
  return x
"#,
        );
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        assert_eq!(result.value, Value::Int(6));
    }

    #[test]
    fn preserves_record_mutation() {
        let module = lower(
            r#"
record Customer:
  email: Text

action main() -> Text:
  var customer = Customer(email: "before")
  customer.email := "after"
  return customer.email
"#,
        );
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        assert_eq!(result.value, Value::Text("after".to_string()));
    }

    #[test]
    fn preserves_let_immutability_across_var_copies() {
        let module = lower(
            r#"
record Customer:
  email: Text

action main() -> Text:
  let frozen = Customer(email: "before")
  var current = frozen
  current.email := "after"
  return frozen.email
"#,
        );
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        assert_eq!(result.value, Value::Text("before".to_string()));
    }

    #[test]
    fn reports_failed_expectations() {
        let module = lower(
            r#"
test smoke:
  expect false
"#,
        );
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let results = interpreter.run_tests().expect("tests");
        assert!(!results[0].passed);
        assert_eq!(results[0].failures.len(), 1);
    }

    #[test]
    fn dispatches_validated_extern_calls() {
        let module = lower(
            r#"
extern action add(a: Int, b: Int) -> Int

action main() -> Int:
  return add(2, 5)
"#,
        );
        let config = r#"
[extern.add]
provider = "rust"
symbol = "builtin::add_int"
params = ["Int", "Int"]
return = "Int"
"#;
        let registry =
            crate::externs::ExternRegistry::from_config_text(&module, None, Some(config))
                .expect("registry");
        let interpreter = Interpreter::with_externs(&module, registry).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        assert_eq!(result.value, Value::Int(7));
    }

    #[test]
    fn supports_standard_runtime_console_calls() {
        let module = lower(
            r#"
action main() -> Result[None, Text]:
  return console.print("")
"#,
        );
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        assert_eq!(format!("{:?}", result.value), "Ok(())");
    }

    #[test]
    fn supports_standard_runtime_file_round_trip() {
        let dir = std::env::temp_dir().join(format!("vulgata-runtime-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("runtime-roundtrip.txt");
        let path = path.to_string_lossy().replace('\\', "\\\\");
        let module = lower(&format!(
            r#"
action main() -> Result[Text, Text]:
  let _written = file.write_text("{path}", "hello")
  return file.read_text("{path}")
"#
        ));
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        assert_eq!(format!("{:?}", result.value), "Ok(\"hello\")");
    }

    #[test]
    fn reports_standard_runtime_file_errors_explicitly() {
        let dir = std::env::temp_dir().join(format!("vulgata-runtime-{}", std::process::id()));
        let path = dir.join("missing-file.txt");
        let path = path.to_string_lossy().replace('\\', "\\\\");
        let module = lower(&format!(
            r#"
action main() -> Result[Text, Text]:
  return file.read_text("{path}")
"#
        ));
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        match result.value {
            Value::ResultErr(inner) => match *inner {
                Value::Text(message) => assert!(!message.is_empty()),
                other => panic!("unexpected error payload: {other:?}"),
            },
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn supports_result_inspection_operations() {
        let dir = std::env::temp_dir().join(format!("vulgata-result-{}", std::process::id()));
        let path = dir.join("missing-file.txt");
        let path = path.to_string_lossy().replace('\\', "\\\\");
        let module = lower(&format!(
            r#"
action main() -> Bool:
  let ok = console.print("")
  let err = file.read_text("{path}")
  return ok.is_ok() and not ok.is_err() and err.is_err() and not err.is_ok()
"#
        ));
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        assert_eq!(result.value, Value::Bool(true));
    }

    #[test]
    fn extracts_result_value_from_ok() {
        let dir = std::env::temp_dir().join(format!("vulgata-result-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("result-value.txt");
        let path = path.to_string_lossy().replace('\\', "\\\\");
        let module = lower(&format!(
            r#"
action main() -> Text:
  let _written = file.write_text("{path}", "hello")
  let result = file.read_text("{path}")
  return result.value()
"#
        ));
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        assert_eq!(result.value, Value::Text("hello".to_string()));
    }

    #[test]
    fn reports_invalid_result_value_access() {
        let dir = std::env::temp_dir().join(format!("vulgata-result-{}", std::process::id()));
        let path = dir.join("missing-file.txt");
        let path = path.to_string_lossy().replace('\\', "\\\\");
        let module = lower(&format!(
            r#"
action main() -> Text:
  let result = file.read_text("{path}")
  return result.value()
"#
        ));
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let diagnostics = interpreter.run_main().expect_err("run should fail");
        assert!(diagnostics[0].message.contains("InvalidResultValueAccess"));
        assert!(diagnostics[0].message.contains("value()"));
        assert!(diagnostics[0].message.contains("Err"));
    }

    #[test]
    fn extracts_result_error_from_err() {
        let dir = std::env::temp_dir().join(format!("vulgata-result-{}", std::process::id()));
        let path = dir.join("missing-file.txt");
        let path = path.to_string_lossy().replace('\\', "\\\\");
        let module = lower(&format!(
            r#"
action main() -> Text:
  let result = file.read_text("{path}")
  return result.error()
"#
        ));
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        match result.value {
            Value::Text(message) => assert!(!message.is_empty()),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn reports_invalid_result_error_access() {
        let module = lower(
            r#"
action main() -> Text:
  let result = console.print("")
  return result.error()
"#,
        );
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let diagnostics = interpreter.run_main().expect_err("run should fail");
        assert!(diagnostics[0].message.contains("InvalidResultErrorAccess"));
        assert!(diagnostics[0].message.contains("error()"));
        assert!(diagnostics[0].message.contains("Ok"));
    }

    #[test]
    fn normalizes_typed_option_values_and_supports_option_inspection() {
        let result = run_with_mode(
            r#"
action main() -> Bool:
  let some: Option[Int] = 10
  let none_value: Option[Int] = none
  return some.is_some() and not some.is_none() and none_value.is_none() and not none_value.is_some()
"#,
            ExecutionMode::Checked,
        )
        .expect("run");
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn extracts_option_value_from_some() {
        let result = run_with_mode(
            r#"
action main() -> Int:
  let value: Option[Int] = 10
  return value.value()
"#,
            ExecutionMode::Checked,
        )
        .expect("run");
        assert_eq!(result, Value::Int(10));
    }

    #[test]
    fn reports_invalid_option_value_access() {
        let diagnostics = run_with_mode(
            r#"
action main() -> Int:
  let value: Option[Int] = none
  return value.value()
"#,
            ExecutionMode::Checked,
        )
        .expect_err("run should fail");
        assert!(diagnostics[0].message.contains("InvalidOptionValueAccess"));
        assert!(diagnostics[0].message.contains("value()"));
        assert!(diagnostics[0].message.contains("None"));
    }

    #[test]
    fn returns_options_with_dedicated_runtime_variants() {
        let some = run_with_mode(
            r#"
action main() -> Option[Int]:
  return Some(10)
"#,
            ExecutionMode::Checked,
        )
        .expect("run");
        assert_eq!(some, Value::OptionSome(Box::new(Value::Int(10))));

        let none_value = run_with_mode(
            r#"
action main() -> Option[Int]:
  return none
"#,
            ExecutionMode::Checked,
        )
        .expect("run");
        assert_eq!(none_value, Value::OptionNone);
    }

    #[test]
    fn descriptive_constructs_do_not_change_results_in_any_mode() {
        let source = r#"
record Customer:
  email: Text
    meaning: "primary address"

action main() -> Int:
  intent:
    goal: "return one"
  explain:
    "ignore this"
  step compute:
    return 1
"#;

        for mode in [
            ExecutionMode::Release,
            ExecutionMode::Checked,
            ExecutionMode::Debug,
            ExecutionMode::Tooling,
        ] {
            let value = run_with_mode(source, mode).expect("run");
            assert_eq!(value, Value::Int(1), "mode {}", mode.as_cli_str());
        }
    }

    #[test]
    fn enforces_requires_only_in_checked_and_debug_modes() {
        let source = r#"
action main(value: Int) -> Int:
  requires value > 0
  return value
"#;

        let release = {
            let module = lower(source);
            let interpreter =
                Interpreter::new_with_mode(&module, ExecutionMode::Release).expect("interpreter");
            interpreter
                .call_action("main", vec![Value::Int(-1)])
                .expect("release run")
        };
        assert_eq!(release, Value::Int(-1));

        let module = lower(source);
        let interpreter =
            Interpreter::new_with_mode(&module, ExecutionMode::Checked).expect("interpreter");
        let diagnostics = interpreter
            .call_action("main", vec![Value::Int(-1)])
            .expect_err("checked run should fail");
        assert!(diagnostics[0].message.contains("requires clause failed"));
    }

    #[test]
    fn enforces_ensures_with_result_binding() {
        let source = r#"
action main() -> Int:
  ensures result > 0
  return 0
"#;

        let module = lower(source);
        let interpreter =
            Interpreter::new_with_mode(&module, ExecutionMode::Checked).expect("interpreter");
        let diagnostics = interpreter.run_main().expect_err("checked run should fail");
        assert!(diagnostics[0].message.contains("ensures clause failed"));
    }

    #[test]
    fn executes_example_blocks_in_checked_mode_and_skips_them_in_release() {
        let source = r#"
action main(value: Int) -> Int:
  example passing:
    input:
      value = 2
    output:
      result = 2
  return value
"#;

        let checked = {
            let module = lower(source);
            let interpreter =
                Interpreter::new_with_mode(&module, ExecutionMode::Checked).expect("interpreter");
            interpreter
                .call_action("main", vec![Value::Int(1)])
                .expect("checked example should pass")
        };
        assert_eq!(checked, Value::Int(1));

        let release = {
            let module = lower(source);
            let interpreter =
                Interpreter::new_with_mode(&module, ExecutionMode::Release).expect("interpreter");
            interpreter
                .call_action("main", vec![Value::Int(1)])
                .expect("release example should be skipped")
        };
        assert_eq!(release, Value::Int(1));
    }

    #[test]
    fn reports_failing_examples() {
        let source = r#"
action main(value: Int) -> Int:
  example failing:
    input:
      value = 2
    output:
      result = 3
  return value
"#;

        let module = lower(source);
        let interpreter =
            Interpreter::new_with_mode(&module, ExecutionMode::Checked).expect("interpreter");
        let diagnostics = interpreter
            .call_action("main", vec![Value::Int(1)])
            .expect_err("example should fail");
        assert!(diagnostics[0].message.contains("example `failing` failed"));
    }

    #[test]
    fn skips_action_expects_in_release_mode() {
        let source = r#"
action main() -> Int:
  expect false
  return 1
"#;

        let release = run_with_mode(source, ExecutionMode::Release).expect("release run");
        assert_eq!(release, Value::Int(1));
    }
}

fn format_result_value(value: &Value) -> String {
    match value {
        Value::Text(text) => format!("{text:?}"),
        Value::None => "()".to_string(),
        Value::OptionNone => "None".to_string(),
        other => format!("{other:?}"),
    }
}

fn expect_text_arg(
    args: &[Value],
    span: &SourceSpan,
    name: &str,
    index: usize,
) -> Result<String, Vec<Diagnostic>> {
    let value = args.get(index).ok_or_else(|| {
        vec![runtime_error(
            span,
            format!("runtime action `{name}` expected argument {}", index + 1),
        )]
    })?;
    match value {
        Value::Text(value) => Ok(value.clone()),
        other => Err(vec![runtime_error(
            span,
            format!(
                "runtime action `{name}` argument {} expected Text, found {other:?}",
                index + 1
            ),
        )]),
    }
}

fn write_console(
    value: &str,
    newline: bool,
    stderr: bool,
    _span: &SourceSpan,
) -> Result<Value, Vec<Diagnostic>> {
    let result = if stderr {
        let mut handle = io::stderr();
        if newline {
            writeln!(handle, "{value}")
        } else {
            write!(handle, "{value}")
        }
    } else {
        let mut handle = io::stdout();
        if newline {
            writeln!(handle, "{value}")
        } else {
            write!(handle, "{value}")
        }
    };

    match result {
        Ok(()) => Ok(Value::ok(Value::None)),
        Err(error) => Ok(Value::err_text(error.to_string())),
    }
}

fn read_console_line(_span: &SourceSpan) -> Result<Value, Vec<Diagnostic>> {
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) => Ok(Value::err_text("end of input")),
        Ok(_) => {
            while matches!(line.chars().last(), Some('\n' | '\r')) {
                line.pop();
            }
            Ok(Value::ok(Value::Text(line)))
        }
        Err(error) => Ok(Value::err_text(error.to_string())),
    }
}
