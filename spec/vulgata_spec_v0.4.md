# Vulgata Design Specification version 0.4

## 1. Purpose

Vulgata is a compact, human-readable, executable language designed as a lingua franca for humans and AI systems collaborating on software design, algorithm specification, workflow definition, and lightweight application logic.

It is not intended to compete with Python, Rust, or Java as a general-purpose systems language. Its design target is different:

* readable enough for non-specialist technical users to verify
* regular enough for AI systems to generate reliably
* formal enough to compile and interpret unambiguously
* expressive enough to describe real algorithms and structured software behavior
* restricted enough to remain compact and auditable

Vulgata must support two equally valid execution modes:

1. **Interpreter mode** — source is parsed, analyzed, and executed in a managed runtime environment.
2. **Compiler mode** — source is translated to Rust and then compiled to native code with no dependency on the interpreter runtime other than minimal emitted support code strictly required by the program itself.

The canonical source language is the same in both cases. Semantics must match.

Version 0.4 keeps the v0.3 mutability model and adds two implemented surfaces:

* an initial standard runtime library through the `console` and `file` modules
* an interactive, source-persistent `vulgata repl` mode

---

## 2. Design goals

### 2.1 Primary goals

Vulgata should provide:

* a small surface grammar
* explicit semantics
* predictable execution
* strong readability
* explicit mutation
* explicit type model
* straightforward foreign function calls
* deterministic formatting
* easy implementation in Rust

### 2.2 Non-goals

Vulgata does not aim to provide:

* complex metaprogramming
* user-defined syntax
* inheritance-heavy object orientation
* hidden control flow
* implicit coercion culture
* magical dynamic dispatch everywhere
* a large standard library in the language core

### 2.3 Guiding principles

1. **One canonical readable form.** Indentation-based source is canonical.
2. **No hidden meaning.** Mutation, conversion, branching, and failure must be explicit.
3. **Minimal grammar, rich IR.** Keep syntax small; place richness in typed semantic IR.
4. **Same meaning in interpreter and compiler.** Divergence is a bug.
5. **External integration is first-class.** Calls to configured external functions must be easy.
6. **Generated code must be auditable.** Canonical formatting is mandatory.

---

## 3. Language overview

Vulgata consists of:

* modules
* imports
* constants
* type declarations
* record declarations
* enum declarations
* action declarations
* extern declarations
* tests
* statements
* expressions

The language is primarily statement-oriented, with expressions used where values are required.

The language uses indentation as the canonical block structure. A brace form may exist as a transport or serialization variant, but it is not the canonical source form.

---

## 4. Execution architecture

## 4.1 Front-end pipeline

Both interpreter and compiler share the same front-end:

1. lexical analysis
2. parsing
3. AST creation
4. name resolution
5. type resolution and checking
6. semantic lowering to typed IR
7. validation and optimization passes

After typed IR, the pipeline diverges.

## 4.2 Interpreter pipeline

Typed IR is executed by a Rust interpreter runtime.

The interpreter runtime contains:

* value model
* heap management for dynamic structures where needed
* call dispatcher
* module loader
* extern binding resolver
* standard library bindings
* error reporting with source spans
* test runner harness

## 4.3 Compiler pipeline

Typed IR is lowered to a Rust code generation IR, then emitted as Rust source, then compiled by Rust tooling.

The compiler output must:

* produce normal Rust source
* avoid dependence on the interpreter runtime
* inline or emit only the minimal support structures required for the compiled program
* generate explicit Rust types and functions wherever possible
* map Vulgata semantics predictably to Rust semantics

This means compiled output is not "Vulgata bytecode plus VM". It is ordinary Rust code generated from Vulgata.

## 4.4 Semantic consistency rule

The interpreter and compiler must be tested against a shared conformance suite.

Every language feature must have:

* parser tests
* type-check tests
* interpreter execution tests
* compiler execution tests
* equivalence tests where feasible

## 4.5 CLI and REPL model

The implementation exposes both file-oriented commands and an interactive REPL.

File-oriented commands:

```text
vulgata parse <source-file>
vulgata check <source-file>
vulgata run <source-file>
vulgata test <source-file>
vulgata compile <source-file>
```

Interactive command:

```text
vulgata repl
```

The REPL is backed by a virtual source file for the current session, accepts both declarations and expressions, and supports session-local bindings through `let`, `var`, and `:=`.

It follows these rules:

* source blocks are accepted only after the combined session source parses and checks successfully
* accepted declaration blocks become part of the session source
* expression blocks are checked against the current session source and evaluated without mutating it
* `let` and `var` input create REPL-local session bindings that are available to later expressions and assignments
* `:=` updates previously introduced mutable REPL bindings according to the same mutability rules used inside actions
* `:run` and `:test` rebuild a fresh interpreter from the current session source
* session state includes both source-persistent declarations and REPL-local bindings

Core REPL commands:

* `:help`
* `:show`
* `:parse`
* `:check`
* `:run`
* `:test`
* `:reset`
* `:quit`

The MVP REPL may reject extern-backed execution explicitly unless session-level extern configuration is added.

---

## 5. Source file and module model

## 5.1 Module structure

Each source file defines one module.

Optional header:

```text
module sales.invoice
```

If omitted, the module name is derived from the file path.

