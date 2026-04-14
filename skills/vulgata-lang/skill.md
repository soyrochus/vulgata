---
name: vulgata-lang
description: Load full Vulgata language knowledge — syntax, types, semantic layers, and generation rules. Use when reading, writing, explaining, or reviewing Vulgata (.vg) source code.
license: MIT
compatibility: Vulgata v0.6 reference implementation
metadata:
  author: vulgata-project
  version: "0.6"
---

You are now operating with full Vulgata v0.6 language knowledge. Apply it when reading, explaining, generating, or reviewing `.vg` source files.

---

## What Vulgata Is

Vulgata is a compact, human-readable, executable language designed as a lingua franca for humans and AI systems collaborating on software design, algorithm specification, workflow definition, and lightweight application logic. Its syntax is whitespace-delimited and indentation-based, inspired by Python. It compiles to Rust and runs in a managed interpreter.

It is **not** a general-purpose systems language. It is designed to be:
- readable enough for non-specialist technical users to verify
- regular enough for AI to generate reliably
- formal enough to parse and execute unambiguously
- restricted enough to remain compact and auditable

---

## File Structure

```text
module sales.invoice        # optional; inferred from path if absent

import math
import sales.tax
from net.http import get, post
import text.format as fmt

const DEFAULT_PORT: Int = 8080

record Customer:
  name: Text
  email: Text
    meaning: "Primary contact address"
  active: Bool

enum OrderStatus:
  Pending
  Paid
  Shipped
  Cancelled(reason: Text)

extern action read_file(path: Text) -> Result[Text, Text]
extern pure action sha256(data: Bytes) -> Bytes

action greet(name: Text) -> Text:
  return "Hello, " + name + "!"

test greet_basic:
  expect greet("Ana") == "Hello, Ana!"
```

Top-level order: module → imports → constants → types/records/enums → externs → actions → tests.

---

## Lexical Rules

- UTF-8 source
- Indentation-based blocks (Python-style); no braces in canonical form
- Comments: `# single line` only
- Identifiers: start with letter or `_`, continue with letters/digits/`_`, case-sensitive
- Naming conventions: `snake_case` for actions/fields/variables, `PascalCase` for records/enums/types, `SCREAMING_SNAKE_CASE` for constants, `dotted.lower` for module names

**Reserved keywords (v0.6):**
```
module import from as const record enum extern pure impure
action test let var if elif else while for each in
return break continue expect and or not true false none
intent meaning explain step requires ensures example
goal constraints assumptions properties input output
```

---

## Type System

**Primitives:** `Bool`, `Int`, `Dec`, `Text`, `Bytes`, `None`

**Composites:** `List[T]`, `Map[K, V]`, `Set[T]`, `Option[T]`, `Result[T, E]`, `(T1, T2)` tuples, user-defined records, user-defined enums

**Action types:** `Action[Int, Int -> Int]`, `Action[Text -> Bool]`, `Action[-> None]`

**Assignability rules:**
- `T` assigns to `Option[T]`
- `none` assigns only to `None` or `Option[T]`
- `Int → Dec` widening allowed where numeric rules permit
- `Dec → Int` narrowing: forbidden implicitly
- No truthiness conversions
- No implicit `Text ↔ Bytes`

**Type inference:** allowed locally (`let x = 1`); required explicitly when ambiguous (`let empty: List[Text] = []`). Action signatures must always be explicit.

---

## Declarations

### Constants
```text
const MAX_RETRIES: Int = 3
```

### Records
```text
record Point:
  x: Dec
  y: Dec

record Customer:
  name: Text
  email: Text
    meaning: "Primary contact address"   # descriptive metadata — no runtime effect
  active: Bool
```
Construction: `Customer(name: "Ana", email: "a@b.com", active: true)`

### Enums
```text
enum Direction:
  North
  South
  East
  West

enum Result:
  Ok(value: Int)
  Err(message: Text)
```

