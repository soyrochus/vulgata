## 1. Runtime Value Variants

- [ ] 1.1 Add `Value::OptionSome(Box<Value>)` and `Value::OptionNone` variants to the `Value` enum in `src/runtime.rs`
- [ ] 1.2 Implement `Display` for the new variants (`Some(...)` / `None`)
- [ ] 1.3 Implement `snapshot()` for the new variants in the `Value::snapshot` method
- [ ] 1.4 Update all exhaustive `match` arms on `Value` in `runtime.rs`, `codegen.rs`, `tir.rs`, and `types.rs` to handle the new variants

## 2. TIR — New Typed IR Nodes

- [ ] 2.1 Add seven new variants to `TypedExprKind` in `src/tir.rs`: `ResultIsOk`, `ResultIsErr`, `ResultValue`, `ResultError`, `OptionIsSome`, `OptionIsNone`, `OptionValue` — each with a `target: Box<TypedExpr>` field

## 3. Semantic Lowering

- [ ] 3.1 In `lower_expr` in `src/tir.rs`, detect the pattern `Call { callee: FieldAccess { base, field }, args: [] }` where `base` has static type `Result[T, E]` or `Option[T]` and `field` is one of the seven operation names
- [ ] 3.2 Emit the corresponding dedicated TIR node instead of a generic `Call(FieldAccess(...))` for each of the seven operations
- [ ] 3.3 Assign the correct return type to each new TIR node (`Bool` for inspection ops, `T` for `value()`, `E` for `error()`)

## 4. Type-Checker — Type Error for Wrong Receiver

- [ ] 4.1 In `src/types.rs`, emit a type error when `is_ok()`, `is_err()`, `value()`, or `error()` is called on a non-`Result` type
- [ ] 4.2 In `src/types.rs`, emit a type error when `is_some()`, `is_none()`, or `value()` is called on a non-`Option` type (when receiver is not `Result` either)

## 5. Interpreter Evaluation

- [ ] 5.1 In `src/runtime.rs`, add interpreter evaluation arms for `ResultIsOk` and `ResultIsErr` — inspect the `Value::ResultOk` / `Value::ResultErr` tag and return the appropriate `Value::Bool`
- [ ] 5.2 Add evaluation for `ResultValue` — return inner value from `ResultOk`, raise `InvalidResultValueAccess` diagnostic on `ResultErr`
- [ ] 5.3 Add evaluation for `ResultError` — return inner value from `ResultErr`, raise `InvalidResultErrorAccess` diagnostic on `ResultOk`
- [ ] 5.4 Add evaluation arms for `OptionIsSome` and `OptionIsNone` — inspect `OptionSome` / `OptionNone` tag and return the appropriate `Value::Bool`
- [ ] 5.5 Add evaluation for `OptionValue` — return inner value from `OptionSome`, raise `InvalidOptionValueAccess` diagnostic on `OptionNone`
- [ ] 5.6 Ensure runtime error diagnostics include the source span, operation name, and actual variant name

## 6. Rust Codegen

- [ ] 6.1 In `src/codegen.rs`, add codegen arms for `ResultIsOk` and `ResultIsErr` — emit `x.is_ok()` and `x.is_err()`
- [ ] 6.2 Add codegen for `ResultValue` — emit `match x { Ok(v) => v, Err(_) => panic!("InvalidResultValueAccess") }`
- [ ] 6.3 Add codegen for `ResultError` — emit `match x { Err(e) => e, Ok(_) => panic!("InvalidResultErrorAccess") }`
- [ ] 6.4 Add codegen for `OptionIsSome` and `OptionIsNone` — emit `x.is_some()` and `x.is_none()`
- [ ] 6.5 Add codegen for `OptionValue` — emit `match x { Some(v) => v, None => panic!("InvalidOptionValueAccess") }`

## 7. Option Construction — Runtime Representation

- [ ] 7.1 Ensure `Some(x)` literals evaluated in a typed `Option[T]` context produce `Value::OptionSome` (update the interpreter's literal/constructor evaluation)
- [ ] 7.2 Ensure `none` / `None` in a typed `Option[T]` context produces `Value::OptionNone` rather than `Value::None`

## 8. Tests and Conformance

- [ ] 8.1 Add interpreter test: `r.is_ok()` and `r.is_err()` return correct booleans for `Ok` and `Err`
- [ ] 8.2 Add interpreter test: `r.value()` extracts from `Ok(42)` returning `42`
- [ ] 8.3 Add interpreter test: `r.value()` on `Err(...)` raises `InvalidResultValueAccess`
- [ ] 8.4 Add interpreter test: `r.error()` extracts from `Err("bad")` returning `"bad"`
- [ ] 8.5 Add interpreter test: `r.error()` on `Ok(...)` raises `InvalidResultErrorAccess`
- [ ] 8.6 Add interpreter test: `x.is_some()` and `x.is_none()` return correct booleans for `Some` and `None`
- [ ] 8.7 Add interpreter test: `x.value()` extracts from `Some(10)` returning `10`
- [ ] 8.8 Add interpreter test: `x.value()` on `None` raises `InvalidOptionValueAccess`
- [ ] 8.9 Add type-checker test: calling `is_ok()` on a non-Result type produces a type error
