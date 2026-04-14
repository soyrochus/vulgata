# Vulgata Design Proposal — Native `Result` and `Option` Operations

## 1. Purpose

This proposal defines a minimal, built-in operation set for `Result[T, E]` and `Option[T]` in Vulgata.

The goal is to make these compound types fully usable in ordinary programs without:

* introducing full pattern matching yet
* requiring dozens of type-specific extern helper actions
* weakening the explicit error model already defined in Vulgata v0.5 

This proposal covers:

* source-language surface
* type-checking rules
* interpreter/runtime behavior
* mapping to generated Rust code
* conformance requirements

---

## 2. Design intent

Vulgata already treats `Result[T, E]` and `Option[T]` as first-class compound types. 
What is missing is a standard, built-in way to:

* test success versus failure
* extract the success value
* extract the error value
* test presence versus absence
* extract the contained option value

This proposal adds that missing operational surface.

The design must remain:

* small
* explicit
* readable
* compatible with both interpreter and compiler backends

---

## 3. Core rule

`Result[T, E]` and `Option[T]` gain a minimal built-in method-like surface.

These operations are part of the language semantics. They are **not** extern actions and must not require library declarations in user code.

They are resolved by the semantic core and lowered explicitly by the interpreter and compiler.

---

## 4. Source-language surface

## 4.1 Built-in operations on `Result[T, E]`

Allowed forms:

```text
result.is_ok()
result.is_err()
result.value()
result.error()
```

Semantics:

* `is_ok()` returns `Bool`
* `is_err()` returns `Bool`
* `value()` returns `T`
* `error()` returns `E`

## 4.2 Built-in operations on `Option[T]`

Allowed forms:

```text
opt.is_some()
opt.is_none()
opt.value()
```

Semantics:

* `is_some()` returns `Bool`
* `is_none()` returns `Bool`
* `value()` returns `T`

---

## 5. Examples

### 5.1 Result success/failure branching

```text
let r = file.read_text("notes.txt")

if r.is_err():
  return Err(r.error())

let text = r.value()
return Ok(text)
```

### 5.2 Option inspection

```text
let maybe_name = lookup_name(id)

if maybe_name.is_none():
  return "unknown"

return maybe_name.value()
```

### 5.3 Invalid extraction

```text
let r = file.read_text("missing.txt")
let text = r.value()
```

This is legal to parse and type-check, but may fail at runtime if `r` is `Err(...)`.

---

## 6. Type-checking rules

## 6.1 Method resolution model

These operations are not general-purpose object methods.

They are **built-in typed member operations** available only on:

* `Result[T, E]`
* `Option[T]`

Any attempt to call them on other types is a type error.

Examples:

```text
let x = 1
x.is_ok()       # invalid
```

```text
let text = "abc"
text.value()    # invalid
```

## 6.2 Type rules for `Result`

If `x: Result[T, E]`, then:

* `x.is_ok()` has type `Bool`
* `x.is_err()` has type `Bool`
* `x.value()` has type `T`
* `x.error()` has type `E`

## 6.3 Type rules for `Option`

If `x: Option[T]`, then:

* `x.is_some()` has type `Bool`
* `x.is_none()` has type `Bool`
* `x.value()` has type `T`

## 6.4 No implicit flow narrowing in v0.5

This proposal does **not** require flow-sensitive narrowing.

So in:

```text
if r.is_ok():
  let x = r.value()
```

the call to `value()` is allowed because the type is known statically, not because the compiler proves the branch is safe.

Safety remains a runtime matter for now.

Flow-sensitive refinement may be added later, but is out of scope for this proposal.

---

## 7. Runtime semantics

## 7.1 Runtime behavior for `Result`

If `x` is:

* `Ok(v)`:

  * `x.is_ok()` → `true`
  * `x.is_err()` → `false`
  * `x.value()` → `v`
  * `x.error()` → runtime error

* `Err(e)`:

  * `x.is_ok()` → `false`
  * `x.is_err()` → `true`
  * `x.value()` → runtime error
  * `x.error()` → `e`

## 7.2 Runtime behavior for `Option`

If `x` is:

* `Some(v)`:

  * `x.is_some()` → `true`
  * `x.is_none()` → `false`
  * `x.value()` → `v`

* `None`:

  * `x.is_some()` → `false`
  * `x.is_none()` → `true`
  * `x.value()` → runtime error

## 7.3 Runtime error category

Invalid extraction must raise a named runtime error.

Recommended error names:

* `InvalidResultValueAccess`
* `InvalidResultErrorAccess`
* `InvalidOptionValueAccess`

Diagnostics should include:

* source span
* operation name
* actual runtime variant

Example diagnostic:

```text
runtime error: InvalidResultValueAccess
cannot call value() on Err(...)
```

---

## 8. Execution mode behavior

These operations belong to the **executable layer**.

They must behave identically in:

* `release`
* `checked`
* `debug`
* `tooling`

Unlike `expect`, `requires`, `ensures`, and `example`, they are not mode-governed checkable constructs. 

---

## 9. AST and semantic model

## 9.1 Surface representation

The parser may continue to represent these as ordinary postfix field-plus-call syntax:

