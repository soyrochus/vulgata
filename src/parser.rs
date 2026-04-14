use std::path::{Path, PathBuf};

use crate::ast::{
    ActionDecl, AstModule, BinaryOp, BindingPattern, BindingPatternKind, CallArg,
    ConditionalBranch, ConstDecl, Decl, EnumDecl, EnumVariant, Expr, ExprKind, ExternDecl,
    FieldDecl, ImportDecl, ImportKind, MatchArm, ModuleDecl, ModuleName, NodeId, Param, Pattern,
    PatternKind, PatternLiteral, Purity, RecordBindingField, RecordDecl, RecordPatternField, Stmt,
    StmtKind, Target, TestDecl, TypeRef, UnaryOp, VariantField,
};
use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::lexer::{SpannedToken, Token};

pub struct Parser {
    path: PathBuf,
    tokens: Vec<SpannedToken>,
    position: usize,
    next_node_id: NodeId,
    action_body_depth: usize,
}

impl Parser {
    pub fn new(path: &Path, tokens: Vec<SpannedToken>) -> Self {
        Self {
            path: path.to_path_buf(),
            tokens,
            position: 0,
            next_node_id: 1,
            action_body_depth: 0,
        }
    }

    pub fn parse_module(mut self) -> Result<AstModule, Vec<Diagnostic>> {
        let span = self.current_span();
        let mut module_decl = None;
        let mut imports = Vec::new();
        let mut declarations = Vec::new();

        self.skip_newlines();
        while !self.at(&Token::Eof) {
            if self.at(&Token::Module) {
                if module_decl.is_some() {
                    return Err(vec![
                        self.error_here("module declaration may only appear once"),
                    ]);
                }
                module_decl = Some(self.parse_module_decl()?);
            } else if self.at(&Token::Import) || self.at(&Token::From) {
                imports.push(self.parse_import_decl()?);
            } else {
                declarations.push(self.parse_decl()?);
            }
            self.skip_newlines();
        }

        Ok(AstModule {
            module_decl,
            imports,
            declarations,
            span,
        })
    }

    pub fn parse_expression(mut self) -> Result<Expr, Vec<Diagnostic>> {
        self.skip_newlines();
        let expr = self.parse_expr()?;
        self.skip_newlines();
        if !self.at(&Token::Eof) {
            return Err(vec![
                self.error_here("expected end of input after expression"),
            ]);
        }
        Ok(expr)
    }

    pub fn parse_statement(mut self) -> Result<Stmt, Vec<Diagnostic>> {
        self.skip_newlines();
        let stmt = self.parse_stmt()?;
        self.skip_newlines();
        if !self.at(&Token::Eof) {
            return Err(vec![
                self.error_here("expected end of input after statement"),
            ]);
        }
        Ok(stmt)
    }

    fn parse_module_decl(&mut self) -> Result<ModuleDecl, Vec<Diagnostic>> {
        let span = self.expect_simple(Token::Module, "expected `module`")?.span;
        let name = self.parse_module_name()?;
        self.expect_newline("expected newline after module declaration")?;
        Ok(ModuleDecl { name, span })
    }

    fn parse_import_decl(&mut self) -> Result<ImportDecl, Vec<Diagnostic>> {
        let span = self.current_span();
        if self.at(&Token::Import) {
            self.bump();
            let module = self.parse_module_name()?;
            let alias = if self.at(&Token::As) {
                self.bump();
                Some(self.expect_identifier("expected alias name after `as`")?)
            } else {
                None
            };
            self.expect_newline("expected newline after import")?;
            Ok(ImportDecl {
                kind: ImportKind::Module { module, alias },
                span,
            })
        } else {
            self.expect_simple(Token::From, "expected `from`")?;
            let module = self.parse_module_name()?;
            self.expect_simple(
                Token::Import,
                "expected `import` in from-import declaration",
            )?;
            let mut names = vec![self.expect_identifier("expected imported symbol name")?];
            while self.at(&Token::Comma) {
                self.bump();
                names.push(self.expect_identifier("expected imported symbol name")?);
            }
            self.expect_newline("expected newline after import")?;
            Ok(ImportDecl {
                kind: ImportKind::From { module, names },
                span,
            })
        }
    }

    fn parse_decl(&mut self) -> Result<Decl, Vec<Diagnostic>> {
        match self.current_token() {
            Token::Const => self.parse_const_decl().map(Decl::Const),
            Token::Record => self.parse_record_decl().map(Decl::Record),
            Token::Enum => self.parse_enum_decl().map(Decl::Enum),
            Token::Extern => self.parse_extern_decl().map(Decl::Extern),
            Token::Action => self.parse_action_decl().map(Decl::Action),
            Token::Test => self.parse_test_decl().map(Decl::Test),
            Token::Intent
            | Token::Explain
            | Token::Step
            | Token::Requires
            | Token::Ensures
            | Token::Example => Err(vec![self.error_here(
                "semantic-layer constructs are only allowed inside action bodies",
            )]),
            Token::Meaning => Err(vec![self.error_here(
                "`meaning:` annotations are only allowed beneath record field declarations",
            )]),
            _ => Err(vec![self.error_here("expected a top-level declaration")]),
        }
    }

    fn parse_const_decl(&mut self) -> Result<ConstDecl, Vec<Diagnostic>> {
        let span = self.expect_simple(Token::Const, "expected `const`")?.span;
        let name = self.expect_identifier("expected constant name")?;
        self.expect_simple(Token::Colon, "expected `:` after constant name")?;
        let ty = self.parse_type()?;
        self.expect_simple(Token::Assign, "expected `=` in constant declaration")?;
        let value = self.parse_expr()?;
        self.expect_newline("expected newline after constant declaration")?;
        Ok(ConstDecl {
            name,
            ty,
            value,
            span,
        })
    }

    fn parse_record_decl(&mut self) -> Result<RecordDecl, Vec<Diagnostic>> {
        let span = self.expect_simple(Token::Record, "expected `record`")?.span;
        let name = self.expect_identifier("expected record name")?;
        self.expect_simple(Token::Colon, "expected `:` after record name")?;
        self.expect_newline("expected newline after record header")?;
        self.expect_simple(Token::Indent, "expected an indented record body")?;
        let mut fields = Vec::new();
        while !self.at(&Token::Dedent) && !self.at(&Token::Eof) {
            let field_span = self.current_span();
            let field_name = self.expect_identifier("expected record field name")?;
            self.expect_simple(Token::Colon, "expected `:` after field name")?;
            let field_type = self.parse_type()?;
            self.expect_newline("expected newline after field declaration")?;
            let meaning = if self.at(&Token::Indent) {
                self.bump();
                self.expect_simple(Token::Meaning, "expected `meaning:` annotation")?;
                self.expect_simple(Token::Colon, "expected `:` after `meaning`")?;
                let meaning = self.expect_string_literal("expected text literal after `meaning:`")?;
                self.expect_newline("expected newline after `meaning:` annotation")?;
                self.expect_simple(
                    Token::Dedent,
                    "expected dedent after record field annotation block",
                )?;
                Some(meaning)
            } else {
                None
            };
            fields.push(FieldDecl {
                name: field_name,
                ty: field_type,
                meaning,
                span: field_span,
            });
        }
        self.expect_simple(Token::Dedent, "expected dedent after record body")?;
        Ok(RecordDecl { name, fields, span })
    }

