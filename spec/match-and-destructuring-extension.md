# Vulgata Design Proposal — Match and Destructuring Extension

## 1. Purpose

This document defines three related extensions to Vulgata:

1. `match` as a statement-level branching construct
2. destructuring in `let` and `var` declarations
3. name-based record destructuring for both `match` and declaration destructuring

The design goal is to add the minimum expressive power needed to make `Result`, `Option`, tuples, enums, and records ergonomic, without turning Vulgata into a large pattern language.

This proposal follows these constraints:

* `match` is introduced first as a **statement**, not an expression
* destructuring is introduced only for **binding**, not for `:=`
* record destructuring is **name-based**, not positional
* pattern bindings are immutable by default
* full flow-sensitive type narrowing is not required in the first implementation
* semantic layers remain intact: `match` is executable, while `intent`, `explain`, `step`, `requires`, `ensures`, and `example` continue to behave as already defined in v0.5 

---

# Part 1 — `match`

## 2. Design intent

`match` provides structured branching based on the shape and variant of a value.

It is intended primarily for:

* `Result[T, E]`
* `Option[T]`
* user-defined enums
* tuples
* records, through name-based patterns

The first version of `match` is statement-only. It does not produce a value directly. Expression-form `match` may be added later.

## 3. Surface syntax

### 3.1 Basic form

```text
match value:
  pattern_1:
    block_1
  pattern_2:
    block_2
```

### 3.2 Example with `Result`

```text
match result:
  Ok(value):
    return value
  Err(error):
    return fallback(error)
```

### 3.3 Example with `Option`

```text
match maybe_name:
  Some(name):
    return name
  None:
    return "unknown"
```

### 3.4 Example with tuple

```text
match pair:
  (left, right):
    return left + right
```

## 4. Allowed pattern forms in phase 1

The first implementation should support only the following patterns:

* wildcard: `_`
* literal pattern
* binding pattern: bare identifier
* enum variant pattern
* tuple pattern
* name-based record pattern

This keeps the feature narrow and predictable.

## 5. Pattern meanings

### 5.1 Wildcard

```text
_
```

Matches anything and binds nothing.

### 5.2 Literal pattern

```text
0
"ok"
true
none
```

Matches when the value equals that literal.

### 5.3 Binding pattern

```text
name
```

Matches anything and binds the value to `name` for the current branch.

This is equivalent to “capture the matched value”.

### 5.4 Enum variant pattern

```text
Ok(value)
Err(error)
Some(x)
None
Cancelled(reason)
```

Matches a specific variant and optionally destructures its payload.

### 5.5 Tuple pattern

```text
(left, right)
(a, b, c)
```

Matches a tuple by arity and destructures its elements.

### 5.6 Name-based record pattern

```text
Customer(name: n, email: e)
Customer(active: true)
Customer(name: n, active: false)
```

Matches a record of the named type and destructures only the named fields.

Record matching is nominal, not structural. The record type name must match exactly.

## 6. Branch semantics

A `match` statement evaluates its target expression once.

Arms are tested in source order.

The first matching arm executes. No fallthrough exists.

If no arm matches, runtime behavior depends on implementation mode:

* in all execution modes, failure to match is a runtime error unless a wildcard or otherwise exhaustive arm is present
* later versions may add exhaustiveness checking at compile time

## 7. Binding scope

Variables introduced by a pattern exist only inside the block of that arm.

They behave like immutable `let` bindings.

Example:

```text
match result:
  Ok(value):
    return value
  Err(error):
    return fallback(error)
```

Here `value` exists only inside the `Ok` arm, and `error` exists only inside the `Err` arm.

## 8. Exhaustiveness

The first implementation does not require full static exhaustiveness analysis.

However:

* implementations should perform exhaustiveness checks where trivial, especially for `Option` and `Result`
* non-exhaustive `match` must remain a well-defined runtime error, not undefined behavior

Recommended runtime error name:

* `NonExhaustiveMatch`

## 9. Interaction with semantic layers

Each `match` arm contains a normal block.

That means the following remain legal inside an arm:

* `let`
* `var`
* `:=`
* `if`
* `while`
* `for each`
* `return`
* `expect`
* `intent`
* `explain`
* `step`
* `requires`
* `ensures`
* `example`

`match` itself belongs to the executable layer.

## 10. Grammar additions for `match`