### Actions
```text
action gcd(a: Int, b: Int) -> Int:
  requires a >= 0
  requires b >= 0
  intent:
    goal: "Return the greatest common divisor"
    properties:
      - "gcd(a, b) == gcd(b, a % b)"
  var x = a
  var y = b
  step iterate:
    while y != 0:
      let r = x % y
      x := y
      y := r
  ensures result >= 0
  return x
```

### Externs
```text
extern action read_file(path: Text) -> Result[Text, Text]
extern pure action sha256(data: Bytes) -> Bytes
extern impure action log(msg: Text) -> None
```

### Tests
```text
test gcd_basic:
  expect gcd(84, 30) == 6
```

---

## Statement Reference

| Statement | Layer | Notes |
|---|---|---|
| `let x = expr` | executable | immutable binding |
| `var x = expr` | executable | mutable binding |
| `target := expr` | executable | mutation; target must be rooted in `var` |
| `if / elif / else` | executable | condition must be `Bool` |
| `while cond:` | executable | |
| `for each item in list:` | executable | |
| `match expr:` | executable | pattern arms in source order; raises `NonExhaustiveMatch` if no arm matches |
| `return [expr]` | executable | bare `return` only for `None`-returning actions |
| `break` / `continue` | executable | loops only |
| `action_call(args)` | executable | expression statement |
| `expect expr` | **checkable** | enforced in checked/debug; skipped in release |
| `requires expr` | **checkable** | pre-condition; checked before action body |
| `ensures expr` | **checkable** | post-condition; `result` = return value |
| `example name:` | **checkable** | embedded test; run in checked/debug only |
| `intent:` | **descriptive** | goal/constraints/assumptions/properties |
| `explain:` | **descriptive** | free-text description; no runtime effect |
| `step name:` | **descriptive** | transparent wrapper; label used in debug trace |

---

## Semantic Layers

Three layers govern which constructs run and when:

| Layer | Runtime effect | release | checked | debug | tooling |
|---|---|---|---|---|---|
| Executable | always runs | ✓ | ✓ | ✓ | ✓ |
| Checkable | conditional | skip | enforce | enforce | expose |
| Descriptive | never runs | skip | skip | skip/trace | expose |

**`intent:` syntax:**
```text
intent:
  goal: "One-line description of the action's purpose"
  constraints:
    - "Inputs must be non-negative"
  assumptions:
    - "Caller has already validated the record"
  properties:
    - "Pure — no side effects"
```

**`explain:` syntax:**
```text
explain:
  "First step normalizes the value"
  "Second step clamps to the allowed range"
```

**`step` syntax:**
```text
step normalize:
  let score = raw / 10
  ...
```
Body executes identically to unwrapped statements. Label traced in debug mode.

**`requires` / `ensures`:**
```text
requires amount >= 0
ensures result >= 0
```
`result` is the return value, available only inside `ensures` expressions.

**`example` syntax:**
```text
example gcd_basic:
  input:
    a = 84
    b = 30
  output:
    result = 6
```
Both `input:` and `output:` are required. Bindings are `name = literal`.

---

## Match Statement

`match` evaluates the scrutinee once, tests arms in source order, executes the first matching arm, and raises `NonExhaustiveMatch` if no arm matches.

```text
match result:
  Ok(value):
    return value
  Err(message):
    return message

match direction:
  North():
    return "up"
  South():
    return "down"
  _:
    return "other"
```

**Phase-1 patterns:**

| Pattern form | Example | Notes |
|---|---|---|
| Wildcard | `_` | matches anything, binds nothing |
| Literal | `42`, `"hello"`, `true` | exact value match |
| Binding | `x` | binds the matched value to `x` |
| Tuple | `(a, b)` | destructures a tuple |
| Nominal record | `Customer(name: n, active: a)` | binds named fields |
| Variant (empty) | `North()` | matches enum variant with no payload |
| Variant (payload) | `Cancelled(reason)` | binds payload fields |

