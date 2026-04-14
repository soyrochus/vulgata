use std::collections::HashMap;

use crate::ast::{
    BinaryOp, BindingPattern, BindingPatternKind, CallArg, Decl, Expr, ExprKind, Pattern,
    PatternKind, PatternLiteral, RecordBindingField, RecordPatternField, Stmt, StmtKind, Target,
    UnaryOp,
};
use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::resolver::SymbolKind;
use crate::standard_runtime;
use crate::types::{CheckedModule, CheckedStmt, Type};

#[derive(Debug, Clone, PartialEq)]
pub struct TypedIrModule {
    pub module_name: Option<String>,
    pub declarations: Vec<TypedDecl>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedDecl {
    Const(TypedConstDecl),
    Record(TypedRecordDecl),
    Enum(TypedEnumDecl),
    Extern(TypedExternDecl),
    Action(TypedActionDecl),
    Test(TypedTestDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedConstDecl {
    pub name: String,
    pub ty: Type,
    pub value: TypedExpr,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedRecordDecl {
    pub name: String,
    pub fields: Vec<TypedRecordField>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedRecordField {
    pub name: String,
    pub ty: Type,
    pub meaning: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedEnumDecl {
    pub name: String,
    pub variants: Vec<TypedEnumVariant>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedEnumVariant {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedExternDecl {
    pub name: String,
    pub ty: Type,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedActionDecl {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub body: Vec<TypedStmt>,
    pub ty: Type,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedTestDecl {
    pub name: String,
    pub body: Vec<TypedStmt>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedStmt {
    pub kind: TypedStmtKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedStmtKind {
    IntentBlock {
        goal: Option<String>,
        constraints: Vec<String>,
        assumptions: Vec<String>,
        properties: Vec<String>,
    },
    ExplainBlock {
        lines: Vec<String>,
    },
    StepBlock {
        label: String,
        body: Vec<TypedStmt>,
    },
    RequiresClause(TypedExpr),
    EnsuresClause(TypedExpr),
    ExampleBlock {
        name: String,
        inputs: Vec<(String, TypedExpr)>,
        outputs: Vec<(String, TypedExpr)>,
    },
    Let {
        pattern: TypedBindingPattern,
        ty: Type,
        value: TypedExpr,
    },
    Var {
        pattern: TypedBindingPattern,
        ty: Type,
        value: TypedExpr,
    },
    Assign {
        target: TypedTarget,
        value: TypedExpr,
    },
    If {
        branches: Vec<(TypedExpr, Vec<TypedStmt>)>,
        else_branch: Vec<TypedStmt>,
    },
    While {
        condition: TypedExpr,
        body: Vec<TypedStmt>,
    },
    Match {
        scrutinee: TypedExpr,
        arms: Vec<TypedMatchArm>,
    },
    ForEach {
        binding: String,
        iterable: TypedExpr,
        body: Vec<TypedStmt>,
    },
    Return(Option<TypedExpr>),
    Break,
    Continue,
    Expect(TypedExpr),
    Expr(TypedExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedMatchArm {
    pub pattern: TypedPattern,
    pub body: Vec<TypedStmt>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedBindingPattern {
    pub kind: TypedBindingPatternKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedBindingPatternKind {
    Name(TypedBinding),
    Tuple(Vec<TypedBinding>),
    Record {
        name: String,
        fields: Vec<TypedRecordBindingField>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedBinding {
    pub name: String,
    pub ty: Type,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedRecordBindingField {
    pub field: String,
    pub binding: TypedBinding,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedPattern {
    pub kind: TypedPatternKind,
    pub ty: Type,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedPatternKind {
    Wildcard,
    Literal(TypedLiteral),
    Binding(String),
    Variant {
        type_name: String,
        variant_name: String,
        field_names: Vec<String>,
        args: Vec<TypedPattern>,
    },
    Tuple(Vec<TypedPattern>),
    Record {
        name: String,
        fields: Vec<TypedRecordPatternField>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedRecordPatternField {
    pub name: String,
    pub pattern: TypedPattern,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedTarget {
    Name {
        symbol: TypedSymbol,
        ty: Type,
        writable_root: bool,
        span: SourceSpan,
    },
    Field {
        base: Box<TypedTarget>,
        field: String,
        ty: Type,
        writable_root: bool,
        span: SourceSpan,
    },
    Index {
        base: Box<TypedTarget>,
        index: TypedExpr,
        ty: Type,
        writable_root: bool,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedExprKind {
    Literal(TypedLiteral),
    Symbol(TypedSymbol),
    VariantConstructor {
        type_name: String,
        variant_name: String,
        field_names: Vec<String>,
        args: Vec<TypedExpr>,
    },
    Call {
        callee: Box<TypedExpr>,
        args: Vec<TypedCallArg>,
    },
    FieldAccess {
        base: Box<TypedExpr>,
        field: String,
    },
    ResultIsOk {
        target: Box<TypedExpr>,
    },
    ResultIsErr {
        target: Box<TypedExpr>,
    },
    ResultValue {
        target: Box<TypedExpr>,
    },
    ResultError {
        target: Box<TypedExpr>,
    },
    OptionIsSome {
        target: Box<TypedExpr>,
    },
    OptionIsNone {
        target: Box<TypedExpr>,
    },
    OptionValue {
        target: Box<TypedExpr>,
    },
    Index {
        base: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<TypedExpr>,
    },
    Binary {
        left: Box<TypedExpr>,
        op: BinaryOp,
        right: Box<TypedExpr>,
    },
    List(Vec<TypedExpr>),
    Map(Vec<(TypedExpr, TypedExpr)>),
    Tuple(Vec<TypedExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedLiteral {
    Int(i64),
    Dec(String),
    String(String),
    Bool(bool),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedCallArg {
    pub name: Option<String>,
    pub expr: TypedExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedSymbol {
    pub name: String,
    pub kind: Option<SymbolKind>,
}

pub fn lower_module(checked: &CheckedModule) -> Result<TypedIrModule, Vec<Diagnostic>> {
    let mut declarations = Vec::new();
    for decl in &checked.module.declarations {
        declarations.push(lower_decl(checked, decl)?);
    }
    Ok(TypedIrModule {
        module_name: checked
            .module
            .module_decl
            .as_ref()
            .map(|module_decl| module_decl.name.as_string()),
        declarations,
        span: checked.module.span.clone(),
    })
}

pub fn lower_expression(
    checked: &CheckedModule,
    expr: &Expr,
    expr_types: &HashMap<u32, Type>,
) -> Result<TypedExpr, Vec<Diagnostic>> {
    let mut checked = checked.clone();
    checked.expr_types.extend(expr_types.clone());
    lower_expr(&checked, expr)
}

pub fn lower_statement(
    checked: &CheckedModule,
    stmt: &Stmt,
    checked_stmt: &CheckedStmt,
) -> Result<TypedStmt, Vec<Diagnostic>> {
    let mut checked = checked.clone();
    checked.expr_types.extend(checked_stmt.expr_types.clone());
    checked
        .target_types
        .extend(checked_stmt.target_types.clone());
    checked
        .target_root_mutability
        .extend(checked_stmt.target_root_mutability.clone());
    lower_stmt(&checked, stmt)
}

fn lower_decl(checked: &CheckedModule, decl: &Decl) -> Result<TypedDecl, Vec<Diagnostic>> {
    match decl {
        Decl::Const(const_decl) => Ok(TypedDecl::Const(TypedConstDecl {
            name: const_decl.name.clone(),
            ty: lower_type_ref(&const_decl.ty, checked, &const_decl.span)?,
            value: lower_expr(checked, &const_decl.value)?,
            span: const_decl.span.clone(),
        })),
        Decl::Record(record_decl) => {
            let fields = record_decl
                .fields
                .iter()
                .map(|field| {
                    Ok(TypedRecordField {
                        name: field.name.clone(),
                        ty: lower_type_ref(&field.ty, checked, &field.span)?,
                        meaning: field.meaning.clone(),
                    })
                })
                .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?;
            Ok(TypedDecl::Record(TypedRecordDecl {
                name: record_decl.name.clone(),
                fields,
                span: record_decl.span.clone(),
            }))
        }
        Decl::Enum(enum_decl) => Ok(TypedDecl::Enum(TypedEnumDecl {
            name: enum_decl.name.clone(),
            variants: enum_decl
                .variants
                .iter()
                .map(|variant| {
                    Ok(TypedEnumVariant {
                        name: variant.name.clone(),
                        fields: variant
                            .fields
                            .iter()
                            .map(|field| {
                                Ok((
                                    field.name.clone(),
                                    lower_type_ref(&field.ty, checked, &field.span)?,
                                ))
                            })
                            .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
                    })
                })
                .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
            span: enum_decl.span.clone(),
        })),
        Decl::Extern(extern_decl) => {
            let signature = checked
                .externs
                .get(&extern_decl.name)
                .expect("extern signature must exist");
            Ok(TypedDecl::Extern(TypedExternDecl {
                name: extern_decl.name.clone(),
                ty: Type::ExternAction(signature.ty.clone()),
                span: extern_decl.span.clone(),
            }))
        }
        Decl::Action(action_decl) => {
            let ty = Type::Action(
                checked
                    .actions
                    .get(&action_decl.name)
                    .expect("action signature must exist")
                    .clone(),
            );
            Ok(TypedDecl::Action(TypedActionDecl {
                name: action_decl.name.clone(),
                params: action_decl
                    .params
                    .iter()
                    .map(|param| {
                        Ok((
                            param.name.clone(),
                            lower_type_ref(&param.ty, checked, &param.span)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
                body: action_decl
                    .body
                    .iter()
                    .map(|stmt| lower_stmt(checked, stmt))
                    .collect::<Result<Vec<_>, _>>()?,
                ty,
                span: action_decl.span.clone(),
            }))
        }
        Decl::Test(test_decl) => Ok(TypedDecl::Test(TypedTestDecl {
            name: test_decl.name.clone(),
            body: test_decl
                .body
                .iter()
                .map(|stmt| lower_stmt(checked, stmt))
                .collect::<Result<Vec<_>, _>>()?,
            span: test_decl.span.clone(),
        })),
    }
}

fn lower_stmt(checked: &CheckedModule, stmt: &Stmt) -> Result<TypedStmt, Vec<Diagnostic>> {
    let kind = match &stmt.kind {
        StmtKind::IntentBlock {
            goal,
            constraints,
            assumptions,
            properties,
        } => TypedStmtKind::IntentBlock {
            goal: goal.clone(),
            constraints: constraints.clone(),
            assumptions: assumptions.clone(),
            properties: properties.clone(),
        },
        StmtKind::ExplainBlock { lines } => TypedStmtKind::ExplainBlock {
            lines: lines.clone(),
        },
        StmtKind::StepBlock { label, body } => TypedStmtKind::StepBlock {
            label: label.clone(),
            body: body
                .iter()
                .map(|stmt| lower_stmt(checked, stmt))
                .collect::<Result<Vec<_>, _>>()?,
        },
        StmtKind::RequiresClause { condition } => {
            TypedStmtKind::RequiresClause(lower_expr(checked, condition)?)
        }
        StmtKind::EnsuresClause { condition } => {
            TypedStmtKind::EnsuresClause(lower_expr(checked, condition)?)
        }
        StmtKind::ExampleBlock {
            name,
            inputs,
            outputs,
        } => TypedStmtKind::ExampleBlock {
            name: name.clone(),
            inputs: inputs
                .iter()
                .map(|(name, expr)| Ok((name.clone(), lower_expr(checked, expr)?)))
                .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
            outputs: outputs
                .iter()
                .map(|(name, expr)| Ok((name.clone(), lower_expr(checked, expr)?)))
                .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
        },
        StmtKind::Let {
            pattern,
            explicit_type,
            value,
        } => TypedStmtKind::Let {
            pattern: lower_binding_pattern(
                checked,
                pattern,
                &if let Some(explicit_type) = explicit_type {
                    lower_type_ref(explicit_type, checked, &stmt.span)?
                } else {
                    expr_type(checked, value)?
                },
            )?,
            ty: if let Some(explicit_type) = explicit_type {
                lower_type_ref(explicit_type, checked, &stmt.span)?
            } else {
                expr_type(checked, value)?
            },
            value: lower_expr(checked, value)?,
        },
        StmtKind::Var {
            pattern,
            explicit_type,
            value,
        } => TypedStmtKind::Var {
            pattern: lower_binding_pattern(
                checked,
                pattern,
                &if let Some(explicit_type) = explicit_type {
                    lower_type_ref(explicit_type, checked, &stmt.span)?
                } else {
                    expr_type(checked, value)?
                },
            )?,
            ty: if let Some(explicit_type) = explicit_type {
                lower_type_ref(explicit_type, checked, &stmt.span)?
            } else {
                expr_type(checked, value)?
            },
            value: lower_expr(checked, value)?,
        },
        StmtKind::Assign { target, value } => TypedStmtKind::Assign {
            target: lower_target(checked, target)?,
            value: lower_expr(checked, value)?,
        },
        StmtKind::If {
            branches,
            else_branch,
        } => TypedStmtKind::If {
            branches: branches
                .iter()
                .map(|branch| {
                    Ok((
                        lower_expr(checked, &branch.condition)?,
                        branch
                            .body
                            .iter()
                            .map(|stmt| lower_stmt(checked, stmt))
                            .collect::<Result<Vec<_>, _>>()?,
                    ))
                })
                .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
            else_branch: else_branch
                .iter()
                .map(|stmt| lower_stmt(checked, stmt))
                .collect::<Result<Vec<_>, _>>()?,
        },
        StmtKind::While { condition, body } => TypedStmtKind::While {
            condition: lower_expr(checked, condition)?,
            body: body
                .iter()
                .map(|stmt| lower_stmt(checked, stmt))
                .collect::<Result<Vec<_>, _>>()?,
        },
        StmtKind::Match { scrutinee, arms } => TypedStmtKind::Match {
            scrutinee: lower_expr(checked, scrutinee)?,
            arms: arms
                .iter()
                .map(|arm| {
                    Ok(TypedMatchArm {
                        pattern: lower_pattern(
                            checked,
                            &arm.pattern,
                            &expr_type(checked, scrutinee)?,
                        )?,
                        body: arm
                            .body
                            .iter()
                            .map(|stmt| lower_stmt(checked, stmt))
                            .collect::<Result<Vec<_>, _>>()?,
                        span: arm.span.clone(),
                    })
                })
                .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
        },
        StmtKind::ForEach {
            binding,
            iterable,
            body,
        } => TypedStmtKind::ForEach {
            binding: binding.clone(),
            iterable: lower_expr(checked, iterable)?,
            body: body
                .iter()
                .map(|stmt| lower_stmt(checked, stmt))
                .collect::<Result<Vec<_>, _>>()?,
        },
        StmtKind::Return(expr) => TypedStmtKind::Return(
            expr.as_ref()
                .map(|expr| lower_expr(checked, expr))
                .transpose()?,
        ),
        StmtKind::Break => TypedStmtKind::Break,
        StmtKind::Continue => TypedStmtKind::Continue,
        StmtKind::Expect(expr) => TypedStmtKind::Expect(lower_expr(checked, expr)?),
        StmtKind::Expr(expr) => TypedStmtKind::Expr(lower_expr(checked, expr)?),
    };
    Ok(TypedStmt {
        kind,
        span: stmt.span.clone(),
    })
}

fn lower_target(checked: &CheckedModule, target: &Target) -> Result<TypedTarget, Vec<Diagnostic>> {
    match target {
        Target::Name { name, span } => Ok(TypedTarget::Name {
            symbol: TypedSymbol {
                name: name.clone(),
                kind: checked
                    .resolution
                    .symbols
                    .get(name)
                    .map(|symbol| symbol.kind),
            },
            ty: lookup_target_type(checked, span)?,
            writable_root: lookup_target_writable(checked, span)?,
            span: span.clone(),
        }),
        Target::Field { base, field, span } => Ok(TypedTarget::Field {
            base: Box::new(lower_target(checked, base)?),
            field: field.clone(),
            ty: lookup_target_type(checked, span)?,
            writable_root: lookup_target_writable(checked, span)?,
            span: span.clone(),
        }),
        Target::Index { base, index, span } => Ok(TypedTarget::Index {
            base: Box::new(lower_target(checked, base)?),
            index: lower_expr(checked, index)?,
            ty: lookup_target_type(checked, span)?,
            writable_root: lookup_target_writable(checked, span)?,
            span: span.clone(),
        }),
    }
}

fn lower_binding_pattern(
    checked: &CheckedModule,
    pattern: &BindingPattern,
    whole_type: &Type,
) -> Result<TypedBindingPattern, Vec<Diagnostic>> {
    let kind = match &pattern.kind {
        BindingPatternKind::Name(name) => TypedBindingPatternKind::Name(TypedBinding {
            name: name.clone(),
            ty: whole_type.clone(),
            span: pattern.span.clone(),
        }),
        BindingPatternKind::Tuple(names) => {
            let item_types = match whole_type {
                Type::Tuple(types) if types.len() == names.len() => types.clone(),
                Type::Unknown => vec![Type::Unknown; names.len()],
                other => {
                    return Err(vec![Diagnostic::new(
                        pattern.span.clone(),
                        Phase::Lower,
                        format!(
                            "cannot lower tuple destructuring for `{}`",
                            other.describe()
                        ),
                    )]);
                }
            };
            TypedBindingPatternKind::Tuple(
                names
                    .iter()
                    .zip(item_types.iter())
                    .map(|(name, ty)| TypedBinding {
                        name: name.clone(),
                        ty: ty.clone(),
                        span: pattern.span.clone(),
                    })
                    .collect(),
            )
        }
        BindingPatternKind::Record { name, fields } => TypedBindingPatternKind::Record {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|field| lower_record_binding_field(checked, name, field))
                .collect::<Result<Vec<_>, _>>()?,
        },
    };
    Ok(TypedBindingPattern {
        kind,
        span: pattern.span.clone(),
    })
}

fn lower_record_binding_field(
    checked: &CheckedModule,
    record_name: &str,
    field: &RecordBindingField,
) -> Result<TypedRecordBindingField, Vec<Diagnostic>> {
    let ty = checked
        .records
        .get(record_name)
        .and_then(|record| record.fields.get(&field.field))
        .cloned()
        .unwrap_or(Type::Unknown);
    Ok(TypedRecordBindingField {
        field: field.field.clone(),
        binding: TypedBinding {
            name: field.binding.clone(),
            ty,
            span: field.span.clone(),
        },
        span: field.span.clone(),
    })
}

fn lower_pattern(
    checked: &CheckedModule,
    pattern: &Pattern,
    expected: &Type,
) -> Result<TypedPattern, Vec<Diagnostic>> {
    let (kind, ty) = match &pattern.kind {
        PatternKind::Wildcard => (TypedPatternKind::Wildcard, expected.clone()),
        PatternKind::Literal(literal) => (
            TypedPatternKind::Literal(match literal {
                PatternLiteral::Int(value) => TypedLiteral::Int(*value),
                PatternLiteral::Dec(value) => TypedLiteral::Dec(value.clone()),
                PatternLiteral::String(value) => TypedLiteral::String(value.clone()),
                PatternLiteral::Bool(value) => TypedLiteral::Bool(*value),
                PatternLiteral::None => TypedLiteral::None,
            }),
            expected.clone(),
        ),
        PatternKind::Binding(name) => (TypedPatternKind::Binding(name.clone()), expected.clone()),
        PatternKind::Tuple(items) => {
            let item_types = match expected {
                Type::Tuple(types) if types.len() == items.len() => types.clone(),
                Type::Unknown => vec![Type::Unknown; items.len()],
                other => {
                    return Err(vec![Diagnostic::new(
                        pattern.span.clone(),
                        Phase::Lower,
                        format!(
                            "cannot lower tuple pattern for non-tuple type `{}`",
                            other.describe()
                        ),
                    )]);
                }
            };
            (
                TypedPatternKind::Tuple(
                    items
                        .iter()
                        .zip(item_types.iter())
                        .map(|(item, item_ty)| lower_pattern(checked, item, item_ty))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                expected.clone(),
            )
        }
        PatternKind::Record { name, fields } => (
            TypedPatternKind::Record {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|field| lower_record_pattern_field(checked, field, name))
                    .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
            },
            Type::Record(name.clone()),
        ),
        PatternKind::Variant { name, args } => {
            let (type_name, field_names, field_types) =
                lower_pattern_variant_signature(checked, expected, name, &pattern.span)?;
            (
                TypedPatternKind::Variant {
                    type_name: type_name.clone(),
                    variant_name: name.clone(),
                    field_names,
                    args: args
                        .iter()
                        .zip(field_types.iter())
                        .map(|(arg, arg_ty)| lower_pattern(checked, arg, arg_ty))
                        .collect::<Result<Vec<_>, _>>()?,
                },
                type_name_to_type(&type_name, expected),
            )
        }
    };

    Ok(TypedPattern {
        kind,
        ty,
        span: pattern.span.clone(),
    })
}

fn lower_record_pattern_field(
    checked: &CheckedModule,
    field: &RecordPatternField,
    record_name: &str,
) -> Result<TypedRecordPatternField, Vec<Diagnostic>> {
    let field_type = checked
        .records
        .get(record_name)
        .and_then(|record| record.fields.get(&field.name))
        .cloned()
        .unwrap_or(Type::Unknown);
    Ok(TypedRecordPatternField {
        name: field.name.clone(),
        pattern: lower_pattern(checked, &field.pattern, &field_type)?,
        span: field.span.clone(),
    })
}

fn lower_pattern_variant_signature(
    checked: &CheckedModule,
    expected: &Type,
    name: &str,
    span: &SourceSpan,
) -> Result<(String, Vec<String>, Vec<Type>), Vec<Diagnostic>> {
    match (expected, name) {
        (Type::Result(ok, _), "Ok") => Ok((
            "Result".to_string(),
            vec!["value".to_string()],
            vec![(**ok).clone()],
        )),
        (Type::Result(_, err), "Err") => Ok((
            "Result".to_string(),
            vec!["error".to_string()],
            vec![(**err).clone()],
        )),
        (Type::Option(inner), "Some") => Ok((
            "Option".to_string(),
            vec!["value".to_string()],
            vec![(**inner).clone()],
        )),
        (Type::Option(_), "None") | (Type::None, "None") => {
            Ok(("Option".to_string(), Vec::new(), Vec::new()))
        }
        (Type::Enum(enum_name), variant_name) => {
            let variant = checked
                .enums
                .get(enum_name)
                .and_then(|enum_type| enum_type.variants.get(variant_name))
                .ok_or_else(|| {
                    vec![Diagnostic::new(
                        span.clone(),
                        Phase::Lower,
                        format!("unknown enum variant `{variant_name}` for `{enum_name}`"),
                    )]
                })?;
            Ok((
                enum_name.clone(),
                variant.fields.iter().map(|(name, _)| name.clone()).collect(),
                variant.fields.iter().map(|(_, ty)| ty.clone()).collect(),
            ))
        }
        (Type::Unknown, "Ok") => Ok((
            "Result".to_string(),
            vec!["value".to_string()],
            vec![Type::Unknown],
        )),
        (Type::Unknown, "Err") => Ok((
            "Result".to_string(),
            vec!["error".to_string()],
            vec![Type::Unknown],
        )),
        (Type::Unknown, "Some") => Ok((
            "Option".to_string(),
            vec!["value".to_string()],
            vec![Type::Unknown],
        )),
        (Type::Unknown, "None") => Ok(("Option".to_string(), Vec::new(), Vec::new())),
        (Type::Unknown, variant_name) => {
            let matches = checked
                .enums
                .iter()
                .filter_map(|(enum_name, enum_type)| {
                    enum_type
                        .variants
                        .get(variant_name)
                        .map(|variant| (enum_name, variant))
                })
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [(enum_name, variant)] => Ok((
                    (*enum_name).clone(),
                    variant.fields.iter().map(|(name, _)| name.clone()).collect(),
                    variant.fields.iter().map(|(_, ty)| ty.clone()).collect(),
                )),
                _ => Err(vec![Diagnostic::new(
                    span.clone(),
                    Phase::Lower,
                    format!("ambiguous variant pattern `{variant_name}` during lowering"),
                )]),
            }
        }
        (other, variant_name) => Err(vec![Diagnostic::new(
            span.clone(),
            Phase::Lower,
            format!(
                "variant pattern `{variant_name}` cannot be lowered for `{}`",
                other.describe()
            ),
        )]),
    }
}

fn type_name_to_type(type_name: &str, expected: &Type) -> Type {
    match type_name {
        "Option" => match expected {
            Type::Option(_) => expected.clone(),
            _ => Type::Option(Box::new(Type::Unknown)),
        },
        "Result" => match expected {
            Type::Result(_, _) => expected.clone(),
            _ => Type::Result(Box::new(Type::Unknown), Box::new(Type::Unknown)),
        },
        other => Type::Enum(other.to_string()),
    }
}

fn lower_zero_arg_variant_constructor(
    checked: &CheckedModule,
    expr: &Expr,
    name: &str,
) -> Result<Option<TypedExprKind>, Vec<Diagnostic>> {
    let ty = expr_type(checked, expr)?;
    match ty {
        Type::Enum(enum_name) => {
            let Some(variant) = checked
                .enums
                .get(&enum_name)
                .and_then(|enum_type| enum_type.variants.get(name))
            else {
                return Ok(None);
            };
            if !variant.fields.is_empty() {
                return Ok(None);
            }
            Ok(Some(TypedExprKind::VariantConstructor {
                type_name: enum_name,
                variant_name: name.to_string(),
                field_names: Vec::new(),
                args: Vec::new(),
            }))
        }
        _ => Ok(None),
    }
}

fn lower_variant_constructor_call(
    checked: &CheckedModule,
    callee: &Expr,
    args: &[CallArg],
) -> Result<Option<TypedExprKind>, Vec<Diagnostic>> {
    let ExprKind::Name(name) = &callee.kind else {
        return Ok(None);
    };

    let lowered_args = args
        .iter()
        .map(|arg| lower_expr(checked, &arg.expr))
        .collect::<Result<Vec<_>, _>>()?;

    if name == "Some" {
        return Ok(Some(TypedExprKind::VariantConstructor {
            type_name: "Option".to_string(),
            variant_name: "Some".to_string(),
            field_names: vec!["value".to_string()],
            args: lowered_args,
        }));
    }

    if name == "Ok" || name == "Err" {
        return Ok(Some(TypedExprKind::VariantConstructor {
            type_name: "Result".to_string(),
            variant_name: name.clone(),
            field_names: vec![if name == "Ok" {
                "value".to_string()
            } else {
                "error".to_string()
            }],
            args: lowered_args,
        }));
    }

    let variant_type = checked
        .enums
        .iter()
        .find_map(|(enum_name, enum_type)| {
            enum_type.variants.get(name).map(|variant| (enum_name, variant))
        });

    let Some((enum_name, variant)) = variant_type else {
        return Ok(None);
    };

    Ok(Some(TypedExprKind::VariantConstructor {
        type_name: enum_name.clone(),
        variant_name: name.clone(),
        field_names: variant.fields.iter().map(|(field, _)| field.clone()).collect(),
        args: lowered_args,
    }))
}

fn lower_expr(checked: &CheckedModule, expr: &Expr) -> Result<TypedExpr, Vec<Diagnostic>> {
    let ty = expr_type(checked, expr)?;
    let kind = match &expr.kind {
        ExprKind::Int(value) => TypedExprKind::Literal(TypedLiteral::Int(*value)),
        ExprKind::Dec(value) => TypedExprKind::Literal(TypedLiteral::Dec(value.clone())),
        ExprKind::String(value) => TypedExprKind::Literal(TypedLiteral::String(value.clone())),
        ExprKind::Bool(value) => TypedExprKind::Literal(TypedLiteral::Bool(*value)),
        ExprKind::None => TypedExprKind::Literal(TypedLiteral::None),
        ExprKind::List(items) => TypedExprKind::List(
            items
                .iter()
                .map(|item| lower_expr(checked, item))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ExprKind::Map(pairs) => TypedExprKind::Map(
            pairs
                .iter()
                .map(|(key, value)| Ok((lower_expr(checked, key)?, lower_expr(checked, value)?)))
                .collect::<Result<Vec<_>, Vec<Diagnostic>>>()?,
        ),
        ExprKind::Tuple(items) => TypedExprKind::Tuple(
            items
                .iter()
                .map(|item| lower_expr(checked, item))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ExprKind::Name(name) => {
            if let Some(kind) = lower_zero_arg_variant_constructor(checked, expr, name)? {
                kind
            } else {
                TypedExprKind::Symbol(TypedSymbol {
                    name: name.clone(),
                    kind: checked
                        .resolution
                        .symbols
                        .get(name)
                        .map(|symbol| symbol.kind),
                })
            }
        }
        ExprKind::Call { callee, args } => {
            if let Some(kind) = lower_variant_constructor_call(checked, callee, args)? {
                kind
            } else if let Some(kind) = lower_builtin_result_or_option_call(checked, callee, args)? {
                kind
            } else {
                TypedExprKind::Call {
                    callee: Box::new(lower_expr(checked, callee)?),
                    args: args
                        .iter()
                        .map(|arg| lower_call_arg(checked, arg))
                        .collect::<Result<Vec<_>, _>>()?,
                }
            }
        }
        ExprKind::FieldAccess { base, field } => {
            if let Some(action) = lookup_standard_runtime_action(base, field) {
                TypedExprKind::Symbol(TypedSymbol {
                    name: action.qualified_name().to_string(),
                    kind: Some(SymbolKind::Runtime),
                })
            } else {
                TypedExprKind::FieldAccess {
                    base: Box::new(lower_expr(checked, base)?),
                    field: field.clone(),
                }
            }
        }
        ExprKind::Index { base, index } => TypedExprKind::Index {
            base: Box::new(lower_expr(checked, base)?),
            index: Box::new(lower_expr(checked, index)?),
        },
        ExprKind::Unary { op, expr } => TypedExprKind::Unary {
            op: *op,
            expr: Box::new(lower_expr(checked, expr)?),
        },
        ExprKind::Binary { left, op, right } => TypedExprKind::Binary {
            left: Box::new(lower_expr(checked, left)?),
            op: *op,
            right: Box::new(lower_expr(checked, right)?),
        },
    };
    Ok(TypedExpr {
        kind,
        ty,
        span: expr.span.clone(),
    })
}

fn lower_builtin_result_or_option_call(
    checked: &CheckedModule,
    callee: &Expr,
    args: &[CallArg],
) -> Result<Option<TypedExprKind>, Vec<Diagnostic>> {
    let ExprKind::FieldAccess { base, field } = &callee.kind else {
        return Ok(None);
    };
    if !args.is_empty() {
        return Ok(None);
    }

    let base_type = expr_type(checked, base)?;
    let op = match (&base_type, field.as_str()) {
        (Type::Result(_, _), "is_ok") => Some(BuiltinMemberOp::ResultIsOk),
        (Type::Result(_, _), "is_err") => Some(BuiltinMemberOp::ResultIsErr),
        (Type::Result(_, _), "value") => Some(BuiltinMemberOp::ResultValue),
        (Type::Result(_, _), "error") => Some(BuiltinMemberOp::ResultError),
        (Type::Option(_), "is_some") => Some(BuiltinMemberOp::OptionIsSome),
        (Type::Option(_), "is_none") => Some(BuiltinMemberOp::OptionIsNone),
        (Type::Option(_), "value") => Some(BuiltinMemberOp::OptionValue),
        _ => None,
    };

    let Some(op) = op else {
        return Ok(None);
    };

    let target = Box::new(lower_expr(checked, base)?);
    let kind = match op {
        BuiltinMemberOp::ResultIsOk => TypedExprKind::ResultIsOk { target },
        BuiltinMemberOp::ResultIsErr => TypedExprKind::ResultIsErr { target },
        BuiltinMemberOp::ResultValue => TypedExprKind::ResultValue { target },
        BuiltinMemberOp::ResultError => TypedExprKind::ResultError { target },
        BuiltinMemberOp::OptionIsSome => TypedExprKind::OptionIsSome { target },
        BuiltinMemberOp::OptionIsNone => TypedExprKind::OptionIsNone { target },
        BuiltinMemberOp::OptionValue => TypedExprKind::OptionValue { target },
    };

    Ok(Some(kind))
}

enum BuiltinMemberOp {
    ResultIsOk,
    ResultIsErr,
    ResultValue,
    ResultError,
    OptionIsSome,
    OptionIsNone,
    OptionValue,
}

fn lower_call_arg(checked: &CheckedModule, arg: &CallArg) -> Result<TypedCallArg, Vec<Diagnostic>> {
    Ok(TypedCallArg {
        name: arg.name.clone(),
        expr: lower_expr(checked, &arg.expr)?,
    })
}

fn lookup_standard_runtime_action(
    base: &Expr,
    field: &str,
) -> Option<standard_runtime::StandardRuntimeAction> {
    let ExprKind::Name(module_name) = &base.kind else {
        return None;
    };
    standard_runtime::lookup_standard_runtime_member(module_name, field)
}

fn expr_type(checked: &CheckedModule, expr: &Expr) -> Result<Type, Vec<Diagnostic>> {
    checked.expr_types.get(&expr.id).cloned().ok_or_else(|| {
        vec![Diagnostic::new(
            expr.span.clone(),
            Phase::Lower,
            format!("missing type annotation for expression node {}", expr.id),
        )]
    })
}

fn lookup_target_type(checked: &CheckedModule, span: &SourceSpan) -> Result<Type, Vec<Diagnostic>> {
    checked.target_types.get(span).cloned().ok_or_else(|| {
        vec![Diagnostic::new(
            span.clone(),
            Phase::Lower,
            "missing lowered target type",
        )]
    })
}

fn lookup_target_writable(
    checked: &CheckedModule,
    span: &SourceSpan,
) -> Result<bool, Vec<Diagnostic>> {
    checked
        .target_root_mutability
        .get(span)
        .copied()
        .ok_or_else(|| {
            vec![Diagnostic::new(
                span.clone(),
                Phase::Lower,
                "missing lowered target mutability",
            )]
        })
}

fn lower_type_ref(
    type_ref: &crate::ast::TypeRef,
    checked: &CheckedModule,
    span: &SourceSpan,
) -> Result<Type, Vec<Diagnostic>> {
    match type_ref {
        crate::ast::TypeRef::Named(name) => Ok(match name.as_str() {
            "Bool" => Type::Bool,
            "Int" => Type::Int,
            "Dec" => Type::Dec,
            "Text" => Type::Text,
            "Bytes" => Type::Bytes,
            "None" => Type::None,
            other if checked.records.contains_key(other) => Type::Record(other.to_string()),
            other if checked.enums.contains_key(other) => Type::Enum(other.to_string()),
            other => {
                return Err(vec![Diagnostic::new(
                    span.clone(),
                    Phase::Lower,
                    format!("unknown type `{other}` during lowering"),
                )]);
            }
        }),
        crate::ast::TypeRef::Generic { name, args } => match name.as_str() {
            "List" if args.len() == 1 => Ok(Type::List(Box::new(lower_type_ref(
                &args[0], checked, span,
            )?))),
            "Map" if args.len() == 2 => Ok(Type::Map(
                Box::new(lower_type_ref(&args[0], checked, span)?),
                Box::new(lower_type_ref(&args[1], checked, span)?),
            )),
            "Set" if args.len() == 1 => Ok(Type::Set(Box::new(lower_type_ref(
                &args[0], checked, span,
            )?))),
            "Option" if args.len() == 1 => Ok(Type::Option(Box::new(lower_type_ref(
                &args[0], checked, span,
            )?))),
            "Result" if args.len() == 2 => Ok(Type::Result(
                Box::new(lower_type_ref(&args[0], checked, span)?),
                Box::new(lower_type_ref(&args[1], checked, span)?),
            )),
            _ => Err(vec![Diagnostic::new(
                span.clone(),
                Phase::Lower,
                format!("unsupported generic type `{name}` during lowering"),
            )]),
        },
        crate::ast::TypeRef::Tuple(items) => Ok(Type::Tuple(
            items
                .iter()
                .map(|item| lower_type_ref(item, checked, span))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        crate::ast::TypeRef::Action { params, result } => {
            Ok(Type::Action(crate::types::ActionType {
                params: params
                    .iter()
                    .map(|param| lower_type_ref(param, checked, span))
                    .collect::<Result<Vec<_>, _>>()?,
                result: Box::new(lower_type_ref(result, checked, span)?),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolver::Resolver;
    use crate::types::TypeChecker;

    use super::{TypedDecl, TypedStmtKind, lower_module};

    #[test]
    fn lowers_var_assign_test_and_expect_nodes() {
        let source = r#"
record Customer:
  email: Text

action main(customer: Customer) -> Int:
  var current = customer
  current.email := "updated"
  return 1

test smoke:
  expect main(Customer(email: "before")) == 1
"#;
        let path = Path::new("test.vg");
        let tokens = Lexer::new(path, source).tokenize().expect("tokenize");
        let module = Parser::new(path, tokens).parse_module().expect("parse");
        let resolution = Resolver::new(&module).resolve().expect("resolve");
        let checked = TypeChecker::new(&module, &resolution)
            .check()
            .expect("check");
        let lowered = lower_module(&checked).expect("lower");

        match &lowered.declarations[1] {
            TypedDecl::Action(action) => {
                assert!(matches!(action.body[0].kind, TypedStmtKind::Var { .. }));
                assert!(matches!(action.body[1].kind, TypedStmtKind::Assign { .. }));
            }
            other => panic!("unexpected declaration: {other:?}"),
        }
        match &lowered.declarations[2] {
            TypedDecl::Test(test_decl) => {
                assert!(matches!(test_decl.body[0].kind, TypedStmtKind::Expect(_)));
            }
            other => panic!("unexpected declaration: {other:?}"),
        }
    }

    #[test]
    fn lowers_semantic_layer_nodes() {
        let source = r#"
record Customer:
  email: Text
    meaning: "primary contact"

action main(value: Int) -> Int:
  intent:
    goal: "return the value"
  explain:
    "first line"
  requires value > 0
  step compute:
    return value
  ensures result > 0
"#;
        let path = Path::new("test.vg");
        let tokens = Lexer::new(path, source).tokenize().expect("tokenize");
        let module = Parser::new(path, tokens).parse_module().expect("parse");
        let resolution = Resolver::new(&module).resolve().expect("resolve");
        let checked = TypeChecker::new(&module, &resolution)
            .check()
            .expect("check");
        let lowered = lower_module(&checked).expect("lower");

        match &lowered.declarations[0] {
            TypedDecl::Record(record) => {
                assert_eq!(record.fields[0].meaning.as_deref(), Some("primary contact"));
            }
            other => panic!("unexpected declaration: {other:?}"),
        }
        match &lowered.declarations[1] {
            TypedDecl::Action(action) => {
                assert!(matches!(action.body[0].kind, TypedStmtKind::IntentBlock { .. }));
                assert!(matches!(action.body[1].kind, TypedStmtKind::ExplainBlock { .. }));
                assert!(matches!(action.body[2].kind, TypedStmtKind::RequiresClause(_)));
                assert!(matches!(action.body[3].kind, TypedStmtKind::StepBlock { .. }));
                assert!(matches!(action.body[4].kind, TypedStmtKind::EnsuresClause(_)));
            }
            other => panic!("unexpected declaration: {other:?}"),
        }
    }

    #[test]
    fn lowers_result_and_option_member_calls_to_dedicated_nodes() {
        let source = r#"
action result_flag() -> Bool:
  let written = console.print("")
  return written.is_ok()

action option_value() -> Int:
  let maybe: Option[Int] = 10
  return maybe.value()
"#;
        let path = Path::new("test.vg");
        let tokens = Lexer::new(path, source).tokenize().expect("tokenize");
        let module = Parser::new(path, tokens).parse_module().expect("parse");
        let resolution = Resolver::new(&module).resolve().expect("resolve");
        let checked = TypeChecker::new(&module, &resolution)
            .check()
            .expect("check");
        let lowered = lower_module(&checked).expect("lower");

        match &lowered.declarations[0] {
            TypedDecl::Action(action) => match &action.body[1].kind {
                TypedStmtKind::Return(Some(expr)) => {
                    assert!(matches!(expr.kind, super::TypedExprKind::ResultIsOk { .. }));
                }
                other => panic!("unexpected statement: {other:?}"),
            },
            other => panic!("unexpected declaration: {other:?}"),
        }

        match &lowered.declarations[1] {
            TypedDecl::Action(action) => match &action.body[1].kind {
                TypedStmtKind::Return(Some(expr)) => {
                    assert!(matches!(expr.kind, super::TypedExprKind::OptionValue { .. }));
                }
                other => panic!("unexpected statement: {other:?}"),
            },
            other => panic!("unexpected declaration: {other:?}"),
        }
    }

    #[test]
    fn lowers_match_statements_and_variant_patterns() {
        let source = r#"
action main(result: Result[Int, Text]) -> Int:
  match result:
    Ok(value):
      return value
    Err(_):
      return 0
"#;
        let path = Path::new("test.vg");
        let tokens = Lexer::new(path, source).tokenize().expect("tokenize");
        let module = Parser::new(path, tokens).parse_module().expect("parse");
        let resolution = Resolver::new(&module).resolve().expect("resolve");
        let checked = TypeChecker::new(&module, &resolution)
            .check()
            .expect("check");
        let lowered = lower_module(&checked).expect("lower");

        match &lowered.declarations[0] {
            TypedDecl::Action(action) => match &action.body[0].kind {
                TypedStmtKind::Match { arms, .. } => {
                    assert_eq!(arms.len(), 2);
                    assert!(matches!(
                        arms[0].pattern.kind,
                        super::TypedPatternKind::Variant { .. }
                    ));
                    assert!(matches!(
                        arms[1].pattern.kind,
                        super::TypedPatternKind::Variant { .. }
                    ));
                }
                other => panic!("unexpected statement: {other:?}"),
            },
            other => panic!("unexpected declaration: {other:?}"),
        }
    }

    #[test]
    fn lowers_binding_destructuring_patterns() {
        let source = r#"
record Customer:
  name: Text

action main(pair: (Int, Int), customer: Customer) -> Int:
  let (left, right) = pair
  var Customer(name: current) = customer
  return left
"#;
        let path = Path::new("test.vg");
        let tokens = Lexer::new(path, source).tokenize().expect("tokenize");
        let module = Parser::new(path, tokens).parse_module().expect("parse");
        let resolution = Resolver::new(&module).resolve().expect("resolve");
        let checked = TypeChecker::new(&module, &resolution)
            .check()
            .expect("check");
        let lowered = lower_module(&checked).expect("lower");

        match &lowered.declarations[1] {
            TypedDecl::Action(action) => {
                assert!(matches!(
                    action.body[0].kind,
                    TypedStmtKind::Let {
                        pattern: super::TypedBindingPattern {
                            kind: super::TypedBindingPatternKind::Tuple(_),
                            ..
                        },
                        ..
                    }
                ));
                assert!(matches!(
                    action.body[1].kind,
                    TypedStmtKind::Var {
                        pattern: super::TypedBindingPattern {
                            kind: super::TypedBindingPatternKind::Record { .. },
                            ..
                        },
                        ..
                    }
                ));
            }
            other => panic!("unexpected declaration: {other:?}"),
        }
    }
}
