use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::Path;
use std::rc::Rc;

use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::externs::ExternRegistry;
use crate::resolver::SymbolKind;
use crate::tir::{
    TypedCallArg, TypedDecl, TypedExpr, TypedExprKind, TypedIrModule, TypedLiteral, TypedStmt,
    TypedStmtKind, TypedSymbol, TypedTarget,
};

#[derive(Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Dec(String),
    Text(String),
    None,
    Record(Rc<RefCell<BTreeMap<String, Value>>>),
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<Vec<(Value, Value)>>>),
    Callable(String),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(value) => write!(f, "Bool({value})"),
            Value::Int(value) => write!(f, "Int({value})"),
            Value::Dec(value) => write!(f, "Dec({value})"),
            Value::Text(value) => write!(f, "Text({value:?})"),
            Value::None => write!(f, "None"),
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

pub struct Interpreter<'a> {
    module: &'a TypedIrModule,
    actions: HashMap<&'a str, &'a crate::tir::TypedActionDecl>,
    tests: Vec<&'a crate::tir::TypedTestDecl>,
    globals: HashMap<String, Value>,
    externs: ExternRegistry,
}

impl<'a> Interpreter<'a> {
    pub fn new(module: &'a TypedIrModule) -> Result<Self, Vec<Diagnostic>> {
        let externs = ExternRegistry::from_module(module)?;
        Self::with_externs(module, externs)
    }

    pub fn from_path(
        module: &'a TypedIrModule,
        externs_path: &Path,
    ) -> Result<Self, Vec<Diagnostic>> {
        let externs = ExternRegistry::from_path(module, externs_path)?;
        Self::with_externs(module, externs)
    }

    pub fn with_externs(
        module: &'a TypedIrModule,
        externs: ExternRegistry,
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
            let flow = self.execute_block(&test_decl.body, &mut env, &mut failures)?;
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

    fn call_action(&self, name: &str, args: Vec<Value>) -> Result<Value, Vec<Diagnostic>> {
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
        for ((param_name, _), value) in action.params.iter().zip(args) {
            env.insert(param_name.clone(), value);
        }

        match self.execute_block(&action.body, &mut env, &mut Vec::new())? {
            ExecFlow::Continue => Ok(Value::None),
            ExecFlow::Return(value) => Ok(value),
            ExecFlow::Break | ExecFlow::ContinueLoop => Err(vec![runtime_error(
                &action.span,
                "break/continue escaped its loop boundary",
            )]),
        }
    }

    fn execute_block(
        &self,
        block: &[TypedStmt],
        env: &mut HashMap<String, Value>,
        failures: &mut Vec<TestFailure>,
    ) -> Result<ExecFlow, Vec<Diagnostic>> {
        for stmt in block {
            match self.execute_stmt(stmt, env, failures)? {
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
    ) -> Result<ExecFlow, Vec<Diagnostic>> {
        match &stmt.kind {
            TypedStmtKind::Let { name, value, .. } => {
                let evaluated = self.eval_expr(value, env)?;
                env.insert(name.clone(), evaluated);
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::Set { target, value } => {
                let new_value = self.eval_expr(value, env)?;
                self.assign_target(target, new_value, env)?;
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::If {
                branches,
                else_branch,
            } => {
                for (condition, body) in branches {
                    if self.eval_expr(condition, env)?.as_bool(&condition.span)? {
                        return self.execute_block(body, env, failures);
                    }
                }
                self.execute_block(else_branch, env, failures)
            }
            TypedStmtKind::While { condition, body } => {
                while self.eval_expr(condition, env)?.as_bool(&condition.span)? {
                    match self.execute_block(body, env, failures)? {
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
                    match self.execute_block(body, env, failures)? {
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
                let value = self.eval_expr(expr, env)?;
                if !value.as_bool(&expr.span)? {
                    failures.push(TestFailure {
                        span: expr.span.clone(),
                        message: "expect expression evaluated to false".to_string(),
                    });
                }
                Ok(ExecFlow::Continue)
            }
            TypedStmtKind::Expr(expr) => {
                let _ = self.eval_expr(expr, env)?;
                Ok(ExecFlow::Continue)
            }
        }
    }

    fn eval_expr(
        &self,
        expr: &TypedExpr,
        env: &mut HashMap<String, Value>,
    ) -> Result<Value, Vec<Diagnostic>> {
        match &expr.kind {
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
        }
    }

    fn eval_call(
        &self,
        callee: &TypedExpr,
        args: &[TypedCallArg],
        env: &mut HashMap<String, Value>,
        span: &SourceSpan,
    ) -> Result<Value, Vec<Diagnostic>> {
        if let TypedExprKind::Symbol(symbol) = &callee.kind {
            if symbol.kind == Some(SymbolKind::Record) {
                let mut fields = BTreeMap::new();
                for arg in args {
                    let name = arg.name.clone().ok_or_else(|| {
                        vec![runtime_error(
                            &arg.expr.span,
                            "record construction requires named arguments",
                        )]
                    })?;
                    fields.insert(name, self.eval_expr(&arg.expr, env)?);
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
            return Ok(value.clone());
        }
        if self.actions.contains_key(symbol.name.as_str()) {
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
            TypedTarget::Name { symbol, span, .. } => self.resolve_symbol(symbol, env, span),
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
}

#[derive(Debug, Clone, PartialEq)]
enum ExecFlow {
    Continue,
    ContinueLoop,
    Break,
    Return(Value),
}

enum TargetRef {
    Local(String),
    RecordField(Rc<RefCell<BTreeMap<String, Value>>>, String),
    ListIndex(Rc<RefCell<Vec<Value>>>, usize),
    MapEntry(Rc<RefCell<Vec<(Value, Value)>>>, Value),
}

fn eval_const_expr(expr: &TypedExpr) -> Result<Value, Vec<Diagnostic>> {
    match &expr.kind {
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
    }
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

fn runtime_error(span: &SourceSpan, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(span.clone(), Phase::Runtime, message)
}

impl Value {
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
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolver::Resolver;
    use crate::tir::lower_module;
    use crate::types::TypeChecker;

    use super::{Interpreter, Value};

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

    #[test]
    fn executes_arithmetic_and_control_flow() {
        let module = lower(
            r#"
action main() -> Int:
  let x = 4
  let y = 2
  while y > 0:
    set x = x + 1
    set y = y - 1
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
  let customer = Customer(email: "before")
  set customer.email = "after"
  return customer.email
"#,
        );
        let interpreter = Interpreter::new(&module).expect("interpreter");
        let result = interpreter.run_main().expect("run");
        assert_eq!(result.value, Value::Text("after".to_string()));
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
}
