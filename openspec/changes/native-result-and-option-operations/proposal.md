## Why

`Result[T, E]` and `Option[T]` are first-class types in Vulgata but currently have no built-in way to inspect or extract their contents — users cannot call `is_ok()`, `value()`, `is_some()`, etc. without extern helpers. This gap makes idiomatic error handling impossible and must be closed before `Result` and `Option` can be used in real programs.

## What Changes

- Add built-in member operations on `Result[T, E]`: `is_ok()`, `is_err()`, `value()`, `error()`
- Add built-in member operations on `Option[T]`: `is_some()`, `is_none()`, `value()`
- Add `Value::OptionSome(Box<Value>)` and `Value::OptionNone` runtime variants (currently `Option` has no dedicated runtime representation)
- Extend TIR with dedicated typed IR nodes for all seven operations (`ResultIsOk`, `ResultIsErr`, `ResultValue`, `ResultError`, `OptionIsSome`, `OptionIsNone`, `OptionValue`)
- Extend the semantic lowering pass to recognise these operations on typed receivers and emit the new IR nodes
- Implement interpreter evaluation for all seven operations with named runtime errors on invalid extraction (`InvalidResultValueAccess`, `InvalidResultErrorAccess`, `InvalidOptionValueAccess`)
- Implement codegen lowering to explicit Rust `match` expressions (no `unwrap()`)
- These operations require no import — they are always in scope when the receiver type matches

## Capabilities

### New Capabilities

- `result-operations`: Built-in inspection and extraction operations on `Result[T, E]` — `is_ok()`, `is_err()`, `value()`, `error()` — covering type rules, runtime semantics, and codegen mapping
- `option-operations`: Built-in inspection and extraction operations on `Option[T]` — `is_some()`, `is_none()`, `value()` — covering runtime representation (`OptionSome`/`OptionNone`), type rules, runtime semantics, and codegen mapping

### Modified Capabilities

- `types`: `Option[T]` gains dedicated runtime value variants; type-checker must resolve member operations on `Result` and `Option` to the new built-in forms rather than treating them as generic field calls

## Impact

- `src/tir.rs` — new `TypedExprKind` variants; updated `FieldAccess` lowering to detect typed receiver and emit built-in nodes
- `src/runtime.rs` — new `Value::OptionSome` / `Value::OptionNone` variants; interpreter branches for all seven operations
- `src/codegen.rs` — codegen arms for all seven new TIR nodes, emitting Rust `match` expressions
- `src/types.rs` — type-checker must assign correct return types for the new operations
- No breaking changes to existing programs; `Value::None` is retained for the untyped `none` literal