    fn parse_enum_decl(&mut self) -> Result<EnumDecl, Vec<Diagnostic>> {
        let span = self.expect_simple(Token::Enum, "expected `enum`")?.span;
        let name = self.expect_identifier("expected enum name")?;
        self.expect_simple(Token::Colon, "expected `:` after enum name")?;
        self.expect_newline("expected newline after enum header")?;
        self.expect_simple(Token::Indent, "expected an indented enum body")?;
        let mut variants = Vec::new();
        while !self.at(&Token::Dedent) && !self.at(&Token::Eof) {
            let variant_span = self.current_span();
            let variant_name = self.expect_identifier("expected enum variant name")?;
            let mut fields = Vec::new();
            if self.at(&Token::LParen) {
                self.bump();
                if !self.at(&Token::RParen) {
                    loop {
                        let field_span = self.current_span();
                        let field_name = self.expect_identifier("expected variant field name")?;
                        self.expect_simple(Token::Colon, "expected `:` after variant field name")?;
                        let field_type = self.parse_type()?;
                        fields.push(VariantField {
                            name: field_name,
                            ty: field_type,
                            span: field_span,
                        });
                        if !self.at(&Token::Comma) {
                            break;
                        }
                        self.bump();
                    }
                }
                self.expect_simple(Token::RParen, "expected `)` after variant fields")?;
            }
            self.expect_newline("expected newline after enum variant")?;
            variants.push(EnumVariant {
                name: variant_name,
                fields,
                span: variant_span,
            });
        }
        self.expect_simple(Token::Dedent, "expected dedent after enum body")?;
        Ok(EnumDecl {
            name,
            variants,
            span,
        })
    }

    fn parse_extern_decl(&mut self) -> Result<ExternDecl, Vec<Diagnostic>> {
        let span = self.expect_simple(Token::Extern, "expected `extern`")?.span;
        let purity = if self.at(&Token::Pure) {
            self.bump();
            Some(Purity::Pure)
        } else if self.at(&Token::Impure) {
            self.bump();
            Some(Purity::Impure)
        } else {
            None
        };
        self.expect_simple(Token::Action, "expected `action` in extern declaration")?;
        let name = self.expect_identifier("expected extern action name")?;
        let params = self.parse_params()?;
        self.expect_simple(Token::Arrow, "expected `->` in extern declaration")?;
        let return_type = self.parse_type()?;
        self.expect_newline("expected newline after extern declaration")?;
        Ok(ExternDecl {
            name,
            purity,
            params,
            return_type,
            span,
        })
    }

    fn parse_action_decl(&mut self) -> Result<ActionDecl, Vec<Diagnostic>> {
        let span = self.expect_simple(Token::Action, "expected `action`")?.span;
        let name = self.expect_identifier("expected action name")?;
        let params = self.parse_params()?;
        let return_type = if self.at(&Token::Arrow) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect_simple(Token::Colon, "expected `:` after action signature")?;
        self.expect_newline("expected newline after action header")?;
        self.action_body_depth += 1;
        let body = self.parse_block()?;
        self.action_body_depth -= 1;
        Ok(ActionDecl {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_test_decl(&mut self) -> Result<TestDecl, Vec<Diagnostic>> {
        let span = self.expect_simple(Token::Test, "expected `test`")?.span;
        let name = self.expect_identifier("expected test name")?;
        self.expect_simple(Token::Colon, "expected `:` after test name")?;
        self.expect_newline("expected newline after test header")?;
        let body = self.parse_block()?;
        Ok(TestDecl { name, body, span })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        self.expect_simple(Token::Indent, "expected indented block")?;
        let mut statements = Vec::new();
        while !self.at(&Token::Dedent) && !self.at(&Token::Eof) {
            self.skip_newlines();
            if self.at(&Token::Dedent) {
                break;
            }
            statements.push(self.parse_stmt()?);
        }
        self.expect_simple(Token::Dedent, "expected dedent after block")?;
        Ok(statements)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let span = self.current_span();
        let kind = match self.current_token() {
            Token::Intent => {
                self.ensure_action_body_context("`intent:` blocks")?;
                self.parse_intent_block()?
            }
            Token::Explain => {
                self.ensure_action_body_context("`explain:` blocks")?;
                self.parse_explain_block()?
            }
            Token::Step => {
                self.ensure_action_body_context("`step` blocks")?;
                self.parse_step_block()?
            }
            Token::Requires => {
                self.ensure_action_body_context("`requires` clauses")?;
                self.parse_requires_clause()?
            }
            Token::Ensures => {
                self.ensure_action_body_context("`ensures` clauses")?;
                self.parse_ensures_clause()?
            }
            Token::Example => {
                self.ensure_action_body_context("`example` blocks")?;
                self.parse_example_block()?
            }
            Token::Let => self.parse_let_stmt()?,
            Token::Var => self.parse_var_stmt()?,
            Token::If => self.parse_if_stmt()?,
            Token::While => self.parse_while_stmt()?,
            Token::Match => self.parse_match_stmt()?,
            Token::For => self.parse_for_stmt()?,
            Token::Return => self.parse_return_stmt()?,
            Token::Break => {
                self.bump();
                self.expect_newline("expected newline after `break`")?;
                StmtKind::Break
            }
            Token::Continue => {
                self.bump();
                self.expect_newline("expected newline after `continue`")?;
                StmtKind::Continue
            }
            Token::Expect => {
                self.bump();
                let expr = self.parse_expr()?;
                self.expect_newline("expected newline after `expect`")?;
                StmtKind::Expect(expr)
            }
            _ => {
                if self.is_legacy_set_stmt() {
                    return Err(vec![self.error_here(
                        "legacy `set target = value` syntax is not supported; use `target := value`",
                    )]);
                } else if self.is_unsupported_destructuring_assignment_stmt() {
                    return Err(vec![self.error_here(
                        "destructuring in `:=` is not supported",
                    )]);
                } else if self.is_assignment_stmt() {
                    self.parse_assign_stmt()?
                } else {
                    let expr = self.parse_expr()?;
                    self.expect_newline("expected newline after expression")?;
                    StmtKind::Expr(expr)
                }
            }
        };

        Ok(Stmt {
            id: self.alloc_node_id(),
            kind,
            span,
        })
    }

    fn parse_intent_block(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Intent, "expected `intent`")?;
        self.expect_simple(Token::Colon, "expected `:` after `intent`")?;
        self.expect_newline("expected newline after `intent:`")?;
        self.expect_simple(Token::Indent, "expected an indented intent block")?;

        let mut goal = None;
        let mut constraints = Vec::new();
        let mut assumptions = Vec::new();
        let mut properties = Vec::new();

        while !self.at(&Token::Dedent) && !self.at(&Token::Eof) {
            match self.current_token() {
                Token::Goal => {
                    self.bump();
                    self.expect_simple(Token::Colon, "expected `:` after `goal`")?;
                    goal = Some(self.expect_string_literal("expected text literal for `goal:`")?);
                    self.expect_newline("expected newline after `goal:`")?;
                }
                Token::Constraints => {
                    self.bump();
                    self.expect_simple(Token::Colon, "expected `:` after `constraints`")?;
                    self.expect_newline("expected newline after `constraints:`")?;
                    constraints = self.parse_string_list_block("constraints")?;
                }
                Token::Assumptions => {
                    self.bump();
                    self.expect_simple(Token::Colon, "expected `:` after `assumptions`")?;
                    self.expect_newline("expected newline after `assumptions:`")?;
                    assumptions = self.parse_string_list_block("assumptions")?;
                }
                Token::Properties => {
                    self.bump();
                    self.expect_simple(Token::Colon, "expected `:` after `properties`")?;
                    self.expect_newline("expected newline after `properties:`")?;
                    properties = self.parse_string_list_block("properties")?;
                }
                _ => {
                    return Err(vec![self.error_here(
                        "unknown `intent:` field; expected `goal`, `constraints`, `assumptions`, or `properties`",
                    )]);
                }
            }
        }

        self.expect_simple(Token::Dedent, "expected dedent after `intent:` block")?;
        Ok(StmtKind::IntentBlock {
            goal,
            constraints,
            assumptions,
            properties,
        })
    }

    fn parse_explain_block(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Explain, "expected `explain`")?;
        self.expect_simple(Token::Colon, "expected `:` after `explain`")?;
        self.expect_newline("expected newline after `explain:`")?;
        self.expect_simple(Token::Indent, "expected an indented explain block")?;

        let mut lines = Vec::new();
        while !self.at(&Token::Dedent) && !self.at(&Token::Eof) {
            lines.push(self.expect_string_literal(
                "expected a text literal line inside `explain:`",
            )?);
            self.expect_newline("expected newline after explain text")?;
        }

        self.expect_simple(Token::Dedent, "expected dedent after `explain:` block")?;
        if lines.is_empty() {
            return Err(vec![self.error_here("`explain:` block must contain at least one line")]);
        }
        Ok(StmtKind::ExplainBlock { lines })
    }

    fn parse_step_block(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Step, "expected `step`")?;
        let label = self.expect_identifier("expected step label")?;
        self.expect_simple(Token::Colon, "expected `:` after step label")?;
        self.expect_newline("expected newline after step header")?;
        let body = self.parse_block()?;
        if body.is_empty() {
            return Err(vec![self.error_here("`step` block must not be empty")]);
        }
        Ok(StmtKind::StepBlock { label, body })
    }

