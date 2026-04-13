use serde_json::{Map, Value, json};

use crate::ast::{
    ActionDecl, AstModule, BinaryOp, Decl, Expr, ExprKind, Stmt, StmtKind, UnaryOp,
};

pub fn emit_metadata(module: &AstModule) -> Value {
    let mut object = Map::new();
    object.insert(
        "module".to_string(),
        Value::String(
            module
                .module_decl
                .as_ref()
                .map(|decl| decl.name.as_string())
                .unwrap_or_default(),
        ),
    );
    object.insert(
        "actions".to_string(),
        Value::Array(
            module
                .declarations
                .iter()
                .filter_map(|decl| match decl {
                    Decl::Action(action) => Some(emit_action(action)),
                    _ => None,
                })
                .collect(),
        ),
    );

    let records: Vec<_> = module
        .declarations
        .iter()
        .filter_map(|decl| match decl {
            Decl::Record(record) => {
                let fields: Vec<_> = record
                    .fields
                    .iter()
                    .filter_map(|field| {
                        field.meaning.as_ref().map(|meaning| {
                            let mut field_obj = Map::new();
                            field_obj.insert("name".to_string(), Value::String(field.name.clone()));
                            field_obj.insert(
                                "meaning".to_string(),
                                Value::String(meaning.clone()),
                            );
                            Value::Object(field_obj)
                        })
                    })
                    .collect();
                if fields.is_empty() {
                    None
                } else {
                    Some(json!({
                        "name": record.name,
                        "fields": fields,
                    }))
                }
            }
            _ => None,
        })
        .collect();
    if !records.is_empty() {
        object.insert("records".to_string(), Value::Array(records));
    }

    Value::Object(object)
}

fn emit_action(action: &ActionDecl) -> Value {
    let mut intent = None;
    let mut requires = Vec::new();
    let mut ensures = Vec::new();
    let mut steps = Vec::new();
    let mut explain = Vec::new();
    let mut examples = Vec::new();

    for stmt in &action.body {
        collect_stmt_metadata(
            stmt,
            &mut intent,
            &mut requires,
            &mut ensures,
            &mut steps,
            &mut explain,
            &mut examples,
        );
    }

    let mut object = Map::new();
    object.insert("name".to_string(), Value::String(action.name.clone()));
    if let Some(intent) = intent {
        object.insert("intent".to_string(), Value::Object(intent));
    }
    if !requires.is_empty() || !ensures.is_empty() {
        let mut contracts = Map::new();
        if !requires.is_empty() {
            contracts.insert(
                "requires".to_string(),
                Value::Array(requires.into_iter().map(Value::String).collect()),
            );
        }
        if !ensures.is_empty() {
            contracts.insert(
                "ensures".to_string(),
                Value::Array(ensures.into_iter().map(Value::String).collect()),
            );
        }
        object.insert("contracts".to_string(), Value::Object(contracts));
    }
    if !steps.is_empty() {
        object.insert(
            "steps".to_string(),
            Value::Array(steps.into_iter().map(Value::String).collect()),
        );
    }
    if !explain.is_empty() {
        object.insert(
            "explain".to_string(),
            Value::Array(explain.into_iter().map(Value::String).collect()),
        );
    }
    if !examples.is_empty() {
        object.insert("examples".to_string(), Value::Array(examples));
    }
    Value::Object(object)
}