## 5.2 Imports

```text
import math
import sales.tax
import text.format as fmt
from net.http import get, post
```

Imports are purely lexical and semantic. No runtime side effects are permitted during import resolution.

## 5.3 Visibility

For v0.4, all top-level declarations are module-public by default. A later version may add `private`.

---

## 6. Lexical rules

## 6.1 Character set

Source is UTF-8.

## 6.2 Identifiers

Identifiers:

* start with a letter or `_`
* continue with letters, digits, `_`
* are case-sensitive

Convention:

* module names: dotted lowercase
* actions: snake_case
* records/enums/types: PascalCase
* fields/variables: snake_case
* constants: SCREAMING_SNAKE_CASE optional by style, not grammar

## 6.3 Comments

```text
# single line comment
```

Block comments are omitted in v0.4 to keep lexing simple.

## 6.4 Literals

Supported literal classes:

* integer
* decimal
* string
* boolean
* none
* list
* map

Examples:

```text
42
3.14
"hello"
true
false
none
[1, 2, 3]
{"x": 1, "y": 2}
```

---

## 7. Type system

## 7.1 General approach

Vulgata uses a **gradually explicit static type system**.

That means:

* all declarations and expressions have types
* type inference exists locally where obvious
* action signatures, extern signatures, record fields, enum payloads, and public constants should generally be explicit
* interpreter and compiler share the same type rules

The core type system must be strong enough for real execution, code generation, and FFI bridging.

## 7.2 Built-in primitive types

Primitive built-ins:

* `Bool`
* `Int`
* `Dec`
* `Text`
* `Bytes`
* `None`

The surface alias `Number` is removed in favor of `Int` and `Dec` for clarity.

## 7.3 Composite types

Composite types:

* `List[T]`
* `Map[K, V]`
* `Set[T]`
* `Option[T]`
* `Result[T, E]`
* tuples: `(T1, T2, ...)`
* user-defined records
* user-defined enums
* action types

## 7.4 Action types

Action types are first-class in the type system, though closures are deferred.

Syntax:

```text
Action[Int, Int -> Int]
Action[Text -> Bool]
Action[-> None]
```

This denotes callable values.

## 7.5 Type inference

Allowed local inference:

```text
let x = 1          # x: Int
let ok = true      # ok: Bool
let names = ["a"] # names: List[Text]
```

Required explicit types in ambiguous situations:

```text
let empty_names: List[Text] = []
let index: Int = 0
```

## 7.6 Structural versus nominal typing

* records are **nominally typed**
* enums are **nominally typed**
* tuples, lists, maps, sets are structural by shape and parameter types

Nominal records avoid accidental compatibility mistakes and improve generated Rust code.

## 7.7 Assignability rules

General rules:

* exact type matches assign
* `T` may assign to `Option[T]`
* `none` may assign only to `None` or `Option[T]`
* numeric widening `Int -> Dec` is allowed only where explicit numeric rules permit
* no implicit `Dec -> Int`
* no implicit `Text <-> Bytes`
* no truthiness conversions

## 7.8 Equality rules

`==` and `!=` are defined only for types with equality semantics.

Actions are not equatable in v0.4.

Maps and lists are equatable structurally if element/key/value types are equatable.

---

## 8. Declaration forms

## 8.1 Constants

```text
const DEFAULT_PORT: Int = 8080
const APP_NAME: Text = "vulgata-demo"
```

Constants are immutable.

## 8.2 Records

```text
record Customer:
  name: Text
  age: Int
  email: Text
  active: Bool
```

Records are product types with named fields.

Construction:

```text
let c = Customer(name: "Ana", age: 21, email: "a@example.com", active: true)
```

## 8.3 Enums

```text
enum OrderStatus:
  Pending
  Paid
  Shipped
  Cancelled(reason: Text)
```

Pattern matching is deferred for the first implementation. Instead, helper functions and predicates may be used initially. A later version may add `match`.

## 8.4 Actions

```text
action gcd(a: Int, b: Int) -> Int:
  var x = a
  var y = b
  while y != 0:
    let r = x % y
    x := y
    y := r
  return x
```

## 8.5 Extern declarations

Externs allow the source program to call functions not implemented in Vulgata.

```text
extern action read_file(path: Text) -> Result[Text, Text]
extern action write_log(level: Text, message: Text) -> None
```

These are declarations only. Binding is provided by configuration in interpreter mode or code generation mapping in compiler mode.

## 8.6 Tests

```text
test gcd_basic:
  expect gcd(84, 30) == 6
```

Tests are top-level executable blocks.

---

## 9. Statement model

Statements in v0.4:

* variable declaration
* mutable variable declaration
* assignment
* conditional
* loop
* for-each loop
* return
* break
* continue
* assertion
* expression statement

## 9.1 Variable declaration

```text
let total: Int = 0
let active = true
var count: Int = 0
```

`let` introduces an immutable binding.

`var` introduces a mutable binding.

## 9.2 Assignment

```text
count := count + 1
customer.email := "new@example.com"
items[0] := 99
```

Mutation and reassignment are always explicit via `:=`.

Only a target rooted in a `var` binding is writable.

A target rooted in a `let` binding is not writable, even for record fields and indexed collection elements.

## 9.3 Conditionals