Pattern rules:
- Arms are tested in source order; the first match wins
- `_` (wildcard) should appear last as a catch-all
- Enum empty variants must be matched as `VariantName()`, not bare identifiers
- Nested patterns are not yet supported beyond what is listed above

---

## Destructuring in let / var

Tuple and nominal-record destructuring are supported in `let` and `var` declarations:

```text
let (left, right) = pair
var Customer(name: current_name, active: current_active) = customer
```

Rules:
- Only tuple and nominal record forms are allowed in declarations
- Destructured outputs must be plain identifiers (no nesting)
- Wildcard and enum-style destructuring are rejected in `let` / `var`
- Destructuring in `:=` (assignment) is rejected

---

## Expression Operators

**Unary:** `-`, `not`

**Binary (precedence high → low):**
1. postfix: `.field`, `[index]`, `(args)` — call / access
2. unary: `-`, `not`
3. `*`, `/`, `%`
4. `+`, `-`
5. `==`, `!=`, `<`, `<=`, `>`, `>=`
6. `and`
7. `or`

Short-circuit: `and` and `or`.

**Calls:**
```text
gcd(84, 30)
format(text: "Hello, {name}", name: customer.name)  # named args recommended
```

**Call argument rules:**
- Positional arguments must come first
- Named arguments may follow positional ones
- Once a named argument is used, all remaining arguments must be named
- Duplicate names and unknown names are compile-time errors
- No function overloading — each name resolves to exactly one action
- Dispatch is static and lexical, not dynamic

---

## Mutability Model

```text
let port: Int = 8080          # immutable — cannot be reassigned
var retries: Int = 0          # mutable
retries := retries + 1        # mutation always uses :=
```

**Action parameters are always immutable.** To mutate a parameter value locally, copy it into a `var` first:

```text
action countdown(n: Int) -> Int:
  var i = n          # copy parameter into mutable local
  while i > 0:
    i := i - 1
  return i
```

**Records and collections follow the same let/var rule:**

```text
let customer = Customer(name: "Ana", email: "a@b.com", active: true)
# customer.email := "x"   ← compile error: let binding is immutable

var mutable_c = Customer(name: "Ana", email: "a@b.com", active: true)
mutable_c.email := "new@b.com"   # ok

var items = [1, 2, 3]
items[1] := 42                   # ok
```

Mutating a `var`-rooted value must not implicitly mutate any value visible through a `let` binding.

---

## Standard Library

Vulgata v0.6 ships two implemented modules: `console` and `file`. All I/O returns `Result[T, Text]`.

### `console` module

```text
console.print(value: Text) -> Result[None, Text]      # write without newline
console.println(value: Text) -> Result[None, Text]    # write with newline
console.eprint(value: Text) -> Result[None, Text]     # stderr, no newline
console.eprintln(value: Text) -> Result[None, Text]   # stderr, with newline
console.read_line() -> Result[Text, Text]             # read line (no trailing \n)
```

`read_line` returns `Err(...)` at end-of-input.

### `file` module

```text
file.read_text(path: Text) -> Result[Text, Text]              # read full file as UTF-8
file.write_text(path: Text, content: Text) -> Result[None, Text]   # overwrite file
file.append_text(path: Text, content: Text) -> Result[None, Text]  # append (creates if absent)
file.exists(path: Text) -> Bool                               # check file presence
```

### Using `Result` and `Option`

There is no exception system. All fallible operations return `Result[T, E]`. Use built-in member operations or `match` to handle them:

**`Result[T, E]` operations** (no import needed):
- `is_ok()` → `Bool`
- `is_err()` → `Bool`
- `value()` → `T` (runtime error if called on `Err`)
- `error()` → `E` (runtime error if called on `Ok`)

**`Option[T]` operations** (no import needed):
- `is_some()` → `Bool`
- `is_none()` → `Bool`
- `value()` → `T` (runtime error if called on `None`)

Branch explicitly:

```text
let res = file.read_text("config.txt")
if res.is_ok():
  let content = res.value()
  let _ = console.println(content)
else:
  let _ = console.eprintln("could not read file")
```

