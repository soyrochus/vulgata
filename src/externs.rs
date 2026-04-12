use std::collections::HashMap;
use std::path::Path;

use crate::diagnostics::{Diagnostic, Phase, SourceSpan};
use crate::runtime::Value;
use crate::tir::{TypedDecl, TypedIrModule};
use crate::types::{ActionType, Type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternBinding {
    pub provider: String,
    pub symbol: String,
    pub params: Vec<Type>,
    pub return_type: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinAdapter {
    AddInt,
    EchoText,
    TextLen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternEntry {
    pub signature: ActionType,
    pub binding: ExternBinding,
    adapter: BuiltinAdapter,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExternRegistry {
    entries: HashMap<String, ExternEntry>,
}

impl ExternRegistry {
    pub fn from_module(module: &TypedIrModule) -> Result<Self, Vec<Diagnostic>> {
        Self::from_config_text(module, None, None)
    }

    pub fn from_path(module: &TypedIrModule, path: &Path) -> Result<Self, Vec<Diagnostic>> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            vec![Diagnostic::new(
                SourceSpan::for_path(path, 1, 1),
                Phase::Extern,
                format!("failed to read extern config: {error}"),
            )]
        })?;
        Self::from_config_text(module, Some(path), Some(&source))
    }

    pub fn from_config_text(
        module: &TypedIrModule,
        path: Option<&Path>,
        source: Option<&str>,
    ) -> Result<Self, Vec<Diagnostic>> {
        let declared = declared_externs(module);
        if declared.is_empty() && source.is_none() {
            return Ok(Self::default());
        }

        let Some(source) = source else {
            return Err(vec![Diagnostic::new(
                span_for(path, 1),
                Phase::Extern,
                "extern declarations require a configuration file",
            )]);
        };

        let parsed = parse_extern_config(source, path)?;
        let mut diagnostics = Vec::new();
        let mut entries = HashMap::new();
        let mut configured = HashMap::new();

        for (name, binding) in &parsed {
            configured.insert(name.clone(), binding.line);
            let Some(signature) = declared.get(name) else {
                diagnostics.push(Diagnostic::new(
                    span_for(path, binding.line),
                    Phase::Extern,
                    format!("extern config `{name}` has no matching declaration"),
                ));
                continue;
            };

            let binding_signature = ActionType {
                params: binding.params.clone(),
                result: Box::new(binding.return_type.clone()),
            };
            if signature != &binding_signature {
                diagnostics.push(Diagnostic::new(
                    span_for(path, binding.line),
                    Phase::Extern,
                    format!(
                        "extern `{name}` signature mismatch: declared `{}`, configured `{}`",
                        format_action_type(signature),
                        format_action_type(&binding_signature)
                    ),
                ));
                continue;
            }

            match resolve_builtin_adapter(&binding.provider, &binding.symbol, signature) {
                Ok(adapter) => {
                    entries.insert(
                        name.clone(),
                        ExternEntry {
                            signature: signature.clone(),
                            binding: ExternBinding {
                                provider: binding.provider.clone(),
                                symbol: binding.symbol.clone(),
                                params: binding.params.clone(),
                                return_type: binding.return_type.clone(),
                            },
                            adapter,
                        },
                    );
                }
                Err(message) => diagnostics.push(Diagnostic::new(
                    span_for(path, binding.line),
                    Phase::Extern,
                    format!("extern `{name}` binding is invalid: {message}"),
                )),
            }
        }

        for name in declared.keys() {
            if !entries.contains_key(name) && !configured.contains_key(name) {
                diagnostics.push(Diagnostic::new(
                    span_for(path, 1),
                    Phase::Extern,
                    format!("extern `{name}` is declared but has no configuration binding"),
                ));
            }
        }

        if diagnostics.is_empty() {
            Ok(Self { entries })
        } else {
            Err(diagnostics)
        }
    }

    pub fn insert(&mut self, name: String, entry: ExternEntry) {
        self.entries.insert(name, entry);
    }

    pub fn get(&self, name: &str) -> Option<&ExternEntry> {
        self.entries.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn call(
        &self,
        name: &str,
        args: Vec<Value>,
        span: &SourceSpan,
    ) -> Result<Value, Vec<Diagnostic>> {
        let entry = self.entries.get(name).ok_or_else(|| {
            vec![Diagnostic::new(
                span.clone(),
                Phase::Extern,
                format!("extern `{name}` is not registered"),
            )]
        })?;

        if entry.signature.params.len() != args.len() {
            return Err(vec![Diagnostic::new(
                span.clone(),
                Phase::Extern,
                format!(
                    "extern `{name}` expected {} arguments, found {}",
                    entry.signature.params.len(),
                    args.len()
                ),
            )]);
        }

        match entry.adapter {
            BuiltinAdapter::AddInt => {
                let left = expect_int(&args[0], span, name, 0)?;
                let right = expect_int(&args[1], span, name, 1)?;
                Ok(Value::Int(left + right))
            }
            BuiltinAdapter::EchoText => {
                let text = expect_text(&args[0], span, name, 0)?;
                Ok(Value::Text(text))
            }
            BuiltinAdapter::TextLen => {
                let text = expect_text(&args[0], span, name, 0)?;
                Ok(Value::Int(text.len() as i64))
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedBinding {
    line: usize,
    provider: String,
    symbol: String,
    params: Vec<Type>,
    return_type: Type,
}

fn declared_externs(module: &TypedIrModule) -> HashMap<String, ActionType> {
    let mut declared = HashMap::new();
    for decl in &module.declarations {
        if let TypedDecl::Extern(extern_decl) = decl {
            if let Type::ExternAction(signature) = &extern_decl.ty {
                declared.insert(extern_decl.name.clone(), signature.clone());
            }
        }
    }
    declared
}

fn parse_extern_config(
    source: &str,
    path: Option<&Path>,
) -> Result<HashMap<String, ParsedBinding>, Vec<Diagnostic>> {
    let mut current_name: Option<String> = None;
    let mut current_line = 1usize;
    let mut props: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut line_numbers: HashMap<String, usize> = HashMap::new();

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len() - 1];
            if let Some(name) = section.strip_prefix("extern.") {
                current_name = Some(name.to_string());
                current_line = line_number;
                props.entry(name.to_string()).or_default();
                line_numbers.insert(name.to_string(), line_number);
            } else {
                return Err(vec![Diagnostic::new(
                    span_for(path, line_number),
                    Phase::Extern,
                    format!("unsupported config section `{section}`"),
                )]);
            }
            continue;
        }

        let Some(current) = &current_name else {
            return Err(vec![Diagnostic::new(
                span_for(path, line_number),
                Phase::Extern,
                "extern config entries must appear inside [extern.<name>] sections",
            )]);
        };

        let Some((key, value)) = line.split_once('=') else {
            return Err(vec![Diagnostic::new(
                span_for(path, line_number),
                Phase::Extern,
                "expected `key = value` entry in extern config",
            )]);
        };
        props
            .entry(current.clone())
            .or_default()
            .insert(key.trim().to_string(), value.trim().to_string());
        line_numbers.entry(current.clone()).or_insert(current_line);
    }

    let mut bindings = HashMap::new();
    let mut diagnostics = Vec::new();
    for (name, values) in props {
        let line = *line_numbers.get(&name).unwrap_or(&1);
        let provider = values
            .get("provider")
            .and_then(|value| parse_string(value))
            .ok_or_else(|| {
                Diagnostic::new(
                    span_for(path, line),
                    Phase::Extern,
                    format!("extern `{name}` is missing `provider`"),
                )
            });
        let symbol = values
            .get("symbol")
            .and_then(|value| parse_string(value))
            .ok_or_else(|| {
                Diagnostic::new(
                    span_for(path, line),
                    Phase::Extern,
                    format!("extern `{name}` is missing `symbol`"),
                )
            });
        let params = values
            .get("params")
            .map(|value| parse_type_array(value))
            .unwrap_or_else(|| Ok(Vec::new()));
        let return_type = values
            .get("return")
            .map(|value| parse_type_string(value))
            .unwrap_or_else(|| Err("missing `return`".to_string()));

        match (provider, symbol, params, return_type) {
            (Ok(provider), Ok(symbol), Ok(params), Ok(return_type)) => {
                bindings.insert(
                    name,
                    ParsedBinding {
                        line,
                        provider,
                        symbol,
                        params,
                        return_type,
                    },
                );
            }
            (provider, symbol, params, return_type) => {
                for result in [
                    provider.err(),
                    symbol.err(),
                    params.err().map(|message| {
                        Diagnostic::new(
                            span_for(path, line),
                            Phase::Extern,
                            format!("extern config type error: {message}"),
                        )
                    }),
                    return_type.err().map(|message| {
                        Diagnostic::new(
                            span_for(path, line),
                            Phase::Extern,
                            format!("extern config type error: {message}"),
                        )
                    }),
                ]
                .into_iter()
                .flatten()
                {
                    diagnostics.push(result);
                }
            }
        }
    }

    if diagnostics.is_empty() {
        Ok(bindings)
    } else {
        Err(diagnostics)
    }
}

fn resolve_builtin_adapter(
    provider: &str,
    symbol: &str,
    signature: &ActionType,
) -> Result<BuiltinAdapter, String> {
    if provider != "rust" {
        return Err(format!("unsupported provider `{provider}`"));
    }

    match symbol {
        "builtin::add_int" => {
            let expected = ActionType {
                params: vec![Type::Int, Type::Int],
                result: Box::new(Type::Int),
            };
            if signature == &expected {
                Ok(BuiltinAdapter::AddInt)
            } else {
                Err(format!(
                    "symbol `{symbol}` expects `{}`",
                    format_action_type(&expected)
                ))
            }
        }
        "builtin::echo_text" => {
            let expected = ActionType {
                params: vec![Type::Text],
                result: Box::new(Type::Text),
            };
            if signature == &expected {
                Ok(BuiltinAdapter::EchoText)
            } else {
                Err(format!(
                    "symbol `{symbol}` expects `{}`",
                    format_action_type(&expected)
                ))
            }
        }
        "builtin::text_len" => {
            let expected = ActionType {
                params: vec![Type::Text],
                result: Box::new(Type::Int),
            };
            if signature == &expected {
                Ok(BuiltinAdapter::TextLen)
            } else {
                Err(format!(
                    "symbol `{symbol}` expects `{}`",
                    format_action_type(&expected)
                ))
            }
        }
        _ => Err(format!("unsupported rust extern symbol `{symbol}`")),
    }
}

fn expect_int(
    value: &Value,
    span: &SourceSpan,
    name: &str,
    index: usize,
) -> Result<i64, Vec<Diagnostic>> {
    match value {
        Value::Int(value) => Ok(*value),
        other => Err(vec![Diagnostic::new(
            span.clone(),
            Phase::Extern,
            format!(
                "extern `{name}` argument {} expected Int, found {other:?}",
                index + 1
            ),
        )]),
    }
}

fn expect_text(
    value: &Value,
    span: &SourceSpan,
    name: &str,
    index: usize,
) -> Result<String, Vec<Diagnostic>> {
    match value {
        Value::Text(value) => Ok(value.clone()),
        other => Err(vec![Diagnostic::new(
            span.clone(),
            Phase::Extern,
            format!(
                "extern `{name}` argument {} expected Text, found {other:?}",
                index + 1
            ),
        )]),
    }
}

fn parse_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        Some(value[1..value.len() - 1].to_string())
    } else {
        None
    }
}

fn parse_type_array(value: &str) -> Result<Vec<Type>, String> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(format!("expected array of type strings, found `{value}`"));
    }
    let inner = &value[1..value.len() - 1];
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    for ch in inner.chars() {
        match ch {
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if bracket_depth == 0 && paren_depth == 0 => {
                items.push(parse_type_string(current.trim())?);
                current.clear();
            }
            ch => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        items.push(parse_type_string(current.trim())?);
    }
    Ok(items)
}