    fn parse_requires_clause(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Requires, "expected `requires`")?;
        let condition = self.parse_expr()?;
        self.expect_newline("expected newline after `requires` clause")?;
        Ok(StmtKind::RequiresClause { condition })
    }

    fn parse_ensures_clause(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Ensures, "expected `ensures`")?;
        let condition = self.parse_expr()?;
        self.expect_newline("expected newline after `ensures` clause")?;
        Ok(StmtKind::EnsuresClause { condition })
    }

    fn parse_example_block(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Example, "expected `example`")?;
        let name = self.expect_identifier("expected example name")?;
        self.expect_simple(Token::Colon, "expected `:` after example name")?;
        self.expect_newline("expected newline after example header")?;
        self.expect_simple(Token::Indent, "expected an indented example block")?;

        let mut inputs = None;
        let mut outputs = None;
        while !self.at(&Token::Dedent) && !self.at(&Token::Eof) {
            match self.current_token() {
                Token::Input => {
                    self.bump();
                    self.expect_simple(Token::Colon, "expected `:` after `input`")?;
                    self.expect_newline("expected newline after `input:`")?;
                    inputs = Some(self.parse_example_bindings("input")?);
                }
                Token::Output => {
                    self.bump();
                    self.expect_simple(Token::Colon, "expected `:` after `output`")?;
                    self.expect_newline("expected newline after `output:`")?;
                    outputs = Some(self.parse_example_bindings("output")?);
                }
                _ => {
                    return Err(vec![self.error_here(
                        "example blocks only support `input:` and `output:` sub-blocks",
                    )]);
                }
            }
        }

        self.expect_simple(Token::Dedent, "expected dedent after `example:` block")?;
        Ok(StmtKind::ExampleBlock {
            name,
            inputs: inputs.ok_or_else(|| vec![self.error_here("example block requires `input:`")])?,
            outputs: outputs
                .ok_or_else(|| vec![self.error_here("example block requires `output:`")])?,
        })
    }

    fn parse_let_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Let, "expected `let`")?;
        let pattern = self.parse_binding_pattern()?;
        let explicit_type = if self.at(&Token::Colon) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect_simple(Token::Assign, "expected `=` in let statement")?;
        let value = self.parse_expr()?;
        self.expect_newline("expected newline after let statement")?;
        Ok(StmtKind::Let {
            pattern,
            explicit_type,
            value,
        })
    }

    fn parse_var_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Var, "expected `var`")?;
        let pattern = self.parse_binding_pattern()?;
        let explicit_type = if self.at(&Token::Colon) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect_simple(Token::Assign, "expected `=` in var statement")?;
        let value = self.parse_expr()?;
        self.expect_newline("expected newline after var statement")?;
        Ok(StmtKind::Var {
            pattern,
            explicit_type,
            value,
        })
    }

    fn parse_assign_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        let target = self.parse_target()?;
        self.expect_simple(Token::ColonAssign, "expected `:=` in assignment statement")?;
        let value = self.parse_expr()?;
        self.expect_newline("expected newline after assignment statement")?;
        Ok(StmtKind::Assign { target, value })
    }

    fn parse_if_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::If, "expected `if`")?;
        let if_span = self.current_span();
        let condition = self.parse_expr()?;
        self.expect_simple(Token::Colon, "expected `:` after if condition")?;
        self.expect_newline("expected newline after if header")?;
        let body = self.parse_block()?;
        let mut branches = vec![ConditionalBranch {
            condition,
            body,
            span: if_span,
        }];

        while self.at(&Token::Elif) {
            let branch_span = self.current_span();
            self.bump();
            let branch_condition = self.parse_expr()?;
            self.expect_simple(Token::Colon, "expected `:` after elif condition")?;
            self.expect_newline("expected newline after elif header")?;
            let branch_body = self.parse_block()?;
            branches.push(ConditionalBranch {
                condition: branch_condition,
                body: branch_body,
                span: branch_span,
            });
        }

        let else_branch = if self.at(&Token::Else) {
            self.bump();
            self.expect_simple(Token::Colon, "expected `:` after else")?;
            self.expect_newline("expected newline after else header")?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(StmtKind::If {
            branches,
            else_branch,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::While, "expected `while`")?;
        let condition = self.parse_expr()?;
        self.expect_simple(Token::Colon, "expected `:` after while condition")?;
        self.expect_newline("expected newline after while header")?;
        let body = self.parse_block()?;
        Ok(StmtKind::While { condition, body })
    }

    fn parse_match_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Match, "expected `match`")?;
        let scrutinee = self.parse_expr()?;
        self.expect_simple(Token::Colon, "expected `:` after match target")?;
        self.expect_newline("expected newline after match header")?;
        self.expect_simple(Token::Indent, "expected an indented match body")?;

        let mut arms = Vec::new();
        while !self.at(&Token::Dedent) && !self.at(&Token::Eof) {
            let span = self.current_span();
            let pattern = self.parse_pattern()?;
            self.expect_simple(Token::Colon, "expected `:` after match pattern")?;
            self.expect_newline("expected newline after match arm header")?;
            let body = self.parse_block()?;
            arms.push(MatchArm {
                pattern,
                body,
                span,
            });
        }

        self.expect_simple(Token::Dedent, "expected dedent after match body")?;
        if arms.is_empty() {
            return Err(vec![self.error_here("match statement must contain at least one arm")]);
        }
        Ok(StmtKind::Match { scrutinee, arms })
    }

    fn parse_for_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::For, "expected `for`")?;
        self.expect_simple(Token::Each, "expected `each` after `for`")?;
        let binding = self.expect_identifier("expected loop binding name")?;
        self.expect_simple(Token::In, "expected `in` after loop binding")?;
        let iterable = self.parse_expr()?;
        self.expect_simple(Token::Colon, "expected `:` after for-each iterable")?;
        self.expect_newline("expected newline after for-each header")?;
        let body = self.parse_block()?;
        Ok(StmtKind::ForEach {
            binding,
            iterable,
            body,
        })
    }

    fn parse_return_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Return, "expected `return`")?;
        let expr = if self.at(&Token::Newline) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect_newline("expected newline after return statement")?;
        Ok(StmtKind::Return(expr))
    }

    fn parse_binding_pattern(&mut self) -> Result<BindingPattern, Vec<Diagnostic>> {
        let span = self.current_span();
        let kind = match self.current_token() {
            Token::Identifier(name) if name == "_" => {
                return Err(vec![self.error_here(
                    "wildcard destructuring is not supported in declarations",
                )]);
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.bump();
                if self.at(&Token::LParen) {
                    self.parse_record_binding_pattern(name)?
                } else {
                    BindingPatternKind::Name(name)
                }
            }
            Token::LParen => self.parse_tuple_binding_pattern()?,
            _ => return Err(vec![self.error_here("expected a binding pattern")]),
        };
        Ok(BindingPattern { kind, span })
    }

    fn parse_tuple_binding_pattern(&mut self) -> Result<BindingPatternKind, Vec<Diagnostic>> {
        self.expect_simple(Token::LParen, "expected `(`")?;
        let first = self.parse_binding_name_in_pattern("tuple destructuring")?;
        if !self.at(&Token::Comma) {
            return Err(vec![self.error_here(
                "tuple destructuring requires at least two binding names",
            )]);
        }

        let mut items = vec![first];
        while self.at(&Token::Comma) {
            self.bump();
            items.push(self.parse_binding_name_in_pattern("tuple destructuring")?);
        }
        self.expect_simple(Token::RParen, "expected `)` after tuple destructuring")?;
        Ok(BindingPatternKind::Tuple(items))
    }

    fn parse_record_binding_pattern(
        &mut self,
        name: String,
    ) -> Result<BindingPatternKind, Vec<Diagnostic>> {
        self.expect_simple(Token::LParen, "expected `(` after record name")?;
        if self.at(&Token::RParen) {
            return Err(vec![self.error_here(
                "record destructuring must bind at least one field",
            )]);
        }
        let is_record_pattern = matches!(
            (self.current_token(), self.peek_token()),
            (Token::Identifier(_), Some(token)) if token.same_variant(&Token::Colon)
        );
        if !is_record_pattern {
            return Err(vec![self.error_here(
                "enum or positional destructuring is not supported in declarations",
            )]);
        }

        let mut fields = Vec::new();
        loop {
            let span = self.current_span();
            let field = self.expect_identifier("expected record field name")?;
            self.expect_simple(Token::Colon, "expected `:` after record field name")?;
            let binding = self.parse_binding_name_in_pattern("record destructuring")?;
            fields.push(RecordBindingField {
                field,
                binding,
                span,
            });
            if !self.at(&Token::Comma) {
                break;
            }
            self.bump();
        }
        self.expect_simple(Token::RParen, "expected `)` after record destructuring")?;
        Ok(BindingPatternKind::Record { name, fields })
    }

    fn parse_binding_name_in_pattern(
        &mut self,
        context: &str,
    ) -> Result<String, Vec<Diagnostic>> {
        match self.current_token() {
            Token::Identifier(name) if name == "_" => Err(vec![self.error_here(format!(
                "wildcard destructuring is not supported in {context}"
            ))]),
            Token::Identifier(name) => {
                let name = name.clone();
                self.bump();
                if self.at(&Token::LParen) {
                    return Err(vec![self.error_here(format!(
                        "nested destructuring is not supported in {context}"
                    ))]);
                }
                Ok(name)
            }
            Token::LParen => Err(vec![self.error_here(format!(
                "nested destructuring is not supported in {context}"
            ))]),
            _ => Err(vec![self.error_here(format!(
                "expected a binding name in {context}"
            ))]),
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        let span = self.current_span();
        let kind = match self.current_token() {
            Token::Identifier(name) if name == "_" => {
                self.bump();
                PatternKind::Wildcard
            }
            Token::IntLiteral(value) => {
                let value = *value;
                self.bump();
                PatternKind::Literal(PatternLiteral::Int(value))
            }
            Token::DecLiteral(value) => {
                let value = value.clone();
                self.bump();
                PatternKind::Literal(PatternLiteral::Dec(value))
            }
            Token::StringLiteral(value) => {
                let value = value.clone();
                self.bump();
                PatternKind::Literal(PatternLiteral::String(value))
            }
            Token::True => {
                self.bump();
                PatternKind::Literal(PatternLiteral::Bool(true))
            }
            Token::False => {
                self.bump();
                PatternKind::Literal(PatternLiteral::Bool(false))
            }
            Token::None => {
                self.bump();
                PatternKind::Variant {
                    name: "None".to_string(),
                    args: Vec::new(),
                }
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.bump();
                if self.at(&Token::LParen) {
                    self.parse_named_pattern(name)?
                } else {
                    PatternKind::Binding(name)
                }
            }
            Token::LParen => self.parse_group_or_tuple_pattern()?,
            _ => return Err(vec![self.error_here("expected a match pattern")]),
        };
        Ok(Pattern { kind, span })
    }

    fn parse_named_pattern(&mut self, name: String) -> Result<PatternKind, Vec<Diagnostic>> {
        self.expect_simple(Token::LParen, "expected `(` after pattern name")?;
        if self.at(&Token::RParen) {
            self.bump();
            return Ok(PatternKind::Variant {
                name,
                args: Vec::new(),
            });
        }

        let is_record_pattern = matches!(
            (self.current_token(), self.peek_token()),
            (Token::Identifier(_), Some(token)) if token.same_variant(&Token::Colon)
        );

        if is_record_pattern {
            let mut fields = Vec::new();
            loop {
                let span = self.current_span();
                let field_name = self.expect_identifier("expected record pattern field name")?;
                self.expect_simple(Token::Colon, "expected `:` after record pattern field name")?;
                let pattern = self.parse_pattern()?;
                fields.push(RecordPatternField {
                    name: field_name,
                    pattern,
                    span,
                });
                if !self.at(&Token::Comma) {
                    break;
                }
                self.bump();
            }
            self.expect_simple(Token::RParen, "expected `)` after record pattern")?;
            Ok(PatternKind::Record { name, fields })
        } else {
            let mut args = Vec::new();
            loop {
                args.push(self.parse_pattern()?);
                if !self.at(&Token::Comma) {
                    break;
                }
                self.bump();
            }
            self.expect_simple(Token::RParen, "expected `)` after variant pattern")?;
            Ok(PatternKind::Variant { name, args })
        }
    }

    fn parse_group_or_tuple_pattern(&mut self) -> Result<PatternKind, Vec<Diagnostic>> {
        self.expect_simple(Token::LParen, "expected `(`")?;
        let first = self.parse_pattern()?;
        if self.at(&Token::Comma) {
            let mut items = vec![first];
            while self.at(&Token::Comma) {
                self.bump();
                items.push(self.parse_pattern()?);
            }
            self.expect_simple(Token::RParen, "expected `)` after tuple pattern")?;
            Ok(PatternKind::Tuple(items))
        } else {
            self.expect_simple(Token::RParen, "expected `)` after grouped pattern")?;
            Ok(first.kind)
        }
    }

    fn parse_target(&mut self) -> Result<Target, Vec<Diagnostic>> {
        let span = self.current_span();
        let name = self.expect_identifier("expected assignment target name")?;
        let mut target = Target::Name { name, span };

        loop {
            if self.at(&Token::Dot) {
                self.bump();
                let field_span = self.current_span();
                let field = self.expect_identifier("expected field name after `.`")?;
                target = Target::Field {
                    base: Box::new(target),
                    field,
                    span: field_span,
                };
            } else if self.at(&Token::LBracket) {
                let index_span = self.current_span();
                self.bump();
                let index = self.parse_expr()?;
                self.expect_simple(Token::RBracket, "expected `]` after target index")?;
                target = Target::Index {
                    base: Box::new(target),
                    index,
                    span: index_span,
                };
            } else {
                break;
            }
        }

        Ok(target)
    }

    fn parse_string_list_block(&mut self, field_name: &str) -> Result<Vec<String>, Vec<Diagnostic>> {
        self.expect_simple(
            Token::Indent,
            format!("expected an indented `{field_name}:` list"),
        )?;
        let mut items = Vec::new();
        while !self.at(&Token::Dedent) && !self.at(&Token::Eof) {
            self.expect_simple(Token::Minus, format!("expected `-` list item in `{field_name}:`"))?;
            items.push(self.expect_string_literal(format!(
                "expected text literal list item in `{field_name}:`"
            ))?);
            self.expect_newline("expected newline after list item")?;
        }
        self.expect_simple(Token::Dedent, "expected dedent after list block")?;
        Ok(items)
    }

    fn parse_example_bindings(
        &mut self,
        block_name: &str,
    ) -> Result<Vec<(String, Expr)>, Vec<Diagnostic>> {
        self.expect_simple(
            Token::Indent,
            format!("expected an indented `{block_name}:` block"),
        )?;
        let mut bindings = Vec::new();
        while !self.at(&Token::Dedent) && !self.at(&Token::Eof) {
            let name = self.expect_identifier("expected binding name")?;
            self.expect_simple(Token::Assign, "expected `=` in example binding")?;
            let expr = self.parse_expr()?;
            if !expr_is_literal(&expr) {
                return Err(vec![Diagnostic::new(
                    expr.span.clone(),
                    Phase::Parse,
                    "example bindings must use literal values",
                )]);
            }
            self.expect_newline("expected newline after example binding")?;
            bindings.push((name, expr));
        }
        self.expect_simple(Token::Dedent, "expected dedent after example binding block")?;
        if bindings.is_empty() {
            return Err(vec![self.error_here(format!(
                "`{block_name}:` block must contain at least one binding"
            ))]);
        }
        Ok(bindings)
    }

    fn is_assignment_stmt(&self) -> bool {
        let mut cursor = self.position;
        if !matches!(
            self.tokens.get(cursor).map(|token| &token.token),
            Some(Token::Identifier(_))
        ) {
            return false;
        }
        cursor += 1;
        loop {
            match self.tokens.get(cursor).map(|token| &token.token) {
                Some(Token::Dot) => {
                    cursor += 1;
                    if !matches!(
                        self.tokens.get(cursor).map(|token| &token.token),
                        Some(Token::Identifier(_))
                    ) {
                        return false;
                    }
                    cursor += 1;
                }
                Some(Token::LBracket) => {
                    cursor += 1;
                    let mut depth = 1usize;
                    while let Some(token) = self.tokens.get(cursor).map(|token| &token.token) {
                        match token {
                            Token::LBracket => depth += 1,
                            Token::RBracket => {
                                depth -= 1;
                                if depth == 0 {
                                    cursor += 1;
                                    break;
                                }
                            }
                            Token::Eof | Token::Newline => return false,
                            _ => {}
                        }
                        cursor += 1;
                    }
                    if depth != 0 {
                        return false;
                    }
                }
                Some(Token::ColonAssign) => return true,
                _ => return false,
            }
        }
    }

    fn is_unsupported_destructuring_assignment_stmt(&self) -> bool {
        match self.tokens.get(self.position).map(|token| &token.token) {
            Some(Token::LParen) => self
                .scan_balanced_parens(self.position)
                .is_some_and(|cursor| {
                    self.tokens
                        .get(cursor)
                        .is_some_and(|token| token.token.same_variant(&Token::ColonAssign))
                }),
            Some(Token::Identifier(_)) => {
                if !self
                    .tokens
                    .get(self.position + 1)
                    .is_some_and(|token| token.token.same_variant(&Token::LParen))
                {
                    return false;
                }
                self.scan_balanced_parens(self.position + 1)
                    .is_some_and(|cursor| {
                        self.tokens
                            .get(cursor)
                            .is_some_and(|token| token.token.same_variant(&Token::ColonAssign))
                    })
            }
            _ => false,
        }
    }

    fn scan_balanced_parens(&self, start: usize) -> Option<usize> {
        let mut cursor = start;
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(cursor).map(|token| &token.token) {
            match token {
                Token::LParen => depth += 1,
                Token::RParen => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(cursor + 1);
                    }
                }
                Token::Newline | Token::Indent | Token::Dedent | Token::Eof => return None,
                _ => {}
            }
            cursor += 1;
        }
        None
    }

    fn is_legacy_set_stmt(&self) -> bool {
        let mut cursor = self.position;
        if !matches!(
            self.tokens.get(cursor).map(|token| &token.token),
            Some(Token::Identifier(name)) if name == "set"
        ) {
            return false;
        }

        cursor += 1;
        if !matches!(
            self.tokens.get(cursor).map(|token| &token.token),
            Some(Token::Identifier(_))
        ) {
            return false;
        }
        cursor += 1;

        loop {
            match self.tokens.get(cursor).map(|token| &token.token) {
                Some(Token::Dot) => {
                    cursor += 1;
                    if !matches!(
                        self.tokens.get(cursor).map(|token| &token.token),
                        Some(Token::Identifier(_))
                    ) {
                        return false;
                    }
                    cursor += 1;
                }
                Some(Token::LBracket) => {
                    cursor += 1;
                    let mut depth = 1usize;
                    while let Some(token) = self.tokens.get(cursor).map(|token| &token.token) {
                        match token {
                            Token::LBracket => depth += 1,
                            Token::RBracket => {
                                depth -= 1;
                                if depth == 0 {
                                    cursor += 1;
                                    break;
                                }
                            }
                            Token::Eof | Token::Newline => return false,
                            _ => {}
                        }
                        cursor += 1;
                    }
                    if depth != 0 {
                        return false;
                    }
                }
                Some(Token::Assign) => return true,
                _ => return false,
            }
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut expr = self.parse_and_expr()?;
        while self.at(&Token::Or) {
            let span = expr.span.clone();
            self.bump();
            let right = self.parse_and_expr()?;
            expr = Expr {
                id: self.alloc_node_id(),
                span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Or,
                    right: Box::new(right),
                },
            };
        }
        Ok(expr)
    }

    fn parse_and_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut expr = self.parse_equality_expr()?;
        while self.at(&Token::And) {
            let span = expr.span.clone();
            self.bump();
            let right = self.parse_equality_expr()?;
            expr = Expr {
                id: self.alloc_node_id(),
                span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::And,
                    right: Box::new(right),
                },
            };
        }
        Ok(expr)
    }

    fn parse_equality_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut expr = self.parse_compare_expr()?;
        while self.at(&Token::EqualEqual) || self.at(&Token::NotEqual) {
            let op = if self.at(&Token::EqualEqual) {
                BinaryOp::Equal
            } else {
                BinaryOp::NotEqual
            };
            let span = expr.span.clone();
            self.bump();
            let right = self.parse_compare_expr()?;
            expr = Expr {
                id: self.alloc_node_id(),
                span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
            };
        }
        Ok(expr)
    }

    fn parse_compare_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut expr = self.parse_add_expr()?;
        while self.at(&Token::Less)
            || self.at(&Token::LessEqual)
            || self.at(&Token::Greater)
            || self.at(&Token::GreaterEqual)
        {
            let op = match self.current_token() {
                Token::Less => BinaryOp::Less,
                Token::LessEqual => BinaryOp::LessEqual,
                Token::Greater => BinaryOp::Greater,
                Token::GreaterEqual => BinaryOp::GreaterEqual,
                _ => unreachable!(),
            };
            let span = expr.span.clone();
            self.bump();
            let right = self.parse_add_expr()?;
            expr = Expr {
                id: self.alloc_node_id(),
                span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
            };
        }
        Ok(expr)
    }

    fn parse_add_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut expr = self.parse_mul_expr()?;
        while self.at(&Token::Plus) || self.at(&Token::Minus) {
            let op = if self.at(&Token::Plus) {
                BinaryOp::Add
            } else {
                BinaryOp::Subtract
            };
            let span = expr.span.clone();
            self.bump();
            let right = self.parse_mul_expr()?;
            expr = Expr {
                id: self.alloc_node_id(),
                span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
            };
        }
        Ok(expr)
    }

    fn parse_mul_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut expr = self.parse_unary_expr()?;
        while self.at(&Token::Star) || self.at(&Token::Slash) || self.at(&Token::Percent) {
            let op = match self.current_token() {
                Token::Star => BinaryOp::Multiply,
                Token::Slash => BinaryOp::Divide,
                Token::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            let span = expr.span.clone();
            self.bump();
            let right = self.parse_unary_expr()?;
            expr = Expr {
                id: self.alloc_node_id(),
                span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
            };
        }
        Ok(expr)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        if self.at(&Token::Minus) || self.at(&Token::Not) {
            let span = self.current_span();
            let op = if self.at(&Token::Minus) {
                UnaryOp::Negate
            } else {
                UnaryOp::Not
            };
            self.bump();
            let expr = self.parse_unary_expr()?;
            return Ok(Expr {
                id: self.alloc_node_id(),
                span,
                kind: ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
            });
        }
        self.parse_postfix_expr()
    }

    fn parse_postfix_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut expr = self.parse_primary_expr()?;
        loop {
            if self.at(&Token::Dot) {
                let span = expr.span.clone();
                self.bump();
                let field = self.expect_identifier("expected field name after `.`")?;
                expr = Expr {
                    id: self.alloc_node_id(),
                    span,
                    kind: ExprKind::FieldAccess {
                        base: Box::new(expr),
                        field,
                    },
                };
            } else if self.at(&Token::LBracket) {
                let span = expr.span.clone();
                self.bump();
                let index = self.parse_expr()?;
                self.expect_simple(Token::RBracket, "expected `]` after index expression")?;
                expr = Expr {
                    id: self.alloc_node_id(),
                    span,
                    kind: ExprKind::Index {
                        base: Box::new(expr),
                        index: Box::new(index),
                    },
                };
            } else if self.at(&Token::LParen) {
                let span = expr.span.clone();
                let args = self.parse_args()?;
                expr = Expr {
                    id: self.alloc_node_id(),
                    span,
                    kind: ExprKind::Call {
                        callee: Box::new(expr),
                        args,
                    },
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let span = self.current_span();
        match self.current_token() {
            Token::IntLiteral(value) => {
                let value = *value;
                self.bump();
                Ok(self.expr(span, ExprKind::Int(value)))
            }
            Token::DecLiteral(value) => {
                let value = value.clone();
                self.bump();
                Ok(self.expr(span, ExprKind::Dec(value)))
            }
            Token::StringLiteral(value) => {
                let value = value.clone();
                self.bump();
                Ok(self.expr(span, ExprKind::String(value)))
            }
            Token::True => {
                self.bump();
                Ok(self.expr(span, ExprKind::Bool(true)))
            }
            Token::False => {
                self.bump();
                Ok(self.expr(span, ExprKind::Bool(false)))
            }
            Token::None => {
                self.bump();
                Ok(self.expr(span, ExprKind::None))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.bump();
                Ok(self.expr(span, ExprKind::Name(name)))
            }
            Token::LBracket => self.parse_list_literal(),
            Token::LBrace => self.parse_map_literal(),
            Token::LParen => self.parse_group_or_tuple_expr(),
            _ => Err(vec![self.error_here("expected an expression")]),
        }
    }

    fn parse_list_literal(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let span = self.current_span();
        self.expect_simple(Token::LBracket, "expected `[`")?;
        let mut items = Vec::new();
        if !self.at(&Token::RBracket) {
            loop {
                items.push(self.parse_expr()?);
                if !self.at(&Token::Comma) {
                    break;
                }
                self.bump();
            }
        }
        self.expect_simple(Token::RBracket, "expected `]` after list literal")?;
        Ok(self.expr(span, ExprKind::List(items)))
    }

    fn parse_map_literal(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let span = self.current_span();
        self.expect_simple(Token::LBrace, "expected `{`")?;
        let mut pairs = Vec::new();
        if !self.at(&Token::RBrace) {
            loop {
                let key = self.parse_expr()?;
                self.expect_simple(Token::Colon, "expected `:` between map key and value")?;
                let value = self.parse_expr()?;
                pairs.push((key, value));
                if !self.at(&Token::Comma) {
                    break;
                }
                self.bump();
            }
        }
        self.expect_simple(Token::RBrace, "expected `}` after map literal")?;
        Ok(self.expr(span, ExprKind::Map(pairs)))
    }

    fn parse_group_or_tuple_expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let span = self.current_span();
        self.expect_simple(Token::LParen, "expected `(`")?;
        let first = self.parse_expr()?;
        if self.at(&Token::Comma) {
            let mut items = vec![first];
            while self.at(&Token::Comma) {
                self.bump();
                items.push(self.parse_expr()?);
            }
            self.expect_simple(Token::RParen, "expected `)` after tuple literal")?;
            Ok(self.expr(span, ExprKind::Tuple(items)))
        } else {
            self.expect_simple(Token::RParen, "expected `)` after grouped expression")?;
            Ok(first)
        }
    }

    fn parse_args(&mut self) -> Result<Vec<CallArg>, Vec<Diagnostic>> {
        self.expect_simple(Token::LParen, "expected `(`")?;
        let mut args = Vec::new();
        if !self.at(&Token::RParen) {
            loop {
                let span = self.current_span();
                let name = if let Token::Identifier(_) = self.current_token() {
                    if self
                        .peek_token()
                        .is_some_and(|token| token.same_variant(&Token::Colon))
                    {
                        Some(self.expect_identifier("expected argument name")?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if name.is_some() {
                    self.expect_simple(Token::Colon, "expected `:` after argument name")?;
                }
                let expr = self.parse_expr()?;
                args.push(CallArg { name, expr, span });
                if !self.at(&Token::Comma) {
                    break;
                }
                self.bump();
            }
        }
        self.expect_simple(Token::RParen, "expected `)` after call arguments")?;
        Ok(args)
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, Vec<Diagnostic>> {
        self.expect_simple(Token::LParen, "expected `(` in parameter list")?;
        let mut params = Vec::new();
        if !self.at(&Token::RParen) {
            loop {
                let span = self.current_span();
                let name = self.expect_identifier("expected parameter name")?;
                self.expect_simple(Token::Colon, "expected `:` after parameter name")?;
                let ty = self.parse_type()?;
                params.push(Param { name, ty, span });
                if !self.at(&Token::Comma) {
                    break;
                }
                self.bump();
            }
        }
        self.expect_simple(Token::RParen, "expected `)` after parameter list")?;
        Ok(params)
    }

    fn parse_type(&mut self) -> Result<TypeRef, Vec<Diagnostic>> {
        match self.current_token() {
            Token::Identifier(name) if name == "Action" => self.parse_action_or_generic_type(),
            Token::Identifier(name) => {
                let name = name.clone();
                self.bump();
                if self.at(&Token::LBracket) {
                    self.bump();
                    let mut args = vec![self.parse_type()?];
                    while self.at(&Token::Comma) {
                        self.bump();
                        args.push(self.parse_type()?);
                    }
                    self.expect_simple(Token::RBracket, "expected `]` after type arguments")?;
                    Ok(TypeRef::Generic { name, args })
                } else {
                    Ok(TypeRef::Named(name))
                }
            }
            Token::LParen => {
                self.bump();
                let mut items = vec![self.parse_type()?];
                self.expect_simple(Token::Comma, "expected `,` in tuple type")?;
                items.push(self.parse_type()?);
                while self.at(&Token::Comma) {
                    self.bump();
                    items.push(self.parse_type()?);
                }
                self.expect_simple(Token::RParen, "expected `)` after tuple type")?;
                Ok(TypeRef::Tuple(items))
            }
            _ => Err(vec![self.error_here("expected a type")]),
        }
    }

    fn parse_action_or_generic_type(&mut self) -> Result<TypeRef, Vec<Diagnostic>> {
        let action_name = self.expect_identifier("expected type name")?;
        if action_name != "Action" || !self.at(&Token::LBracket) {
            return Ok(TypeRef::Named(action_name));
        }
        self.expect_simple(Token::LBracket, "expected `[` after Action")?;
        let mut params = Vec::new();
        if !self.at(&Token::Arrow) {
            params.push(self.parse_type()?);
            while self.at(&Token::Comma) {
                self.bump();
                params.push(self.parse_type()?);
            }
        }
        self.expect_simple(Token::Arrow, "expected `->` in action type")?;
        let result = self.parse_type()?;
        self.expect_simple(Token::RBracket, "expected `]` after action type")?;
        Ok(TypeRef::Action {
            params,
            result: Box::new(result),
        })
    }

    fn parse_module_name(&mut self) -> Result<ModuleName, Vec<Diagnostic>> {
        let mut segments = vec![self.expect_identifier("expected module name segment")?];
        while self.at(&Token::Dot) {
            self.bump();
            segments.push(self.expect_identifier("expected module name segment")?);
        }
        Ok(ModuleName { segments })
    }

    fn expr(&mut self, span: SourceSpan, kind: ExprKind) -> Expr {
        Expr {
            id: self.alloc_node_id(),
            kind,
            span,
        }
    }

    fn alloc_node_id(&mut self) -> NodeId {
        let id = self.next_node_id;
        self.next_node_id += 1;
        id
    }

    fn expect_simple(
        &mut self,
        expected: Token,
        message: impl Into<String>,
    ) -> Result<SpannedToken, Vec<Diagnostic>> {
        if self.at(&expected) {
            let token = self.tokens[self.position].clone();
            self.bump();
            Ok(token)
        } else {
            Err(vec![self.error_here(message)])
        }
    }

    fn expect_identifier(&mut self, message: &'static str) -> Result<String, Vec<Diagnostic>> {
        match self.current_token() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.bump();
                Ok(name)
            }
            _ => Err(vec![self.error_here(message)]),
        }
    }

    fn expect_string_literal(
        &mut self,
        message: impl Into<String>,
    ) -> Result<String, Vec<Diagnostic>> {
        match self.current_token() {
            Token::StringLiteral(value) => {
                let value = value.clone();
                self.bump();
                Ok(value)
            }
            _ => Err(vec![self.error_here(message)]),
        }
    }

    fn expect_newline(&mut self, message: &'static str) -> Result<(), Vec<Diagnostic>> {
        self.expect_simple(Token::Newline, message)?;
        self.skip_newlines();
        Ok(())
    }

    fn skip_newlines(&mut self) {
        while self.at(&Token::Newline) {
            self.bump();
        }
    }

    fn at(&self, token: &Token) -> bool {
        self.current_token().same_variant(token)
    }

    fn current_token(&self) -> &Token {
        &self.tokens[self.position].token
    }

    fn peek_token(&self) -> Option<&Token> {
        self.tokens.get(self.position + 1).map(|token| &token.token)
    }

    fn current_span(&self) -> SourceSpan {
        self.tokens
            .get(self.position)
            .map(|token| token.span.clone())
            .unwrap_or_else(|| SourceSpan::for_path(&self.path, 1, 1))
    }

    fn error_here(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.current_span(), Phase::Parse, message)
    }

    fn ensure_action_body_context(&self, construct: &str) -> Result<(), Vec<Diagnostic>> {
        if self.action_body_depth == 0 {
            Err(vec![self.error_here(format!(
                "{construct} are only allowed inside action bodies"
            ))])
        } else {
            Ok(())
        }
    }

    fn bump(&mut self) {
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
    }
}

fn expr_is_literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Int(_)
        | ExprKind::Dec(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::None => true,
        ExprKind::List(items) | ExprKind::Tuple(items) => items.iter().all(expr_is_literal),
        ExprKind::Map(entries) => entries
            .iter()
            .all(|(key, value)| expr_is_literal(key) && expr_is_literal(value)),
        ExprKind::Name(_)
        | ExprKind::Call { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Index { .. }
        | ExprKind::Unary { .. }
        | ExprKind::Binary { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::ast::{BindingPatternKind, Decl, ExprKind, PatternKind, StmtKind, Target};
    use crate::lexer::Lexer;

    use super::Parser;

    fn parse(source: &str) -> crate::ast::AstModule {
        let tokens = Lexer::new(Path::new("test.vg"), source)
            .tokenize()
            .expect("tokenize");
        Parser::new(Path::new("test.vg"), tokens)
            .parse_module()
            .expect("parse")
    }

    #[test]
    fn parses_valid_program_with_var_assign_test_and_expect() {
        let module = parse(
            r#"
module demo.core

record Customer:
  email: Text

action main(customer: Customer) -> Int:
  var current = customer
  current.email := "updated"
  return 1

test main_basic:
  expect main(Customer(email: "before")) == 1
"#,
        );

        assert_eq!(module.declarations.len(), 3);
        match &module.declarations[1] {
            Decl::Action(action) => match &action.body[0].kind {
                StmtKind::Var { pattern, .. } => match &pattern.kind {
                    BindingPatternKind::Name(name) => assert_eq!(name, "current"),
                    other => panic!("unexpected binding pattern: {other:?}"),
                },
                other => panic!("unexpected statement: {other:?}"),
            },
            other => panic!("unexpected declaration: {other:?}"),
        }
        match &module.declarations[1] {
            Decl::Action(action) => match &action.body[1].kind {
                StmtKind::Assign { target, .. } => match target {
                    Target::Field { field, .. } => assert_eq!(field, "email"),
                    other => panic!("unexpected target: {other:?}"),
                },
                other => panic!("unexpected statement: {other:?}"),
            },
            other => panic!("unexpected declaration: {other:?}"),
        }
        match &module.declarations[2] {
            Decl::Test(test_decl) => match &test_decl.body[0].kind {
                StmtKind::Expect(expr) => assert!(matches!(expr.kind, ExprKind::Binary { .. })),
                other => panic!("unexpected statement: {other:?}"),
            },
            other => panic!("unexpected declaration: {other:?}"),
        }
    }

    #[test]
    fn reports_malformed_input() {
        let tokens = Lexer::new(Path::new("bad.vg"), "action main( -> None:\n  return\n")
            .tokenize()
            .expect("tokenize");
        let diagnostics = Parser::new(Path::new("bad.vg"), tokens)
            .parse_module()
            .expect_err("parse should fail");
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::Parse);
    }

    #[test]
    fn rejects_legacy_set_syntax() {
        let tokens = Lexer::new(
            Path::new("bad.vg"),
            "action main() -> Int:\n  var total = 1\n  set total = total + 1\n  return total\n",
        )
        .tokenize()
        .expect("tokenize");
        let diagnostics = Parser::new(Path::new("bad.vg"), tokens)
            .parse_module()
            .expect_err("parse should fail");
        assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::Parse);
        assert!(
            diagnostics[0]
                .message
                .contains("legacy `set target = value` syntax is not supported")
        );
    }

    #[test]
    fn parses_semantic_layer_constructs_inside_actions() {
        let module = parse(
            r#"
record Customer:
  email: Text
    meaning: "primary address"

action main(value: Int) -> Int:
  intent:
    goal: "return the value"
    constraints:
      - "must stay positive"
    assumptions:
      - "caller provides valid input"
    properties:
      - "deterministic"
  explain:
    "first line"
    "second line"
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

        match &module.declarations[0] {
            Decl::Record(record) => {
                assert_eq!(
                    record.fields[0].meaning.as_deref(),
                    Some("primary address")
                );
            }
            other => panic!("unexpected declaration: {other:?}"),
        }

        match &module.declarations[1] {
            Decl::Action(action) => {
                assert!(matches!(action.body[0].kind, StmtKind::IntentBlock { .. }));
                assert!(matches!(action.body[1].kind, StmtKind::ExplainBlock { .. }));
                assert!(matches!(action.body[2].kind, StmtKind::RequiresClause { .. }));
                assert!(matches!(action.body[3].kind, StmtKind::StepBlock { .. }));
                assert!(matches!(action.body[4].kind, StmtKind::EnsuresClause { .. }));
                assert!(matches!(action.body[5].kind, StmtKind::ExampleBlock { .. }));
            }
            other => panic!("unexpected declaration: {other:?}"),
        }
    }

    #[test]
    fn parses_match_statements_with_variant_and_record_patterns() {
        let module = parse(
            r#"
record Customer:
  name: Text
  active: Bool

action main(result: Result[Customer, Text]) -> Text:
  match result:
    Ok(Customer(name: n, active: true)):
      return n
    Err(message):
      return message
"#,
        );

        match &module.declarations[1] {
            Decl::Action(action) => match &action.body[0].kind {
                StmtKind::Match { arms, .. } => {
                    assert_eq!(arms.len(), 2);
                    match &arms[0].pattern.kind {
                        PatternKind::Variant { name, args } => {
                            assert_eq!(name, "Ok");
                            assert_eq!(args.len(), 1);
                            assert!(matches!(args[0].kind, PatternKind::Record { .. }));
                        }
                        other => panic!("unexpected first arm pattern: {other:?}"),
                    }
                    match &arms[1].pattern.kind {
                        PatternKind::Variant { name, args } => {
                            assert_eq!(name, "Err");
                            assert_eq!(args.len(), 1);
                            assert!(matches!(args[0].kind, PatternKind::Binding(_)));
                        }
                        other => panic!("unexpected second arm pattern: {other:?}"),
                    }
                }
                other => panic!("unexpected statement: {other:?}"),
            },
            other => panic!("unexpected declaration: {other:?}"),
        }
    }

    #[test]
    fn parses_tuple_and_record_binding_destructuring() {
        let module = parse(
            r#"
record Customer:
  name: Text
  email: Text

action main(pair: (Int, Int), customer: Customer) -> Int:
  let (left, right) = pair
  var Customer(name: name, email: address) = customer
  return left
"#,
        );

        match &module.declarations[1] {
            Decl::Action(action) => {
                match &action.body[0].kind {
                    StmtKind::Let { pattern, .. } => match &pattern.kind {
                        BindingPatternKind::Tuple(items) => {
                            assert_eq!(items, &vec!["left".to_string(), "right".to_string()]);
                        }
                        other => panic!("unexpected tuple binding pattern: {other:?}"),
                    },
                    other => panic!("unexpected statement: {other:?}"),
                }
                match &action.body[1].kind {
                    StmtKind::Var { pattern, .. } => match &pattern.kind {
                        BindingPatternKind::Record { name, fields } => {
                            assert_eq!(name, "Customer");
                            assert_eq!(fields.len(), 2);
                            assert_eq!(fields[0].field, "name");
                            assert_eq!(fields[0].binding, "name");
                            assert_eq!(fields[1].field, "email");
                            assert_eq!(fields[1].binding, "address");
                        }
                        other => panic!("unexpected record binding pattern: {other:?}"),
                    },
                    other => panic!("unexpected statement: {other:?}"),
                }
            }
            other => panic!("unexpected declaration: {other:?}"),
        }
    }

    #[test]
    fn rejects_unsupported_binding_destructuring_forms() {
        for source in [
            "action main(maybe: Option[Int]) -> Int:\n  let Some(value) = maybe\n  return 0\n",
            "action main(pair: (Int, (Int, Int))) -> Int:\n  let (a, (b, c)) = pair\n  return a\n",
            "action main(pair: (Int, Int)) -> Int:\n  let (_, b) = pair\n  return b\n",
            "action main(pair: (Int, Int)) -> Int:\n  let (a) = pair\n  return 0\n",
            "action main(pair: (Int, Int)) -> Int:\n  (a, b) := pair\n  return 0\n",
        ] {
            let tokens = Lexer::new(Path::new("bad.vg"), source)
                .tokenize()
                .expect("tokenize");
            let diagnostics = Parser::new(Path::new("bad.vg"), tokens)
                .parse_module()
                .expect_err("parse should fail");
            assert_eq!(diagnostics[0].phase, crate::diagnostics::Phase::Parse);
        }
    }

    #[test]
    fn rejects_semantic_layer_constructs_outside_action_bodies() {
        let tokens = Lexer::new(Path::new("bad.vg"), "intent:\n  goal: \"bad\"\n")
            .tokenize()
            .expect("tokenize");
        let diagnostics = Parser::new(Path::new("bad.vg"), tokens)
            .parse_module()
            .expect_err("parse should fail");
        assert!(diagnostics[0].message.contains("only allowed inside action bodies"));
    }

    #[test]
    fn rejects_unknown_intent_fields() {
        let tokens = Lexer::new(
            Path::new("bad.vg"),
            "action main() -> None:\n  intent:\n    mystery: \"bad\"\n  return\n",
        )
        .tokenize()
        .expect("tokenize");
        let diagnostics = Parser::new(Path::new("bad.vg"), tokens)
            .parse_module()
            .expect_err("parse should fail");
        assert!(diagnostics[0].message.contains("unknown `intent:` field"));
    }

    #[test]
    fn rejects_non_text_explain_lines() {
        let tokens = Lexer::new(
            Path::new("bad.vg"),
            "action main() -> None:\n  explain:\n    1\n  return\n",
        )
        .tokenize()
        .expect("tokenize");
        let diagnostics = Parser::new(Path::new("bad.vg"), tokens)
            .parse_module()
            .expect_err("parse should fail");
        assert!(diagnostics[0].message.contains("text literal line"));
    }

    #[test]
    fn rejects_empty_step_blocks() {
        let tokens = Lexer::new(
            Path::new("bad.vg"),
            "action main() -> None:\n  step compute:\n  return\n",
        )
        .tokenize()
        .expect("tokenize");
        let diagnostics = Parser::new(Path::new("bad.vg"), tokens)
            .parse_module()
            .expect_err("parse should fail");
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn rejects_example_blocks_without_required_sections() {
        let tokens = Lexer::new(
            Path::new("bad.vg"),
            "action main(value: Int) -> Int:\n  example bad:\n    input:\n      value = 1\n  return value\n",
        )
        .tokenize()
        .expect("tokenize");
        let diagnostics = Parser::new(Path::new("bad.vg"), tokens)
            .parse_module()
            .expect_err("parse should fail");
        assert!(diagnostics[0].message.contains("requires `output:`"));
    }

    #[test]
    fn rejects_non_literal_example_bindings() {
        let tokens = Lexer::new(
            Path::new("bad.vg"),
            "action main(value: Int) -> Int:\n  example bad:\n    input:\n      value = value\n    output:\n      result = 1\n  return value\n",
        )
        .tokenize()
        .expect("tokenize");
        let diagnostics = Parser::new(Path::new("bad.vg"), tokens)
            .parse_module()
            .expect_err("parse should fail");
        assert!(diagnostics[0].message.contains("literal values"));
    }
}
