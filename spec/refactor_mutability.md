# Mutability Refactor for Vulgata v0.3

## 1. Scope

This document defines one language change only:

* replace the v0.2 mutability model with an immutable-by-default local binding model
* introduce `var` for mutable local bindings
* replace mutation syntax based on `set ... = ...` with `:=`

No other language change is intended here.

---

## 2. Previous model

In v0.2:

* `let` introduced a binding
* later reassignment used `set`
* record fields and indexed collection elements were also updated with `set`
* records and collections behaved like managed mutable values with observable aliasing

Examples:

```text
let total = 0
set total = total + 1

let customer = Customer(email: "before")
set customer.email = "after"
```

---

## 3. New model

In v0.3:

* `let` introduces an immutable binding
* `var` introduces a mutable binding
* all reassignment and in-place update use `:=`
* only targets rooted in `var` are writable
* values visible through `let` are not writable, including compound values

Examples:

```text
let port = 8080
var retries = 0
retries := retries + 1
```

---

## 4. Binding rules

## 4.1 `let`

`let` means immutable.

The following are invalid:

```text
let x = 1
x := 2
```

```text
let customer = Customer(email: "before")
customer.email := "after"
```

```text
let items = [1, 2, 3]
items[0] := 99
```

## 4.2 `var`

`var` means mutable.

The following are valid:

```text
var x = 1
x := 2
```

```text
var customer = Customer(email: "before")
customer.email := "after"
```

```text
var items = [1, 2, 3]
items[0] := 99
```

Action parameters should behave like `let`, not like `var`.

If a parameter needs local mutation, it must be copied into a mutable local first:

```text
action gcd(a: Int, b: Int) -> Int:
  var x = a
  var y = b
  ...
```

---

## 5. Record example

Given:

```text
record Customer:
  email: Text
```

The semantic difference is:

### Immutable binding

```text
let customer = Customer(email: "a@example.com")
```

Allowed:

* `customer.email`
* passing `customer` as a value
* returning `customer`

Not allowed:

* `customer := Customer(email: "b@example.com")`
* `customer.email := "b@example.com"`

### Mutable binding

```text
var customer = Customer(email: "a@example.com")
```

Allowed:

* `customer := Customer(email: "b@example.com")`
* `customer.email := "b@example.com"`

---

## 6. Writable target rule

A target is writable only if its root is a `var` binding.

Valid writable roots:

* `var x = ...` then `x := ...`
* `var customer = ...` then `customer.email := ...`
* `var items = ...` then `items[0] := ...`

Invalid writable roots:

* `let x = ...` then `x := ...`
* `let customer = ...` then `customer.email := ...`
* `let items = ...` then `items[0] := ...`

---

## 7. Aliasing and compound values

To make `let` strict for compound types, the language must not expose hidden shared mutable aliasing through immutable bindings.

The required observable rule is:

* mutating a value through a `var` binding must not silently mutate a value visible through a `let` binding

Implementation strategies may vary:

* copying
* structural sharing
* copy-on-write

But the user-visible semantics must remain:

* `let` gives an immutable value
* `var` gives writable storage

---

## 8. Grammar delta

Old forms:

```text
let_stmt = "let" Identifier [ ":" type ] "=" expr
set_stmt = "set" target "=" expr
```

New forms:

```text
let_stmt    = "let" Identifier [ ":" type ] "=" expr
var_stmt    = "var" Identifier [ ":" type ] "=" expr
assign_stmt = target ":=" expr
```

---

## 9. Type-checking delta

The type checker must now enforce:

* `let` bindings are not writable
* `var` bindings are writable
* any assignment target must be rooted in a `var` binding
* field and index updates through `let` roots are rejected

---

## 10. Summary

This refactor changes the language from:

* explicit mutation with `set` over generally mutable bindings

to:

* immutable-by-default bindings with `let`
* explicit mutable bindings with `var`
* explicit mutation syntax with `:=`

This preserves visible mutation while giving the language a clearer and stricter mutability model.