fn parse_type_string(value: &str) -> Result<Type, String> {
    let raw = parse_string(value).unwrap_or_else(|| value.trim().to_string());
    parse_type_expr(raw.trim())
}

fn parse_type_expr(raw: &str) -> Result<Type, String> {
    match raw {
        "Bool" => Ok(Type::Bool),
        "Int" => Ok(Type::Int),
        "Dec" => Ok(Type::Dec),
        "Text" => Ok(Type::Text),
        "Bytes" => Ok(Type::Bytes),
        "None" => Ok(Type::None),
        _ if raw.starts_with("List[") && raw.ends_with(']') => Ok(Type::List(Box::new(
            parse_type_expr(&raw[5..raw.len() - 1])?,
        ))),
        _ if raw.starts_with("Set[") && raw.ends_with(']') => Ok(Type::Set(Box::new(
            parse_type_expr(&raw[4..raw.len() - 1])?,
        ))),
        _ if raw.starts_with("Option[") && raw.ends_with(']') => Ok(Type::Option(Box::new(
            parse_type_expr(&raw[7..raw.len() - 1])?,
        ))),
        _ if raw.starts_with("Map[") && raw.ends_with(']') => {
            let inner = &raw[4..raw.len() - 1];
            let parts = split_top_level(inner)?;
            if parts.len() != 2 {
                return Err(format!("Map type requires 2 parameters, found `{raw}`"));
            }
            Ok(Type::Map(
                Box::new(parse_type_expr(parts[0])?),
                Box::new(parse_type_expr(parts[1])?),
            ))
        }
        _ if raw.starts_with("Result[") && raw.ends_with(']') => {
            let inner = &raw[7..raw.len() - 1];
            let parts = split_top_level(inner)?;
            if parts.len() != 2 {
                return Err(format!("Result type requires 2 parameters, found `{raw}`"));
            }
            Ok(Type::Result(
                Box::new(parse_type_expr(parts[0])?),
                Box::new(parse_type_expr(parts[1])?),
            ))
        }
        _ if raw.starts_with('(') && raw.ends_with(')') => {
            let parts = split_top_level(&raw[1..raw.len() - 1])?;
            Ok(Type::Tuple(
                parts
                    .into_iter()
                    .map(parse_type_expr)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        other => Ok(Type::Record(other.to_string())),
    }
}

fn split_top_level(input: &str) -> Result<Vec<&str>, String> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    for (index, ch) in input.char_indices() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => {
                if bracket_depth == 0 {
                    return Err(format!("unbalanced `]` in `{input}`"));
                }
                bracket_depth -= 1;
            }
            '(' => paren_depth += 1,
            ')' => {
                if paren_depth == 0 {
                    return Err(format!("unbalanced `)` in `{input}`"));
                }
                paren_depth -= 1;
            }
            ',' if bracket_depth == 0 && paren_depth == 0 => {
                parts.push(input[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if bracket_depth != 0 || paren_depth != 0 {
        return Err(format!("unbalanced type expression `{input}`"));
    }
    parts.push(input[start..].trim());
    Ok(parts)
}

fn span_for(path: Option<&Path>, line: usize) -> SourceSpan {
    match path {
        Some(path) => SourceSpan::for_path(path, line, 1),
        None => SourceSpan::new("<extern-config>", line, 1),
    }
}

fn format_action_type(action: &ActionType) -> String {
    let params = action
        .params
        .iter()
        .map(Type::describe)
        .collect::<Vec<_>>()
        .join(", ");
    format!("Action[{params} -> {}]", action.result.describe())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::lower_source;
    use crate::runtime::Value;

    use super::ExternRegistry;

    fn lowered(source: &str) -> crate::tir::TypedIrModule {
        lower_source(Path::new("test.vg"), source).expect("lower")
    }

    #[test]
    fn loads_compatible_bindings() {
        let module = lowered("extern action add(a: Int, b: Int) -> Int\n");
        let config = r#"
[extern.add]
provider = "rust"
symbol = "builtin::add_int"
params = ["Int", "Int"]
return = "Int"
"#;
        let registry =
            ExternRegistry::from_config_text(&module, None, Some(config)).expect("registry");
        let value = registry
            .call(
                "add",
                vec![Value::Int(2), Value::Int(3)],
                &crate::diagnostics::SourceSpan::new("<test>", 1, 1),
            )
            .expect("call");
        assert_eq!(value, Value::Int(5));
    }

    #[test]
    fn rejects_signature_mismatch() {
        let module = lowered("extern action add(a: Int, b: Int) -> Int\n");
        let config = r#"
[extern.add]
provider = "rust"
symbol = "builtin::echo_text"
params = ["Text"]
return = "Text"
"#;
        let diagnostics = ExternRegistry::from_config_text(&module, None, Some(config))
            .expect_err("registry should fail");
        assert!(diagnostics[0].message.contains("signature mismatch"));
    }

    #[test]
    fn rejects_missing_declaration() {
        let module = lowered("action main() -> None:\n  return\n");
        let config = r#"
[extern.missing]
provider = "rust"
symbol = "builtin::echo_text"
params = ["Text"]
return = "Text"
"#;
        let diagnostics = ExternRegistry::from_config_text(&module, None, Some(config))
            .expect_err("registry should fail");
        assert!(diagnostics[0].message.contains("no matching declaration"));
    }
}