fn collect_stmt_metadata(
    stmt: &Stmt,
    intent: &mut Option<Map<String, Value>>,
    requires: &mut Vec<String>,
    ensures: &mut Vec<String>,
    steps: &mut Vec<String>,
    explain: &mut Vec<String>,
    examples: &mut Vec<Value>,
) {
    match &stmt.kind {
        StmtKind::IntentBlock {
            goal,
            constraints,
            assumptions,
            properties,
        } => {
            let mut object = Map::new();
            if let Some(goal) = goal {
                object.insert("goal".to_string(), Value::String(goal.clone()));
            }
            if !constraints.is_empty() {
                object.insert(
                    "constraints".to_string(),
                    Value::Array(constraints.iter().cloned().map(Value::String).collect()),
                );
            }
            if !assumptions.is_empty() {
                object.insert(
                    "assumptions".to_string(),
                    Value::Array(assumptions.iter().cloned().map(Value::String).collect()),
                );
            }
            if !properties.is_empty() {
                object.insert(
                    "properties".to_string(),
                    Value::Array(properties.iter().cloned().map(Value::String).collect()),
                );
            }
            *intent = Some(object);
        }
        StmtKind::ExplainBlock { lines } => {
            explain.extend(lines.iter().cloned());
        }
        StmtKind::StepBlock { label, body } => {
            steps.push(label.clone());
            for stmt in body {
                collect_stmt_metadata(stmt, intent, requires, ensures, steps, explain, examples);
            }
        }
        StmtKind::RequiresClause { condition } => requires.push(render_expr(condition)),
        StmtKind::EnsuresClause { condition } => ensures.push(render_expr(condition)),
        StmtKind::ExampleBlock {
            name,
            inputs,
            outputs,
        } => {
            let mut object = Map::new();
            object.insert("name".to_string(), Value::String(name.clone()));
            object.insert("inputs".to_string(), render_bindings(inputs));
            object.insert("outputs".to_string(), render_bindings(outputs));
            examples.push(Value::Object(object));
        }
        StmtKind::If {
            branches,
            else_branch,
        } => {
            for branch in branches {
                for stmt in &branch.body {
                    collect_stmt_metadata(
                        stmt, intent, requires, ensures, steps, explain, examples,
                    );
                }
            }
            for stmt in else_branch {
                collect_stmt_metadata(stmt, intent, requires, ensures, steps, explain, examples);
            }
        }
        StmtKind::While { body, .. } | StmtKind::ForEach { body, .. } => {
            for stmt in body {
                collect_stmt_metadata(stmt, intent, requires, ensures, steps, explain, examples);
            }
        }
        StmtKind::Let { .. }
        | StmtKind::Var { .. }
        | StmtKind::Assign { .. }
        | StmtKind::Return(_)
        | StmtKind::Break
        | StmtKind::Continue
        | StmtKind::Expect(_)
        | StmtKind::Expr(_) => {}
    }
}

fn render_bindings(bindings: &[(String, Expr)]) -> Value {
    let mut object = Map::new();
    for (name, expr) in bindings {
        object.insert(name.clone(), Value::String(render_expr(expr)));
    }
    Value::Object(object)
}

fn render_expr(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Int(value) => value.to_string(),
        ExprKind::Dec(value) => value.clone(),
        ExprKind::String(value) => format!("{value:?}"),
        ExprKind::Bool(value) => value.to_string(),
        ExprKind::None => "none".to_string(),
        ExprKind::List(items) => format!(
            "[{}]",
            items.iter().map(render_expr).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::Map(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|(key, value)| format!("{}: {}", render_expr(key), render_expr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ExprKind::Tuple(items) => format!(
            "({})",
            items.iter().map(render_expr).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::Name(name) => name.clone(),
        ExprKind::Call { callee, args } => format!(
            "{}({})",
            render_expr(callee),
            args.iter()
                .map(|arg| render_expr(&arg.expr))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ExprKind::FieldAccess { base, field } => format!("{}.{}", render_expr(base), field),
        ExprKind::Index { base, index } => {
            format!("{}[{}]", render_expr(base), render_expr(index))
        }
        ExprKind::Unary { op, expr } => {
            let op = match op {
                UnaryOp::Negate => "-",
                UnaryOp::Not => "not ",
            };
            format!("{op}{}", render_expr(expr))
        }
        ExprKind::Binary { left, op, right } => format!(
            "{} {} {}",
            render_expr(left),
            render_binary_op(*op),
            render_expr(right)
        ),
    }
}

fn render_binary_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Modulo => "%",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::Value;

    use crate::{metadata::emit_metadata, parse_source};

    fn metadata(source: &str) -> Value {
        let module = parse_source(Path::new("test.vg"), source).expect("parse");
        emit_metadata(&module)
    }

    #[test]
    fn emits_action_with_all_semantic_layer_constructs() {
        let value = metadata(
            r#"
module sample.layers

record Customer:
  email: Text
    meaning: "primary address"

action main(value: Int) -> Int:
  intent:
    goal: "return the value"
  explain:
    "first"
  requires value > 0
  step compute:
    return value
  ensures result > 0
  example works:
    input:
      value = 1
    output:
      result = 1
"#,
        );

        let action = &value["actions"][0];
        assert_eq!(action["name"], "main");
        assert!(action.get("intent").is_some());
        assert!(action.get("contracts").is_some());
        assert!(action.get("steps").is_some());
        assert!(action.get("examples").is_some());
        assert!(value.get("records").is_some());
    }

    #[test]
    fn omits_optional_keys_when_action_has_no_semantic_layer_constructs() {
        let value = metadata(
            r#"
action main() -> Int:
  return 1
"#,
        );

        let action = value["actions"][0].as_object().expect("object");
        assert_eq!(action.get("name").unwrap(), "main");
        assert_eq!(action.len(), 1);
    }

    #[test]
    fn metadata_is_deterministic() {
        let source = r#"
action main(value: Int) -> Int:
  requires value > 0
  return value
"#;
        let first = serde_json::to_string_pretty(&metadata(source)).expect("json");
        let second = serde_json::to_string_pretty(&metadata(source)).expect("json");
        assert_eq!(first, second);
    }
}