```text
if amount > 0:
  return true
elif amount == 0:
  return false
else:
  return false
```

Condition must be `Bool`.

## 9.4 While loop

```text
while i < limit:
  i := i + 1
```

## 9.5 For-each loop

```text
for each item in items:
  process(item)
```

The iterator source must be iterable according to the standard semantics.

## 9.6 Return

```text
return value
return
```

Bare `return` is valid only in actions returning `None`.

## 9.7 Break and continue

```text
break
continue
```

Only valid inside loops.

## 9.8 Expect

```text
expect total == 10
```

Inside tests, failed `expect` marks the test as failed.
Outside tests, `expect` is legal only if the execution environment permits assertions in production mode. For v0.4, it is best limited to tests.

---

## 10. Expression model

Expressions include:

* literals
* variable references
* field access
* indexing
* constructor calls
* action calls
* unary operators
* binary operators
* grouped expressions

## 10.1 Field access

```text
customer.email
```

## 10.2 Indexing

```text
items[0]
map["key"]
```

## 10.3 Calls

```text
gcd(84, 30)
format(text: "Hello, {name}", name: customer.name)
```

Named arguments are strongly recommended for public or generated APIs.

## 10.4 Operators

Unary:

* `-`
* `not`

Binary arithmetic:

* `+`
* `-`
* `*`
* `/`
* `%`

Comparison:

* `==`
* `!=`
* `<`
* `<=`
* `>`
* `>=`

Boolean:

* `and`
* `or`

Short-circuit semantics apply to `and` and `or`.

## 10.5 Operator precedence

Highest to lowest:

1. postfix: `.`, `[]`, `()`
2. unary: `-`, `not`
3. multiplicative: `*`, `/`, `%`
4. additive: `+`, `-`
5. comparison
6. `and`
7. `or`

---

## 11. Foreign function and external integration model

This is essential and must be explicit.

## 11.1 Goal

A Vulgata program must be able to call external functions in both interpreter and compiler modes without changing source semantics.

## 11.2 Extern declaration syntax

```text
extern action now_iso() -> Text
extern action http_get(url: Text, headers: Map[Text, Text]) -> Result[Text, Text]
extern action sha256(data: Bytes) -> Bytes
```

## 11.3 Binding strategy

Bindings are not fully encoded in source. They are supplied by configuration.

Interpreter example configuration:

```toml
[extern.now_iso]
provider = "rust"
symbol = "runtime::time::now_iso"

[extern.http_get]
provider = "rust"
symbol = "runtime::http::get"
```

Compiler example configuration:

```toml
[extern.now_iso]
provider = "rust_emit"
path = "crate::support::time::now_iso"

[extern.http_get]
provider = "rust_emit"
path = "crate::support::http::get"
```

## 11.4 Type contract

Every extern declaration must have a fully explicit signature.

The binder must prove compatibility at registration time.

## 11.5 Runtime behavior

Interpreter mode:

* extern actions resolve through a registry
* values are converted between runtime values and native Rust values
* type mismatch is an interpreter setup error, not a silent runtime error

Compiler mode:

* extern calls become direct Rust calls to configured functions
* conversion code is emitted only when necessary
* if the types map directly, the call should compile to near-zero overhead wrapper code

## 11.6 Purity and side effects

Optional metadata may classify externs:

```text
extern pure action sha256(data: Bytes) -> Bytes
extern impure action write_log(level: Text, message: Text) -> None
```

If purity metadata is included, it is advisory for future optimization and analysis.

---

## 12. Standard library model

The standard library should be small and mostly defined as modules of actions rather than syntax.

Candidate modules:

* `text`
* `math`
* `list`
* `map`
* `set`
* `time`
* `result`
* `option`
* `test`
* `console`
* `file`

Examples:

```text
text.length(value: Text) -> Int
text.contains(value: Text, part: Text) -> Bool
math.max(a: Int, b: Int) -> Int
list.push(items: List[T], value: T) -> List[T]   # or mutation policy later
```

For v0.4, mutable updates use `:=` and require a target rooted in `var`. Standard library collection helpers may still return new values rather than mutating in place.

## 12.1 Initial standard runtime library

The first implemented standard runtime library consists of two built-in modules:

* `console`
* `file`

These remain ordinary module actions at the source level rather than special syntax.

Examples:

```text
let _ = console.println("hello")
let line = console.read_line()
let data = file.read_text("notes.txt")
let present = file.exists("config.txt")
```

### `console` module

```text
console.print(value: Text) -> Result[None, Text]
console.println(value: Text) -> Result[None, Text]
console.eprint(value: Text) -> Result[None, Text]
console.eprintln(value: Text) -> Result[None, Text]
console.read_line() -> Result[Text, Text]
```

Semantics:

* `print` and `eprint` write without a newline
* `println` and `eprintln` write with a newline terminator
* `read_line` returns `Ok(line)` without the trailing newline
* end-of-input is represented as `Err(...)`, not as an implicit sentinel value

### `file` module

```text
file.read_text(path: Text) -> Result[Text, Text]
file.write_text(path: Text, content: Text) -> Result[None, Text]
file.append_text(path: Text, content: Text) -> Result[None, Text]
file.exists(path: Text) -> Bool
```

Semantics:

