use crate::ast::{BinaryOp, UnaryOp};
use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::resolver::SymbolKind;
use crate::tir::{
    TypedCallArg, TypedDecl, TypedExpr, TypedExprKind, TypedIrModule, TypedLiteral, TypedStmt,
    TypedStmtKind, TypedSymbol, TypedTarget,
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
        variants: Vec<String>,
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
                for (name, ty) in &record.fields {
                    fields.push((name.clone(), lower_storage_type(ty, &record.span)?));
                }
                items.push(RustItem::Struct {
                    name: record.name.clone(),
                    fields,
                });
            }
            TypedDecl::Enum(enum_decl) => {
                items.push(RustItem::Enum {
                    name: enum_decl.name.clone(),
                    variants: enum_decl.variants.clone(),
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
                                mutable: true,
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
        uses: vec![
            "use std::cell::RefCell;".to_string(),
            "use std::rc::Rc;".to_string(),
        ],
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
    let mut body = Vec::new();
    for stmt in &action.body {
        body.push(lower_stmt(stmt, false)?);
    }

    let result_ty = action.ty.action_result();
    if *result_ty == Type::None {
        body.push(RustStmt::Raw("()".to_string()));
    } else {
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
    let mut body = vec![RustStmt::Let {
        name: "failures".to_string(),
        ty: Some(RustType::Path("Vec<String>".to_string())),
        value: RustExpr::raw("Vec::new()"),
        mutable: true,
    }];
    for stmt in &test_decl.body {
        body.push(lower_stmt(stmt, true)?);
    }
    body.push(RustStmt::Return(Some(RustExpr::raw("failures"))));
    Ok(body)
}

fn lower_stmt(stmt: &TypedStmt, in_test: bool) -> Result<RustStmt, Vec<Diagnostic>> {
    let lowered = match &stmt.kind {
        TypedStmtKind::Let { name, ty, value } => RustStmt::Let {
            name: name.clone(),
            ty: Some(lower_storage_type(ty, &stmt.span)?),
            value: lower_expr(value)?,
            mutable: true,
        },
        TypedStmtKind::Set { target, value } => lower_assignment(target, value)?,
        TypedStmtKind::If {
            branches,
            else_branch,
        } => lower_if(branches, else_branch, in_test)?,
        TypedStmtKind::While { condition, body } => RustStmt::While {
            condition: lower_expr(condition)?,
            body: lower_block(body, in_test)?,
        },
        TypedStmtKind::ForEach {
            binding,
            iterable,
            body,
        } => RustStmt::ForEach {
            binding: binding.clone(),
            iterable: lower_expr(iterable)?,
            body: lower_block(body, in_test)?,
        },
        TypedStmtKind::Return(expr) => RustStmt::Return(expr.as_ref().map(lower_expr).transpose()?),
        TypedStmtKind::Break => RustStmt::Break,
        TypedStmtKind::Continue => RustStmt::Continue,
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

fn lower_block(block: &[TypedStmt], in_test: bool) -> Result<Vec<RustStmt>, Vec<Diagnostic>> {
    block.iter().map(|stmt| lower_stmt(stmt, in_test)).collect()
}

fn lower_if(
    branches: &[(TypedExpr, Vec<TypedStmt>)],
    else_branch: &[TypedStmt],
    in_test: bool,
) -> Result<RustStmt, Vec<Diagnostic>> {
    if branches.is_empty() {
        return Ok(RustStmt::Raw("{}".to_string()));
    }

    let (condition, body) = &branches[0];
    let mut statement = RustStmt::If {
        condition: lower_expr(condition)?,
        then_body: lower_block(body, in_test)?,
        else_body: lower_block(else_branch, in_test)?,
    };

    for (condition, body) in branches.iter().skip(1).rev() {
        statement = RustStmt::If {
            condition: lower_expr(condition)?,
            then_body: lower_block(body, in_test)?,
            else_body: vec![statement],
        };
    }

    Ok(statement)
}

fn lower_assignment(target: &TypedTarget, value: &TypedExpr) -> Result<RustStmt, Vec<Diagnostic>> {
    let value = lower_expr(value)?;
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

fn lower_expr(expr: &TypedExpr) -> Result<RustExpr, Vec<Diagnostic>> {
    match &expr.kind {
        TypedExprKind::Literal(literal) => Ok(lower_literal(literal)),
        TypedExprKind::Symbol(symbol) => lower_symbol(symbol, &expr.ty, &expr.span),
        TypedExprKind::Call { callee, args } => lower_call(callee, args, expr),
        TypedExprKind::FieldAccess { base, field } => {
            let base_expr = lower_expr(base)?;
            match &base.ty {
                Type::Record(_) => Ok(RustExpr::raw(format!(
                    "{}.borrow().{}.clone()",
                    render_expr(&base_expr),
                    field
                ))),
                other => Err(vec![codegen_error(
                    &expr.span,
                    format!("unsupported field access on `{}`", other.describe()),
                )]),
            }
        }
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
            "Rc::new(RefCell::new(vec![{}]))",
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
            "Rc::new(RefCell::new(vec![{}]))",
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
    _ty: &Type,
    span: &SourceSpan,
) -> Result<RustExpr, Vec<Diagnostic>> {
    match symbol.kind {
        Some(SymbolKind::Const) => Ok(RustExpr::raw(format!("{}()", const_fn_name(&symbol.name)))),
        Some(SymbolKind::Action) | Some(SymbolKind::Extern) => Err(vec![codegen_error(
            span,
            format!(
                "first-class callable `{}` is not supported by compile mode yet",
                symbol.name
            ),
        )]),
        _ => Ok(RustExpr::raw(format!("{}.clone()", symbol.name))),
    }
}

fn lower_call(
    callee: &TypedExpr,
    args: &[TypedCallArg],
    expr: &TypedExpr,
) -> Result<RustExpr, Vec<Diagnostic>> {
    if let TypedExprKind::Symbol(symbol) = &callee.kind {
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
                    "Rc::new(RefCell::new({} {{ {} }}))",
                    symbol.name,
                    fields.join(", ")
                )));
            }
            Some(SymbolKind::Action) => {
                return Ok(RustExpr::Call {
                    callee: action_fn_name(&symbol.name),
                    args: args
                        .iter()
                        .map(|arg| lower_expr(&arg.expr))
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
        Type::Record(name) => format!("Rc<RefCell<{name}>>"),
        Type::Enum(name) => name.clone(),
        Type::List(inner) | Type::Set(inner) => {
            format!(
                "Rc<RefCell<Vec<{}>>>",
                render_type(&lower_storage_type(inner, span)?)
            )
        }
        Type::Map(key, value) => format!(
            "Rc<RefCell<Vec<({}, {})>>>",
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
        TypedDecl::Action(action) if action.name == "main" && action.params.is_empty() => Some(action),
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
        }
        RustItem::Enum { name, variants } => {
            line(indent, "#[derive(Clone, Debug, PartialEq)]", output);
            line(indent, &format!("enum {name} {{"), output);
            for variant in variants {
                line(indent + 1, &format!("{variant},"), output);
            }
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
            format!("({} {} {})", render_expr(left), op, render_expr(right))
        }
        RustExpr::Unary { op, expr } => format!("({}{})", op, render_expr(expr)),
    }
}

fn render_type(ty: &RustType) -> String {
    match ty {
        RustType::Unit => "()".to_string(),
        RustType::Path(path) => path.clone(),
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
    use std::cell::RefCell;
    use std::rc::Rc;

    pub fn text_add(left: String, right: String) -> String {
        let mut joined = left;
        joined.push_str(&right);
        joined
    }

    pub fn list_get<T: Clone>(items: &Rc<RefCell<Vec<T>>>, index: i64) -> T {
        items.borrow()[index as usize].clone()
    }

    pub fn list_set<T>(items: &Rc<RefCell<Vec<T>>>, index: i64, value: T) {
        items.borrow_mut()[index as usize] = value;
    }

    pub fn map_get<K: Clone + PartialEq, V: Clone>(
        entries: &Rc<RefCell<Vec<(K, V)>>>,
        key: &K,
    ) -> V {
        entries
            .borrow()
            .iter()
            .find(|(existing_key, _)| existing_key == key)
            .map(|(_, value)| value.clone())
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

    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolver::Resolver;
    use crate::tir::lower_module as lower_tir;
    use crate::types::TypeChecker;

    use super::{emit_module, lower_module};

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
  let customer = Customer(email: "before")
  let count = 2
  while count > 0:
    set customer.email = "after"
    set count = count - 1
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
  let x = 4
  let y = 2
  while y > 0:
    set x = x + 1
    set y = y - 1
  return x
"#,
        );

        let lowered = lower_module(&module).expect("codegen lower");
        let emitted = emit_module(&lowered);

        let dir = std::env::temp_dir().join(format!("vulgata-codegen-{}", std::process::id()));
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
}
