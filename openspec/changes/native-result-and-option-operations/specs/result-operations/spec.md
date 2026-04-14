## ADDED Requirements

### Requirement: Result built-in inspection operations
`Result[T, E]` SHALL expose two built-in boolean operations: `is_ok()` and `is_err()`. These operations SHALL be resolved by the semantic core without any import declaration. They SHALL be available on any expression whose static type is `Result[T, E]` and SHALL be a type error on any other type.

`is_ok()` SHALL return `Bool` with value `true` when the receiver is `Ok(v)` and `false` when it is `Err(e)`.
`is_err()` SHALL return `Bool` with value `true` when the receiver is `Err(e)` and `false` when it is `Ok(v)`.

#### Scenario: is_ok returns true for Ok
- **WHEN** a `Result[T, E]` variable holds `Ok(v)`
- **THEN** calling `is_ok()` on it returns `true`

#### Scenario: is_ok returns false for Err
- **WHEN** a `Result[T, E]` variable holds `Err(e)`
- **THEN** calling `is_ok()` on it returns `false`

#### Scenario: is_err returns true for Err
- **WHEN** a `Result[T, E]` variable holds `Err(e)`
- **THEN** calling `is_err()` on it returns `true`

#### Scenario: is_err returns false for Ok
- **WHEN** a `Result[T, E]` variable holds `Ok(v)`
- **THEN** calling `is_err()` on it returns `false`

#### Scenario: is_ok on non-Result is a type error
- **WHEN** `is_ok()` is called on a value whose static type is not `Result[T, E]`
- **THEN** the type-checker SHALL emit a type error and reject the program

---

### Requirement: Result built-in extraction operations
`Result[T, E]` SHALL expose two built-in extraction operations: `value()` and `error()`. These operations SHALL be resolved without any import declaration.

`value()` SHALL have static return type `T` and SHALL return the inner value when the receiver is `Ok(v)`. If the receiver is `Err(e)` at runtime, the interpreter SHALL raise a named runtime error `InvalidResultValueAccess`.

`error()` SHALL have static return type `E` and SHALL return the inner error when the receiver is `Err(e)`. If the receiver is `Ok(v)` at runtime, the interpreter SHALL raise a named runtime error `InvalidResultErrorAccess`.

#### Scenario: value extracts from Ok
- **WHEN** a `Result[T, E]` variable holds `Ok(42)`
- **THEN** calling `value()` on it returns `42`

#### Scenario: value on Err raises runtime error
- **WHEN** a `Result[T, E]` variable holds `Err("bad")` and `value()` is called on it
- **THEN** the interpreter SHALL raise `InvalidResultValueAccess` with a diagnostic that includes the operation name and the actual variant

#### Scenario: error extracts from Err
- **WHEN** a `Result[T, E]` variable holds `Err("bad")`
- **THEN** calling `error()` on it returns `"bad"`

#### Scenario: error on Ok raises runtime error
- **WHEN** a `Result[T, E]` variable holds `Ok(1)` and `error()` is called on it
- **THEN** the interpreter SHALL raise `InvalidResultErrorAccess` with a diagnostic that includes the operation name and the actual variant

---

### Requirement: Result operations in TIR
The TIR lowering pass SHALL recognise calls to `is_ok()`, `is_err()`, `value()`, and `error()` on a receiver whose static type is `Result[T, E]` and SHALL emit dedicated typed IR nodes: `ResultIsOk`, `ResultIsErr`, `ResultValue`, `ResultError`. These SHALL NOT be represented as generic `Call(FieldAccess(...))` nodes in the typed IR.

#### Scenario: TIR contains ResultIsOk node
- **WHEN** the source contains `r.is_ok()` and `r` has type `Result[T, E]`
- **THEN** the TIR for that expression SHALL be a `ResultIsOk { target }` node, not a generic `Call` or `FieldAccess` node

#### Scenario: TIR contains ResultValue node
- **WHEN** the source contains `r.value()` and `r` has type `Result[T, E]`
- **THEN** the TIR for that expression SHALL be a `ResultValue { target }` node

---

### Requirement: Result operations codegen to Rust
The Rust codegen pass SHALL lower `ResultValue` to an explicit `match` expression that panics with the string `"InvalidResultValueAccess"` on the `Err` arm. `ResultError` SHALL similarly panic with `"InvalidResultErrorAccess"` on the `Ok` arm. `ResultIsOk` and `ResultIsErr` SHALL lower to the native Rust `is_ok()` and `is_err()` methods.

The generated code SHALL NOT use `.unwrap()` or `.unwrap_err()` for `value()` and `error()`.

#### Scenario: value() codegen uses explicit match
- **WHEN** `r.value()` is compiled to Rust and `r` has type `Result<T, E>`
- **THEN** the generated Rust SHALL be `match r { Ok(v) => v, Err(_) => panic!("InvalidResultValueAccess") }`

#### Scenario: is_ok() codegen uses native method
- **WHEN** `r.is_ok()` is compiled to Rust
- **THEN** the generated Rust SHALL be `r.is_ok()`