* `read_text` reads the full file contents as UTF-8 text
* `write_text` replaces existing file contents
* `append_text` appends text and may create the file if needed
* `exists` reports whether a filesystem entry exists at the given path

Paths are represented as `Text` and interpreted according to the host environment.

## 12.2 No print statement rule

Vulgata does not introduce a `print` statement in v0.4.

Console and file operations remain ordinary action calls so side effects stay explicit and the language core stays small.

---

## 13. Error model

## 13.1 Core rule

Do not introduce exceptions in v0.4.

Instead, use `Result[T, E]` and `Option[T]` explicitly.

## 13.2 Result handling

Without full pattern matching, initial programs may use helper actions:

```text
if result.is_ok():
  ...
```

But a better long-term plan is to add controlled `match` later. For now, the language design should reserve the word `match` but not require it in the first implementation.

The initial standard runtime library follows this rule directly:

* console output returns `Result[None, Text]`
* console input returns `Result[Text, Text]`
* file reads return `Result[Text, Text]`
* file writes and appends return `Result[None, Text]`

## 13.3 Interpreter errors

Interpreter internal failures should report:

* source span
* module
* action
* operation
* diagnostic message

## 13.4 Compiler errors

Compiler diagnostics should reference original Vulgata spans, even when Rust compilation fails in generated code.

This requires source maps or generated comment markers.

---

## 14. Mutability and value model

## 14.1 Variables

Variables declared with `let` are immutable bindings.

Variables declared with `var` are mutable bindings.

Examples:

```text
let port: Int = 8080
var retries: Int = 0
retries := retries + 1
```

`let` bindings may not be reassigned.

`var` bindings may be reassigned only with `:=`.

Action parameters should be treated as immutable bindings.

If a parameter needs local mutation, it should first be copied into `var`.

## 14.2 Records and collections

For v0.4, compound values follow strict immutable-versus-mutable binding rules.

If a record, list, map, or other compound value is bound with `let`, the value is not writable through that binding.

If a record, list, map, or other compound value is bound with `var`, the value is writable through targets rooted in that binding using `:=`.

Examples:

```text
let customer = Customer(email: "before")
# invalid:
# customer.email := "after"

var mutable_customer = Customer(email: "before")
mutable_customer.email := "after"

var items = [1, 2, 3]
items[1] := 42
```

This rule is strict:

* `let customer = Customer(...)` forbids `customer := ...`
* `let customer = Customer(...)` also forbids `customer.email := ...`
* `let items = [1, 2, 3]` forbids `items[0] := ...`

In compiler mode, the generated Rust code may still use mutable implementation details internally.
In interpreter mode, runtime values may still use heap-backed storage internally.

Those implementation choices must not weaken the source-level immutability guarantees of `let`.

## 14.3 Aliasing

Aliasing semantics in v0.4 must preserve strict immutability for `let`.

The source-language model is value-oriented:

* assigning a compound value to `let` captures an immutable value
* assigning a compound value to `var` creates mutable storage for that variable
* mutating a `var`-rooted value must not implicitly mutate a value visible through an immutable `let` binding

Implementations may use copying, structural sharing, copy-on-write, or other optimizations internally as long as those observable semantics are preserved.

This means v0.4 should behave like an immutable-by-default language with explicit mutable variables, not like a shared-reference object model by default.

---

## 15. Full grammar proposal

Below is the expanded grammar for v0.4.