```ebnf
statement       = intent_stmt
                | explain_stmt
                | step_stmt
                | requires_stmt
                | ensures_stmt
                | example_stmt
                | match_stmt
                | let_stmt
                | var_stmt
                | assign_stmt
                | if_stmt
                | while_stmt
                | for_stmt
                | return_stmt
                | break_stmt
                | continue_stmt
                | expect_stmt
                | expr_stmt ;

match_stmt      = "match" expr ":" NEWLINE
                  INDENT { match_arm } DEDENT ;

match_arm       = pattern ":" NEWLINE
                  INDENT block DEDENT ;

pattern         = wildcard_pattern
                | literal_pattern
                | binding_pattern
                | enum_pattern
                | tuple_pattern
                | record_pattern ;

wildcard_pattern = "_" ;
literal_pattern  = literal ;
binding_pattern  = Identifier ;

enum_pattern    = Identifier [ "(" [ pattern { "," pattern } ] ")" ] ;

tuple_pattern   = "(" pattern { "," pattern } ")" ;

record_pattern  = Identifier "(" [ record_field_pattern { "," record_field_pattern } ] ")" ;
record_field_pattern = Identifier ":" pattern ;
```

## 11. Type-checking rules for `match`

The type checker must enforce:

* the match target expression is well-typed
* literal patterns must be assignable to the target type
* tuple patterns must match tuple arity
* enum variant patterns must refer to a variant of the target enum type, or of built-in `Result` / `Option`
* record patterns must refer to the exact nominal record type of the target
* field names in record patterns must exist on that record type
* identifiers introduced by a pattern are scoped only to that arm
* duplicate binding names inside the same pattern are invalid

---

# Part 2 — `let` and `var` binding destructuring

## 12. Design intent

Destructuring in bindings allows a compound value to be unpacked into multiple new names.

This is introduced only for declarations:

* `let`
* `var`

It is not introduced for `:=`.

This keeps mutation semantics simple and avoids partial writable target semantics.

## 13. Allowed forms in phase 1

The first version of declaration destructuring should support:

* tuple destructuring
* name-based record destructuring

It should not support enum destructuring through `let` / `var` in phase 1.

That is intentional: enum and `Result` / `Option` destructuring belong in `match`, not in direct declarations.

## 14. Tuple destructuring

### 14.1 Syntax

```text
let (a, b) = pair
var (x, y) = get_coordinates()
```

### 14.2 Semantics

The right-hand side is evaluated once.

Its value must be a tuple of the same arity.

Bindings introduced on the left are new bindings:

* immutable for `let`
* mutable for `var`

Example:

```text
let (left, right) = pair
```

is equivalent in intent to:

```text
let left = tuple_get_0(pair)
let right = tuple_get_1(pair)
```

but remains one source-level binding form.

## 15. Record destructuring in declarations

### 15.1 Syntax

```text
let Customer(name: n, email: e) = customer
var Customer(name: n, active: is_active) = customer
```

### 15.2 Semantics

The right-hand side is evaluated once.

It must be a value of the named nominal record type.

Only the named fields are destructured.

Bindings introduced by the pattern are:

* immutable for `let`
* mutable for `var`

The mutability applies to the new binding, not to the original record.

## 16. No destructuring in assignment

The following remains invalid:

```text
(a, b) := pair
Customer(name: n) := customer
```

`:=` continues to target only ordinary writable places rooted in `var`.

This is a deliberate restriction.

## 17. Grammar changes for binding destructuring

Current `let` / `var` forms accept a single identifier. This proposal extends them to accept a binding pattern.

```ebnf
let_stmt        = "let" binding_pattern [ ":" type ] "=" expr NEWLINE ;
var_stmt        = "var" binding_pattern [ ":" type ] "=" expr NEWLINE ;

binding_pattern = Identifier
                | tuple_binding_pattern
                | record_binding_pattern ;

tuple_binding_pattern  = "(" Identifier { "," Identifier } ")" ;

record_binding_pattern = Identifier "(" record_binding_bind { "," record_binding_bind } ")" ;
record_binding_bind    = Identifier ":" Identifier ;
```

This first version keeps declaration destructuring narrower than general `match` patterns:

* only identifiers may appear as bound outputs
* no wildcard in declaration destructuring
* no nested destructuring in declaration destructuring
* no enum destructuring in declaration destructuring

That is a deliberate simplification.

## 18. Type-checking rules for declaration destructuring

The type checker must enforce:

* tuple binding arity matches tuple value arity
* record binding type name matches the right-hand-side record type
* record field names exist on the record
* duplicate binding names are invalid
* optional type annotations on destructuring declarations, if allowed, must match the whole right-hand-side type, not the individual extracted fields

Example:

```text
let (a, b): (Int, Int) = pair
```

is valid.

But this is not part of the first recommendation:

```text
let (a: Int, b: Int) = pair
```

Per-element annotation should be deferred.

---

# Part 3 — Name-based record destructuring for both `match` and declaration destructuring

## 19. Design intent

Records in Vulgata are nominal, field-based product types. Record destructuring should therefore be:

* explicit
* name-based
* nominal
* partial by field selection

It should not be positional.

This keeps record matching aligned with how records are declared and constructed in the current language. 

## 20. Record destructuring in `match`

### 20.1 Basic syntax

```text
match customer:
  Customer(name: n, email: e):
    return e
  Customer(active: false):
    return "inactive"
```

