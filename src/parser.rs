use std::path::{Path, PathBuf};

use crate::ast::{
    ActionDecl, AstModule, BinaryOp, CallArg, ConditionalBranch, ConstDecl, Decl, EnumDecl,
    EnumVariant, Expr, ExprKind, ExternDecl, FieldDecl, ImportDecl, ImportKind, ModuleDecl,
    ModuleName, NodeId, Param, Purity, RecordDecl, Stmt, StmtKind, Target, TestDecl, TypeRef,
    UnaryOp, VariantField,
};
use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::lexer::{SpannedToken, Token};

pub struct Parser {
    path: PathBuf,
    tokens: Vec<SpannedToken>,
    position: usize,
    next_node_id: NodeId,
}

impl Parser {
    pub fn new(path: &Path, tokens: Vec<SpannedToken>) -> Self {
        Self {
            path: path.to_path_buf(),
            tokens,
            position: 0,
            next_node_id: 1,
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
            fields.push(FieldDecl {
                name: field_name,
                ty: field_type,
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
        let body = self.parse_block()?;
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
            Token::Let => self.parse_let_stmt()?,
            Token::Var => self.parse_var_stmt()?,
            Token::If => self.parse_if_stmt()?,
            Token::While => self.parse_while_stmt()?,
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

    fn parse_let_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Let, "expected `let`")?;
        let name = self.expect_identifier("expected variable name")?;
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
            name,
            explicit_type,
            value,
        })
    }

    fn parse_var_stmt(&mut self) -> Result<StmtKind, Vec<Diagnostic>> {
        self.expect_simple(Token::Var, "expected `var`")?;
        let name = self.expect_identifier("expected variable name")?;
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
            name,
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
        message: &'static str,
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

    fn bump(&mut self) {
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::ast::{Decl, ExprKind, StmtKind, Target};
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
                StmtKind::Var { name, .. } => assert_eq!(name, "current"),
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
}