```ebnf
file            = { top_item } EOF ;

top_item        = module_decl
                | import_decl
                | const_decl
                | record_decl
                | enum_decl
                | extern_decl
                | action_decl
                | test_decl ;

module_decl     = "module" module_name NEWLINE ;
module_name     = Identifier { "." Identifier } ;

import_decl     = "import" module_name [ "as" Identifier ] NEWLINE
                | "from" module_name "import" import_list NEWLINE ;

import_list     = Identifier { "," Identifier } ;

const_decl      = "const" Identifier ":" type "=" expr NEWLINE ;

record_decl     = "record" Identifier ":" NEWLINE
                  INDENT { field_decl } DEDENT ;

field_decl      = Identifier ":" type NEWLINE ;

enum_decl       = "enum" Identifier ":" NEWLINE
                  INDENT { enum_variant } DEDENT ;

enum_variant    = Identifier [ "(" [ variant_fields ] ")" ] NEWLINE ;
variant_fields  = variant_field { "," variant_field } ;
variant_field   = Identifier ":" type ;

extern_decl     = "extern" [ purity_kw ] "action" Identifier
                  "(" [ params ] ")" "->" type NEWLINE ;

purity_kw       = "pure" | "impure" ;

action_decl     = "action" Identifier
                  "(" [ params ] ")"
                  [ "->" type ] ":" NEWLINE
                  INDENT block DEDENT ;

params          = param { "," param } ;
param           = Identifier ":" type ;

test_decl       = "test" Identifier ":" NEWLINE
                  INDENT block DEDENT ;

block           = { statement } ;

statement       = let_stmt
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

let_stmt        = "let" Identifier [ ":" type ] "=" expr NEWLINE ;
var_stmt        = "var" Identifier [ ":" type ] "=" expr NEWLINE ;

assign_stmt     = target ":=" expr NEWLINE ;

target          = Identifier { "." Identifier | "[" expr "]" } ;

if_stmt         = "if" expr ":" NEWLINE
                  INDENT block DEDENT
                  { "elif" expr ":" NEWLINE INDENT block DEDENT }
                  [ "else" ":" NEWLINE INDENT block DEDENT ] ;

while_stmt      = "while" expr ":" NEWLINE
                  INDENT block DEDENT ;

for_stmt        = "for" "each" Identifier "in" expr ":" NEWLINE
                  INDENT block DEDENT ;

return_stmt     = "return" [ expr ] NEWLINE ;

break_stmt      = "break" NEWLINE ;
continue_stmt   = "continue" NEWLINE ;
expect_stmt     = "expect" expr NEWLINE ;
expr_stmt       = expr NEWLINE ;

expr            = or_expr ;

or_expr         = and_expr { "or" and_expr } ;
and_expr        = equality_expr { "and" equality_expr } ;
equality_expr   = compare_expr { ("==" | "!=") compare_expr } ;
compare_expr    = add_expr { ("<" | "<=" | ">" | ">=") add_expr } ;
add_expr        = mul_expr { ("+" | "-") mul_expr } ;
mul_expr        = unary_expr { ("*" | "/" | "%") unary_expr } ;
unary_expr      = [ "-" | "not" ] unary_expr | postfix_expr ;

postfix_expr    = primary { postfix_op } ;
postfix_op      = "." Identifier
                | "[" expr "]"
                | "(" [ args ] ")" ;

args            = arg { "," arg } ;
arg             = [ Identifier ":" ] expr ;

primary         = literal
                | Identifier
                | constructor_call
                | list_literal
                | map_literal
                | tuple_literal
                | "(" expr ")" ;

constructor_call = Identifier "(" [ args ] ")" ;

list_literal    = "[" [ expr { "," expr } ] "]" ;
map_literal     = "{" [ map_pair { "," map_pair } ] "}" ;
map_pair        = expr ":" expr ;

tuple_literal   = "(" expr "," expr { "," expr } ")" ;

type            = simple_type | generic_type | tuple_type | action_type ;

simple_type     = Identifier ;
generic_type    = Identifier "[" type { "," type } "]" ;
tuple_type      = "(" type "," type { "," type } ")" ;
action_type     = "Action" "[" [ type { "," type } ] "->" type "]" ;

literal         = INT_LIT
                | DEC_LIT
                | STRING_LIT
                | "true"
                | "false"
                | "none" ;
```

---

## 16. Semantics of calls

This section is important for both interpreter and compiler.

## 16.1 Call categories

A call expression may target:

* a declared Vulgata action
* an extern action
* a constructor for a record
* a constructor for an enum variant if variant payloads are used as constructor forms later
* a first-class action value

## 16.2 Named and positional arguments

Both are supported.

Rules:

* positional arguments must come first
* named arguments may follow
* once a named argument is used, all following arguments must be named
* no duplicate names
* unknown names are compile-time errors

## 16.3 Overloading

No function overloading in v0.4.

This keeps resolution trivial and generated code clearer.

## 16.4 Dispatch

Dispatch is lexical and static by symbol identity, not dynamic multimethod dispatch.

This is a major simplification versus Smalltalk and intentional.

---

## 17. Interpreter design guidance for Rust agent

The interpreter should be implemented in layered modules.

Suggested structure:

* `lexer`
* `parser`
* `ast`
* `resolver`
* `types`
* `tir` (typed intermediate representation)
* `runtime`
* `externs`
* `stdlib`
* `diagnostics`
* `tests`

### Runtime value enum

A likely runtime value model:

* Bool
* Int
* Dec
* Text
* Bytes
* None
* List
* Map
* Set
* Tuple
* RecordInstance
* EnumInstance
* ActionRef
* Result
* Option

The interpreter should avoid embedding language semantics directly in parsing. Semantic phases must remain separate.

---

## 18. Compiler design guidance for Rust agent

The compiler path should lower typed IR into a Rust generation model.

Suggested phases:

1. typed IR validation
2. monomorphization or generic strategy decision if generics are implemented immediately
3. Rust AST or structured Rust emission model
4. support code emission
5. cargo project emission or inline module emission
6. rustc / cargo compilation

### Compiler output principle

Generated code should be understandable Rust, not opaque low-level output.

This matters because the generated Rust becomes the real executable form and may be inspected, profiled, or integrated into larger systems.

### No runtime dependency principle

Do not make the compiled output depend on the interpreter runtime crate.

Allowed:

* small emitted helper modules per compiled target
* generated structs, enums, wrappers
* direct calls to configured Rust functions

Not allowed:

* compiled program requiring a Vulgata VM to execute normal code paths

---

## 19. Minimal support code in compiled output

The compiler may emit support code only where necessary for language semantics.

Examples:

* helper type aliases
* generated `Result` conversion helpers
* map/list construction helpers if needed
* small source-location utilities for diagnostics if configured

This support code should be target-local and tree-shakeable by Rust.

---

## 20. Conformance examples

### 20.1 Basic algorithm

```text
action gcd(a: Int, b: Int) -> Int:
  var x: Int = a
  var y: Int = b
  while y != 0:
    let r: Int = x % y
    x := y
    y := r
  return x
```

### 20.2 Record and extern use