### 20.2 Semantics

A record pattern matches if:

* the target value is of the named nominal record type
* every named field pattern matches the corresponding field value

Unnamed fields are ignored.

### 20.3 Field patterns

Field patterns in `match` may use full patterns, not just identifiers.

Examples:

```text
Customer(active: true)
Customer(name: n, active: false)
Customer(email: _)
```

This is one reason record destructuring in `match` is more expressive than declaration destructuring.

## 21. Record destructuring in `let` and `var`

### 21.1 Basic syntax

```text
let Customer(name: n, email: e) = customer
var Customer(name: n, active: flag) = customer
```

### 21.2 Semantics

This is declaration-time unpacking, not pattern matching in the broad sense.

The right-hand side must be a record of the named type.

Only selected fields are extracted.

The extracted names become new bindings.

### 21.3 Why this is narrower than `match`

In declaration destructuring, the goal is simple unpacking, not conditional branching.

So declaration destructuring should not support:

* literal subpatterns
* wildcard `_`
* nested record subpatterns
* enum subpatterns

That belongs to `match`.

## 22. Examples

### 22.1 `match` with record destructuring

```text
match customer:
  Customer(active: false):
    return "inactive"
  Customer(name: n, email: e):
    return e
```

### 22.2 `let` with record destructuring

```text
let Customer(name: n, email: e) = customer
return e
```

### 22.3 `var` with record destructuring

```text
var Customer(name: n, active: is_active) = customer
if is_active:
  n := "updated"
```

This mutates the local binding `n`, not the original record field.

## 23. Challenges and restrictions

### 23.1 No writable aliasing back into the record

Destructured bindings are values, not live field references.

This follows the existing Vulgata value-oriented model. A destructured field does not become an alias into the original record. 

So:

```text
var Customer(name: n) = customer
n := "x"
```

does not mutate `customer.name`.

### 23.2 No positional record destructuring

This is invalid:

```text
let (name, email) = customer
```

unless `customer` is actually a tuple.

Records remain name-based.

### 23.3 No mixed declaration pattern language in phase 1

Declaration destructuring remains intentionally narrower than `match`. This keeps parsing, error reporting, and code generation simpler.

---

# 24. Typed IR recommendations

The semantic core should not leave these features as ad hoc surface syntax.

Recommended IR forms:

```text
TStmt
  Match { target: TValueExpr, arms: Vec<TMatchArm> }
  LetPattern { pattern: TBindingPattern, init: TValueExpr }
  VarPattern { pattern: TBindingPattern, init: TValueExpr }
  ...

TMatchArm
  pattern: TPattern
  body: TBlock

TPattern
  Wildcard
  Literal
  Bind { name }
  Enum { variant, subpatterns }
  Tuple { subpatterns }
  Record { type_name, fields }

TBindingPattern
  Bind { name }
  TupleBind { names }
  RecordBind { type_name, fields }
```

This makes lowering to both interpreter and compiler backends much clearer.

---

# 25. Conformance requirements

The conformance suite should include at least:

### 25.1 `Result` match

```text
test result_match_ok:
  let r = Ok(10)
  match r:
    Ok(v):
      expect v == 10
    Err(e):
      expect false
```

### 25.2 `Option` match

```text
test option_match_none:
  let x = None
  match x:
    Some(v):
      expect false
    None:
      expect true
```

### 25.3 Tuple destructuring

```text
test tuple_let_destructure:
  let pair = (1, 2)
  let (a, b) = pair
  expect a == 1
  expect b == 2
```

### 25.4 Record `let` destructuring

```text
record Customer:
  name: Text
  email: Text

test record_let_destructure:
  let c = Customer(name: "Ana", email: "a@example.com")
  let Customer(name: n, email: e) = c
  expect n == "Ana"
  expect e == "a@example.com"
```

### 25.5 Record `match` destructuring

```text
record Customer:
  name: Text
  active: Bool

test record_match_destructure:
  let c = Customer(name: "Ana", active: false)
  match c:
    Customer(active: false):
      expect true
    Customer(name: n):
      expect false
```

---

# 26. Final recommendations

The recommended initial implementation is:

For `match`:

* statement-only
* support wildcard, literal, binding, enum, tuple, and name-based record patterns
* no expression-form `match` yet
* no full exhaustiveness requirement yet, but allow basic checks where trivial

For binding destructuring:

* allow destructuring only in `let` and `var`
* support tuple destructuring
* support name-based record destructuring
* do not allow destructuring in `:=`
* do not allow enum destructuring in declarations yet

For record destructuring:

* keep it nominal
* keep it name-based
* keep declaration destructuring narrower than `match`
* ensure destructured bindings are values, not writable aliases into the original record

That gives Vulgata most of the practical power needed for `Result`, `Option`, tuples, and record-oriented data flow, without pushing it into the complexity class of Rust, Scala, or Haskell.
