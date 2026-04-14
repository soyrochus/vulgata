use std::collections::HashMap;

use crate::ast::{
    AstModule, BinaryOp, CallArg, Decl, Expr, ExprKind, MatchArm, Param, Pattern, PatternKind,
    PatternLiteral, Purity, RecordPatternField, Stmt, StmtKind, Target, TypeRef, UnaryOp,
};
use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::resolver::{Resolution, SymbolKind};
use crate::standard_runtime::{self, StandardRuntimeAction};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Bool,
    Int,
    Dec,
    Text,
    Bytes,
    None,
    Record(String),
    Enum(String),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Set(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Action(ActionType),
    ExternAction(ActionType),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionType {
    pub params: Vec<Type>,
    pub result: Box<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordType {
    pub fields: HashMap<String, Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumType {
    pub variants: HashMap<String, EnumVariantType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantType {
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternSignature {
    pub purity: Option<Purity>,
    pub ty: ActionType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedModule {
    pub module: AstModule,
    pub resolution: Resolution,
    pub expr_types: HashMap<u32, Type>,
    pub target_types: HashMap<SourceSpan, Type>,
    pub target_root_mutability: HashMap<SourceSpan, bool>,
    pub records: HashMap<String, RecordType>,
    pub enums: HashMap<String, EnumType>,
    pub actions: HashMap<String, ActionType>,
    pub externs: HashMap<String, ExternSignature>,
    pub constants: HashMap<String, Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedExpr {
    pub ty: Type,
    pub expr_types: HashMap<u32, Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplBinding {
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedStmt {
    pub expr_types: HashMap<u32, Type>,
    pub target_types: HashMap<SourceSpan, Type>,
    pub target_root_mutability: HashMap<SourceSpan, bool>,
    pub binding: Option<(String, ReplBinding)>,
}

pub struct TypeChecker<'a> {
    module: &'a AstModule,
    resolution: &'a Resolution,
    records: HashMap<String, RecordType>,
    enums: HashMap<String, EnumType>,
    actions: HashMap<String, ActionType>,
    externs: HashMap<String, ExternSignature>,
    constants: HashMap<String, Type>,
    expr_types: HashMap<u32, Type>,
    target_types: HashMap<SourceSpan, Type>,
    target_root_mutability: HashMap<SourceSpan, bool>,
}

impl<'a> TypeChecker<'a> {
    pub fn new(module: &'a AstModule, resolution: &'a Resolution) -> Self {
        Self {
            module,
            resolution,
            records: HashMap::new(),
            enums: HashMap::new(),
            actions: HashMap::new(),
            externs: HashMap::new(),
            constants: HashMap::new(),
            expr_types: HashMap::new(),
            target_types: HashMap::new(),
            target_root_mutability: HashMap::new(),
        }
    }

    pub fn check(mut self) -> Result<CheckedModule, Vec<Diagnostic>> {
        self.register_top_level_types()?;
        let mut diagnostics = Vec::new();

        for decl in &self.module.declarations {
            match decl {
                Decl::Const(const_decl) => {
                    let expected = self.lower_type_ref(&const_decl.ty, &const_decl.span)?;
                    let mut scope = Scope::default();
                    let actual = self.type_expr(&const_decl.value, &mut scope, false)?;
                    if !is_assignable(&expected, &actual) {
                        diagnostics.push(type_error(
                            &const_decl.value.span,
                            format!(
                                "constant `{}` expected type `{}`, found `{}`",
                                const_decl.name,
                                expected.describe(),
                                actual.describe()
                            ),
                        ));
                    } else {
                        self.constants.insert(const_decl.name.clone(), expected);
                    }
                }
                Decl::Action(action) => {
                    let signature = self
                        .actions
                        .get(&action.name)
                        .expect("action registered in first pass")
                        .clone();
                    let mut scope = Scope::default();
                    for (param, ty) in action.params.iter().zip(signature.params.iter()) {
                        scope.insert(param.name.clone(), ty.clone(), false);
                    }
                    self.type_block(&action.body, &mut scope, false, &signature.result, 0)?;
                }
                Decl::Test(test_decl) => {
                    let mut scope = Scope::default();
                    self.type_block(&test_decl.body, &mut scope, true, &Type::None, 0)?;
                }
                Decl::Extern(_) | Decl::Record(_) | Decl::Enum(_) => {}
            }
        }

        if diagnostics.is_empty() {
            Ok(CheckedModule {
                module: self.module.clone(),
                resolution: self.resolution.clone(),
                expr_types: self.expr_types,
                target_types: self.target_types,
                target_root_mutability: self.target_root_mutability,
                records: self.records,
                enums: self.enums,
                actions: self.actions,
                externs: self.externs,
                constants: self.constants,
            })
        } else {
            Err(diagnostics)
        }
    }

    fn register_top_level_types(&mut self) -> Result<(), Vec<Diagnostic>> {
        for decl in &self.module.declarations {
            match decl {
                Decl::Record(record) => {
                    let mut fields = HashMap::new();
                    for field in &record.fields {
                        let ty = self.lower_type_ref(&field.ty, &field.span)?;
                        fields.insert(field.name.clone(), ty);
                    }
                    self.records
                        .insert(record.name.clone(), RecordType { fields });
                }
                Decl::Enum(enum_decl) => {
                    self.enums.insert(
                        enum_decl.name.clone(),
                        EnumType {
                            variants: HashMap::new(),
                        },
                    );
                    let mut variants = HashMap::new();
                    for variant in &enum_decl.variants {
                        let mut fields = Vec::new();
                        for field in &variant.fields {
                            fields.push((
                                field.name.clone(),
                                self.lower_type_ref(&field.ty, &field.span)?,
                            ));
                        }
                        variants.insert(variant.name.clone(), EnumVariantType { fields });
                    }
                    self.enums.insert(enum_decl.name.clone(), EnumType { variants });
                }
                Decl::Action(action) => {
                    let signature = action_signature(
                        &action.params,
                        action.return_type.as_ref(),
                        &action.span,
                        self,
                    )?;
                    self.actions.insert(action.name.clone(), signature);
                }
                Decl::Extern(extern_decl) => {
                    let signature = action_signature(
                        &extern_decl.params,
                        Some(&extern_decl.return_type),
                        &extern_decl.span,
                        self,
                    )?;
                    self.externs.insert(
                        extern_decl.name.clone(),
                        ExternSignature {
                            purity: extern_decl.purity,
                            ty: signature,
                        },
                    );
                }
                Decl::Const(_) | Decl::Test(_) => {}
            }
        }
        Ok(())
    }

    fn type_block(
        &mut self,
        block: &[Stmt],
        scope: &mut Scope,
        in_test: bool,
        expected_return: &Type,
        loop_depth: usize,
    ) -> Result<(), Vec<Diagnostic>> {
        for stmt in block {
            self.type_stmt(stmt, scope, in_test, expected_return, loop_depth)?;
        }
        Ok(())
    }

    fn type_stmt(
        &mut self,
        stmt: &Stmt,
        scope: &mut Scope,
        in_test: bool,
        expected_return: &Type,
        loop_depth: usize,
    ) -> Result<(), Vec<Diagnostic>> {
        match &stmt.kind {
            StmtKind::IntentBlock { .. } | StmtKind::ExplainBlock { .. } => {}
            StmtKind::StepBlock { body, .. } => {
                self.type_block(body, scope, in_test, expected_return, loop_depth)?;
            }
            StmtKind::RequiresClause { condition } => {
                let condition_type = self.type_expr(condition, scope, in_test)?;
                if condition_type != Type::Bool {
                    return Err(vec![type_error(
                        &condition.span,
                        format!(
                            "requires expression must be `Bool`, found `{}`",
                            condition_type.describe()
                        ),
                    )]);
                }
            }
            StmtKind::EnsuresClause { condition } => {
                let mut ensures_scope = scope.clone();
                ensures_scope.insert("result".to_string(), expected_return.clone(), false);
                let condition_type = self.type_expr(condition, &mut ensures_scope, in_test)?;
                if condition_type != Type::Bool {
                    return Err(vec![type_error(
                        &condition.span,
                        format!(
                            "ensures expression must be `Bool`, found `{}`",
                            condition_type.describe()
                        ),
                    )]);
                }
            }
            StmtKind::ExampleBlock {
                inputs, outputs, ..
            } => {
                for (_, expr) in inputs {
                    self.type_expr(expr, scope, in_test)?;
                }
                for (_, expr) in outputs {
                    let actual = self.type_expr(expr, scope, in_test)?;
                    if !is_assignable(expected_return, &actual) {
                        return Err(vec![type_error(
                            &expr.span,
                            format!(
                                "example output expected `{}`, found `{}`",
                                expected_return.describe(),
                                actual.describe()
                            ),
                        )]);
                    }
                }
            }
            StmtKind::Let {
                name,
                explicit_type,
                value,
            } => {
                let value_type = self.type_expr(value, scope, in_test)?;
                let declared_type = if let Some(explicit_type) = explicit_type {
                    let declared = self.lower_type_ref(explicit_type, &stmt.span)?;
                    if !is_assignable(&declared, &value_type) {
                        return Err(vec![type_error(
                            &value.span,
                            format!(
                                "variable `{name}` expected type `{}`, found `{}`",
                                declared.describe(),
                                value_type.describe()
                            ),
                        )]);
                    }
                    declared
                } else {
                    value_type
                };
                scope.insert(name.clone(), declared_type, false);
            }
            StmtKind::Var {
                name,
                explicit_type,
                value,
            } => {
                let value_type = self.type_expr(value, scope, in_test)?;
                let declared_type = if let Some(explicit_type) = explicit_type {
                    let declared = self.lower_type_ref(explicit_type, &stmt.span)?;
                    if !is_assignable(&declared, &value_type) {
                        return Err(vec![type_error(
                            &value.span,
                            format!(
                                "variable `{name}` expected type `{}`, found `{}`",
                                declared.describe(),
                                value_type.describe()
                            ),
                        )]);
                    }
                    declared
                } else {
                    value_type
                };
                scope.insert(name.clone(), declared_type, true);
            }
            StmtKind::Assign { target, value } => {
                let target_type = self.type_target(target, scope, in_test)?;
                let value_type = self.type_expr(value, scope, in_test)?;
                if !is_assignable(&target_type, &value_type) {
                    return Err(vec![type_error(
                        &value.span,
                        format!(
                            "assignment target expects `{}`, found `{}`",
                            target_type.describe(),
                            value_type.describe()
                        ),
                    )]);
                }
            }
            StmtKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    let condition_type = self.type_expr(&branch.condition, scope, in_test)?;
                    if condition_type != Type::Bool {
                        return Err(vec![type_error(
                            &branch.condition.span,
                            format!(
                                "if condition must be `Bool`, found `{}`",
                                condition_type.describe()
                            ),
                        )]);
                    }
                    let mut branch_scope = scope.clone();
                    self.type_block(
                        &branch.body,
                        &mut branch_scope,
                        in_test,
                        expected_return,
                        loop_depth,
                    )?;
                }
                let mut else_scope = scope.clone();
                self.type_block(
                    else_branch,
                    &mut else_scope,
                    in_test,
                    expected_return,
                    loop_depth,
                )?;
            }
            StmtKind::While { condition, body } => {
                let condition_type = self.type_expr(condition, scope, in_test)?;
                if condition_type != Type::Bool {
                    return Err(vec![type_error(
                        &condition.span,
                        format!(
                            "while condition must be `Bool`, found `{}`",
                            condition_type.describe()
                        ),
                    )]);
                }
                let mut body_scope = scope.clone();
                self.type_block(
                    body,
                    &mut body_scope,
                    in_test,
                    expected_return,
                    loop_depth + 1,
                )?;
            }
            StmtKind::Match { scrutinee, arms } => {
                let scrutinee_type = self.type_expr(scrutinee, scope, in_test)?;
                for arm in arms {
                    self.type_match_arm(
                        arm,
                        &scrutinee_type,
                        scope,
                        in_test,
                        expected_return,
                        loop_depth,
                    )?;
                }
            }
            StmtKind::ForEach {
                binding,
                iterable,
                body,
            } => {
                let iterable_type = self.type_expr(iterable, scope, in_test)?;
                let item_type = match iterable_type {
                    Type::List(inner) | Type::Set(inner) => *inner,
                    other => {
                        return Err(vec![type_error(
                            &iterable.span,
                            format!(
                                "for-each iterable must be a collection, found `{}`",
                                other.describe()
                            ),
                        )]);
                    }
                };
                let mut body_scope = scope.clone();
                body_scope.insert(binding.clone(), item_type, false);
                self.type_block(
                    body,
                    &mut body_scope,
                    in_test,
                    expected_return,
                    loop_depth + 1,
                )?;
            }
            StmtKind::Return(expr) => {
                let actual = if let Some(expr) = expr {
                    self.type_expr(expr, scope, in_test)?
                } else {
                    Type::None
                };
                if !is_assignable(expected_return, &actual) {
                    return Err(vec![type_error(
                        &stmt.span,
                        format!(
                            "return expected `{}`, found `{}`",
                            expected_return.describe(),
                            actual.describe()
                        ),
                    )]);
                }
            }
            StmtKind::Break | StmtKind::Continue => {
                if loop_depth == 0 {
                    return Err(vec![type_error(
                        &stmt.span,
                        "break/continue is only valid inside loops",
                    )]);
                }
            }
            StmtKind::Expect(expr) => {
                let condition_type = self.type_expr(expr, scope, in_test)?;
                if condition_type != Type::Bool {
                    return Err(vec![type_error(
                        &expr.span,
                        format!(
                            "expect expression must be `Bool`, found `{}`",
                            condition_type.describe()
                        ),
                    )]);
                }
            }
            StmtKind::Expr(expr) => {
                self.type_expr(expr, scope, in_test)?;
            }
        }
        Ok(())
    }

    fn type_match_arm(
        &mut self,
        arm: &MatchArm,
        scrutinee_type: &Type,
        scope: &Scope,
        in_test: bool,
        expected_return: &Type,
        loop_depth: usize,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut bindings = HashMap::new();
        self.check_pattern(&arm.pattern, scrutinee_type, &mut bindings)?;

        let mut arm_scope = scope.clone();
        for (name, ty) in bindings {
            arm_scope.insert(name, ty, false);
        }

        self.type_block(
            &arm.body,
            &mut arm_scope,
            in_test,
            expected_return,
            loop_depth,
        )
    }

    fn check_pattern(
        &self,
        pattern: &Pattern,
        expected: &Type,
        bindings: &mut HashMap<String, Type>,
    ) -> Result<(), Vec<Diagnostic>> {
        match &pattern.kind {
            PatternKind::Wildcard => Ok(()),
            PatternKind::Literal(literal) => {
                let literal_type = match literal {
                    PatternLiteral::Int(_) => Type::Int,
                    PatternLiteral::Dec(_) => Type::Dec,
                    PatternLiteral::String(_) => Type::Text,
                    PatternLiteral::Bool(_) => Type::Bool,
                    PatternLiteral::None => Type::None,
                };
                if is_assignable(expected, &literal_type) {
                    Ok(())
                } else {
                    Err(vec![type_error(
                        &pattern.span,
                        format!(
                            "pattern literal `{}` is not compatible with `{}`",
                            describe_pattern_literal(literal),
                            expected.describe()
                        ),
                    )])
                }
            }
            PatternKind::Binding(name) => {
                if bindings.contains_key(name) {
                    return Err(vec![type_error(
                        &pattern.span,
                        format!("duplicate binding `{name}` in match pattern"),
                    )]);
                }
                bindings.insert(name.clone(), expected.clone());
                Ok(())
            }
            PatternKind::Tuple(items) => match expected {
                Type::Tuple(types) if types.len() == items.len() => {
                    for (item, item_type) in items.iter().zip(types.iter()) {
                        self.check_pattern(item, item_type, bindings)?;
                    }
                    Ok(())
                }
                Type::Tuple(types) => Err(vec![type_error(
                    &pattern.span,
                    format!(
                        "tuple pattern has arity {}, but matched value has arity {}",
                        items.len(),
                        types.len()
                    ),
                )]),
                Type::Unknown => {
                    for item in items {
                        self.check_pattern(item, &Type::Unknown, bindings)?;
                    }
                    Ok(())
                }
                other => Err(vec![type_error(
                    &pattern.span,
                    format!("tuple pattern requires a tuple value, found `{}`", other.describe()),
                )]),
            },
            PatternKind::Record { name, fields } => {
                let record = self.records.get(name).ok_or_else(|| {
                    vec![type_error(
                        &pattern.span,
                        format!("unknown record type `{name}` in match pattern"),
                    )]
                })?;

                match expected {
                    Type::Record(expected_name) if expected_name == name => {}
                    Type::Unknown => {}
                    Type::Record(expected_name) => {
                        return Err(vec![type_error(
                            &pattern.span,
                            format!(
                                "record pattern `{name}` does not match value of type `{expected_name}`"
                            ),
                        )]);
                    }
                    other => {
                        return Err(vec![type_error(
                            &pattern.span,
                            format!(
                                "record pattern `{name}` requires `{name}`, found `{}`",
                                other.describe()
                            ),
                        )]);
                    }
                }

                for field in fields {
                    self.check_record_pattern_field(field, record, bindings)?;
                }
                Ok(())
            }
            PatternKind::Variant { name, args } => {
                let (field_types, pattern_type) =
                    self.variant_pattern_signature(name, expected, &pattern.span)?;
                if field_types.len() != args.len() {
                    return Err(vec![type_error(
                        &pattern.span,
                        format!(
                            "variant pattern `{name}` expects {} payload field(s), found {}",
                            field_types.len(),
                            args.len()
                        ),
                    )]);
                }
                if !matches!(expected, Type::Unknown) && !is_assignable(expected, &pattern_type) {
                    return Err(vec![type_error(
                        &pattern.span,
                        format!(
                            "variant pattern `{name}` is not compatible with `{}`",
                            expected.describe()
                        ),
                    )]);
                }
                for (arg, field_type) in args.iter().zip(field_types.iter()) {
                    self.check_pattern(arg, field_type, bindings)?;
                }
                Ok(())
            }
        }
    }

    fn check_record_pattern_field(
        &self,
        field: &RecordPatternField,
        record: &RecordType,
        bindings: &mut HashMap<String, Type>,
    ) -> Result<(), Vec<Diagnostic>> {
        let field_type = record.fields.get(&field.name).ok_or_else(|| {
            vec![type_error(
                &field.span,
                format!("record pattern references unknown field `{}`", field.name),
            )]
        })?;
        self.check_pattern(&field.pattern, field_type, bindings)
    }

    fn variant_pattern_signature(
        &self,
        name: &str,
        expected: &Type,
        span: &SourceSpan,
    ) -> Result<(Vec<Type>, Type), Vec<Diagnostic>> {
        match (expected, name) {
            (Type::Result(ok, _), "Ok") => Ok((vec![(**ok).clone()], expected.clone())),
            (Type::Result(_, err), "Err") => Ok((vec![(**err).clone()], expected.clone())),
            (Type::Option(inner), "Some") => Ok((vec![(**inner).clone()], expected.clone())),
            (Type::Option(_), "None") => Ok((Vec::new(), expected.clone())),
            (Type::None, "None") => Ok((Vec::new(), Type::None)),
            (Type::Enum(enum_name), variant_name) => {
                let enum_type = self.enums.get(enum_name).ok_or_else(|| {
                    vec![type_error(
                        span,
                        format!("unknown enum type `{enum_name}` in match pattern"),
                    )]
                })?;
                let variant = enum_type.variants.get(variant_name).ok_or_else(|| {
                    vec![type_error(
                        span,
                        format!("enum `{enum_name}` has no variant `{variant_name}`"),
                    )]
                })?;
                Ok((
                    variant.fields.iter().map(|(_, ty)| ty.clone()).collect(),
                    Type::Enum(enum_name.clone()),
                ))
            }
            (Type::Unknown, "Ok") => Ok((vec![Type::Unknown], Type::Result(
                Box::new(Type::Unknown),
                Box::new(Type::Unknown),
            ))),
            (Type::Unknown, "Err") => Ok((vec![Type::Unknown], Type::Result(
                Box::new(Type::Unknown),
                Box::new(Type::Unknown),
            ))),
            (Type::Unknown, "Some") => {
                Ok((vec![Type::Unknown], Type::Option(Box::new(Type::Unknown))))
            }
            (Type::Unknown, "None") => Ok((Vec::new(), Type::Unknown)),
            (Type::Unknown, variant_name) => {
                let (enum_name, variant) = self.find_unique_enum_variant(variant_name, span)?;
                Ok((
                    variant.fields.iter().map(|(_, ty)| ty.clone()).collect(),
                    Type::Enum(enum_name),
                ))
            }
            (other, variant_name) => Err(vec![type_error(
                span,
                format!(
                    "variant pattern `{variant_name}` is not valid for `{}`",
                    other.describe()
                ),
            )]),
        }
    }

    fn type_expr(
        &mut self,
        expr: &Expr,
        scope: &mut Scope,
        in_test: bool,
    ) -> Result<Type, Vec<Diagnostic>> {
        let ty = match &expr.kind {
            ExprKind::Int(_) => Type::Int,
            ExprKind::Dec(_) => Type::Dec,
            ExprKind::String(_) => Type::Text,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::None => Type::None,
            ExprKind::List(items) => {
                let mut item_type = Type::Unknown;
                for item in items {
                    let candidate = self.type_expr(item, scope, in_test)?;
                    if item_type == Type::Unknown {
                        item_type = candidate;
                    } else if !is_assignable(&item_type, &candidate) {
                        return Err(vec![type_error(
                            &item.span,
                            format!(
                                "list literal element expected `{}`, found `{}`",
                                item_type.describe(),
                                candidate.describe()
                            ),
                        )]);
                    }
                }
                Type::List(Box::new(item_type))
            }
            ExprKind::Map(pairs) => {
                let mut key_type = Type::Unknown;
                let mut value_type = Type::Unknown;
                for (key, value) in pairs {
                    let current_key = self.type_expr(key, scope, in_test)?;
                    let current_value = self.type_expr(value, scope, in_test)?;
                    if key_type == Type::Unknown {
                        key_type = current_key;
                    } else if !is_assignable(&key_type, &current_key) {
                        return Err(vec![type_error(
                            &key.span,
                            format!(
                                "map literal key expected `{}`, found `{}`",
                                key_type.describe(),
                                current_key.describe()
                            ),
                        )]);
                    }
                    if value_type == Type::Unknown {
                        value_type = current_value;
                    } else if !is_assignable(&value_type, &current_value) {
                        return Err(vec![type_error(
                            &value.span,
                            format!(
                                "map literal value expected `{}`, found `{}`",
                                value_type.describe(),
                                current_value.describe()
                            ),
                        )]);
                    }
                }
                Type::Map(Box::new(key_type), Box::new(value_type))
            }
            ExprKind::Tuple(items) => {
                let mut types = Vec::new();
                for item in items {
                    types.push(self.type_expr(item, scope, in_test)?);
                }
                Type::Tuple(types)
            }
            ExprKind::Name(name) => match self.lookup_name(name, scope, &expr.span) {
                Ok(ty) => ty,
                Err(diagnostics) => match self.find_unique_enum_variant(name, &expr.span) {
                    Ok((enum_name, variant)) if variant.fields.is_empty() => Type::Enum(enum_name),
                    Ok((_, variant)) => {
                        return Err(vec![type_error(
                            &expr.span,
                            format!(
                                "enum variant `{name}` requires {} argument(s)",
                                variant.fields.len()
                            ),
                        )]);
                    }
                    Err(variant_diagnostics)
                        if variant_diagnostics[0].message == format!("unknown enum variant `{name}`") =>
                    {
                        return Err(diagnostics);
                    }
                    Err(variant_diagnostics) => return Err(variant_diagnostics),
                },
            },
            ExprKind::Call { callee, args } => {
                self.type_call(callee, args, scope, in_test, &expr.span)?
            }
            ExprKind::FieldAccess { base, field } => {
                if let Some(action) = lookup_standard_runtime_action(base, field) {
                    Type::Action(action.signature())
                } else {
                    let base_type = self.type_expr(base, scope, in_test)?;
                    match base_type {
                        Type::Record(record_name) => {
                            let record = self.records.get(&record_name).ok_or_else(|| {
                                vec![type_error(
                                    &expr.span,
                                    format!("unknown record type `{record_name}`"),
                                )]
                            })?;
                            record.fields.get(field).cloned().ok_or_else(|| {
                                vec![type_error(
                                    &expr.span,
                                    format!("record `{record_name}` has no field `{field}`"),
                                )]
                            })?
                        }
                        other => {
                            return Err(vec![type_error(
                                &expr.span,
                                format!(
                                    "field access requires a record, found `{}`",
                                    other.describe()
                                ),
                            )]);
                        }
                    }
                }
            }
            ExprKind::Index { base, index } => {
                let base_type = self.type_expr(base, scope, in_test)?;
                let index_type = self.type_expr(index, scope, in_test)?;
                match base_type {
                    Type::List(item) => {
                        if index_type != Type::Int {
                            return Err(vec![type_error(
                                &index.span,
                                format!(
                                    "list index must be `Int`, found `{}`",
                                    index_type.describe()
                                ),
                            )]);
                        }
                        *item
                    }
                    Type::Map(key, value) => {
                        if !is_assignable(&key, &index_type) {
                            return Err(vec![type_error(
                                &index.span,
                                format!(
                                    "map index must be `{}`, found `{}`",
                                    key.describe(),
                                    index_type.describe()
                                ),
                            )]);
                        }
                        *value
                    }
                    other => {
                        return Err(vec![type_error(
                            &expr.span,
                            format!(
                                "indexing requires a list or map, found `{}`",
                                other.describe()
                            ),
                        )]);
                    }
                }
            }
            ExprKind::Unary { op, expr: inner } => {
                let inner_type = self.type_expr(inner, scope, in_test)?;
                match op {
                    UnaryOp::Negate => match inner_type {
                        Type::Int | Type::Dec => inner_type,
                        other => {
                            return Err(vec![type_error(
                                &expr.span,
                                format!(
                                    "unary `-` requires `Int` or `Dec`, found `{}`",
                                    other.describe()
                                ),
                            )]);
                        }
                    },
                    UnaryOp::Not => {
                        if inner_type != Type::Bool {
                            return Err(vec![type_error(
                                &expr.span,
                                format!("`not` requires `Bool`, found `{}`", inner_type.describe()),
                            )]);
                        }
                        Type::Bool
                    }
                }
            }
            ExprKind::Binary { left, op, right } => {
                let left_type = self.type_expr(left, scope, in_test)?;
                let right_type = self.type_expr(right, scope, in_test)?;
                type_binary(op, &left_type, &right_type, &expr.span)?
            }
        };

        self.expr_types.insert(expr.id, ty.clone());
        Ok(ty)
    }

    fn type_call(
        &mut self,
        callee: &Expr,
        args: &[CallArg],
        scope: &mut Scope,
        in_test: bool,
        span: &SourceSpan,
    ) -> Result<Type, Vec<Diagnostic>> {
        if let Some(action) = lookup_standard_runtime_callee(callee) {
            self.expr_types
                .insert(callee.id, Type::Action(action.signature()));
            return self.type_standard_runtime_call(action, args, scope, in_test, span);
        }

        if let ExprKind::Name(name) = &callee.kind {
            if name == "Some" {
                if args.len() != 1 {
                    return Err(vec![type_error(
                        span,
                        "`Some(...)` expects exactly one argument",
                    )]);
                }
                if args[0].name.is_some() {
                    return Err(vec![type_error(
                        &args[0].span,
                        "`Some(...)` does not support named arguments",
                    )]);
                }
                let inner = self.type_expr(&args[0].expr, scope, in_test)?;
                return Ok(Type::Option(Box::new(inner)));
            }
            if name == "Ok" || name == "Err" {
                if args.len() != 1 {
                    return Err(vec![type_error(
                        span,
                        format!("`{name}(...)` expects exactly one argument"),
                    )]);
                }
                if args[0].name.is_some() {
                    return Err(vec![type_error(
                        &args[0].span,
                        format!("`{name}(...)` does not support named arguments"),
                    )]);
                }
                let payload = self.type_expr(&args[0].expr, scope, in_test)?;
                return Ok(match name.as_str() {
                    "Ok" => Type::Result(Box::new(payload), Box::new(Type::Unknown)),
                    "Err" => Type::Result(Box::new(Type::Unknown), Box::new(payload)),
                    _ => unreachable!(),
                });
            }
        }

        if let ExprKind::FieldAccess { base, field } = &callee.kind {
            let base_type = self.type_expr(base, scope, in_test)?;
            if let Some(result_type) =
                builtin_result_or_option_call_type(&base_type, field, args, span)?
            {
                return Ok(result_type);
            }
        }

        if let ExprKind::Name(name) = &callee.kind {
            if let Some(record) = self.records.get(name) {
                self.expr_types
                    .insert(callee.id, Type::Record(name.clone()));
                let mut remaining = record.fields.clone();
                for arg in args {
                    let arg_name = arg.name.clone().ok_or_else(|| {
                        vec![type_error(
                            &arg.span,
                            "record constructors require named arguments",
                        )]
                    })?;
                    let expected = remaining.remove(&arg_name).ok_or_else(|| {
                        vec![type_error(
                            &arg.span,
                            format!("record `{name}` has no field `{arg_name}`"),
                        )]
                    })?;
                    let actual = self.type_expr(&arg.expr, scope, in_test)?;
                    if !is_assignable(&expected, &actual) {
                        return Err(vec![type_error(
                            &arg.expr.span,
                            format!(
                                "field `{arg_name}` expected `{}`, found `{}`",
                                expected.describe(),
                                actual.describe()
                            ),
                        )]);
                    }
                }
                if !remaining.is_empty() {
                    let mut missing: Vec<_> = remaining.keys().cloned().collect();
                    missing.sort();
                    return Err(vec![type_error(
                        span,
                        format!("record `{name}` is missing fields: {}", missing.join(", ")),
                    )]);
                }
                return Ok(Type::Record(name.clone()));
            }
            if let Some(enum_name) =
                self.type_enum_variant_constructor(name, args, scope, in_test, span)?
            {
                return Ok(Type::Enum(enum_name));
            }
        }

        let callee_type = self.type_expr(callee, scope, in_test)?;
        let action_type = match callee_type {
            Type::Action(action) | Type::ExternAction(action) => action,
            other => {
                return Err(vec![type_error(
                    span,
                    format!("call target must be callable, found `{}`", other.describe()),
                )]);
            }
        };

        if action_type.params.len() != args.len() {
            return Err(vec![type_error(
                span,
                format!(
                    "call expects {} arguments, found {}",
                    action_type.params.len(),
                    args.len()
                ),
            )]);
        }

        for (expected, arg) in action_type.params.iter().zip(args) {
            let actual = self.type_expr(&arg.expr, scope, in_test)?;
            if !is_assignable(expected, &actual) {
                return Err(vec![type_error(
                    &arg.expr.span,
                    format!(
                        "argument expected `{}`, found `{}`",
                        expected.describe(),
                        actual.describe()
                    ),
                )]);
            }
        }

        Ok(*action_type.result)
    }

    fn type_standard_runtime_call(
        &mut self,
        action: StandardRuntimeAction,
        args: &[CallArg],
        scope: &mut Scope,
        in_test: bool,
        span: &SourceSpan,
    ) -> Result<Type, Vec<Diagnostic>> {
        let signature = action.signature();

        if signature.params.len() != args.len() {
            return Err(vec![type_error(
                span,
                format!(
                    "call expects {} arguments, found {}",
                    signature.params.len(),
                    args.len()
                ),
            )]);
        }

        for (expected, arg) in signature.params.iter().zip(args) {
            if arg.name.is_some() {
                return Err(vec![type_error(
                    &arg.span,
                    "standard runtime calls do not support named arguments",
                )]);
            }
            let actual = self.type_expr(&arg.expr, scope, in_test)?;
            if !is_assignable(expected, &actual) {
                return Err(vec![type_error(
                    &arg.expr.span,
                    format!(
                        "argument expected `{}`, found `{}`",
                        expected.describe(),
                        actual.describe()
                    ),
                )]);
            }
        }

        Ok(*signature.result)
    }

    fn type_target(
        &mut self,
        target: &Target,
        scope: &mut Scope,
        in_test: bool,
    ) -> Result<Type, Vec<Diagnostic>> {
        let (ty, root_mutable) = match target {
            Target::Name { name, span } => {
                if scope.contains(name) {
                    if !scope.is_mutable(name) {
                        Err(vec![type_error(
                            span,
                            format!("cannot assign to immutable binding `{name}`"),
                        )])
                    } else {
                        Ok((scope.get(name).expect("checked above").clone(), true))
                    }
                } else if self.constants.contains_key(name) {
                    Err(vec![type_error(
                        span,
                        format!("cannot assign to constant `{name}`"),
                    )])
                } else {
                    match self.lookup_name(name, scope, span) {
                        Ok(_) => Err(vec![type_error(
                            span,
                            format!("cannot assign to non-local symbol `{name}`"),
                        )]),
                        Err(diagnostics) => Err(diagnostics),
                    }
                }
            }
            Target::Field { base, field, span } => {
                let (base_type, base_mutable) =
                    self.type_target_with_root_mutability(base, scope, in_test)?;
                match base_type {
                    Type::Record(record_name) => {
                        let record = self.records.get(&record_name).ok_or_else(|| {
                            vec![type_error(
                                span,
                                format!("unknown record type `{record_name}`"),
                            )]
                        })?;
                        record
                            .fields
                            .get(field)
                            .cloned()
                            .ok_or_else(|| {
                                vec![type_error(
                                    span,
                                    format!("record `{record_name}` has no field `{field}`"),
                                )]
                            })
                            .map(|ty| (ty, base_mutable))
                    }
                    other => Err(vec![type_error(
                        span,
                        format!(
                            "field mutation requires a record, found `{}`",
                            other.describe()
                        ),
                    )]),
                }
            }
            Target::Index { base, index, span } => {
                let (base_type, base_mutable) =
                    self.type_target_with_root_mutability(base, scope, in_test)?;
                let index_type = self.type_expr(index, scope, in_test)?;
                match base_type {
                    Type::List(item) => {
                        if index_type != Type::Int {
                            return Err(vec![type_error(
                                &index.span,
                                format!(
                                    "list index must be `Int`, found `{}`",
                                    index_type.describe()
                                ),
                            )]);
                        }
                        Ok((*item, base_mutable))
                    }
                    Type::Map(key, value) => {
                        if !is_assignable(&key, &index_type) {
                            return Err(vec![type_error(
                                &index.span,
                                format!(
                                    "map index must be `{}`, found `{}`",
                                    key.describe(),
                                    index_type.describe()
                                ),
                            )]);
                        }
                        Ok((*value, base_mutable))
                    }
                    other => Err(vec![type_error(
                        span,
                        format!(
                            "index mutation requires a list or map, found `{}`",
                            other.describe()
                        ),
                    )]),
                }
            }
        }?;
        self.target_types.insert(target.span().clone(), ty.clone());
        self.target_root_mutability
            .insert(target.span().clone(), root_mutable);
        Ok(ty)
    }

    fn type_target_with_root_mutability(
        &mut self,
        target: &Target,
        scope: &mut Scope,
        in_test: bool,
    ) -> Result<(Type, bool), Vec<Diagnostic>> {
        let ty = self.type_target(target, scope, in_test)?;
        let root_mutable = self
            .target_root_mutability
            .get(target.span())
            .copied()
            .unwrap_or(false);
        Ok((ty, root_mutable))
    }

    fn lookup_name(
        &self,
        name: &str,
        scope: &Scope,
        span: &SourceSpan,
    ) -> Result<Type, Vec<Diagnostic>> {
        if let Some(ty) = scope.get(name) {
            return Ok(ty.clone());
        }
        if let Some(ty) = self.constants.get(name) {
            return Ok(ty.clone());
        }
        if let Some(action) = self.actions.get(name) {
            return Ok(Type::Action(action.clone()));
        }
        if let Some(extern_decl) = self.externs.get(name) {
            return Ok(Type::ExternAction(extern_decl.ty.clone()));
        }
        if self.records.contains_key(name) {
            return Ok(Type::Record(name.to_string()));
        }
        if self.enums.contains_key(name) {
            return Ok(Type::Enum(name.to_string()));
        }
        if self
            .resolution
            .symbols
            .get(name)
            .is_some_and(|symbol| symbol.kind == SymbolKind::Import)
        {
            return Ok(Type::Unknown);
        }
        if standard_runtime::is_standard_runtime_module(name) {
            return Ok(Type::Unknown);
        }
        Err(vec![Diagnostic::new(
            span.clone(),
            Phase::Resolve,
            format!("unresolved symbol `{name}`"),
        )])
    }

    fn find_unique_enum_variant(
        &self,
        name: &str,
        span: &SourceSpan,
    ) -> Result<(String, &EnumVariantType), Vec<Diagnostic>> {
        let matches = self
            .enums
            .iter()
            .filter_map(|(enum_name, enum_type)| {
                enum_type
                    .variants
                    .get(name)
                    .map(|variant| (enum_name.clone(), variant))
            })
            .collect::<Vec<_>>();

        match matches.as_slice() {
            [] => Err(vec![type_error(
                span,
                format!("unknown enum variant `{name}`"),
            )]),
            [(enum_name, variant)] => Ok((enum_name.clone(), *variant)),
            _ => Err(vec![type_error(
                span,
                format!("ambiguous enum variant `{name}`"),
            )]),
        }
    }

    fn type_enum_variant_constructor(
        &mut self,
        name: &str,
        args: &[CallArg],
        scope: &mut Scope,
        in_test: bool,
        span: &SourceSpan,
    ) -> Result<Option<String>, Vec<Diagnostic>> {
        let (enum_name, variant) = match self.find_unique_enum_variant(name, span) {
            Ok(found) => found,
            Err(_) => return Ok(None),
        };
        let variant_fields = variant.fields.clone();

        let uses_named = args.iter().any(|arg| arg.name.is_some());
        let uses_positional = args.iter().any(|arg| arg.name.is_none());
        if uses_named && uses_positional {
            return Err(vec![type_error(
                span,
                "enum variant constructors may not mix named and positional arguments",
            )]);
        }

        if uses_named {
            let mut remaining = variant_fields
                .iter()
                .cloned()
                .collect::<HashMap<String, Type>>();
            for arg in args {
                let arg_name = arg.name.clone().ok_or_else(|| {
                    vec![type_error(
                        &arg.span,
                        "enum variant constructors require named arguments consistently",
                    )]
                })?;
                let expected = remaining.remove(&arg_name).ok_or_else(|| {
                    vec![type_error(
                        &arg.span,
                        format!("enum variant `{name}` has no field `{arg_name}`"),
                    )]
                })?;
                let actual = self.type_expr(&arg.expr, scope, in_test)?;
                if !is_assignable(&expected, &actual) {
                    return Err(vec![type_error(
                        &arg.expr.span,
                        format!(
                            "field `{arg_name}` expected `{}`, found `{}`",
                            expected.describe(),
                            actual.describe()
                        ),
                    )]);
                }
            }
            if !remaining.is_empty() {
                let mut missing: Vec<_> = remaining.keys().cloned().collect();
                missing.sort();
                return Err(vec![type_error(
                    span,
                    format!(
                        "enum variant `{name}` is missing fields: {}",
                        missing.join(", ")
                    ),
                )]);
            }
        } else {
            if args.len() != variant_fields.len() {
                return Err(vec![type_error(
                    span,
                    format!(
                        "enum variant `{name}` expects {} argument(s), found {}",
                        variant_fields.len(),
                        args.len()
                    ),
                )]);
            }
            for (arg, (_, expected)) in args.iter().zip(variant_fields.iter()) {
                let actual = self.type_expr(&arg.expr, scope, in_test)?;
                if !is_assignable(expected, &actual) {
                    return Err(vec![type_error(
                        &arg.expr.span,
                        format!(
                            "enum variant `{name}` expected `{}`, found `{}`",
                            expected.describe(),
                            actual.describe()
                        ),
                    )]);
                }
            }
        }

        Ok(Some(enum_name))
    }

    pub(crate) fn lower_type_ref(
        &self,
        type_ref: &TypeRef,
        span: &SourceSpan,
    ) -> Result<Type, Vec<Diagnostic>> {
        match type_ref {
            TypeRef::Named(name) => Ok(match name.as_str() {
                "Bool" => Type::Bool,
                "Int" => Type::Int,
                "Dec" => Type::Dec,
                "Text" => Type::Text,
                "Bytes" => Type::Bytes,
                "None" => Type::None,
                other if self.records.contains_key(other) => Type::Record(other.to_string()),
                other if self.enums.contains_key(other) => Type::Enum(other.to_string()),
                other => return Err(vec![type_error(span, format!("unknown type `{other}`"))]),
            }),
            TypeRef::Generic { name, args } => match name.as_str() {
                "List" if args.len() == 1 => {
                    Ok(Type::List(Box::new(self.lower_type_ref(&args[0], span)?)))
                }
                "Map" if args.len() == 2 => Ok(Type::Map(
                    Box::new(self.lower_type_ref(&args[0], span)?),
                    Box::new(self.lower_type_ref(&args[1], span)?),
                )),
                "Set" if args.len() == 1 => {
                    Ok(Type::Set(Box::new(self.lower_type_ref(&args[0], span)?)))
                }
                "Option" if args.len() == 1 => {
                    Ok(Type::Option(Box::new(self.lower_type_ref(&args[0], span)?)))
                }
                "Result" if args.len() == 2 => Ok(Type::Result(
                    Box::new(self.lower_type_ref(&args[0], span)?),
                    Box::new(self.lower_type_ref(&args[1], span)?),
                )),
                _ => Err(vec![type_error(
                    span,
                    format!("unsupported generic type `{name}`"),
                )]),
            },
            TypeRef::Tuple(items) => {
                let mut types = Vec::new();
                for item in items {
                    types.push(self.lower_type_ref(item, span)?);
                }
                Ok(Type::Tuple(types))
            }
            TypeRef::Action { params, result } => {
                let mut lowered_params = Vec::new();
                for param in params {
                    lowered_params.push(self.lower_type_ref(param, span)?);
                }
                let lowered_result = self.lower_type_ref(result, span)?;
                Ok(Type::Action(ActionType {
                    params: lowered_params,
                    result: Box::new(lowered_result),
                }))
            }
        }
    }
}

pub fn check_expression(
    checked: &CheckedModule,
    expr: &Expr,
) -> Result<CheckedExpr, Vec<Diagnostic>> {
    check_expression_with_bindings(checked, expr, &HashMap::new())
}

pub fn check_expression_with_bindings(
    checked: &CheckedModule,
    expr: &Expr,
    bindings: &HashMap<String, ReplBinding>,
) -> Result<CheckedExpr, Vec<Diagnostic>> {
    let mut checker = TypeChecker::new(&checked.module, &checked.resolution);
    checker.register_top_level_types()?;
    checker.constants = checked.constants.clone();
    let mut scope = Scope::default();
    for (name, binding) in bindings {
        scope.insert(name.clone(), binding.ty.clone(), binding.mutable);
    }
    let ty = checker.type_expr(expr, &mut scope, false)?;
    Ok(CheckedExpr {
        ty,
        expr_types: checker.expr_types,
    })
}

pub fn check_statement(
    checked: &CheckedModule,
    stmt: &Stmt,
    bindings: &HashMap<String, ReplBinding>,
) -> Result<CheckedStmt, Vec<Diagnostic>> {
    match &stmt.kind {
        StmtKind::Let { .. }
        | StmtKind::Var { .. }
        | StmtKind::Assign { .. }
        | StmtKind::Expr(_) => {}
        _ => {
            return Err(vec![Diagnostic::new(
                stmt.span.clone(),
                Phase::Cli,
                "repl currently supports `let`, `var`, `:=`, and expression input",
            )]);
        }
    }

    let mut checker = TypeChecker::new(&checked.module, &checked.resolution);
    checker.register_top_level_types()?;
    checker.constants = checked.constants.clone();
    let mut scope = Scope::default();
    for (name, binding) in bindings {
        scope.insert(name.clone(), binding.ty.clone(), binding.mutable);
    }
    checker.type_stmt(stmt, &mut scope, false, &Type::None, 0)?;

    let binding = match &stmt.kind {
        StmtKind::Let {
            name,
            explicit_type,
            value,
        } => Some((
            name.clone(),
            ReplBinding {
                ty: if let Some(explicit_type) = explicit_type {
                    checker.lower_type_ref(explicit_type, &stmt.span)?
                } else {
                    checker.expr_types.get(&value.id).cloned().ok_or_else(|| {
                        vec![Diagnostic::new(
                            value.span.clone(),
                            Phase::TypeCheck,
                            "missing inferred type for repl binding",
                        )]
                    })?
                },
                mutable: false,
            },
        )),
        StmtKind::Var {
            name,
            explicit_type,
            value,
        } => Some((
            name.clone(),
            ReplBinding {
                ty: if let Some(explicit_type) = explicit_type {
                    checker.lower_type_ref(explicit_type, &stmt.span)?
                } else {
                    checker.expr_types.get(&value.id).cloned().ok_or_else(|| {
                        vec![Diagnostic::new(
                            value.span.clone(),
                            Phase::TypeCheck,
                            "missing inferred type for repl binding",
                        )]
                    })?
                },
                mutable: true,
            },
        )),
        StmtKind::Assign { .. } | StmtKind::Expr(_) => None,
        _ => None,
    };

    Ok(CheckedStmt {
        expr_types: checker.expr_types,
        target_types: checker.target_types,
        target_root_mutability: checker.target_root_mutability,
        binding,
    })
}

#[derive(Debug, Clone, Default)]
struct Scope {
    bindings: HashMap<String, BindingInfo>,
}

#[derive(Debug, Clone)]
struct BindingInfo {
    ty: Type,
    mutable: bool,
}

impl Scope {
    fn insert(&mut self, name: String, ty: Type, mutable: bool) {
        self.bindings.insert(name, BindingInfo { ty, mutable });
    }

    fn get(&self, name: &str) -> Option<&Type> {
        self.bindings.get(name).map(|binding| &binding.ty)
    }

    fn contains(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    fn is_mutable(&self, name: &str) -> bool {
        self.bindings
            .get(name)
            .map(|binding| binding.mutable)
            .unwrap_or(false)
    }
}

fn action_signature(
    params: &[Param],
    return_type: Option<&TypeRef>,
    span: &SourceSpan,
    checker: &TypeChecker<'_>,
) -> Result<ActionType, Vec<Diagnostic>> {
    let mut lowered_params = Vec::new();
    for param in params {
        lowered_params.push(checker.lower_type_ref(&param.ty, &param.span)?);
    }
    let result = if let Some(return_type) = return_type {
        checker.lower_type_ref(return_type, span)?
    } else {
        Type::None
    };
    Ok(ActionType {
        params: lowered_params,
        result: Box::new(result),
    })
}

fn type_binary(
    op: &BinaryOp,
    left: &Type,
    right: &Type,
    span: &SourceSpan,
) -> Result<Type, Vec<Diagnostic>> {
    match op {
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply
        | BinaryOp::Divide
        | BinaryOp::Modulo => {
            if left == right && (*left == Type::Int || *left == Type::Dec) {
                Ok(left.clone())
            } else {
                Err(vec![type_error(
                    span,
                    format!(
                        "arithmetic operands must both be `Int` or both be `Dec`, found `{}` and `{}`",
                        left.describe(),
                        right.describe()
                    ),
                )])
            }
        }
        BinaryOp::Equal | BinaryOp::NotEqual => {
            if is_assignable(left, right) || is_assignable(right, left) {
                Ok(Type::Bool)
            } else {
                Err(vec![type_error(
                    span,
                    format!(
                        "equality operands must be comparable, found `{}` and `{}`",
                        left.describe(),
                        right.describe()
                    ),
                )])
            }
        }
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            if left == right && (*left == Type::Int || *left == Type::Dec) {
                Ok(Type::Bool)
            } else {
                Err(vec![type_error(
                    span,
                    format!(
                        "comparison operands must both be `Int` or both be `Dec`, found `{}` and `{}`",
                        left.describe(),
                        right.describe()
                    ),
                )])
            }
        }
        BinaryOp::And | BinaryOp::Or => {
            if *left == Type::Bool && *right == Type::Bool {
                Ok(Type::Bool)
            } else {
                Err(vec![type_error(
                    span,
                    format!(
                        "boolean operands must both be `Bool`, found `{}` and `{}`",
                        left.describe(),
                        right.describe()
                    ),
                )])
            }
        }
    }
}

fn is_assignable(expected: &Type, actual: &Type) -> bool {
    match (expected, actual) {
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (left, right) if left == right => true,
        (Type::Option(_), Type::None) => true,
        (Type::Option(expected_inner), Type::Option(actual_inner)) => {
            is_assignable(expected_inner, actual_inner)
        }
        (Type::Option(expected_inner), other) => is_assignable(expected_inner, other),
        (Type::Result(expected_ok, expected_err), Type::Result(actual_ok, actual_err)) => {
            is_assignable(expected_ok, actual_ok) && is_assignable(expected_err, actual_err)
        }
        (Type::List(expected_inner), Type::List(actual_inner))
        | (Type::Set(expected_inner), Type::Set(actual_inner)) => {
            is_assignable(expected_inner, actual_inner)
        }
        (Type::Map(expected_key, expected_value), Type::Map(actual_key, actual_value)) => {
            is_assignable(expected_key, actual_key) && is_assignable(expected_value, actual_value)
        }
        (Type::Tuple(expected_items), Type::Tuple(actual_items))
            if expected_items.len() == actual_items.len() =>
        {
            expected_items
                .iter()
                .zip(actual_items.iter())
                .all(|(expected_item, actual_item)| is_assignable(expected_item, actual_item))
        }
        (Type::Dec, Type::Int) => true,
        _ => false,
    }
}

fn type_error(span: &SourceSpan, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(span.clone(), Phase::TypeCheck, message)
}

fn lookup_standard_runtime_action(base: &Expr, field: &str) -> Option<StandardRuntimeAction> {
    let ExprKind::Name(module_name) = &base.kind else {
        return None;
    };
    standard_runtime::lookup_standard_runtime_member(module_name, field)
}

fn lookup_standard_runtime_callee(callee: &Expr) -> Option<StandardRuntimeAction> {
    let ExprKind::FieldAccess { base, field } = &callee.kind else {
        return None;
    };
    lookup_standard_runtime_action(base, field)
}

fn builtin_result_or_option_call_type(
    base_type: &Type,
    field: &str,
    args: &[CallArg],
    span: &SourceSpan,
) -> Result<Option<Type>, Vec<Diagnostic>> {
    let is_builtin = matches!(
        field,
        "is_ok" | "is_err" | "value" | "error" | "is_some" | "is_none"
    );
    if !is_builtin {
        return Ok(None);
    }

    if !args.is_empty() {
        return Err(vec![type_error(
            span,
            format!("built-in operation `{field}()` takes no arguments"),
        )]);
    }

    let result_type = match (base_type, field) {
        (Type::Result(_, _), "is_ok" | "is_err") => Some(Type::Bool),
        (Type::Result(ok, _), "value") => Some((**ok).clone()),
        (Type::Result(_, err), "error") => Some((**err).clone()),
        (Type::Option(_), "is_some" | "is_none") => Some(Type::Bool),
        (Type::Option(inner), "value") => Some((**inner).clone()),
        (other, "is_ok" | "is_err" | "error") => {
            return Err(vec![type_error(
                span,
                format!(
                    "built-in operation `{field}()` requires `Result[_, _]`, found `{}`",
                    other.describe()
                ),
            )]);
        }
        (other, "is_some" | "is_none") => {
            return Err(vec![type_error(
                span,
                format!(
                    "built-in operation `{field}()` requires `Option[_]`, found `{}`",
                    other.describe()
                ),
            )]);
        }
        (other, "value") => {
            return Err(vec![type_error(
                span,
                format!(
                    "built-in operation `value()` requires `Result[_, _]` or `Option[_]`, found `{}`",
                    other.describe()
                ),
            )]);
        }
        _ => None,
    };

    Ok(result_type)
}

fn describe_pattern_literal(literal: &PatternLiteral) -> String {
    match literal {
        PatternLiteral::Int(value) => value.to_string(),
        PatternLiteral::Dec(value) => value.clone(),
        PatternLiteral::String(value) => format!("{value:?}"),
        PatternLiteral::Bool(value) => value.to_string(),
        PatternLiteral::None => "None".to_string(),
    }
}

impl Type {
    pub fn describe(&self) -> String {
        match self {
            Type::Bool => "Bool".to_string(),
            Type::Int => "Int".to_string(),
            Type::Dec => "Dec".to_string(),
            Type::Text => "Text".to_string(),
            Type::Bytes => "Bytes".to_string(),
            Type::None => "None".to_string(),
            Type::Record(name) => name.clone(),
            Type::Enum(name) => name.clone(),
            Type::List(inner) => format!("List[{}]", inner.describe()),
            Type::Map(key, value) => format!("Map[{}, {}]", key.describe(), value.describe()),
            Type::Set(inner) => format!("Set[{}]", inner.describe()),
            Type::Option(inner) => format!("Option[{}]", inner.describe()),
            Type::Result(ok, err) => format!("Result[{}, {}]", ok.describe(), err.describe()),
            Type::Tuple(items) => {
                let items: Vec<_> = items.iter().map(Type::describe).collect();
                format!("({})", items.join(", "))
            }
            Type::Action(action) | Type::ExternAction(action) => {
                let params: Vec<_> = action.params.iter().map(Type::describe).collect();
                format!(
                    "Action[{} -> {}]",
                    params.join(", "),
                    action.result.describe()
                )
            }
            Type::Unknown => "Unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolver::Resolver;

    use super::TypeChecker;

    fn check(source: &str) -> Result<super::CheckedModule, Vec<crate::diagnostics::Diagnostic>> {
        let path = Path::new("test.vg");
        let tokens = Lexer::new(path, source).tokenize().expect("tokenize");
        let module = Parser::new(path, tokens).parse_module().expect("parse");
        let resolution = Resolver::new(&module).resolve().expect("resolve");
        TypeChecker::new(&module, &resolution).check()
    }

    #[test]
    fn reports_type_mismatch() {
        let diagnostics = check(
            r#"
action main() -> Int:
  let x: Int = "bad"
  return x
"#,
        )
        .expect_err("check should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::TypeCheck);
    }

    #[test]
    fn reports_unresolved_symbol() {
        let diagnostics = check(
            r#"
action main() -> Int:
  return missing_value
"#,
        )
        .expect_err("check should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::Resolve);
    }

    #[test]
    fn reports_invalid_mutation_target() {
        let diagnostics = check(
            r#"
action main() -> None:
  var count = 1
  count.value := 2
  return
"#,
        )
        .expect_err("check should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::TypeCheck);
    }

    #[test]
    fn rejects_assignment_to_immutable_let() {
        let diagnostics = check(
            r#"
action main() -> Int:
  let count = 1
  count := 2
  return count
"#,
        )
        .expect_err("check should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::TypeCheck);
        assert!(
            diagnostics[0]
                .message
                .contains("cannot assign to immutable binding `count`")
        );
    }

    #[test]
    fn rejects_field_write_through_immutable_let() {
        let diagnostics = check(
            r#"
record Customer:
  email: Text

action main() -> Text:
  let customer = Customer(email: "before")
  customer.email := "after"
  return customer.email
"#,
        )
        .expect_err("check should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::TypeCheck);
        assert!(
            diagnostics[0]
                .message
                .contains("cannot assign to immutable binding `customer`")
        );
    }

    #[test]
    fn rejects_assignment_to_action_parameter() {
        let diagnostics = check(
            r#"
action main(count: Int) -> Int:
  count := count + 1
  return count
"#,
        )
        .expect_err("check should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::TypeCheck);
        assert!(
            diagnostics[0]
                .message
                .contains("cannot assign to immutable binding `count`")
        );
    }

    #[test]
    fn allows_copying_parameter_into_var_before_mutation() {
        check(
            r#"
record Customer:
  email: Text

action main(customer: Customer) -> Text:
  var current = customer
  current.email := "after"
  return customer.email
"#,
        )
        .expect("check should succeed");
    }

    #[test]
    fn rejects_non_boolean_requires_clause() {
        let diagnostics = check(
            r#"
action main(value: Int) -> Int:
  requires value + 1
  return value
"#,
        )
        .expect_err("check should fail");
        assert!(diagnostics[0].message.contains("requires expression must be `Bool`"));
    }

    #[test]
    fn allows_result_binding_inside_ensures() {
        check(
            r#"
action main(value: Int) -> Int:
  ensures result > 0
  return value
"#,
        )
        .expect("check should succeed");
    }

    #[test]
    fn allows_expect_inside_action_bodies() {
        check(
            r#"
action main() -> Int:
  expect true
  return 1
"#,
        )
        .expect("check should succeed");
    }

    #[test]
    fn rejects_is_ok_on_non_result_receiver() {
        let diagnostics = check(
            r#"
action main() -> Bool:
  let value = 1
  return value.is_ok()
"#,
        )
        .expect_err("check should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::TypeCheck);
        assert!(
            diagnostics[0]
                .message
                .contains("built-in operation `is_ok()` requires `Result[_, _]`")
        );
    }

    #[test]
    fn rejects_duplicate_bindings_in_match_patterns() {
        let diagnostics = check(
            r#"
action main(value: (Int, Int)) -> Int:
  match value:
    (x, x):
      return x
  return 0
"#,
        )
        .expect_err("check should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::TypeCheck);
        assert!(diagnostics[0].message.contains("duplicate binding `x`"));
    }

    #[test]
    fn keeps_match_bindings_scoped_to_their_arm() {
        let diagnostics = check(
            r#"
action main(result: Result[Int, Text]) -> Int:
  match result:
    Ok(value):
      return value
    Err(_):
      return 0
  return value
"#,
        )
        .expect_err("check should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::Resolve);
        assert!(diagnostics[0].message.contains("unresolved symbol `value`"));
    }
}