```text
record Customer:
  name: Text
  email: Text

extern action send_email(to: Text, subject: Text, body: Text) -> Result[None, Text]

action welcome(customer: Customer) -> Result[None, Text]:
  let body = "Welcome, " + customer.name
  return send_email(to: customer.email, subject: "Welcome", body: body)
```

### 20.3 Collection processing

```text
action sum_all(items: List[Int]) -> Int:
  var total: Int = 0
  for each item in items:
    total := total + item
  return total
```

### 20.4 Test

```text
test sum_all_basic:
  expect sum_all([1, 2, 3]) == 6
```

---

## 21. Open issues to freeze before implementation

These are the main points that must be decided early by the implementation agent and kept stable.

1. Whether generics are fully implemented in v0.4 or partially reserved.
2. Whether `Set[T]` is needed in the first iteration.
3. Whether tuples are required immediately.
4. Whether enum pattern matching is deferred entirely or included in the first milestone.
5. Whether collection operations are mutable, persistent, or mixed at the standard library level.
6. Exact behavior of map key equality and hashing.
7. Whether decimal is implemented as `f64` initially or as a decimal library.
8. Exact module-to-file mapping rules.
9. Whether source formatting is part of the first milestone.
10. Whether the compiler emits a full Cargo project or a Rust module by default.

---

## 22. Recommended implementation phases

### Phase 1

* lexer
* parser
* AST
* pretty-printer
* basic diagnostics

### Phase 2

* resolver
* type checker
* typed IR
* constants, records, actions
* expressions and control flow

### Phase 3

* interpreter runtime
* extern binding registry
* standard library core
* test runner

### Phase 4

* Rust code generator
* Cargo project emission
* extern mapping in compiler mode
* conformance test parity

### Phase 5

* richer enums
* optional pattern matching
* optimization passes
* formatter and linter

---

## 23. Implementation contract for the Rust coding agent

This section turns the specification into an execution contract for an AI coding agent or human implementation team.

The purpose is to remove ambiguity at the first implementation milestone and to force interpreter and compiler parity from the beginning.

## 23.1 Mandatory architectural rule

The implementation must be split into three logical layers:

1. **Language front-end**

   * lexer
   * parser
   * AST
   * diagnostics
   * formatter or canonical pretty-printer

2. **Semantic core**

   * resolver
   * type checker
   * typed IR (TIR)
   * validation passes
   * shared semantic test suite

3. **Execution backends**

   * interpreter backend
   * Rust code generation backend

No backend may define language semantics independently of the semantic core.

If interpreter and compiler differ in behavior, the semantic core or backend mapping is wrong.

## 23.2 Frozen Milestone 1 subset

The first implementation milestone must support only the following subset. This subset is deliberately frozen and should not be expanded until parity is achieved.

### Top-level declarations

* `module`
* `import`
* `const`
* `record`
* `extern action`
* `action`
* `test`

### Types

* `Bool`
* `Int`
* `Dec`
* `Text`
* `Bytes`
* `None`
* `List[T]`
* `Map[K, V]`
* `Option[T]`
* `Result[T, E]`
* nominal `record` types

### Statements

* `let`
* `var`
* `:=`
* `if / elif / else`
* `while`
* `for each`
* `return`
* `break`
* `continue`
* `expect`
* expression statement

### Expressions

* literals
* variable reference
* field access
* indexing
* action call
* record constructor call
* unary operators
* binary operators
* list literals
* map literals
* grouped expressions

### Not included in Milestone 1

* enums
* tuples
* action values as first-class runtime values
* pattern matching
* generics beyond standard container forms
* purity analysis
* optimization passes other than trivial constant folding if convenient
* brace syntax source form
* macros
* closures
* exceptions
* async semantics

The coding agent must treat all excluded items as out of scope unless explicitly re-enabled in a later milestone.

## 23.3 Canonical source formatting contract

The implementation must include a formatter or a canonical pretty-printer early.

Formatting rules:

* indentation uses two spaces
* tabs are rejected in canonical source mode
* one statement per line
* spaces around binary operators
* no trailing whitespace
* double quotes for strings only
* imports grouped at top after optional module declaration
* blank lines between top-level declaration groups

Every parser round-trip test should aim for:

1. parse source
2. pretty-print canonical form
3. parse canonical form again
4. verify AST equivalence

This is important because AI-generated code must converge to one stable readable form.

## 23.4 AST contract

The AST must remain close to surface syntax. It is the parsed representation, not the execution representation.

A recommended Rust model follows.

### File and declarations

```text
File
  module: Option<ModuleDecl>
  imports: Vec<ImportDecl>
  items: Vec<TopItem>

TopItem
  Const(ConstDecl)
  Record(RecordDecl)
  Extern(ExternDecl)
  Action(ActionDecl)
  Test(TestDecl)
```

### Declarations

```text
ModuleDecl
  name: ModulePath

ImportDecl
  kind: ImportKind

ConstDecl
  name: Ident
  ty: TypeRef
  value: Expr

RecordDecl
  name: Ident
  fields: Vec<RecordField>

ExternDecl
  name: Ident
  params: Vec<Param>
  ret_ty: TypeRef
  purity: Option<Purity>

ActionDecl
  name: Ident
  params: Vec<Param>
  ret_ty: TypeRef
  body: Block

TestDecl
  name: Ident
  body: Block
```

