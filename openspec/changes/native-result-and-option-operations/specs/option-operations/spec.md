## ADDED Requirements

### Requirement: Option runtime representation
The Vulgata runtime SHALL represent typed `Option[T]` values using two dedicated variants: `OptionSome(Box<Value>)` for a present value and `OptionNone` for an absent value. The existing `Value::None` variant SHALL be retained for the untyped `none` literal but SHALL NOT be used to represent typed `Option[T]` absence. When a value of static type `Option[T]` is constructed, the runtime SHALL produce `OptionSome` or `OptionNone` accordingly.

#### Scenario: Some(x) produces OptionSome at runtime
- **WHEN** a `Some(x)` expression is evaluated and its static type is `Option[T]`
- **THEN** the resulting runtime value SHALL be `OptionSome(x_value)`, not a generic record or `Value::None`

#### Scenario: None produces OptionNone for typed Option
- **WHEN** a `none` literal or `None` constructor is evaluated in a context where the static type is `Option[T]`
- **THEN** the resulting runtime value SHALL be `OptionNone`

#### Scenario: Untyped none remains Value::None
- **WHEN** a `none` literal is evaluated in a context where no `Option[T]` type is expected
- **THEN** the resulting runtime value SHALL be `Value::None`

---

### Requirement: Option built-in inspection operations
`Option[T]` SHALL expose two built-in boolean operations: `is_some()` and `is_none()`. These operations SHALL be resolved by the semantic core without any import declaration. They SHALL be available on any expression whose static type is `Option[T]` and SHALL be a type error on any other type.

`is_some()` SHALL return `Bool` with value `true` when the receiver is `OptionSome(v)` and `false` when it is `OptionNone`.
`is_none()` SHALL return `Bool` with value `true` when the receiver is `OptionNone` and `false` when it is `OptionSome(v)`.

#### Scenario: is_some returns true for Some
- **WHEN** an `Option[T]` variable holds `Some(v)`
- **THEN** calling `is_some()` on it returns `true`

#### Scenario: is_some returns false for None
- **WHEN** an `Option[T]` variable holds `None`
- **THEN** calling `is_some()` on it returns `false`

#### Scenario: is_none returns true for None
- **WHEN** an `Option[T]` variable holds `None`
- **THEN** calling `is_none()` on it returns `true`

#### Scenario: is_none returns false for Some
- **WHEN** an `Option[T]` variable holds `Some(v)`
- **THEN** calling `is_none()` on it returns `false`

#### Scenario: is_some on non-Option is a type error
- **WHEN** `is_some()` is called on a value whose static type is not `Option[T]`
- **THEN** the type-checker SHALL emit a type error and reject the program

---

### Requirement: Option built-in extraction operation
`Option[T]` SHALL expose a built-in extraction operation `value()`. This operation SHALL be resolved without any import declaration.

`value()` SHALL have static return type `T` and SHALL return the inner value when the receiver is `OptionSome(v)`. If the receiver is `OptionNone` at runtime, the interpreter SHALL raise a named runtime error `InvalidOptionValueAccess`.

#### Scenario: value extracts from Some
- **WHEN** an `Option[T]` variable holds `Some(10)`
- **THEN** calling `value()` on it returns `10`

#### Scenario: value on None raises runtime error
- **WHEN** an `Option[T]` variable holds `None` and `value()` is called on it
- **THEN** the interpreter SHALL raise `InvalidOptionValueAccess` with a diagnostic that includes the operation name and the actual variant

---

### Requirement: Option operations in TIR
The TIR lowering pass SHALL recognise calls to `is_some()`, `is_none()`, and `value()` on a receiver whose static type is `Option[T]` and SHALL emit dedicated typed IR nodes: `OptionIsSome`, `OptionIsNone`, `OptionValue`. These SHALL NOT be represented as generic `Call(FieldAccess(...))` nodes in the typed IR.

#### Scenario: TIR contains OptionIsSome node
- **WHEN** the source contains `x.is_some()` and `x` has type `Option[T]`
- **THEN** the TIR for that expression SHALL be an `OptionIsSome { target }` node, not a generic `Call` or `FieldAccess` node

#### Scenario: TIR contains OptionValue node
- **WHEN** the source contains `x.value()` and `x` has type `Option[T]`
- **THEN** the TIR for that expression SHALL be an `OptionValue { target }` node

---

### Requirement: Option operations codegen to Rust
The Rust codegen pass SHALL lower `OptionValue` to an explicit `match` expression that panics with the string `"InvalidOptionValueAccess"` on the `None` arm. `OptionIsSome` and `OptionIsNone` SHALL lower to the native Rust `is_some()` and `is_none()` methods.

The generated code SHALL NOT use `.unwrap()` for `value()`.

#### Scenario: value() codegen uses explicit match
- **WHEN** `x.value()` is compiled to Rust and `x` has type `Option<T>`
- **THEN** the generated Rust SHALL be `match x { Some(v) => v, None => panic!("InvalidOptionValueAccess") }`

#### Scenario: is_some() codegen uses native method
- **WHEN** `x.is_some()` is compiled to Rust
- **THEN** the generated Rust SHALL be `x.is_some()`