Or use `match`:

```text
match file.read_text("config.txt"):
  Ok(content):
    let _ = console.println(content)
  Err(msg):
    let _ = console.eprintln(msg)
```

Discard an unwanted `Result` return value by binding it to `_`:

```text
let _ = console.println("hello")
```

There is no dedicated `print` statement — console calls are ordinary action calls to keep side-effects explicit.

---

## Execution Modes

| Flag | Default for |
|---|---|
| `--mode release` | `vulgata run` |
| `--mode checked` | explicit |
| `--mode debug` | explicit |
| `--mode tooling` | `vulgata repl` |

```sh
vulgata run --mode checked score.vg
vulgata run --mode release score.vg
vulgata check --emit-metadata score.json score.vg
```

`--emit-metadata <path>` writes a JSON document of all semantic-layer data (intent, contracts, step labels, examples, field meanings) without executing the program.

---

## Generation Rules

When writing Vulgata source, follow these rules:

1. **Every `action` must have an explicit return type** (or `-> None` if void).
2. **Mutation is always `:=`**. Never use `=` for assignment after declaration.
3. **Use `let` by default; `var` only when mutation is needed.**
4. **Never add `intent:` or `requires`/`ensures` unless the user asks** — the executable layer is the default.
5. **`intent:` goes first** in an action body, before any executable statements.
6. **`requires` goes before executable statements; `ensures` goes after the last executable statement, immediately before `return`.**
7. **`example` blocks may appear anywhere in the action body** but by convention come before the first executable statement.
8. **`step` labels should be short identifiers** (`normalize`, `iterate`, `validate`).
9. **Record construction uses named fields:** `Customer(name: "Ana", email: "a@b.com", active: true)`.
10. **Conditions must be `Bool`** — there is no truthiness coercion.
11. **`for each` is the only iteration form over lists** — no C-style `for`.
12. **`break` and `continue` are valid only inside `while` or `for each`.**
13. **Module names are dotted lowercase** (`sales.invoice`, `net.http`).
14. **Do not add a trailing semicolon** — Vulgata uses newlines and indentation, not semicolons.
15. **`meaning:` annotations indent one level under the field they describe.**
16. **Action parameters are immutable.** Copy to `var` if local mutation is needed.
17. **All stdlib I/O returns `Result[T, Text]`.** Always handle or discard with `let _ = ...`.
18. **There is no `print` statement.** Use `console.println(...)`.
19. **Positional call arguments must come before named ones.** Once you use a named arg, all remaining args must be named.
20. **`Result` and `Option` use `.value()`, not `.unwrap()`.** The correct extraction method is `.value()`. `.error()` extracts the error side of a `Result`.
21. **`match` arms are tested in source order; include a wildcard `_:` arm last** when exhaustive matching is not guaranteed.
22. **Empty enum variants in `match` require `()`: write `North()`, not `North`.**
23. **Destructuring in `:=` is rejected.** Only `let` and `var` declarations support tuple and nominal-record destructuring.
24. **Destructured identifiers must be plain names.** Nested or wildcard destructuring is not supported in `let`/`var`.

---

## Complete Example

```text
module math.gcd

action gcd(a: Int, b: Int) -> Int:
  requires a >= 0
  requires b >= 0
  intent:
    goal: "Compute the greatest common divisor of two non-negative integers"
    properties:
      - "gcd(a, 0) == a"
      - "gcd(a, b) == gcd(b, a % b)"
  example gcd_basic:
    input:
      a = 84
      b = 30
    output:
      result = 6
  example gcd_zero:
    input:
      a = 5
      b = 0
    output:
      result = 5
  var x = a
  var y = b
  step iterate:
    while y != 0:
      let r = x % y
      x := y
      y := r
  ensures result >= 0
  return x

test gcd_basic:
  expect gcd(84, 30) == 6
  expect gcd(0, 5) == 5
  expect gcd(7, 7) == 7
```
