## Context

Vulgata v0.5 defines `Result[T, E]` and `Option[T]` as first-class compound types, and the runtime already carries `Value::ResultOk` and `Value::ResultErr` variants. However, no surface syntax exists to inspect or extract the contents of these values. The only existing `FieldAccess` lowering in `tir.rs` dispatches standard-runtime module members (e.g., `list.push`) and has no awareness of typed receivers.

`Option[T]` additionally lacks dedicated runtime variants: `Value::None` is a bare, untyped sentinel shared with the `none` literal. This makes it impossible to distinguish a typed `None` from an untyped one at runtime.

## Goals / Non-Goals

**Goals:**
- Give `Result[T, E]` and `Option[T]` a minimal, built-in inspection/extraction surface with no imports required
- Add `Value::OptionSome` / `Value::OptionNone` for typed Option runtime representation
- Extend TIR with seven dedicated typed nodes so interpreter and codegen can dispatch directly
- Implement interpreter evaluation with named runtime errors on invalid extraction
- Implement Rust codegen using explicit `match` expressions (no `unwrap`)

**Non-Goals:**
- Flow-sensitive narrowing (e.g., knowing `r` is `Ok` after `if r.is_ok()`)
- Higher-level combinators: `map`, `unwrap_or`, `map_err`
- Full pattern matching over `Result`/`Option`
- Removing the bare `Value::None` variant — it is retained for the untyped `none` literal

## Decisions

### 1. Dedicated TIR nodes over generic field-call nodes

The `FieldAccess` lowering in `tir.rs` currently only handles module-level standard-runtime dispatch (e.g., `io.read_text`). Extending it to also recognise typed receiver operations requires a type-aware lowering step.

**Decision:** Add seven new `TypedExprKind` variants:
```
ResultIsOk { target }
ResultIsErr { target }
ResultValue { target }
ResultError { target }
OptionIsSome { target }
OptionIsNone { target }
OptionValue { target }
```

**Alternative considered:** Leave them as generic `FieldAccess` + `Call` and handle them late in the interpreter/codegen. Rejected because it pushes semantic knowledge into two backends instead of one lowering pass, and makes diagnostics imprecise.

### 2. Type-aware lowering in the TIR pass

The existing `lookup_standard_runtime_action` in `tir.rs` only inspects the module name string. For Result/Option operations the dispatch key is the *type* of the receiver, not its name.

**Decision:** In `lower_expr`, when lowering a `Call { callee: FieldAccess { base, field }, args: [] }` expression, inspect the type of `base` (already available via `expr_types`). If the type is `Result[T, E]` or `Option[T]` and the field matches a known operation name, emit the corresponding dedicated TIR node instead of a generic `Call(FieldAccess(...))`.

**Alternative considered:** Resolve in the type-checker (`types.rs`) and annotate the AST. Rejected to keep AST-level changes minimal; the TIR lowering pass is the right place to lower to typed IR.

### 3. `Value::OptionSome` / `Value::OptionNone` runtime variants

Currently `Option[T]` has no dedicated runtime representation. `Value::None` is used for both the untyped `none` literal and option absence.

**Decision:** Add `Value::OptionSome(Box<Value>)` and `Value::OptionNone`. When an `Option[T]` value is constructed (e.g., from a `Some(x)` literal or from a function returning `Option`), the runtime must produce these variants. `Value::None` is retained for untyped `none`.

**Alternative considered:** Overload `Value::None` for typed Option absence. Rejected because it prevents the interpreter from knowing whether a `None` is an untyped sentinel or a typed Option absence, which would make `is_none()` ambiguous.

### 4. Named runtime errors on invalid extraction

**Decision:** Invalid extraction (e.g., calling `.value()` on `Err(...)`) raises a `Diagnostic` at the call site with a fixed error name (`InvalidResultValueAccess`, `InvalidResultErrorAccess`, `InvalidOptionValueAccess`) and a message that includes the actual variant. These are raised as interpreter runtime errors, not panics.

### 5. Codegen uses explicit `match`, not `unwrap`

**Decision:** The compiler emits:
```rust
match x { Ok(v) => v, Err(_) => panic!("InvalidResultValueAccess") }
```
rather than `x.unwrap()`. This preserves the Vulgata error name in the panic message and avoids a Clippy warning about `unwrap` in generated code.

## Risks / Trade-offs

- **Risk**: Confusion between `Value::None` (untyped) and `Value::OptionNone` (typed) during transition → **Mitigation**: The interpreter must always produce `OptionSome`/`OptionNone` when the static type is `Option[T]`; `Value::None` is only produced by the `none` literal in an untyped context.
- **Risk**: The type-aware lowering in `lower_expr` depends on `expr_types` being fully populated before the TIR pass runs → **Mitigation**: The type-checker already runs to completion before `lower_module` is called; no ordering change needed.
- **Trade-off**: Seven new TIR variants increase pattern-match exhaustiveness burden across all TIR consumers (interpreter, codegen, any future passes) → Accepted; the explicitness is the point.

## Open Questions

- Should `Some(x)` and `None` in positions where the type is `Option[T]` be lowered to `OptionSome`/`OptionNone` by the TIR pass, or should the interpreter coerce on-the-fly? (Recommendation: TIR pass, for consistency with `ResultOk`/`ResultErr` construction.)
