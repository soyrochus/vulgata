use crate::ast::{BinaryOp, CallArg, Decl, Expr, ExprKind, Stmt, StmtKind, Target, UnaryOp};
use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::resolver::SymbolKind;
use crate::types::{CheckedModule, Type};

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
    pub fields: Vec<(String, Type)>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedEnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub span: SourceSpan,
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
    Let {
        name: String,
        ty: Type,
        value: TypedExpr,
    },
    Var {
        name: String,
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
    Call {
        callee: Box<TypedExpr>,
        args: Vec<TypedCallArg>,
    },
    FieldAccess {
        base: Box<TypedExpr>,
        field: String,
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
                    Ok((
                        field.name.clone(),
                        lower_type_ref(&field.ty, checked, &field.span)?,
                    ))
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
                .map(|variant| variant.name.clone())
                .collect(),
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
        StmtKind::Let {
            name,
            explicit_type,
            value,
        } => TypedStmtKind::Let {
            name: name.clone(),
            ty: if let Some(explicit_type) = explicit_type {
                lower_type_ref(explicit_type, checked, &stmt.span)?
            } else {
                expr_type(checked, value)?
            },
            value: lower_expr(checked, value)?,
        },
        StmtKind::Var {
            name,
            explicit_type,
            value,
        } => TypedStmtKind::Var {
            name: name.clone(),
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
        ExprKind::Name(name) => TypedExprKind::Symbol(TypedSymbol {
            name: name.clone(),
            kind: checked
                .resolution
                .symbols
                .get(name)
                .map(|symbol| symbol.kind),
        }),
        ExprKind::Call { callee, args } => TypedExprKind::Call {
            callee: Box::new(lower_expr(checked, callee)?),
            args: args
                .iter()
                .map(|arg| lower_call_arg(checked, arg))
                .collect::<Result<Vec<_>, _>>()?,
        },
        ExprKind::FieldAccess { base, field } => TypedExprKind::FieldAccess {
            base: Box::new(lower_expr(checked, base)?),
            field: field.clone(),
        },
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

fn lower_call_arg(checked: &CheckedModule, arg: &CallArg) -> Result<TypedCallArg, Vec<Diagnostic>> {
    Ok(TypedCallArg {
        name: arg.name.clone(),
        expr: lower_expr(checked, &arg.expr)?,
    })
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
    checked.target_root_mutability.get(span).copied().ok_or_else(|| {
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
}