```text
result.is_ok()
result.value()
```

No grammar change is strictly required, because v0.5 already supports postfix field access and call syntax. 

## 9.2 Semantic lowering

The semantic core must recognize these specific combinations as built-in operations, not normal field calls.

Recommended typed IR node forms:

```text
TValueExpr
  ...
  ResultIsOk { target: TValueExpr }
  ResultIsErr { target: TValueExpr }
  ResultValue { target: TValueExpr }
  ResultError { target: TValueExpr }
  OptionIsSome { target: TValueExpr }
  OptionIsNone { target: TValueExpr }
  OptionValue { target: TValueExpr }
```

This is preferable to leaving them as generic field calls, because:

* interpreter logic becomes direct
* compiler mapping becomes explicit
* diagnostics become precise

---

## 10. Interpreter implementation rules

The interpreter runtime must implement `Result` and `Option` as tagged runtime values, as already anticipated by the v0.5 implementation guidance. 

Suggested runtime forms:

```text
Value::ResultOk(Box<Value>)
Value::ResultErr(Box<Value>)
Value::OptionSome(Box<Value>)
Value::OptionNone
```

Evaluation rules:

* evaluate the receiver expression
* inspect the tag
* return the correct boolean or inner value
* otherwise raise the named runtime error

These operations must not go through extern dispatch.

---

## 11. Compiler mapping to Rust

## 11.1 General rule

The compiler must lower built-in `Result` / `Option` operations directly to Rust expressions.

They must not be compiled through:

* extern wrappers
* dynamic method lookup
* interpreter runtime dependencies

This is consistent with the v0.5 requirement that compiled output be direct Rust code and not depend on a VM. 

## 11.2 Rust mapping for `Result`

Assume Vulgata `Result[T, E]` maps to Rust `Result<T, E>`.

Then:

* `x.is_ok()` → `x.is_ok()`
* `x.is_err()` → `x.is_err()`
* `x.value()` → extraction from `Ok`
* `x.error()` → extraction from `Err`

### Safe direct mapping

The compiler should generate explicit checked extraction, not blind `unwrap()`.

Recommended forms:

```rust
match x {
    Ok(v) => v,
    Err(_) => panic!("InvalidResultValueAccess")
}
```

and

```rust
match x {
    Err(e) => e,
    Ok(_) => panic!("InvalidResultErrorAccess")
}
```

This preserves Vulgata semantics and yields clear generated code.

Using `.unwrap()` or `.unwrap_err()` is allowed only if the implementation is satisfied with the panic quality and diagnostic mapping.

## 11.3 Rust mapping for `Option`

Assume Vulgata `Option[T]` maps to Rust `Option<T>`.

Then:

* `x.is_some()` → `x.is_some()`
* `x.is_none()` → `x.is_none()`
* `x.value()` → extraction from `Some`

Recommended generated form:

```rust
match x {
    Some(v) => v,
    None => panic!("InvalidOptionValueAccess")
}
```

---

## 12. Standard library positioning

These operations are **standard semantic operations**, not ordinary library actions.

That means:

* they do not need imports
* they do not live in `result.*` or `option.*` modules
* they are always in scope when the receiver type matches

This follows the same general principle as operators: the user writes a compact built-in form, but the semantic core and backends implement it.

The `result` and `option` modules may still exist later for higher-level helper actions, but these core inspection and extraction operations should not depend on them.

---

## 13. Optional future extension

This proposal intentionally stays minimal.

Possible later additions:

* `result.unwrap_or(default)`
* `result.map(...)`
* `result.map_err(...)`
* `option.unwrap_or(default)`
* flow-sensitive narrowing after `is_ok()` / `is_some()`
* full `match`

These are explicitly **not** part of this proposal.

---

## 14. Conformance requirements

The conformance suite must include at least the following cases.

### 14.1 Result inspection

```text
test result_ok_flags:
  let r = Ok(1)
  expect r.is_ok() == true
  expect r.is_err() == false
```

### 14.2 Result extraction

```text
test result_value_extracts_ok:
  let r = Ok(42)
  expect r.value() == 42
```

### 14.3 Result error extraction

```text
test result_error_extracts_err:
  let r = Err("bad")
  expect r.error() == "bad"
```

### 14.4 Option inspection

```text
test option_some_flags:
  let x = Some(10)
  expect x.is_some() == true
  expect x.is_none() == false
```

### 14.5 Option extraction

```text
test option_value_extracts_some:
  let x = Some(10)
  expect x.value() == 10
```

### 14.6 Invalid extraction runtime failures

Interpreter and compiler must both reject or fail deterministically on:

* `Err(...).value()`
* `Ok(...).error()`
* `none.value()` when treated as `Option[T]`

The exact error text may vary, but the error category must be stable.

---

## 15. Final stance

This proposal does not add a new abstraction. It completes the semantics of `Result` and `Option`, which Vulgata already defines as core types. 

The main rule is simple:

* keep the surface small
* make `Result` and `Option` directly usable
* lower them explicitly in both interpreter and compiler
* do not force users to simulate core semantics through extern helpers

That gives Vulgata a much cleaner error-handling story without yet paying the complexity cost of full pattern matching.