### Statements

```text
Stmt
  Let(LetStmt)
  Var(VarStmt)
  Assign(AssignStmt)
  If(IfStmt)
  While(WhileStmt)
  ForEach(ForEachStmt)
  Return(ReturnStmt)
  Break(Span)
  Continue(Span)
  Expect(ExpectStmt)
  Expr(ExprStmt)
```

### Expressions

```text
Expr
  Literal(LiteralExpr)
  Name(NameExpr)
  Field(FieldExpr)
  Index(IndexExpr)
  Call(CallExpr)
  Unary(UnaryExpr)
  Binary(BinaryExpr)
  List(ListExpr)
  Map(MapExpr)
  Group(GroupExpr)
```

### Required AST metadata

Every AST node must carry:

* stable source span
* optional node id assigned after parsing

The coding agent should not skip spans. They are essential for diagnostics and later source mapping.

## 23.5 Typed IR contract

The typed IR is the real semantic core. It should be explicit, normalized, and easier to execute and emit than the surface AST.

The TIR must:

* resolve names to symbols
* resolve every expression type
* distinguish l-values from r-values
* normalize control structures
* represent extern calls explicitly
* make mutable targets explicit

### TIR design principles

1. No surface-level ambiguity remains.
2. Every expression has a single computed type.
3. Calls point to resolved targets.
4. Container and record operations are explicit.
5. Backend code should not perform additional semantic inference.

### Recommended TIR skeleton

```text
Program
  modules: Vec<Module>

Module
  name: ModulePath
  imports: Vec<ResolvedImport>
  consts: Vec<TConst>
  records: Vec<TRecord>
  externs: Vec<TExtern>
  actions: Vec<TAction>
  tests: Vec<TTest>

TAction
  symbol: ActionSymbol
  params: Vec<TParam>
  ret_ty: TypeId
  locals: Vec<TLocal>
  body: TBlock

TStmt
  Let { local: LocalId, init: TValueExpr }
  Assign { target: TPlaceExpr, value: TValueExpr }
  If { cond: TValueExpr, then_block: TBlock, else_block: Option<TBlock> }
  While { cond: TValueExpr, body: TBlock }
  ForEach { item_local: LocalId, iterable: TValueExpr, body: TBlock }
  Return { value: Option<TValueExpr> }
  Break
  Continue
  Expect { value: TValueExpr }
  Expr { value: TValueExpr }

TValueExpr
  Literal
  LocalRef
  ConstRef
  FieldRead
  IndexRead
  Call
  ExternCall
  RecordConstruct
  ListConstruct
  MapConstruct
  Unary
  Binary
  Cast
```

### Places versus values

The TIR must distinguish a readable place from a writable place.

Examples of writable places:

* local variable declared with `var`
* record field rooted in a `var` binding
* list element rooted in a `var` binding
* map entry rooted in a `var` binding if mutable indexing is supported in the first runtime

A dedicated `TPlaceExpr` is strongly recommended.

## 23.6 Symbol resolution contract

The resolver must assign stable symbols for:

* modules
* imports
* constants
* records
* fields
* extern actions
* actions
* parameters
* locals
* tests

The type checker and both backends must use symbols, not raw names.

No backend should perform string-based late lookup for language-level names.

## 23.7 Type-checking contract

The type checker must enforce at least the following.

### Required checks

* variable must be declared before use
* assignment target must be rooted in a `var` binding and therefore be writable
* assignment value must be assignable to target type
* `if`, `elif`, `while`, and `expect` conditions must be `Bool`
* `return` value must match declared action return type
* bare `return` only allowed for `None`
* call arity must match
* named arguments must match declared names
* no duplicate named arguments
* positional arguments cannot follow named arguments
* field access only valid on record values
* indexing only valid on indexable types
* list element types must unify
* map key types and value types must unify
* extern declarations require fully explicit parameter and return types

### Inference boundaries

Local inference is allowed only where deterministic and simple.

If an expression such as `[]` or `{}` is ambiguous, explicit annotation is required.

### Numeric rules

* `Int + Int -> Int`
* `Dec + Dec -> Dec`
* `Int + Dec -> Dec`
* `Dec + Int -> Dec`
* `%` on `Dec` is not required in Milestone 1 unless explicitly implemented
* implicit narrowing is forbidden

## 23.8 Interpreter runtime contract

The interpreter runtime should be correct before it is fast.

### Runtime obligations

* preserve source semantics faithfully
* use explicit runtime values
* support external bindings through a registry
* provide deterministic test execution
* provide source-based stack traces or execution traces where possible

### Recommended runtime modules

* `value`
* `env`
* `heap`
* `call`
* `extern_registry`
* `standard_runtime`
* `repl_session`
* `iter`
* `test_runner`
* `diagnostics`

### Runtime value categories for Milestone 1

* Bool
* Int
* Dec
* Text
* Bytes
* None
* List
* Map
* RecordInstance
* Option
* Result

The implementation agent may initially represent `Option[T]` and `Result[T, E]` using tagged runtime enums.

In v0.4, `Result` is not only theoretical: it is part of the implemented runtime surface for standard console and file actions.

### Mutation behavior

Interpreter and compiler semantics must preserve the v0.4 mutability contract:

