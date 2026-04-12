use std::collections::HashMap;

use crate::ast::{AstModule, Decl, ImportKind};
use crate::diagnostics::{Diagnostic, Phase};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Module,
    Import,
    Const,
    Record,
    Enum,
    Action,
    Extern,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub symbols: HashMap<String, Symbol>,
}

pub struct Resolver<'a> {
    module: &'a AstModule,
}

impl<'a> Resolver<'a> {
    pub fn new(module: &'a AstModule) -> Self {
        Self { module }
    }

    pub fn resolve(&self) -> Result<Resolution, Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        let mut symbols = HashMap::new();

        if let Some(module_decl) = &self.module.module_decl {
            let module_name = module_decl.name.as_string();
            insert_symbol(
                &mut symbols,
                &mut diagnostics,
                module_name.clone(),
                Symbol {
                    name: module_name,
                    kind: SymbolKind::Module,
                },
                &module_decl.span,
            );
        }

        for import_decl in &self.module.imports {
            match &import_decl.kind {
                ImportKind::Module { module, alias } => {
                    let binding = alias.clone().unwrap_or_else(|| {
                        module
                            .segments
                            .last()
                            .cloned()
                            .expect("module names always have at least one segment")
                    });
                    insert_symbol(
                        &mut symbols,
                        &mut diagnostics,
                        binding.clone(),
                        Symbol {
                            name: binding,
                            kind: SymbolKind::Import,
                        },
                        &import_decl.span,
                    );
                }
                ImportKind::From { names, .. } => {
                    for name in names {
                        insert_symbol(
                            &mut symbols,
                            &mut diagnostics,
                            name.clone(),
                            Symbol {
                                name: name.clone(),
                                kind: SymbolKind::Import,
                            },
                            &import_decl.span,
                        );
                    }
                }
            }
        }

        for decl in &self.module.declarations {
            let kind = match decl {
                Decl::Const(_) => SymbolKind::Const,
                Decl::Record(_) => SymbolKind::Record,
                Decl::Enum(_) => SymbolKind::Enum,
                Decl::Extern(_) => SymbolKind::Extern,
                Decl::Action(_) => SymbolKind::Action,
                Decl::Test(_) => SymbolKind::Test,
            };
            insert_symbol(
                &mut symbols,
                &mut diagnostics,
                decl.name().to_string(),
                Symbol {
                    name: decl.name().to_string(),
                    kind,
                },
                decl.span(),
            );
        }

        if diagnostics.is_empty() {
            Ok(Resolution { symbols })
        } else {
            Err(diagnostics)
        }
    }
}

fn insert_symbol(
    symbols: &mut HashMap<String, Symbol>,
    diagnostics: &mut Vec<Diagnostic>,
    key: String,
    symbol: Symbol,
    span: &crate::diagnostics::SourceSpan,
) {
    if symbols.insert(key.clone(), symbol).is_some() {
        diagnostics.push(Diagnostic::new(
            span.clone(),
            Phase::Resolve,
            format!("duplicate top-level symbol `{key}`"),
        ));
    }
}
