use std::collections::HashSet;

use crate::ast::{BinaryOp, UnaryOp};
use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::resolver::SymbolKind;
use crate::standard_runtime::{self, StandardRuntimeAction};
use crate::tir::{
    TypedBindingPattern, TypedBindingPatternKind, TypedCallArg, TypedDecl, TypedExpr,
    TypedExprKind, TypedIrModule, TypedLiteral, TypedMatchArm, TypedPattern, TypedPatternKind,
    TypedStmt, TypedStmtKind, TypedSymbol, TypedTarget,
};
use crate::types::Type;

#[derive(Debug, Clone, PartialEq)]
pub struct RustModule {
    pub uses: Vec<String>,
    pub items: Vec<RustItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RustItem {
    Raw(String),
    Struct {
        name: String,
        fields: Vec<(String, RustType)>,
    },
    Enum {
        name: String,
        variants: Vec<RustEnumVariant>,
    },
    Function {
        name: String,
        params: Vec<RustParam>,
        result: RustType,
        body: Vec<RustStmt>,
        is_pub: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustParam {
    pub name: String,
    pub ty: RustType,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustEnumVariant {
    pub name: String,
    pub fields: Vec<(String, RustType)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RustStmt {
    Let {
        name: String,
        ty: Option<RustType>,
        value: RustExpr,
        mutable: bool,
    },
    Assign {
        target: RustExpr,
        value: RustExpr,
    },
    If {
        condition: RustExpr,
        then_body: Vec<RustStmt>,
        else_body: Vec<RustStmt>,
    },
    While {
        condition: RustExpr,
        body: Vec<RustStmt>,
    },
    ForEach {
        binding: String,
        iterable: RustExpr,
        body: Vec<RustStmt>,
    },
    Return(Option<RustExpr>),
    Break,
    Continue,
    Expr(RustExpr),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RustExpr {
    Raw(String),
    Call {
        callee: String,
        args: Vec<RustExpr>,
    },
    MethodCall {
        receiver: Box<RustExpr>,
        method: String,
        args: Vec<RustExpr>,
    },
    Field {
        base: Box<RustExpr>,
        field: String,
    },
    Binary {
        left: Box<RustExpr>,
        op: String,
        right: Box<RustExpr>,
    },
    Unary {
        op: String,
        expr: Box<RustExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RustType {
    Unit,
    Path(String),
}

impl RustExpr {
    fn raw(value: impl Into<String>) -> Self {
        Self::Raw(value.into())
    }
}

pub fn lower_module(module: &TypedIrModule) -> Result<RustModule, Vec<Diagnostic>> {
    let mut items = vec![RustItem::Raw(SUPPORT_MODULE.to_string())];

    for decl in &module.declarations {
        match decl {
            TypedDecl::Record(record) => {
                let mut fields = Vec::new();
                for field in &record.fields {
                    fields.push((
                        field.name.clone(),
                        lower_storage_type(&field.ty, &record.span)?,
                    ));
                }
                items.push(RustItem::Struct {
                    name: record.name.clone(),
                    fields,
                });
            }
            TypedDecl::Enum(enum_decl) => {
                items.push(RustItem::Enum {
                    name: enum_decl.name.clone(),
                    variants: enum_decl
                        .variants
                        .iter()
                        .map(|variant| {
                            Ok(RustEnumVariant {
                                name: variant.name.clone(),
                                fields: variant
                                    .fields
                                    .iter()
                                    .map(|(field_name, field_ty)| {
                                        Ok((
                                            field_name.clone(),
                                            lower_storage_type(field_ty, &enum_decl.span)?,
                                        ))
                                    })
                                    .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
                            })
                        })
                        .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
                });
            }
            TypedDecl::Const(const_decl) => {
                items.push(RustItem::Function {
                    name: const_fn_name(&const_decl.name),
                    params: Vec::new(),
                    result: lower_storage_type(&const_decl.ty, &const_decl.span)?,
                    body: vec![RustStmt::Return(Some(lower_expr(&const_decl.value)?))],
                    is_pub: false,
                });
            }
            TypedDecl::Extern(extern_decl) => {
                return Err(vec![codegen_error(
                    &extern_decl.span,
                    "compile does not yet support extern declarations without explicit compile-time bindings",
                )]);
            }
            TypedDecl::Action(action) => {
                items.push(RustItem::Function {
                    name: action_fn_name(&action.name),
                    params: action
                        .params
                        .iter()
                        .map(|(name, ty)| {
                            Ok(RustParam {
                                name: name.clone(),
                                ty: lower_storage_type(ty, &action.span)?,
                                mutable: false,
                            })
                        })
                        .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
                    result: lower_storage_type(&action.ty.action_result().clone(), &action.span)?,
                    body: lower_action_body(action)?,
                    is_pub: false,
                });
            }
            TypedDecl::Test(test_decl) => {
                items.push(RustItem::Function {
                    name: test_fn_name(&test_decl.name),
                    params: Vec::new(),
                    result: RustType::Path("Vec<String>".to_string()),
                    body: lower_test_body(test_decl)?,
                    is_pub: false,
                });
            }
        }
    }

    items.push(entrypoint_item(module)?);

    Ok(RustModule {
        uses: Vec::new(),
        items,
    })
}

pub fn emit_module(module: &RustModule) -> String {
    let mut output = String::new();
    for item in &module.uses {
        output.push_str(item);
        output.push('\n');
    }
    output.push('\n');
    for item in &module.items {
        render_item(item, 0, &mut output);
        output.push('\n');
    }
    output
}

fn lower_action_body(
    action: &crate::tir::TypedActionDecl,
) -> Result<Vec<RustStmt>, Vec<Diagnostic>> {
    let reassigned = collect_direct_name_assignments(&action.body);
    let mut body = Vec::new();
    for stmt in &action.body {
        body.push(lower_stmt(
            stmt,
            false,
            &reassigned,
            action.ty.action_result(),
        )?);
    }

    if block_may_fall_through(&body) && *action.ty.action_result() == Type::None {
        body.push(RustStmt::Raw("()".to_string()));
    } else if block_may_fall_through(&body) {
        body.push(RustStmt::Raw(format!(
            "panic!(\"action `{}` fell through without returning\");",
            action.name
        )));
    }
    Ok(body)
}

fn lower_test_body(
    test_decl: &crate::tir::TypedTestDecl,
) -> Result<Vec<RustStmt>, Vec<Diagnostic>> {
    let reassigned = collect_direct_name_assignments(&test_decl.body);
    let mut body = vec![RustStmt::Let {
        name: "failures".to_string(),
        ty: Some(RustType::Path("Vec<String>".to_string())),
        value: RustExpr::raw("Vec::new()"),
        mutable: true,
    }];
    for stmt in &test_decl.body {
        body.push(lower_stmt(stmt, true, &reassigned, &Type::None)?);
    }
    body.push(RustStmt::Return(Some(RustExpr::raw("failures"))));
    Ok(body)
}

fn lower_stmt(
    stmt: &TypedStmt,
    in_test: bool,
    reassigned: &HashSet<String>,
    return_type: &Type,
) -> Result<RustStmt, Vec<Diagnostic>> {
    let lowered = match &stmt.kind {
        TypedStmtKind::Let { pattern, ty, value } => {
            lower_declaration_binding(pattern, ty, value, false, reassigned, &stmt.span)?
        }
        TypedStmtKind::Var { pattern, ty, value } => {
            lower_declaration_binding(pattern, ty, value, true, reassigned, &stmt.span)?
        }
        TypedStmtKind::Assign { target, value } => lower_assignment(target, value)?,
        TypedStmtKind::If {
            branches,
            else_branch,
        } => lower_if(branches, else_branch, in_test, reassigned, return_type)?,
        TypedStmtKind::While { condition, body } => RustStmt::While {
            condition: lower_expr(condition)?,
            body: lower_block(body, in_test, reassigned, return_type)?,
        },
        TypedStmtKind::Match { scrutinee, arms } => {
            lower_match_stmt(scrutinee, arms, in_test, reassigned, return_type)?
        }
        TypedStmtKind::ForEach {
            binding,
            iterable,
            body,
        } => RustStmt::ForEach {
            binding: binding.clone(),
            iterable: lower_expr(iterable)?,
            body: lower_block(body, in_test, reassigned, return_type)?,
        },
        TypedStmtKind::Return(expr) => RustStmt::Return(
            expr.as_ref()
                .map(|expr| -> Result<RustExpr, Vec<Diagnostic>> {
                    Ok(coerce_expr_to_type(
                        lower_expr(expr)?,
                        &expr.ty,
                        return_type,
                    ))
                })
                .transpose()?,
        ),
        TypedStmtKind::Break => RustStmt::Break,
        TypedStmtKind::Continue => RustStmt::Continue,
        TypedStmtKind::IntentBlock { .. }
        | TypedStmtKind::ExplainBlock { .. }
        | TypedStmtKind::StepBlock { .. }
        | TypedStmtKind::RequiresClause(_)
        | TypedStmtKind::EnsuresClause(_)
        | TypedStmtKind::ExampleBlock { .. } => {
            return Err(vec![codegen_error(
                &stmt.span,
                "compile does not yet support semantic-layer constructs",
            )]);
        }
        TypedStmtKind::Expect(expr) => {
            if !in_test {
                return Err(vec![codegen_error(
                    &stmt.span,
                    "`expect` can only be compiled inside test bodies",
                )]);
            }
            RustStmt::Raw(format!(
                "if !{} {{ failures.push({:?}.to_string()); }}",
                render_expr(&lower_expr(expr)?),
                format!(
                    "{span} expect expression evaluated to false",
                    span = expr.span
                )
            ))
        }
        TypedStmtKind::Expr(expr) => RustStmt::Expr(lower_expr(expr)?),
    };
    Ok(lowered)
}

fn lower_block(
    block: &[TypedStmt],
    in_test: bool,
    reassigned: &HashSet<String>,
    return_type: &Type,
) -> Result<Vec<RustStmt>, Vec<Diagnostic>> {
    block
        .iter()
        .map(|stmt| lower_stmt(stmt, in_test, reassigned, return_type))
        .collect()
}

fn lower_if(
    branches: &[(TypedExpr, Vec<TypedStmt>)],
    else_branch: &[TypedStmt],
    in_test: bool,
    reassigned: &HashSet<String>,
    return_type: &Type,
) -> Result<RustStmt, Vec<Diagnostic>> {
    if branches.is_empty() {
        return Ok(RustStmt::Raw("{}".to_string()));
    }

    let (condition, body) = &branches[0];
    let mut statement = RustStmt::If {
        condition: lower_expr(condition)?,
        then_body: lower_block(body, in_test, reassigned, return_type)?,
        else_body: lower_block(else_branch, in_test, reassigned, return_type)?,
    };

    for (condition, body) in branches.iter().skip(1).rev() {
        statement = RustStmt::If {
            condition: lower_expr(condition)?,
            then_body: lower_block(body, in_test, reassigned, return_type)?,
            else_body: vec![statement],
        };
    }

    Ok(statement)
}

fn collect_direct_name_assignments(block: &[TypedStmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    collect_direct_name_assignments_into(block, &mut names);
    names
}

fn collect_direct_name_assignments_into(block: &[TypedStmt], names: &mut HashSet<String>) {
    for stmt in block {
        match &stmt.kind {
            TypedStmtKind::Assign {
                target: TypedTarget::Name { symbol, .. },
                ..
            } => {
                names.insert(symbol.name.clone());
            }
            TypedStmtKind::If {
                branches,
                else_branch,
            } => {
                for (_, body) in branches {
                    collect_direct_name_assignments_into(body, names);
                }
                collect_direct_name_assignments_into(else_branch, names);
            }
            TypedStmtKind::While { body, .. } | TypedStmtKind::ForEach { body, .. } => {
                collect_direct_name_assignments_into(body, names);
            }
            TypedStmtKind::Match { arms, .. } => {
                for arm in arms {
                    collect_direct_name_assignments_into(&arm.body, names);
                }
            }
            TypedStmtKind::StepBlock { body, .. } => {
                collect_direct_name_assignments_into(body, names);
            }
            TypedStmtKind::Let { .. }
            | TypedStmtKind::Var { .. }
            | TypedStmtKind::Assign { .. }
            | TypedStmtKind::Return(_)
            | TypedStmtKind::Break
            | TypedStmtKind::Continue
            | TypedStmtKind::IntentBlock { .. }
            | TypedStmtKind::ExplainBlock { .. }
            | TypedStmtKind::RequiresClause(_)
            | TypedStmtKind::EnsuresClause(_)
            | TypedStmtKind::ExampleBlock { .. }
            | TypedStmtKind::Expect(_)
            | TypedStmtKind::Expr(_) => {}
        }
    }
}

fn lower_match_stmt(
    scrutinee: &TypedExpr,
    arms: &[TypedMatchArm],
    in_test: bool,
    reassigned: &HashSet<String>,
    return_type: &Type,
) -> Result<RustStmt, Vec<Diagnostic>> {
    let scrutinee_expr = render_expr(&lower_expr(scrutinee)?);
    let mut lines = vec![format!("match {scrutinee_expr} {{")];
    for arm in arms {
        let pattern = render_pattern(&arm.pattern)?;
        lines.push(format!("    {pattern} => {{"));
        for stmt in lower_block(&arm.body, in_test, reassigned, return_type)? {
            let mut rendered = String::new();
            render_stmt(&stmt, 2, &mut rendered);
            for line in rendered.lines() {
                lines.push(line.to_string());
            }
        }
        lines.push("    },".to_string());
    }
    lines.push("    _ => panic!(\"NonExhaustiveMatch\"),".to_string());
    lines.push("}".to_string());
    Ok(RustStmt::Raw(lines.join("\n")))
}

fn lower_assignment(target: &TypedTarget, value: &TypedExpr) -> Result<RustStmt, Vec<Diagnostic>> {
    let value = coerce_expr_to_type(lower_expr(value)?, &value.ty, target.ty());
    match target {
        TypedTarget::Name { symbol, .. } => Ok(RustStmt::Assign {
            target: RustExpr::raw(symbol.name.clone()),
            value,
        }),
        TypedTarget::Field {
            base, field, span, ..
        } => {
            let base_expr = lower_target_value(base)?;
            match &base.ty() {
                Type::Record(_) => Ok(RustStmt::Raw(format!(
                    "{}.borrow_mut().{} = {};",
                    render_expr(&base_expr),
                    field,
                    render_expr(&value)
                ))),
                other => Err(vec![codegen_error(
                    span,
                    format!("unsupported field mutation base `{}`", other.describe()),
                )]),
            }
        }
        TypedTarget::Index {
            base, index, span, ..
        } => {
            let base_expr = lower_target_value(base)?;
            let index_expr = lower_expr(index)?;
            let base_code = render_expr(&base_expr);
            let index_code = render_expr(&index_expr);
            match &base.ty() {
                Type::List(_) | Type::Set(_) => Ok(RustStmt::Raw(format!(
                    "support::list_set(&{}, {}, {});",
                    base_code,
                    index_code,
                    render_expr(&value)
                ))),
                Type::Map(_, _) => Ok(RustStmt::Raw(format!(
                    "support::map_set(&{}, {}, {});",
                    base_code,
                    index_code,
                    render_expr(&value)
                ))),
                other => Err(vec![codegen_error(
                    span,
                    format!("unsupported index mutation base `{}`", other.describe()),
                )]),
            }
        }
    }
}

fn lower_declaration_binding(
    pattern: &TypedBindingPattern,
    whole_type: &Type,
    value: &TypedExpr,
    is_var: bool,
    reassigned: &HashSet<String>,
    span: &SourceSpan,
) -> Result<RustStmt, Vec<Diagnostic>> {
    match &pattern.kind {
        TypedBindingPatternKind::Name(binding) => Ok(RustStmt::Let {
            name: binding.name.clone(),
            ty: Some(lower_storage_type(&binding.ty, span)?),
            value: coerce_expr_to_type(lower_expr(value)?, &value.ty, whole_type),
            mutable: is_var && reassigned.contains(&binding.name),
        }),
        TypedBindingPatternKind::Tuple(bindings) => {
            let temp_name = destructure_temp_name(span);
            let mut lines = vec![format!(
                "let {temp_name}: {} = {};",
                render_type(&lower_storage_type(whole_type, span)?),
                render_expr(&coerce_expr_to_type(lower_expr(value)?, &value.ty, whole_type))
            )];
            for (index, binding) in bindings.iter().enumerate() {
                lines.push(render_destructure_binding(
                    &binding.name,
                    &binding.ty,
                    &format!("{temp_name}.{index}"),
                    is_var && reassigned.contains(&binding.name),
                    span,
                )?);
            }
            Ok(RustStmt::Raw(lines.join("\n")))
        }
        TypedBindingPatternKind::Record { fields, .. } => {
            let temp_name = destructure_temp_name(span);
            let mut lines = vec![format!(
                "let {temp_name}: {} = {};",
                render_type(&lower_storage_type(whole_type, span)?),
                render_expr(&coerce_expr_to_type(lower_expr(value)?, &value.ty, whole_type))
            )];
            for field in fields {
                lines.push(render_destructure_binding(
                    &field.binding.name,
                    &field.binding.ty,
                    &format!("{temp_name}.borrow().{}", field.field),
                    is_var && reassigned.contains(&field.binding.name),
                    span,
                )?);
            }
            Ok(RustStmt::Raw(lines.join("\n")))
        }
    }
}

fn render_destructure_binding(
    name: &str,
    ty: &Type,
    source_expr: &str,
    mutable: bool,
    span: &SourceSpan,
) -> Result<String, Vec<Diagnostic>> {
    let mutability = if mutable { "mut " } else { "" };
    Ok(format!(
        "let {mutability}{name}: {} = support::snapshot(&{source_expr});",
        render_type(&lower_storage_type(ty, span)?)
    ))
}

fn destructure_temp_name(span: &SourceSpan) -> String {
    format!(
        "__vulgata_destructure_{}_{}",
        span.line.max(1),
        span.column.max(1)
    )
}

fn coerce_expr_to_type(expr: RustExpr, actual: &Type, expected: &Type) -> RustExpr {
    match (expected, actual) {
        (Type::Option(_), Type::None) => RustExpr::raw("None"),
        (Type::Option(inner), other) if **inner == *other => {
            RustExpr::raw(format!("Some({})", render_expr(&expr)))
        }
        _ => expr,
    }
}

fn lower_expr(expr: &TypedExpr) -> Result<RustExpr, Vec<Diagnostic>> {
    match &expr.kind {
        TypedExprKind::Literal(literal) => Ok(lower_literal(literal)),
        TypedExprKind::Symbol(symbol) => lower_symbol(symbol, &expr.ty, &expr.span),
        TypedExprKind::VariantConstructor {
            type_name,
            variant_name,
            field_names,
            args,
        } => lower_variant_constructor(type_name, variant_name, field_names, args),
        TypedExprKind::Call { callee, args } => lower_call(callee, args, expr),
        TypedExprKind::FieldAccess { base, field } => {
            let base_expr = lower_expr(base)?;
            match &base.ty {
                Type::Record(_) => Ok(RustExpr::raw(snapshot_expr(
                    &format!("{}.borrow().{}", render_expr(&base_expr), field),
                    &expr.ty,
                ))),
                other => Err(vec![codegen_error(
                    &expr.span,
                    format!("unsupported field access on `{}`", other.describe()),
                )]),
            }
        }
        TypedExprKind::ResultIsOk { target } => Ok(RustExpr::raw(format!(
            "{}.is_ok()",
            render_expr(&lower_expr(target)?)
        ))),
        TypedExprKind::ResultIsErr { target } => Ok(RustExpr::raw(format!(
            "{}.is_err()",
            render_expr(&lower_expr(target)?)
        ))),
        TypedExprKind::ResultValue { target } => Ok(RustExpr::raw(format!(
            "match {} {{ Ok(v) => v, Err(_) => panic!(\"InvalidResultValueAccess\") }}",
            render_expr(&lower_expr(target)?)
        ))),
        TypedExprKind::ResultError { target } => Ok(RustExpr::raw(format!(
            "match {} {{ Err(e) => e, Ok(_) => panic!(\"InvalidResultErrorAccess\") }}",
            render_expr(&lower_expr(target)?)
        ))),
        TypedExprKind::OptionIsSome { target } => Ok(RustExpr::raw(format!(
            "{}.is_some()",
            render_expr(&lower_expr(target)?)
        ))),
        TypedExprKind::OptionIsNone { target } => Ok(RustExpr::raw(format!(
            "{}.is_none()",
            render_expr(&lower_expr(target)?)
        ))),
        TypedExprKind::OptionValue { target } => Ok(RustExpr::raw(format!(
            "match {} {{ Some(v) => v, None => panic!(\"InvalidOptionValueAccess\") }}",
            render_expr(&lower_expr(target)?)
        ))),
        TypedExprKind::Index { base, index } => {
            let base_expr = lower_expr(base)?;
            let index_expr = lower_expr(index)?;
            match &base.ty {
                Type::List(_) | Type::Set(_) => Ok(RustExpr::raw(format!(
                    "support::list_get(&{}, {})",
                    render_expr(&base_expr),
                    render_expr(&index_expr)
                ))),
                Type::Map(_, _) => Ok(RustExpr::raw(format!(
                    "support::map_get(&{}, &{})",
                    render_expr(&base_expr),
                    render_expr(&index_expr)
                ))),
                other => Err(vec![codegen_error(
                    &expr.span,
                    format!("unsupported index access on `{}`", other.describe()),
                )]),
            }
        }
        TypedExprKind::Unary { op, expr: inner } => {
            let inner = lower_expr(inner)?;
            Ok(match op {
                UnaryOp::Negate => RustExpr::Unary {
                    op: "-".to_string(),
                    expr: Box::new(inner),
                },
                UnaryOp::Not => RustExpr::Unary {
                    op: "!".to_string(),
                    expr: Box::new(inner),
                },
            })
        }
        TypedExprKind::Binary { left, op, right } => {
            let left = lower_expr(left)?;
            let right = lower_expr(right)?;
            if *op == BinaryOp::Add && expr.ty == Type::Text {
                Ok(RustExpr::raw(format!(
                    "support::text_add({}, {})",
                    render_expr(&left),
                    render_expr(&right)
                )))
            } else {
                Ok(RustExpr::Binary {
                    left: Box::new(left),
                    op: binary_op(op).to_string(),
                    right: Box::new(right),
                })
            }
        }
        TypedExprKind::List(items) => Ok(RustExpr::raw(format!(
            "std::rc::Rc::new(std::cell::RefCell::new(vec![{}]))",
            items
                .iter()
                .map(lower_expr)
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
        TypedExprKind::Map(entries) => Ok(RustExpr::raw(format!(
            "std::rc::Rc::new(std::cell::RefCell::new(vec![{}]))",
            entries
                .iter()
                .map(|(key, value)| {
                    Ok(format!(
                        "({}, {})",
                        render_expr(&lower_expr(key)?),
                        render_expr(&lower_expr(value)?)
                    ))
                })
                .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?
                .join(", ")
        ))),
        TypedExprKind::Tuple(items) => Ok(RustExpr::raw(format!(
            "({})",
            items
                .iter()
                .map(lower_expr)
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn lower_literal(literal: &TypedLiteral) -> RustExpr {
    match literal {
        TypedLiteral::Int(value) => RustExpr::raw(value.to_string()),
        TypedLiteral::Dec(value) => RustExpr::raw(format!("{value}_f64")),
        TypedLiteral::String(value) => RustExpr::raw(format!("{value:?}.to_string()")),
        TypedLiteral::Bool(value) => RustExpr::raw(value.to_string()),
        TypedLiteral::None => RustExpr::raw("()"),
    }
}

fn lower_symbol(
    symbol: &TypedSymbol,
    ty: &Type,
    span: &SourceSpan,
) -> Result<RustExpr, Vec<Diagnostic>> {
    match symbol.kind {
        Some(SymbolKind::Const) => Ok(RustExpr::raw(format!("{}()", const_fn_name(&symbol.name)))),
        Some(SymbolKind::Action) | Some(SymbolKind::Extern) | Some(SymbolKind::Runtime) => {
            Err(vec![codegen_error(
                span,
                format!(
                    "first-class callable `{}` is not supported by compile mode yet",
                    symbol.name
                ),
            )])
        }
        _ => Ok(RustExpr::raw(snapshot_expr(&symbol.name, ty))),
    }
}

fn lower_call(
    callee: &TypedExpr,
    args: &[TypedCallArg],
    expr: &TypedExpr,
) -> Result<RustExpr, Vec<Diagnostic>> {
    if let TypedExprKind::Symbol(symbol) = &callee.kind {
        if let Some(action) = lookup_standard_runtime_symbol(symbol) {
            return lower_standard_runtime_call(action, args, &expr.span);
        }
        if symbol.kind.is_none() && symbol.name == "Some" {
            if args.len() != 1 {
                return Err(vec![codegen_error(
                    &expr.span,
                    "`Some(...)` expects exactly one argument",
                )]);
            }
            if args[0].name.is_some() {
                return Err(vec![codegen_error(
                    &args[0].expr.span,
                    "`Some(...)` does not support named arguments",
                )]);
            }
            return Ok(RustExpr::raw(format!(
                "Some({})",
                render_expr(&lower_expr(&args[0].expr)?)
            )));
        }
        match symbol.kind {
            Some(SymbolKind::Record) => {
                let mut fields = Vec::new();
                for arg in args {
                    let Some(name) = &arg.name else {
                        return Err(vec![codegen_error(
                            &arg.expr.span,
                            "record construction requires named arguments",
                        )]);
                    };
                    fields.push(format!(
                        "{}: {}",
                        name,
                        render_expr(&lower_expr(&arg.expr)?)
                    ));
                }
                return Ok(RustExpr::raw(format!(
                    "std::rc::Rc::new(std::cell::RefCell::new({} {{ {} }}))",
                    symbol.name,
                    fields.join(", ")
                )));
            }
            Some(SymbolKind::Action) => {
                let action_type = match &callee.ty {
                    Type::Action(action) => action,
                    other => {
                        return Err(vec![codegen_error(
                            &expr.span,
                            format!("expected action call type, found `{}`", other.describe()),
                        )]);
                    }
                };
                return Ok(RustExpr::Call {
                    callee: action_fn_name(&symbol.name),
                    args: args
                        .iter()
                        .zip(action_type.params.iter())
                        .map(|(arg, expected)| -> Result<RustExpr, Vec<Diagnostic>> {
                            Ok(coerce_expr_to_type(
                                lower_expr(&arg.expr)?,
                                &arg.expr.ty,
                                expected,
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                });
            }
            Some(SymbolKind::Extern) => {
                return Err(vec![codegen_error(
                    &expr.span,
                    format!(
                        "compile does not yet support calling extern action `{}`",
                        symbol.name
                    ),
                )]);
            }
            _ => {}
        }
    }

    Err(vec![codegen_error(
        &expr.span,
        "compile only supports direct calls to records and actions",
    )])
}

fn lower_variant_constructor(
    type_name: &str,
    variant_name: &str,
    field_names: &[String],
    args: &[TypedExpr],
) -> Result<RustExpr, Vec<Diagnostic>> {
    let rendered_args = args
        .iter()
        .map(lower_expr)
        .collect::<Result<Vec<_>, _>>()?;

    match (type_name, variant_name) {
        ("Option", "Some") => Ok(RustExpr::raw(format!(
            "Some({})",
            render_expr(&rendered_args[0])
        ))),
        ("Result", "Ok") | ("Result", "Err") => Ok(RustExpr::raw(format!(
            "{}({})",
            variant_name,
            render_expr(&rendered_args[0])
        ))),
        _ if field_names.is_empty() => Ok(RustExpr::raw(format!("{type_name}::{variant_name}"))),
        _ => Ok(RustExpr::raw(format!(
            "{type_name}::{variant_name} {{ {} }}",
            field_names
                .iter()
                .zip(rendered_args.iter())
                .map(|(field_name, arg)| format!("{field_name}: {}", render_expr(arg)))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn lower_standard_runtime_call(
    action: StandardRuntimeAction,
    args: &[TypedCallArg],
    span: &SourceSpan,
) -> Result<RustExpr, Vec<Diagnostic>> {
    let rendered_args = args
        .iter()
        .map(|arg| {
            if arg.name.is_some() {
                return Err(vec![codegen_error(
                    &arg.expr.span,
                    "standard runtime calls do not support named arguments",
                )]);
            }
            Ok(render_expr(&lower_expr(&arg.expr)?))
        })
        .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?;

    let expr = match action {
        StandardRuntimeAction::ConsolePrint => {
            format!("support::console_print({}, false, false)", rendered_args[0])
        }
        StandardRuntimeAction::ConsolePrintln => {
            format!("support::console_print({}, true, false)", rendered_args[0])
        }
        StandardRuntimeAction::ConsoleEprint => {
            format!("support::console_print({}, false, true)", rendered_args[0])
        }
        StandardRuntimeAction::ConsoleEprintln => {
            format!("support::console_print({}, true, true)", rendered_args[0])
        }
        StandardRuntimeAction::ConsoleReadLine => "support::console_read_line()".to_string(),
        StandardRuntimeAction::FileReadText => {
            format!("support::file_read_text({})", rendered_args[0])
        }
        StandardRuntimeAction::FileWriteText => format!(
            "support::file_write_text({}, {})",
            rendered_args[0], rendered_args[1]
        ),
        StandardRuntimeAction::FileAppendText => format!(
            "support::file_append_text({}, {})",
            rendered_args[0], rendered_args[1]
        ),
        StandardRuntimeAction::FileExists => format!("support::file_exists({})", rendered_args[0]),
    };

    if rendered_args.len() != action.signature().params.len() {
        return Err(vec![codegen_error(
            span,
            format!(
                "call expects {} arguments, found {}",
                action.signature().params.len(),
                rendered_args.len()
            ),
        )]);
    }

    Ok(RustExpr::raw(expr))
}

fn lookup_standard_runtime_symbol(symbol: &TypedSymbol) -> Option<StandardRuntimeAction> {
    standard_runtime::lookup_standard_runtime_name(&symbol.name)
}

fn lower_target_value(target: &TypedTarget) -> Result<RustExpr, Vec<Diagnostic>> {
    match target {
        TypedTarget::Name { symbol, .. } => Ok(RustExpr::raw(format!("{}.clone()", symbol.name))),
        TypedTarget::Field {
            base, field, span, ..
        } => {
            let base_expr = lower_target_value(base)?;
            match &base.ty() {
                Type::Record(_) => Ok(RustExpr::raw(format!(
                    "{}.borrow().{}.clone()",
                    render_expr(&base_expr),
                    field
                ))),
                other => Err(vec![codegen_error(
                    span,
                    format!("unsupported target field base `{}`", other.describe()),
                )]),
            }
        }
        TypedTarget::Index {
            base, index, span, ..
        } => {
            let base_expr = lower_target_value(base)?;
            let index_expr = lower_expr(index)?;
            match &base.ty() {
                Type::List(_) | Type::Set(_) => Ok(RustExpr::raw(format!(
                    "support::list_get(&{}, {})",
                    render_expr(&base_expr),
                    render_expr(&index_expr)
                ))),
                Type::Map(_, _) => Ok(RustExpr::raw(format!(
                    "support::map_get(&{}, &{})",
                    render_expr(&base_expr),
                    render_expr(&index_expr)
                ))),
                other => Err(vec![codegen_error(
                    span,
                    format!("unsupported target index base `{}`", other.describe()),
                )]),
            }
        }
    }
}

fn lower_storage_type(ty: &Type, span: &SourceSpan) -> Result<RustType, Vec<Diagnostic>> {
    let rendered = match ty {
        Type::Bool => "bool".to_string(),
        Type::Int => "i64".to_string(),
        Type::Dec => "f64".to_string(),
        Type::Text => "String".to_string(),
        Type::Bytes => "Vec<u8>".to_string(),
        Type::None => return Ok(RustType::Unit),
        Type::Record(name) => format!("std::rc::Rc<std::cell::RefCell<{name}>>"),
        Type::Enum(name) => name.clone(),
        Type::List(inner) | Type::Set(inner) => {
            format!(
                "std::rc::Rc<std::cell::RefCell<Vec<{}>>>",
                render_type(&lower_storage_type(inner, span)?)
            )
        }
        Type::Map(key, value) => format!(
            "std::rc::Rc<std::cell::RefCell<Vec<({}, {})>>>",
            render_type(&lower_storage_type(key, span)?),
            render_type(&lower_storage_type(value, span)?)
        ),
        Type::Option(inner) => {
            format!("Option<{}>", render_type(&lower_storage_type(inner, span)?))
        }
        Type::Result(ok, err) => format!(
            "Result<{}, {}>",
            render_type(&lower_storage_type(ok, span)?),
            render_type(&lower_storage_type(err, span)?)
        ),
        Type::Tuple(items) => format!(
            "({})",
            items
                .iter()
                .map(|item| lower_storage_type(item, span).map(|ty| render_type(&ty)))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        Type::Action(_) | Type::ExternAction(_) | Type::Unknown => {
            return Err(vec![codegen_error(
                span,
                format!("unsupported compile-time type `{}`", ty.describe()),
            )]);
        }
    };
    Ok(RustType::Path(rendered))
}

fn entrypoint_item(module: &TypedIrModule) -> Result<RustItem, Vec<Diagnostic>> {
    let test_names = module
        .declarations
        .iter()
        .filter_map(|decl| match decl {
            TypedDecl::Test(test_decl) => Some(test_decl.name.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let main_action = module.declarations.iter().find_map(|decl| match decl {
        TypedDecl::Action(action) if action.name == "main" && action.params.is_empty() => {
            Some(action)
        }
        _ => None,
    });

    let mut body = Vec::new();
    if !test_names.is_empty() {
        body.push(RustStmt::Let {
            name: "failed".to_string(),
            ty: Some(RustType::Path("usize".to_string())),
            value: RustExpr::raw("0usize"),
            mutable: true,
        });
        for name in test_names {
            let fn_name = test_fn_name(&name);
            body.push(RustStmt::Let {
                name: "failures".to_string(),
                ty: Some(RustType::Path("Vec<String>".to_string())),
                value: RustExpr::Call {
                    callee: fn_name,
                    args: Vec::new(),
                },
                mutable: false,
            });
            body.push(RustStmt::Raw(format!(
                "if failures.is_empty() {{ println!(\"PASS {}\"); }} else {{ failed += 1; println!(\"FAIL {}\"); for failure in &failures {{ println!(\"  {{}}\", failure); }} }}",
                name, name
            )));
        }
        body.push(RustStmt::Raw(
            "if failed > 0 { std::process::exit(1); }".to_string(),
        ));
    } else if let Some(action) = main_action {
        let rendered = render_runtime_value("result", action.ty.action_result());
        body.push(RustStmt::Raw(format!(
            "let result = {}(); println!(\"{{}}\", {});",
            action_fn_name("main"),
            rendered
        )));
    } else {
        body.push(RustStmt::Raw(
            "println!(\"compiled module has no runnable entrypoint\");".to_string(),
        ));
    }

    Ok(RustItem::Function {
        name: "main".to_string(),
        params: Vec::new(),
        result: RustType::Unit,
        body,
        is_pub: false,
    })
}

fn render_item(item: &RustItem, indent: usize, output: &mut String) {
    match item {
        RustItem::Raw(raw) => {
            output.push_str(raw);
            if !raw.ends_with('\n') {
                output.push('\n');
            }
        }
        RustItem::Struct { name, fields } => {
            line(indent, "#[derive(Clone, Debug, PartialEq)]", output);
            line(indent, &format!("struct {name} {{"), output);
            for (field, ty) in fields {
                line(
                    indent + 1,
                    &format!("{field}: {},", render_type(ty)),
                    output,
                );
            }
            line(indent, "}", output);
            line(
                indent,
                &format!("impl support::Snapshot for {name} {{"),
                output,
            );
            line(indent + 1, "fn snapshot(&self) -> Self {", output);
            line(indent + 2, &format!("{name} {{"), output);
            for (field, _) in fields {
                line(
                    indent + 3,
                    &format!("{field}: support::snapshot(&self.{field}),"),
                    output,
                );
            }
            line(indent + 2, "}", output);
            line(indent + 1, "}", output);
            line(indent, "}", output);
        }
        RustItem::Enum { name, variants } => {
            line(indent, "#[derive(Clone, Debug, PartialEq)]", output);
            line(indent, &format!("enum {name} {{"), output);
            for variant in variants {
                if variant.fields.is_empty() {
                    line(indent + 1, &format!("{},", variant.name), output);
                } else {
                    line(indent + 1, &format!("{} {{", variant.name), output);
                    for (field, ty) in &variant.fields {
                        line(
                            indent + 2,
                            &format!("{field}: {},", render_type(ty)),
                            output,
                        );
                    }
                    line(indent + 1, "},", output);
                }
            }
            line(indent, "}", output);
            line(
                indent,
                &format!("impl support::Snapshot for {name} {{"),
                output,
            );
            line(indent + 1, "fn snapshot(&self) -> Self {", output);
            line(indent + 2, "self.clone()", output);
            line(indent + 1, "}", output);
            line(indent, "}", output);
        }
        RustItem::Function {
            name,
            params,
            result,
            body,
            is_pub,
        } => {
            let visibility = if *is_pub { "pub " } else { "" };
            let rendered_params = params
                .iter()
                .map(|param| {
                    if param.mutable {
                        format!("mut {}: {}", param.name, render_type(&param.ty))
                    } else {
                        format!("{}: {}", param.name, render_type(&param.ty))
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let signature = if matches!(result, RustType::Unit) {
                format!("{visibility}fn {name}({rendered_params}) {{")
            } else {
                format!(
                    "{visibility}fn {name}({rendered_params}) -> {} {{",
                    render_type(result)
                )
            };
            line(indent, &signature, output);
            for stmt in body {
                render_stmt(stmt, indent + 1, output);
            }
            line(indent, "}", output);
        }
    }
}

fn render_stmt(stmt: &RustStmt, indent: usize, output: &mut String) {
    match stmt {
        RustStmt::Let {
            name,
            ty,
            value,
            mutable,
        } => {
            let mutability = if *mutable { "mut " } else { "" };
            let annotation = ty
                .as_ref()
                .map(|ty| format!(": {}", render_type(ty)))
                .unwrap_or_default();
            line(
                indent,
                &format!(
                    "let {mutability}{name}{annotation} = {};",
                    render_expr(value)
                ),
                output,
            );
        }
        RustStmt::Assign { target, value } => {
            line(
                indent,
                &format!("{} = {};", render_expr(target), render_expr(value)),
                output,
            );
        }
        RustStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            line(indent, &format!("if {} {{", render_expr(condition)), output);
            for stmt in then_body {
                render_stmt(stmt, indent + 1, output);
            }
            if else_body.is_empty() {
                line(indent, "}", output);
            } else if else_body.len() == 1 && matches!(else_body[0], RustStmt::If { .. }) {
                output.push_str(&format!("{:indent$}}} else ", "", indent = indent * 4));
                render_stmt(&else_body[0], 0, output);
            } else {
                line(indent, "} else {", output);
                for stmt in else_body {
                    render_stmt(stmt, indent + 1, output);
                }
                line(indent, "}", output);
            }
        }
        RustStmt::While { condition, body } => {
            line(
                indent,
                &format!("while {} {{", render_expr(condition)),
                output,
            );
            for stmt in body {
                render_stmt(stmt, indent + 1, output);
            }
            line(indent, "}", output);
        }
        RustStmt::ForEach {
            binding,
            iterable,
            body,
        } => {
            line(
                indent,
                &format!(
                    "for {binding} in {}.borrow().iter().cloned() {{",
                    render_expr(iterable)
                ),
                output,
            );
            for stmt in body {
                render_stmt(stmt, indent + 1, output);
            }
            line(indent, "}", output);
        }
        RustStmt::Return(expr) => match expr {
            Some(expr) => line(indent, &format!("return {};", render_expr(expr)), output),
            None => line(indent, "return;", output),
        },
        RustStmt::Break => line(indent, "break;", output),
        RustStmt::Continue => line(indent, "continue;", output),
        RustStmt::Expr(expr) => line(indent, &format!("{};", render_expr(expr)), output),
        RustStmt::Raw(raw) => line(indent, raw, output),
    }
}

fn render_expr(expr: &RustExpr) -> String {
    render_expr_prec(expr, 0, false)
}

fn render_pattern(pattern: &TypedPattern) -> Result<String, Vec<Diagnostic>> {
    match &pattern.kind {
        TypedPatternKind::Wildcard => Ok("_".to_string()),
        TypedPatternKind::Literal(literal) => Ok(match literal {
            TypedLiteral::Int(value) => value.to_string(),
            TypedLiteral::Dec(value) => value.clone(),
            TypedLiteral::String(value) => format!("{value:?}"),
            TypedLiteral::Bool(value) => value.to_string(),
            TypedLiteral::None => "None".to_string(),
        }),
        TypedPatternKind::Binding(name) => Ok(name.clone()),
        TypedPatternKind::Tuple(items) => Ok(format!(
            "({})",
            items
                .iter()
                .map(render_pattern)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        TypedPatternKind::Record { name, fields } => {
            let rendered_fields = fields
                .iter()
                .map(|field| {
                    render_pattern(&field.pattern).map(|pattern| format!("{}: {}", field.name, pattern))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if rendered_fields.is_empty() {
                Ok(format!("{name} {{ .. }}"))
            } else {
                Ok(format!("{name} {{ {}, .. }}", rendered_fields.join(", ")))
            }
        }
        TypedPatternKind::Variant {
            type_name,
            variant_name,
            field_names,
            args,
        } => match (type_name.as_str(), variant_name.as_str()) {
            ("Option", "Some") | ("Result", "Ok") | ("Result", "Err") => Ok(format!(
                "{}({})",
                variant_name,
                args.iter()
                    .map(render_pattern)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )),
            ("Option", "None") => Ok("None".to_string()),
            _ if field_names.is_empty() => Ok(format!("{type_name}::{variant_name}")),
            _ => Ok(format!(
                "{type_name}::{variant_name} {{ {} }}",
                field_names
                    .iter()
                    .zip(args.iter())
                    .map(|(field_name, arg)| {
                        render_pattern(arg).map(|pattern| format!("{field_name}: {pattern}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )),
        },
    }
}

fn render_expr_prec(expr: &RustExpr, parent_prec: u8, is_right_child: bool) -> String {
    match expr {
        RustExpr::Raw(raw) => raw.clone(),
        RustExpr::Call { callee, args } => format!(
            "{callee}({})",
            args.iter().map(render_expr).collect::<Vec<_>>().join(", ")
        ),
        RustExpr::MethodCall {
            receiver,
            method,
            args,
        } => format!(
            "{}.{}({})",
            render_expr(receiver),
            method,
            args.iter().map(render_expr).collect::<Vec<_>>().join(", ")
        ),
        RustExpr::Field { base, field } => format!("{}.{}", render_expr(base), field),
        RustExpr::Binary { left, op, right } => {
            let prec = binary_precedence(op);
            let rendered = format!(
                "{} {} {}",
                render_expr_prec(left, prec, false),
                op,
                render_expr_prec(right, prec, true)
            );
            if prec < parent_prec || (is_right_child && prec == parent_prec) {
                format!("({rendered})")
            } else {
                rendered
            }
        }
        RustExpr::Unary { op, expr } => {
            let prec = 60;
            let rendered = format!("{op}{}", render_expr_prec(expr, prec, false));
            if prec < parent_prec {
                format!("({rendered})")
            } else {
                rendered
            }
        }
    }
}

fn render_type(ty: &RustType) -> String {
    match ty {
        RustType::Unit => "()".to_string(),
        RustType::Path(path) => path.clone(),
    }
}

fn snapshot_expr(expr: &str, ty: &Type) -> String {
    match ty {
        Type::Action(_) | Type::ExternAction(_) | Type::Unknown => expr.to_string(),
        _ => format!("support::snapshot(&{expr})"),
    }
}

fn binary_op(op: &BinaryOp) -> &'static str {
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
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn binary_precedence(op: &str) -> u8 {
    match op {
        "||" => 10,
        "&&" => 20,
        "==" | "!=" | "<" | "<=" | ">" | ">=" => 30,
        "+" | "-" => 40,
        "*" | "/" | "%" => 50,
        _ => 0,
    }
}

fn action_fn_name(name: &str) -> String {
    format!("vulgata_action_{name}")
}

fn const_fn_name(name: &str) -> String {
    format!("vulgata_const_{name}")
}

fn test_fn_name(name: &str) -> String {
    format!("vulgata_test_{name}")
}

fn codegen_error(span: &SourceSpan, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(span.clone(), Phase::Codegen, message)
}

fn render_runtime_value(binding: &str, ty: &Type) -> String {
    match ty {
        Type::Bool => format!("format!(\"Bool({{}})\", {binding})"),
        Type::Int => format!("format!(\"Int({{}})\", {binding})"),
        Type::Dec => format!("format!(\"Dec({{}})\", {binding})"),
        Type::Text => format!("format!(\"Text({{:?}})\", {binding})"),
        Type::None => "\"None\".to_string()".to_string(),
        _ => format!("format!(\"{{:?}}\", {binding})"),
    }
}

fn line(indent: usize, text: &str, output: &mut String) {
    output.push_str(&" ".repeat(indent * 4));
    output.push_str(text);
    output.push('\n');
}

const SUPPORT_MODULE: &str = r#"mod support {
    #![allow(dead_code)]

    use std::cell::RefCell;
    use std::io::Write;
    use std::rc::Rc;

    pub trait Snapshot {
        fn snapshot(&self) -> Self;
    }

    pub fn snapshot<T: Snapshot>(value: &T) -> T {
        value.snapshot()
    }

    impl Snapshot for bool {
        fn snapshot(&self) -> Self {
            *self
        }
    }

    impl Snapshot for i64 {
        fn snapshot(&self) -> Self {
            *self
        }
    }

    impl Snapshot for f64 {
        fn snapshot(&self) -> Self {
            *self
        }
    }

    impl Snapshot for String {
        fn snapshot(&self) -> Self {
            self.clone()
        }
    }

    impl Snapshot for () {
        fn snapshot(&self) -> Self {
            *self
        }
    }

    impl<T: Snapshot> Snapshot for Vec<T> {
        fn snapshot(&self) -> Self {
            self.iter().map(Snapshot::snapshot).collect()
        }
    }

    impl<T: Snapshot> Snapshot for Option<T> {
        fn snapshot(&self) -> Self {
            self.as_ref().map(Snapshot::snapshot)
        }
    }

    impl<T: Snapshot, E: Snapshot> Snapshot for Result<T, E> {
        fn snapshot(&self) -> Self {
            match self {
                Ok(value) => Ok(value.snapshot()),
                Err(error) => Err(error.snapshot()),
            }
        }
    }

    impl<A: Snapshot> Snapshot for (A,) {
        fn snapshot(&self) -> Self {
            (self.0.snapshot(),)
        }
    }

    impl<A: Snapshot, B: Snapshot> Snapshot for (A, B) {
        fn snapshot(&self) -> Self {
            (self.0.snapshot(), self.1.snapshot())
        }
    }

    impl<A: Snapshot, B: Snapshot, C: Snapshot> Snapshot for (A, B, C) {
        fn snapshot(&self) -> Self {
            (self.0.snapshot(), self.1.snapshot(), self.2.snapshot())
        }
    }

    impl<A: Snapshot, B: Snapshot, C: Snapshot, D: Snapshot> Snapshot for (A, B, C, D) {
        fn snapshot(&self) -> Self {
            (
                self.0.snapshot(),
                self.1.snapshot(),
                self.2.snapshot(),
                self.3.snapshot(),
            )
        }
    }

    impl<T: Snapshot> Snapshot for Rc<RefCell<T>> {
        fn snapshot(&self) -> Self {
            Rc::new(RefCell::new(self.borrow().snapshot()))
        }
    }

    pub fn text_add(left: String, right: String) -> String {
        let mut joined = left;
        joined.push_str(&right);
        joined
    }

    pub fn console_print(value: String, newline: bool, stderr: bool) -> Result<(), String> {
        let result = if stderr {
            let mut handle = std::io::stderr();
            if newline {
                writeln!(handle, "{value}")
            } else {
                write!(handle, "{value}")
            }
        } else {
            let mut handle = std::io::stdout();
            if newline {
                writeln!(handle, "{value}")
            } else {
                write!(handle, "{value}")
            }
        };

        result.map_err(|error| error.to_string())
    }

    pub fn console_read_line() -> Result<String, String> {
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(0) => Err("end of input".to_string()),
            Ok(_) => {
                while matches!(line.chars().last(), Some('\n' | '\r')) {
                    line.pop();
                }
                Ok(line)
            }
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn file_read_text(path: String) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|error| error.to_string())
    }

    pub fn file_write_text(path: String, content: String) -> Result<(), String> {
        std::fs::write(path, content).map_err(|error| error.to_string())
    }

    pub fn file_append_text(path: String, content: String) -> Result<(), String> {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(content.as_bytes()))
            .map_err(|error| error.to_string())
    }

    pub fn file_exists(path: String) -> bool {
        std::path::Path::new(&path).exists()
    }

    pub fn list_get<T: Snapshot>(items: &Rc<RefCell<Vec<T>>>, index: i64) -> T {
        items.borrow()[index as usize].snapshot()
    }

    pub fn list_set<T>(items: &Rc<RefCell<Vec<T>>>, index: i64, value: T) {
        items.borrow_mut()[index as usize] = value;
    }

    pub fn map_get<K: PartialEq, V: Snapshot>(
        entries: &Rc<RefCell<Vec<(K, V)>>>,
        key: &K,
    ) -> V {
        entries
            .borrow()
            .iter()
            .find(|(existing_key, _)| existing_key == key)
            .map(|(_, value)| value.snapshot())
            .expect("map key not found")
    }

    pub fn map_set<K: PartialEq, V>(
        entries: &Rc<RefCell<Vec<(K, V)>>>,
        key: K,
        value: V,
    ) {
        let mut entries = entries.borrow_mut();
        if let Some((_, existing_value)) = entries.iter_mut().find(|(existing_key, _)| *existing_key == key) {
            *existing_value = value;
        } else {
            entries.push((key, value));
        }
    }
}"#;

fn block_may_fall_through(block: &[RustStmt]) -> bool {
    let mut may_continue = true;
    for stmt in block {
        if !may_continue {
            return false;
        }
        may_continue = stmt_may_fall_through(stmt);
    }
    may_continue
}

fn stmt_may_fall_through(stmt: &RustStmt) -> bool {
    match stmt {
        RustStmt::Return(_) | RustStmt::Break | RustStmt::Continue => false,
        RustStmt::If {
            then_body,
            else_body,
            ..
        } if !else_body.is_empty() => {
            block_may_fall_through(then_body) || block_may_fall_through(else_body)
        }
        RustStmt::Let { .. }
        | RustStmt::Assign { .. }
        | RustStmt::While { .. }
        | RustStmt::ForEach { .. }
        | RustStmt::Expr(_)
        | RustStmt::Raw(_)
        | RustStmt::If { .. } => true,
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

trait TypedTargetExt {
    fn ty(&self) -> &Type;
}

impl TypedTargetExt for TypedTarget {
    fn ty(&self) -> &Type {
        match self {
            TypedTarget::Name { ty, .. } => ty,
            TypedTarget::Field { ty, .. } => ty,
            TypedTarget::Index { ty, .. } => ty,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolver::Resolver;
    use crate::tir::lower_module as lower_tir;
    use crate::types::TypeChecker;

    use super::{emit_module, lower_module};

    fn unique_codegen_dir(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "vulgata-codegen-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn lower(source: &str) -> crate::tir::TypedIrModule {
        let path = Path::new("test.vg");
        let tokens = Lexer::new(path, source).tokenize().expect("tokenize");
        let module = Parser::new(path, tokens).parse_module().expect("parse");
        let resolution = Resolver::new(&module).resolve().expect("resolve");
        let checked = TypeChecker::new(&module, &resolution)
            .check()
            .expect("check");
        lower_tir(&checked).expect("lower")
    }

    #[test]
    fn emits_rust_for_core_language_constructs() {
        let module = lower(
            r#"
record Customer:
  email: Text

action main() -> Int:
  var customer = Customer(email: "before")
  var count = 2
  while count > 0:
    customer.email := "after"
    count := count - 1
  return count
"#,
        );

        let lowered = lower_module(&module).expect("codegen lower");
        let emitted = emit_module(&lowered);

        assert!(emitted.contains("struct Customer"));
        assert!(emitted.contains("fn vulgata_action_main() -> i64"));
        assert!(emitted.contains(".borrow_mut().email = \"after\".to_string();"));
    }

    #[test]
    fn emitted_rust_compiles_with_rustc() {
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

        let lowered = lower_module(&module).expect("codegen lower");
        let emitted = emit_module(&lowered);

        let dir = unique_codegen_dir("compile");
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let source_path = dir.join("generated.rs");
        let binary_path = dir.join("generated-bin");
        std::fs::write(&source_path, emitted).expect("write generated source");

        let status = Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .status()
            .expect("run rustc");
        assert!(status.success(), "rustc failed with status {status:?}");
    }

    #[test]
    fn emitted_rust_preserves_let_immutability_across_var_copies() {
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

        let lowered = lower_module(&module).expect("codegen lower");
        let emitted = emit_module(&lowered);

        let dir = unique_codegen_dir("aliasing");
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let source_path = dir.join("generated.rs");
        let binary_path = dir.join("generated-bin");
        std::fs::write(&source_path, emitted).expect("write generated source");

        let status = Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .status()
            .expect("run rustc");
        assert!(status.success(), "rustc failed with status {status:?}");

        let output = Command::new(&binary_path)
            .output()
            .expect("run generated binary");
        assert!(
            output.status.success(),
            "binary failed with status {:?}",
            output.status
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "Text(\"before\")"
        );
    }

    #[test]
    fn emits_result_and_option_operations_and_option_coercions() {
        let module = lower(
            r#"
action main() -> Int:
  let written = console.print("")
  let maybe: Option[Int] = 10
  if written.is_ok() and not written.is_err() and maybe.is_some() and not maybe.is_none():
    return maybe.value()
  return 0
"#,
        );

        let lowered = lower_module(&module).expect("codegen lower");
        let emitted = emit_module(&lowered);

        assert!(emitted.contains(".is_ok()"));
        assert!(emitted.contains(".is_err()"));
        assert!(emitted.contains(".is_some()"));
        assert!(emitted.contains(".is_none()"));
        assert!(emitted.contains("InvalidOptionValueAccess"));
        assert!(emitted.contains("let maybe: Option<i64> = Some(10);"));
    }

    #[test]
    fn emits_explicit_match_for_result_value_and_error() {
        let module = lower(
            r#"
action main() -> Text:
  let result = file.read_text("missing.txt")
  if result.is_ok():
    return result.value()
  return result.error()
"#,
        );

        let lowered = lower_module(&module).expect("codegen lower");
        let emitted = emit_module(&lowered);

        assert!(emitted.contains("InvalidResultValueAccess"));
        assert!(emitted.contains("InvalidResultErrorAccess"));
        assert!(emitted.contains("match"));
    }

    #[test]
    fn emits_match_statements_and_enum_variants() {
        let module = lower(
            r#"
record Customer:
  name: Text

enum Decision:
  Accept(reason: Text)
  Reject

action main() -> Text:
  let decision = Accept("ok")
  match decision:
    Accept(reason):
      return reason
    Reject():
      return "reject"
"#,
        );

        let lowered = lower_module(&module).expect("codegen lower");
        let emitted = emit_module(&lowered);

        assert!(emitted.contains("enum Decision"));
        assert!(emitted.contains("Accept {"));
        assert!(emitted.contains("match "));
        assert!(emitted.contains("Decision::Accept { reason: reason }"));
        assert!(emitted.contains("panic!(\"NonExhaustiveMatch\")"));
    }

    #[test]
    fn emits_destructuring_bindings_with_one_time_temporaries() {
        let module = lower(
            r#"
record Customer:
  score: Int

action main(pair: (Int, Int), customer: Customer) -> Int:
  let (left, right) = pair
  var Customer(score: score) = customer
  score := score + left
  return score + right
"#,
        );

        let lowered = lower_module(&module).expect("codegen lower");
        let emitted = emit_module(&lowered);

        assert!(emitted.contains("__vulgata_destructure_"));
        assert!(emitted.contains("support::snapshot(&"));
        assert!(emitted.contains(".0"));
        assert!(emitted.contains(".borrow().score"));
    }
}