* `let` introduces immutable bindings
* `var` introduces mutable bindings
* reassignment and in-place update use `:=`
* compound values visible through `let` are not writable
* compound values reachable through `var` may be updated through rooted writable places
* immutable bindings must not observe hidden mutation through aliasing

## 23.9 Compiler backend contract

The compiler backend must emit Rust source from TIR, not from the raw AST.

### Output principles

* generated Rust must be valid, readable, and compilable
* generated identifiers should be stable and deterministic
* direct Rust structs should be emitted for records
* direct Rust functions should be emitted for actions
* extern actions should become direct calls to configured Rust paths
* the generated program must not depend on the interpreter runtime crate

### Allowed generated support

The compiler may emit minimal support code for:

* Vulgata `Option` and `Result` mapping where needed
* collection helpers
* source-location comments or metadata
* wrapper conversions for extern boundaries

### Not allowed

* embedding a general-purpose Vulgata VM
* interpreting TIR at runtime inside the compiled program
* requiring dynamic symbol resolution for ordinary action calls

### Compiler output modes

The implementation should support at least one of these by Milestone 2:

1. emit a Cargo project
2. emit a Rust module plus manifest template

The default should be a complete Cargo project for simplicity.

## 23.10 Extern binding contract

Extern behavior must be identical in principle across interpreter and compiler modes.

### Source form

```text
extern action read_file(path: Text) -> Result[Text, Text]
```

### Interpreter mode

* an extern registry maps the symbol to a Rust host function
* host function signature compatibility is validated during startup or module load
* conversion failures are setup errors, not silent coercions

### Compiler mode

* configuration maps the extern symbol to a Rust path
* the code generator emits a direct call or a generated wrapper call
* no runtime lookup table should be required for ordinary compiled execution

### Binding validation

The implementation must validate:

* arity
* argument names if used in configuration
* parameter types
* return type

A mismatch must fail fast.

## 23.11 Diagnostics contract

Diagnostics quality is part of the implementation, not an optional afterthought.

Every error should include:

* source file
* line and column span
* phase (`parse`, `resolve`, `typecheck`, `interpret`, `compile`)
* a precise message
* where relevant, expected and actual types

For generated Rust compilation errors, the compiler should retain enough mapping information to point back to original Vulgata constructs.

## 23.12 Conformance suite contract

The coding agent must build a conformance suite alongside the implementation.

The conformance suite should be shared by both backends.

### Test categories

1. lexer tests
2. parser tests
3. pretty-print round-trip tests
4. resolver tests
5. type-check tests
6. interpreter execution tests
7. compiler execution tests
8. interpreter/compiler equivalence tests
9. extern binding tests
10. diagnostics snapshot tests

### Minimum semantic example set

The suite must include at least:

* integer arithmetic
* decimal arithmetic
* string concatenation
* comparison and boolean short-circuiting
* list iteration
* map construction and lookup
* record construction and field mutation
* nested conditionals
* loops with break and continue
* simple extern call returning scalar
* extern call returning `Result`
* import resolution
* failing type checks
* failing expectations in tests

### Equivalence harness

For every executable sample in the conformance suite:

1. run through interpreter
2. compile to Rust executable
3. run compiled executable
4. compare output, exit status, and test results

If they differ, the feature is not complete.

## 23.13 Milestone plan for implementation agent

### Milestone 1 — front-end and semantic core

Required output:

* lexer
* parser
* AST
* formatter or pretty-printer
* resolver
* type checker
* typed IR
* parser/type-check test suite

Acceptance condition:

* all syntax in frozen subset parses
* canonical formatting round-trip works
* type diagnostics are stable enough for snapshots

### Milestone 2 — interpreter

Required output:

* runtime value model
* execution engine for frozen subset
* extern registry
* test runner
* conformance execution suite in interpreter mode

Acceptance condition:

* all milestone conformance programs pass under interpreter mode

### Milestone 3 — compiler to Rust

Required output:

* Rust code generator from TIR
* Cargo project emitter
* extern Rust path mapping
* generated record and action code
* compiled execution harness

Acceptance condition:

* all milestone conformance programs pass in compiled mode
* no generated executable depends on interpreter runtime crate

### Milestone 4 — parity hardening

Required output:

* equivalence harness
* diagnostics improvement
* source mapping for compiler failures
* regression corpus

Acceptance condition:

* interpreter and compiler outputs are equivalent across the conformance corpus

## 23.14 Final stance

Vulgata is only useful if it remains disciplined.

The implementation agent must resist feature expansion during the first working release. The first real success is not language breadth. It is this:

* a small canonical syntax
* a correct shared semantic core
* a working interpreter
* a working Rust compiler backend
* explicit external calls
* deterministic readable output
* demonstrated parity through tests

That is the threshold at which Vulgata stops being an idea and becomes a real executable language.

## 24. Final design stance

Vulgata should remain deliberately constrained.

It should not drift toward a "nicer Python" or a "tiny general-purpose language with everything eventually added." Its value lies in being compact, formal, auditable, and executable across both an interpreter and a compiler.

The decisive architectural choice is this:

* one language front-end
* one shared semantic core
* two execution backends
* no VM dependency in compiled output
* explicit extern binding model
* explicit type model

That is the correct foundation for a real implementation in Rust.
