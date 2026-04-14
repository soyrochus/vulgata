# Vulgata Design Specification version 0.6

This specification is split into a minimal concern-oriented set of subdocuments rather than a single monolithic reference.

Subdivision heuristic:

* `01_vulgata_spec_intro.md` covers purpose, goals, and language overview.
* `02_vulgata_spec_language_reference.md` covers the surface language: source files, lexical rules, types, declarations, statements, expressions, and grammar.
* `03_vulgata_spec_execution_model.md` covers execution architecture, semantic layers, runtime behavior, externs, standard library, errors, mutability, and call semantics.
* `04_vulgata_spec_implementation_contract.md` covers the implementation-facing contract for the Rust reference implementation.

The files preserve the original section numbering so citations remain stable.

## 1. Purpose

Vulgata is a compact, human-readable, executable language designed as a lingua franca for humans and AI systems collaborating on software design, algorithm specification, workflow definition, and lightweight application logic.

It is not intended to compete with Python, Rust, or Java as a general-purpose systems language. Its design target is different:

* readable enough for non-specialist technical users to verify
* regular enough for AI systems to generate reliably
* formal enough to parse, analyze, and execute unambiguously
* expressive enough to describe real algorithms and structured software behavior
* restricted enough to remain compact and auditable

Vulgata supports two long-term execution backends:

1. **Interpreter mode**: source is parsed, analyzed, and executed in a managed runtime environment.
2. **Compiler mode**: source is translated to Rust and then compiled to native code with no dependency on the interpreter runtime beyond target-local emitted support code.

The canonical source language is shared across backends. Divergence in meaning is a bug.

Version 0.5 keeps the v0.4 mutability model and the implemented `console`, `file`, and `repl` surfaces, and adds semantic layers:

* a **descriptive layer** for `intent:`, `meaning:`, `explain:`, and `step`
* a **checkable layer** for `requires`, `ensures`, `example`, and mode-governed `expect`
* four execution modes: `release`, `checked`, `debug`, and `tooling`
* optional JSON metadata export through `--emit-metadata`

Version 0.5 also includes a first phase of richer value handling and structural control flow:

* statement-form `match`
* phase-1 match patterns: wildcard, literals, bindings, tuple patterns, nominal record patterns, and enum-style variant patterns
* tuple and nominal-record destructuring in `let` and `var`
* built-in `Result[T, E]` operations: `is_ok()`, `is_err()`, `value()`, `error()`
* built-in `Option[T]` operations: `is_some()`, `is_none()`, `value()`

The semantic-layer additions are specified as source-language features even where backend parity is still being hardened in the reference implementation.

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
* semantic room for intent, contracts, and documentation
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
3. **Minimal grammar, rich semantic core.** Keep syntax small; place richness in typed IR, execution modes, and metadata.
4. **Same source, stable meaning.** Interpreter and compiler should converge on the same semantics; gaps in parity are implementation debt, not a license for divergence.
5. **External integration is first-class.** Calls to configured external functions must be easy.
6. **Generated code must be auditable.** Canonical formatting and predictable lowering are mandatory.
7. **Descriptive information must not silently change execution.** Intent, explanation, and meaning are first-class, but they are not executable semantics.

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
* executable statements
* checkable statements
* descriptive statements
* expressions

The language is primarily statement-oriented, with expressions used where values are required.

The language uses indentation as the canonical block structure. A brace form may exist as a transport or serialization variant, but it is not the canonical source form.

Vulgata v0.5 distinguishes three semantic layers:

* **Executable**: ordinary computation and side-effecting action calls
* **Checkable**: runtime-validated assertions, contracts, and examples
* **Descriptive**: intent, meaning, explanation, and traceable step structure

These layers share one source language, but they do not share one execution policy. The execution mode determines whether a construct runs, is enforced, or is treated as metadata only.


# Vulgata Design Specification version 0.5

This document covers sections 5-10 and 15 of the split specification: the surface language and grammar reference.

## 5. Source file and module model

### 5.1 Module structure

Each source file defines one module.

Optional header:

```text
module sales.invoice
```

If omitted, the module name may be derived from the file path by the hosting toolchain.

### 5.2 Imports

```text
import math
import sales.tax
import text.format as fmt
from net.http import get, post
```

Imports are lexical and semantic only. Import resolution must not trigger runtime side effects.

### 5.3 Visibility

For v0.5, top-level declarations are module-public by default. A later version may add explicit visibility control such as `private`.

## 6. Lexical rules

### 6.1 Character set

Source is UTF-8.

### 6.2 Identifiers

Identifiers:

* start with a letter or `_`
* continue with letters, digits, and `_`
* are case-sensitive

Convention:

* module names: dotted lowercase
* actions: snake_case
* records, enums, and type names: PascalCase
* fields and variables: snake_case
* constants: `SCREAMING_SNAKE_CASE` by style, not by grammar

### 6.3 Reserved keywords

Core reserved keywords include:

* `module`, `import`, `from`, `as`
* `const`, `record`, `enum`, `extern`, `pure`, `impure`
* `action`, `test`
* `let`, `var`, `if`, `elif`, `else`, `while`, `for`, `each`, `in`
* `return`, `break`, `continue`, `expect`
* `and`, `or`, `not`, `true`, `false`, `none`

Semantic-layer reserved keywords added in v0.5:

* `intent`, `meaning`, `explain`, `step`
* `requires`, `ensures`, `example`
* `goal`, `constraints`, `assumptions`, `properties`
* `input`, `output`

### 6.4 Comments

```text
# single line comment
```

Block comments are omitted to keep lexing simple.

### 6.5 Literals

Supported literal classes:

* integer
* decimal
* string
* boolean
* `none`
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

## 7. Type system

### 7.1 General approach

Vulgata uses a gradually explicit static type system.

That means:

* all declarations and expressions have types
* type inference exists locally where obvious
* action signatures, extern signatures, record fields, enum payloads, and public constants should generally be explicit
* interpreter and compiler share the same type rules

### 7.2 Built-in primitive types

Primitive built-ins:

* `Bool`
* `Int`
* `Dec`
* `Text`
* `Bytes`
* `None`

The surface alias `Number` is intentionally not used in the core type system.

### 7.3 Composite types

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

### 7.4 Action types

Action types are first-class in the type system, though closures remain deferred.

Syntax:

```text
Action[Int, Int -> Int]
Action[Text -> Bool]
Action[-> None]
```

### 7.5 Type inference

Allowed local inference:

```text
let x = 1
let ok = true
let names = ["a"]
```

Required explicit types in ambiguous situations:

```text
let empty_names: List[Text] = []
let index: Int = 0
```

### 7.6 Structural versus nominal typing

* records are nominally typed
* enums are nominally typed
* tuples, lists, maps, and sets are structural by shape and parameter types

### 7.7 Assignability rules

General rules:

* exact type matches assign
* `T` may assign to `Option[T]`
* `none` may assign only to `None` or `Option[T]`
* `Int -> Dec` widening is allowed only where numeric rules permit
* implicit `Dec -> Int` narrowing is forbidden
* implicit `Text <-> Bytes` conversion is forbidden
* truthiness conversions do not exist

### 7.8 Equality rules

`==` and `!=` are defined only for types with equality semantics.

Actions are not equatable.

Maps and lists are equatable structurally only when their component types are equatable.

## 8. Declaration forms

### 8.1 Constants

```text
const DEFAULT_PORT: Int = 8080
const APP_NAME: Text = "vulgata-demo"
```

Constants are immutable.

### 8.2 Records

```text
record Customer:
  name: Text
  email: Text
    meaning: "Primary contact address"
  active: Bool
```

Records are product types with named fields.

Construction:

```text
let c = Customer(name: "Ana", email: "a@example.com", active: true)
```

`meaning:` is descriptive metadata attached to the immediately preceding field. It is not executable and has no runtime effect.

### 8.3 Enums

```text
enum OrderStatus:
  Pending
  Paid
  Shipped
  Cancelled(reason: Text)
```

Enums participate in phase-1 pattern matching through statement-form `match`. Empty variants are matched as `Pending()`-style patterns, and payload variants may bind nested patterns such as `Cancelled(reason)`.

### 8.4 Actions

```text
action gcd(a: Int, b: Int) -> Int:
  requires a >= 0
  requires b >= 0
  intent:
    goal: "Return the greatest common divisor"
  var x = a
  var y = b
  while y != 0:
    let r = x % y
    x := y
    y := r
  ensures result >= 0
  return x
```

An action body may contain executable statements, checkable statements, and descriptive statements.

### 8.5 Extern declarations

Externs allow the source program to call functions not implemented in Vulgata.

```text
extern action read_file(path: Text) -> Result[Text, Text]
extern pure action sha256(data: Bytes) -> Bytes
extern impure action write_log(level: Text, message: Text) -> None
```

Extern declarations are source-level declarations only. Binding is provided by configuration in interpreter mode or code generation mapping in compiler mode.

### 8.6 Tests

```text
test gcd_basic:
  expect gcd(84, 30) == 6
```

Tests are top-level executable blocks evaluated by the test runner.

## 9. Statement model

Statements in v0.5:

* `let`
* `var`
* assignment
* conditional
* `while`
* `for each`
* `match`
* `return`
* `break`
* `continue`
* `expect`
* `intent:`
* `explain:`
* `step <name>:`
* `requires`
* `ensures`
* `example <name>:`
* expression statement

### 9.1 Variable declaration

```text
let total: Int = 0
let active = true
var count: Int = 0
```

`let` introduces an immutable binding.  
`var` introduces a mutable binding.

Tuple and nominal-record destructuring are also supported in declarations:

```text
let (left, right) = pair
var Customer(name: current_name, active: current_active) = customer
```

Phase-1 declaration destructuring is intentionally narrower than `match` patterns:

* only tuple and nominal record forms are allowed
* destructured outputs must be plain identifiers
* wildcard, nested, and enum-style destructuring are rejected in `let` and `var`
* destructuring in `:=` is rejected

### 9.2 Assignment

```text
count := count + 1
customer.email := "new@example.com"
items[0] := 99
```

Mutation and reassignment are always explicit via `:=`.

Only a target rooted in a `var` binding is writable.

### 9.3 Conditionals

```text
if amount > 0:
  return true
elif amount == 0:
  return false
else:
  return false
```

Conditions must be `Bool`.

### 9.4 While loop

```text
while i < limit:
  i := i + 1
```

### 9.5 For-each loop

```text
for each item in items:
  process(item)
```

The iterator source must be iterable according to the standard semantics.

Pattern matching:

```text
match result:
  Ok(value):
    return value
  Err(_):
    return 0
```

`match` evaluates the scrutinee once, tests arms in source order, executes the first matching arm, and raises `NonExhaustiveMatch` if no arm matches. Phase-1 patterns include wildcard, literals, bindings, tuple patterns, nominal record patterns, and variant patterns such as `Ok(value)`, `Err(message)`, and `None`.

### 9.6 Return

```text
return value
return
```

Bare `return` is valid only in actions returning `None`.

### 9.7 Break and continue

```text
break
continue
```

Only valid inside loops.

### 9.8 Expect

```text
expect total == 10
```

`expect` belongs to the checkable layer.

* in `checked` and `debug` modes it is enforced
* in `release` mode it is skipped
* in `tooling` mode it is preserved for tooling and metadata, not executed as a check

Inside tests, a failed enforced `expect` marks the test as failed. Inside actions, a failed enforced `expect` is a runtime error.

### 9.9 Intent blocks

```text
intent:
  goal: "Return the normalized customer score"
  constraints:
    - "Must remain deterministic"
  assumptions:
    - "Input record is already validated"
  properties:
    - "No side effects"
```

`intent:` is a descriptive statement allowed only inside action bodies. It accepts the optional fields `goal`, `constraints`, `assumptions`, and `properties`.

### 9.10 Explain blocks

```text
explain:
  "Normalize the raw score first"
  "Then clamp it to the accepted range"
```

`explain:` is descriptive only. It contains one or more indented text literal lines.

### 9.11 Step blocks

```text
step normalize:
  let score = value / 10
  return score
```

`step` is a transparent wrapper around its body. The body executes normally in all modes. In `debug` mode the label may be emitted as a trace before execution.

### 9.12 Requires clauses

```text
requires amount >= 0
```

`requires` belongs to the checkable layer. It must contain a boolean expression and is evaluated before the action body in `checked` and `debug` modes.

### 9.13 Ensures clauses

```text
ensures result >= 0
```

`ensures` belongs to the checkable layer. It must contain a boolean expression. Inside the expression, `result` refers to the action return value.

### 9.14 Example blocks

```text
example gcd_basic:
  input:
    a = 84
    b = 30
  output:
    result = 6
```

`example` belongs to the checkable layer.

Rules:

* the example name must be an identifier
* both `input:` and `output:` sub-blocks are required
* each sub-block contains one or more `<name> = <literal>` bindings
* `output:` currently describes expectations for the action result; `result = ...` is the canonical form

In `checked` and `debug` modes, examples execute as embedded tests. In `release` and `tooling` modes they are skipped.

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

### 10.1 Field access

```text
customer.email
```

Certain receiver types also expose built-in member operations resolved by the semantic core rather than by imported modules:

* `Result[T, E]`: `is_ok()`, `is_err()`, `value()`, `error()`
* `Option[T]`: `is_some()`, `is_none()`, `value()`

Examples:

```text
if result.is_ok():
  return result.value()

if maybe_name.is_none():
  return "anonymous"
```

### 10.2 Indexing

```text
items[0]
map["key"]
```

### 10.3 Calls

```text
gcd(84, 30)
format(text: "Hello, {name}", name: customer.name)
```

Named arguments are strongly recommended for public or generated APIs.

### 10.4 Operators

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

### 10.5 Operator precedence

Highest to lowest:

1. postfix: `.`, `[]`, `()`
2. unary: `-`, `not`
3. multiplicative: `*`, `/`, `%`
4. additive: `+`, `-`
5. comparison
6. `and`
7. `or`

## 15. Full grammar proposal

Below is the expanded grammar for v0.5.

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

field_decl      = Identifier ":" type NEWLINE
                  [ INDENT meaning_decl DEDENT ] ;
meaning_decl    = "meaning" ":" STRING_LIT NEWLINE ;

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

statement       = intent_stmt
                | explain_stmt
                | step_stmt
                | requires_stmt
                | ensures_stmt
                | example_stmt
                | let_stmt
                | var_stmt
                | assign_stmt
                | if_stmt
                | while_stmt
                | for_stmt
                | match_stmt
                | return_stmt
                | break_stmt
                | continue_stmt
                | expect_stmt
                | expr_stmt ;

intent_stmt     = "intent" ":" NEWLINE
                  INDENT { intent_field } DEDENT ;
intent_field    = "goal" ":" STRING_LIT NEWLINE
                | "constraints" ":" NEWLINE INDENT { string_item } DEDENT
                | "assumptions" ":" NEWLINE INDENT { string_item } DEDENT
                | "properties" ":" NEWLINE INDENT { string_item } DEDENT ;

string_item     = "-" STRING_LIT NEWLINE ;

explain_stmt    = "explain" ":" NEWLINE
                  INDENT { STRING_LIT NEWLINE } DEDENT ;

step_stmt       = "step" Identifier ":" NEWLINE
                  INDENT block DEDENT ;

requires_stmt   = "requires" expr NEWLINE ;
ensures_stmt    = "ensures" expr NEWLINE ;

example_stmt    = "example" Identifier ":" NEWLINE
                  INDENT example_input example_output DEDENT ;
example_input   = "input" ":" NEWLINE
                  INDENT { example_binding } DEDENT ;
example_output  = "output" ":" NEWLINE
                  INDENT { example_binding } DEDENT ;
example_binding = Identifier "=" literal NEWLINE ;

match_stmt      = "match" expr ":" NEWLINE
                  INDENT { match_arm } DEDENT ;
match_arm       = pattern ":" NEWLINE
                  INDENT block DEDENT ;

let_stmt        = "let" binding_pattern [ ":" type ] "=" expr NEWLINE ;
var_stmt        = "var" binding_pattern [ ":" type ] "=" expr NEWLINE ;
binding_pattern = Identifier
                | tuple_binding
                | record_binding ;
tuple_binding   = "(" Identifier "," Identifier { "," Identifier } ")" ;
record_binding  = Identifier "(" record_binding_field { "," record_binding_field } ")" ;
record_binding_field = Identifier ":" Identifier ;

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

pattern         = "_"
                | literal
                | Identifier
                | tuple_pattern
                | named_pattern ;
tuple_pattern   = "(" pattern "," pattern { "," pattern } ")" ;
named_pattern   = Identifier "(" [ pattern_items | pattern_fields ] ")" ;
pattern_items   = pattern { "," pattern } ;
pattern_fields  = pattern_field { "," pattern_field } ;
pattern_field   = Identifier ":" pattern ;

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


# Vulgata Design Specification version 0.5

This document covers sections 4, 11-14, and 16 of the split specification: execution architecture, semantic layers, runtime behavior, and call semantics.

## 4. Execution architecture

### 4.1 Front-end pipeline

Both interpreter and compiler share the same front-end:

1. lexical analysis
2. parsing
3. AST creation
4. name resolution
5. type resolution and checking
6. semantic lowering to typed IR
7. validation and optimization passes

After typed IR, the pipeline diverges.

### 4.2 Interpreter pipeline

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
* execution-mode handling

### 4.3 Compiler pipeline

Typed IR is lowered to a Rust code generation IR, then emitted as Rust source, then compiled by Rust tooling.

The compiler output must:

* produce normal Rust source
* avoid dependence on the interpreter runtime
* inline or emit only minimal support structures required by the program
* generate explicit Rust types and functions wherever possible
* map Vulgata semantics predictably to Rust semantics

For the v0.5 reference implementation, semantic-layer constructs are part of the source language and front-end, but compiler backend parity for those constructs may still be staged separately from interpreter support.

### 4.4 Semantic consistency rule

Interpreter and compiler must be tested against a shared conformance suite.

Every language feature should have:

* parser tests
* type-check tests
* interpreter execution tests
* compiler execution tests where backend support exists
* equivalence tests where feasible

### 4.5 CLI and REPL model

The implementation exposes both file-oriented commands and an interactive REPL.

File-oriented commands:

```text
vulgata parse [--emit-metadata <path>] <source-file>
vulgata check [--emit-metadata <path>] <source-file>
vulgata run [--mode <release|checked|debug|tooling>] [--emit-metadata <path>] <source-file>
vulgata test [--mode <release|checked|debug|tooling>] [--emit-metadata <path>] <source-file>
vulgata compile [--emit-metadata <path>] <source-file>
```

Interactive command:

```text
vulgata repl [--mode <release|checked|debug|tooling>]
```

Defaults:

* `vulgata run` defaults to `release`
* `vulgata repl` defaults to `tooling`
* `vulgata test` is normally run in a checking mode; `checked` is the expected default

The REPL is backed by a virtual source file for the current session, accepts both declarations and expressions, and supports session-local bindings through `let`, `var`, and `:=`.

Core REPL commands:

* `:help`
* `:show`
* `:parse`
* `:check`
* `:run`
* `:test`
* `:reset`
* `:quit`

### 4.6 Semantic layers and execution modes

Vulgata v0.5 distinguishes three semantic layers:

* **Executable**: ordinary computation and side-effecting statements
* **Checkable**: `expect`, `requires`, `ensures`, `example`
* **Descriptive**: `intent`, `meaning`, `explain`, `step`

The runtime supports four named execution modes:

* `release`
* `checked`
* `debug`
* `tooling`

Behavior matrix:

| Layer       | release | checked | debug   | tooling |
|-------------|---------|---------|---------|---------|
| Executable  | run     | run     | run     | run     |
| Checkable   | skip    | enforce | enforce | expose  |
| Descriptive | skip    | skip    | skip    | expose  |

Notes:

* `step` executes its body in all modes. In `debug`, its label may be emitted as a trace.
* `intent`, `meaning`, and `explain` never affect runtime results.
* `example` executes as an embedded test only in `checked` and `debug`.
* `expect` is mode-governed in v0.5: enforced in `checked` and `debug`, skipped in `release`, exposed for tooling.

### 4.7 Metadata export

The CLI accepts `--emit-metadata <path>`.

Metadata emission:

* occurs after successful parsing and type checking
* does not require program execution
* does not change normal execution output or exit behavior other than reporting ordinary argument or write errors
* must be deterministic for unchanged input

The top-level JSON shape is:

```json
{
  "module": "<module-name>",
  "actions": [ ... ],
  "records": [ ... ]
}
```

Action entries always include `"name"` and may include:

* `"intent"`
* `"contracts"` with `"requires"` and `"ensures"`
* `"steps"`
* `"examples"`
* `"explain"`

Record entries may include field-level `"meaning"` metadata.

## 11. Foreign function and external integration model

### 11.1 Goal

A Vulgata program must be able to call external functions in both interpreter and compiler modes without changing source semantics.

### 11.2 Extern declaration syntax

```text
extern action now_iso() -> Text
extern action http_get(url: Text, headers: Map[Text, Text]) -> Result[Text, Text]
extern action sha256(data: Bytes) -> Bytes
```

### 11.3 Binding strategy

Bindings are supplied by configuration rather than encoded fully in source.

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

### 11.4 Type contract

Every extern declaration must have a fully explicit signature. The binder must validate compatibility at registration time.

### 11.5 Runtime behavior

Interpreter mode:

* extern actions resolve through a registry
* values are converted between runtime values and native Rust values
* type mismatch is a setup error, not a silent runtime error

Compiler mode:

* extern calls become direct Rust calls to configured functions
* conversion code is emitted only where necessary
* if types map directly, the call should compile to near-zero-overhead wrapper code

### 11.6 Purity and side effects

Optional metadata may classify externs:

```text
extern pure action sha256(data: Bytes) -> Bytes
extern impure action write_log(level: Text, message: Text) -> None
```

Purity metadata is advisory for future optimization and analysis.

## 12. Standard library model

The standard library should be small and mostly defined as modules of actions rather than syntax.

Candidate modules:

* `text`
* `math`
* `list`
* `map`
* `set`
* `time`
* `test`
* `console`
* `file`

For v0.5, the initial implemented standard runtime library remains:

* `console`
* `file`

`Result[T, E]` and `Option[T]` inspection/extraction are part of the semantic core, not runtime modules. Their member operations require no import.

Examples:

```text
let printed = console.println("hello")
let line = console.read_line()
let data = file.read_text("notes.txt")
let present = file.exists("config.txt")
```

### 12.1 `console` module

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
* end-of-input is represented as `Err(...)`

### 12.2 `file` module

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

### 12.3 No print statement rule

Vulgata does not introduce a dedicated `print` statement. Console and file operations remain ordinary action calls so side effects stay explicit and the language core stays small.

## 13. Error model

### 13.1 Core rule

Do not introduce exceptions as a language-level control-flow system. Use `Result[T, E]` and `Option[T]` explicitly.

### 13.2 Result, Option, and match handling

`Result[T, E]` exposes built-in member operations:

* `is_ok()`
* `is_err()`
* `value()`
* `error()`

`Option[T]` exposes built-in member operations:

* `is_some()`
* `is_none()`
* `value()`

These operations are resolved by the semantic core with no import declaration.

Examples:

```text
if result.is_ok():
  return result.value()

if maybe_name.is_none():
  return "anonymous"
```

Statement-form `match` is also part of the core language and is the canonical branching form for variant-style handling:

```text
match result:
  Ok(value):
    return value
  Err(message):
    return message
```

If no arm matches, execution fails with the named runtime error `NonExhaustiveMatch`.

### 13.3 Interpreter errors

Interpreter failures should report:

* source span
* module
* action
* operation
* diagnostic message

Named runtime errors should exist for checkable-layer failures such as:

* failed `expect`
* failed `requires`
* failed `ensures`
* failed `example`
* `NonExhaustiveMatch`
* `InvalidResultValueAccess`
* `InvalidResultErrorAccess`
* `InvalidOptionValueAccess`

### 13.4 Compiler errors

Compiler diagnostics should reference original Vulgata spans, even when Rust compilation fails in generated code.

### 13.5 Metadata errors

Metadata emission failures are CLI or filesystem errors, not language semantics errors. They must not require program execution to reproduce.

## 14. Mutability and value model

### 14.1 Variables

Variables declared with `let` are immutable bindings. Variables declared with `var` are mutable bindings.

Examples:

```text
let port: Int = 8080
var retries: Int = 0
retries := retries + 1
```

Action parameters are immutable bindings. If local mutation is required, the value should first be copied into a `var`.

### 14.2 Records and collections

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

Descriptive annotations such as `meaning:` do not weaken or alter mutability rules.

### 14.3 Aliasing

The source-language model is value-oriented:

* assigning a compound value to `let` captures an immutable value
* assigning a compound value to `var` creates mutable storage for that variable
* mutating a `var`-rooted value must not implicitly mutate a value visible through an immutable `let` binding

Implementations may use copying, structural sharing, copy-on-write, or other internal strategies as long as those observable semantics are preserved.

## 16. Semantics of calls

### 16.1 Call categories

A call expression may target:

* a declared Vulgata action
* an extern action
* a record constructor
* an enum variant constructor where supported
* a first-class action value where supported

### 16.2 Named and positional arguments

Both are supported.

Rules:

* positional arguments must come first
* named arguments may follow
* once a named argument is used, all following arguments must be named
* no duplicate names
* unknown names are compile-time errors

### 16.3 Overloading

No function overloading is defined in the core language.

### 16.4 Dispatch

Dispatch is lexical and static by symbol identity, not dynamic multimethod dispatch.


# Vulgata Design Specification version 0.5

This document covers sections 17-24 of the split specification: implementation guidance and the execution contract for the Rust reference implementation.

## 17. Interpreter design guidance for Rust agent

The interpreter should be implemented in layered modules.

Suggested structure:

* `lexer`
* `parser`
* `ast`
* `resolver`
* `types`
* `tir`
* `runtime`
* `externs`
* `metadata`
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
* ResultOk / ResultErr
* OptionSome / OptionNone

The interpreter should avoid embedding language semantics directly in parsing. Semantic phases must remain separate.

The interpreter must also carry an explicit `ExecutionMode` value rather than a process-global mode.

## 18. Compiler design guidance for Rust agent

The compiler path should lower typed IR into a Rust generation model.

Suggested phases:

1. typed IR validation
2. monomorphization or generic strategy decision
3. Rust AST or structured Rust emission model
4. support code emission
5. Cargo project or inline module emission
6. rustc or cargo compilation

### Compiler output principle

Generated code should be understandable Rust, not opaque low-level output.

### No runtime dependency principle

Do not make compiled output depend on the interpreter runtime crate.

Allowed:

* small emitted helper modules per compiled target
* generated structs, enums, wrappers
* direct calls to configured Rust functions

Not allowed:

* compiled programs requiring a general-purpose Vulgata VM

### Current v0.5 caveat

The language specification includes semantic-layer constructs, but backend parity for those constructs may be staged after front-end and interpreter support. Until parity lands, rejecting unsupported semantic-layer constructs in code generation is preferable to silently miscompiling them.

## 19. Minimal support code in compiled output

The compiler may emit support code only where necessary for language semantics.

Examples:

* helper type aliases
* generated `Result` conversion helpers
* collection construction helpers
* small source-location utilities for diagnostics
* target-local support for metadata comments where desired

This support code should remain target-local and removable by ordinary Rust optimization.

## 20. Conformance examples

### 20.1 Basic algorithm

```text
action gcd(a: Int, b: Int) -> Int:
  requires a >= 0
  requires b >= 0
  var x: Int = a
  var y: Int = b
  while y != 0:
    step iterate:
      let r: Int = x % y
      x := y
      y := r
  ensures result >= 0
  return x
```

### 20.2 Record and extern use

```text
record Customer:
  name: Text
  email: Text
    meaning: "Primary contact address"

extern action send_email(to: Text, subject: Text, body: Text) -> Result[None, Text]

action welcome(customer: Customer) -> Result[None, Text]:
  intent:
    goal: "Send a welcome email"
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

### 20.5 Semantic-layer action

```text
action normalize(value: Int) -> Int:
  intent:
    goal: "Clamp a value into the accepted range"
  explain:
    "Negative values map to zero"
    "Values above one hundred map to one hundred"
  requires value >= -1000
  example below_zero:
    input:
      value = -1
    output:
      result = 0
  if value < 0:
    return 0
  elif value > 100:
    return 100
  ensures result >= 0
  ensures result <= 100
  return value
```

## 21. Open issues to freeze before implementation

The v0.5 reference spec still leaves a few points intentionally explicit:

1. Exact compiler-backend handling for semantic-layer constructs until parity is complete.
2. Whether tuples and `Set[T]` are part of the near-term executable subset or remain reserved.
3. Exact map key equality and hashing rules.
4. Whether decimal uses `f64` initially or a decimal library.
5. Exact module-to-file mapping rules.
6. Whether metadata JSON is frozen as a public stable tooling contract or remains implementation-internal for one release cycle.
7. Whether `example output:` remains general `<name> = <literal>` syntax or narrows to canonical `result = ...` for single-result actions.

## 22. Recommended implementation phases

### Phase 1

* lexer
* parser
* AST
* diagnostics
* canonical formatting or pretty-printing

### Phase 2

* resolver
* type checker
* typed IR
* constants, records, actions, expressions, and control flow

### Phase 3

* interpreter runtime
* execution modes
* semantic-layer enforcement and skipping
* metadata emitter
* extern binding registry
* standard runtime core
* test runner

### Phase 4

* Rust code generator
* Cargo project emission
* extern mapping in compiler mode
* semantic-layer backend parity
* conformance equivalence hardening

### Phase 5

* richer enums
* broader pattern forms beyond the phase-1 `match` subset
* optimization passes
* formatter and linter hardening

## 23. Implementation contract for the Rust coding agent

This section turns the specification into an execution contract for an AI coding agent or human implementation team.

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
   * typed IR
   * validation passes
   * metadata extraction
   * shared semantic test suite

3. **Execution backends**

   * interpreter backend
   * Rust code generation backend

No backend may define language semantics independently of the semantic core.

## 23.2 Frozen v0.5 surface subset

The reference implementation must support at least the following surface:

### Top-level declarations

* `module`
* `import`
* `const`
* `record`
* `enum`
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
* nominal `record` and `enum` types

### Statements

* `let`
* `var`
* `:=`
* `if / elif / else`
* `while`
* `for each`
* `match`
* `return`
* `break`
* `continue`
* `expect`
* `intent`
* `explain`
* `step`
* `requires`
* `ensures`
* `example`
* expression statement

### Phase-1 binding and pattern features

* tuple destructuring in `let` and `var`
* nominal record destructuring in `let` and `var`
* phase-1 `match` patterns: wildcard, literals, bindings, tuple patterns, nominal record patterns, and enum-style variant patterns
* built-in `Result[T, E]` member operations: `is_ok()`, `is_err()`, `value()`, `error()`
* built-in `Option[T]` member operations: `is_some()`, `is_none()`, `value()`

### Metadata-facing annotations

* record-field `meaning`
* action `intent`
* action `explain`
* action `step`
* `requires` / `ensures`
* `example`

### Not yet guaranteed across every backend

* semantic-layer code-generation parity
* nested destructuring and broader pattern forms beyond the phase-1 subset
* closures
* async semantics
* macros
* user-defined syntax

## 23.3 Canonical source formatting contract

Formatting rules:

* indentation uses two spaces
* tabs are rejected in canonical source mode
* one statement per line
* spaces around binary operators
* no trailing whitespace
* double quotes for strings only
* imports grouped at top after optional module declaration
* blank lines between top-level declaration groups when practical

Round-trip tests should aim for:

1. parse source
2. pretty-print canonical form
3. parse canonical form again
4. verify AST equivalence

## 23.4 AST contract

The AST must remain close to surface syntax.

### File and declarations

```text
File
  module: Option<ModuleDecl>
  imports: Vec<ImportDecl>
  items: Vec<TopItem>

TopItem
  Const(ConstDecl)
  Record(RecordDecl)
  Enum(EnumDecl)
  Extern(ExternDecl)
  Action(ActionDecl)
  Test(TestDecl)
```

### Declarations

```text
RecordDecl
  name: Ident
  fields: Vec<RecordField>

RecordField
  name: Ident
  ty: TypeRef
  meaning: Option<Text>

ActionDecl
  name: Ident
  params: Vec<Param>
  ret_ty: TypeRef
  body: Block
```

### Statements

```text
Stmt
  Intent(IntentStmt)
  Explain(ExplainStmt)
  Step(StepStmt)
  Requires(RequiresStmt)
  Ensures(EnsuresStmt)
  Example(ExampleStmt)
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

Expressions remain surface-oriented: literals, names, field access, indexing, calls, unary, binary, list, map, tuple, grouping.

### Required AST metadata

Every AST node must carry:

* stable source span
* optional node id assigned after parsing

## 23.5 Typed IR contract

The typed IR is the real semantic core.

The TIR must:

* resolve names to symbols
* resolve every expression type
* distinguish l-values from r-values
* normalize control structures
* represent extern calls explicitly
* make mutable targets explicit
* preserve semantic-layer nodes explicitly rather than erasing them too early

### Recommended TIR skeleton

```text
TStmt
  Intent { goal, constraints, assumptions, properties }
  Explain { lines }
  Step { label, body }
  Requires { value: TValueExpr }
  Ensures { value: TValueExpr }
  Example { name, inputs, outputs }
  Let { ... }
  Assign { ... }
  If { ... }
  While { ... }
  ForEach { ... }
  Return { ... }
  Break
  Continue
  Expect { value: TValueExpr }
  Expr { value: TValueExpr }
```

Backends may choose to ignore, enforce, export, or reject these nodes depending on capability, but the semantic core must not lose them.

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
* synthetic checkable-layer bindings such as `result` where required by later semantic phases

## 23.7 Type-checking contract

### Required checks

* variable must be declared before use
* assignment target must be rooted in a `var` binding
* assignment value must be assignable to target type
* `if`, `elif`, `while`, `expect`, `requires`, and `ensures` conditions must be `Bool`
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
* `result` must resolve correctly inside `ensures`
* `example` input and output bindings must type-check against the enclosing action signature

### Inference boundaries

Local inference is allowed only where deterministic and simple.

### Numeric rules

* `Int + Int -> Int`
* `Dec + Dec -> Dec`
* `Int + Dec -> Dec`
* `Dec + Int -> Dec`
* implicit narrowing is forbidden

## 23.8 Interpreter runtime contract

### Runtime obligations

* preserve source semantics faithfully
* use explicit runtime values
* support external bindings through a registry
* provide deterministic test execution
* provide source-based diagnostics
* honor execution modes explicitly
* skip descriptive constructs without side effects
* enforce checkable constructs only in the modes that require enforcement

### Recommended runtime modules

* `value`
* `env`
* `heap`
* `call`
* `extern_registry`
* `standard_runtime`
* `repl_session`
* `metadata`
* `test_runner`
* `diagnostics`

### Runtime value categories

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

### Mutation behavior

Interpreter and compiler semantics must preserve the mutability contract:

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

* `Option` and `Result` mapping where needed
* collection helpers
* source-location comments or metadata
* extern-boundary conversions

### Not allowed

* embedding a general-purpose Vulgata VM
* interpreting TIR at runtime inside the compiled program
* silently dropping semantic-layer constructs

If a semantic-layer construct is unsupported in the current backend stage, compilation should fail explicitly.

## 23.10 Extern binding contract

Extern behavior must be identical in principle across interpreter and compiler modes.

### Source form

```text
extern action read_file(path: Text) -> Result[Text, Text]
```

### Interpreter mode

* an extern registry maps the symbol to a Rust host function
* host function signature compatibility is validated during startup or module load
* conversion failures are setup errors

### Compiler mode

* configuration maps the extern symbol to a Rust path
* the code generator emits a direct call or generated wrapper call
* no runtime lookup table should be required for ordinary compiled execution

## 23.11 Diagnostics contract

Every error should include:

* source file
* line and column span
* phase
* a precise message
* where relevant, expected and actual types

Diagnostics must cover parse, resolve, type-check, runtime, codegen, extern, and CLI failures.

## 23.12 Conformance suite contract

The conformance suite should be shared by both backends where backend support exists.

### Test categories

1. lexer tests
2. parser tests
3. round-trip formatting tests
4. resolver tests
5. type-check tests
6. interpreter execution tests
7. compiler execution tests
8. interpreter/compiler equivalence tests
9. extern binding tests
10. metadata emission tests
11. diagnostics snapshot tests

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
* descriptive-layer no-op behavior
* `requires` and `ensures` enforcement
* `example` success and failure
* deterministic metadata output

## 23.13 Milestone plan for implementation agent

### Milestone 1: front-end and semantic core

Required output:

* lexer
* parser
* AST
* resolver
* type checker
* typed IR
* parser and type-check test suite

### Milestone 2: interpreter

Required output:

* runtime value model
* execution engine
* extern registry
* test runner
* execution-mode support
* semantic-layer runtime handling
* metadata emitter

### Milestone 3: compiler to Rust

Required output:

* Rust code generator from TIR
* Cargo project emitter
* extern Rust path mapping
* generated record and action code
* compiled execution harness

### Milestone 4: parity hardening

Required output:

* equivalence harness
* diagnostics improvement
* source mapping for compiler failures
* semantic-layer backend parity
* regression corpus

## 23.14 Final stance

Vulgata is only useful if it remains disciplined:

* one canonical syntax
* one shared semantic core
* explicit mutation
* explicit type rules
* explicit extern binding
* explicit execution modes
* descriptive constructs that never silently alter computation

## 24. Final design stance

Vulgata should remain deliberately constrained.

Its value lies in being compact, formal, auditable, and executable across both an interpreter and a compiler, while also carrying non-executable intent and contract information for humans and tools.


# Vulgata Design Specification version 0.5

This specification is split into a minimal concern-oriented set of subdocuments rather than a single monolithic reference.

Subdivision heuristic:

* `01_vulgata_spec_intro.md` covers purpose, goals, and language overview.
* `02_vulgata_spec_language_reference.md` covers the surface language: source files, lexical rules, types, declarations, statements, expressions, and grammar.
* `03_vulgata_spec_execution_model.md` covers execution architecture, semantic layers, runtime behavior, externs, standard library, errors, mutability, and call semantics.
* `04_vulgata_spec_implementation_contract.md` covers the implementation-facing contract for the Rust reference implementation.

The files preserve the original section numbering so citations remain stable.

## 1. Purpose

Vulgata is a compact, human-readable, executable language designed as a lingua franca for humans and AI systems collaborating on software design, algorithm specification, workflow definition, and lightweight application logic.

It is not intended to compete with Python, Rust, or Java as a general-purpose systems language. Its design target is different:

* readable enough for non-specialist technical users to verify
* regular enough for AI systems to generate reliably
* formal enough to parse, analyze, and execute unambiguously
* expressive enough to describe real algorithms and structured software behavior
* restricted enough to remain compact and auditable

Vulgata supports two long-term execution backends:

1. **Interpreter mode**: source is parsed, analyzed, and executed in a managed runtime environment.
2. **Compiler mode**: source is translated to Rust and then compiled to native code with no dependency on the interpreter runtime beyond target-local emitted support code.

The canonical source language is shared across backends. Divergence in meaning is a bug.

Version 0.5 keeps the v0.4 mutability model and the implemented `console`, `file`, and `repl` surfaces, and adds semantic layers:

* a **descriptive layer** for `intent:`, `meaning:`, `explain:`, and `step`
* a **checkable layer** for `requires`, `ensures`, `example`, and mode-governed `expect`
* four execution modes: `release`, `checked`, `debug`, and `tooling`
* optional JSON metadata export through `--emit-metadata`

The semantic-layer additions are specified as source-language features even where backend parity is still being hardened in the reference implementation.

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
* semantic room for intent, contracts, and documentation
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
3. **Minimal grammar, rich semantic core.** Keep syntax small; place richness in typed IR, execution modes, and metadata.
4. **Same source, stable meaning.** Interpreter and compiler should converge on the same semantics; gaps in parity are implementation debt, not a license for divergence.
5. **External integration is first-class.** Calls to configured external functions must be easy.
6. **Generated code must be auditable.** Canonical formatting and predictable lowering are mandatory.
7. **Descriptive information must not silently change execution.** Intent, explanation, and meaning are first-class, but they are not executable semantics.

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
* executable statements
* checkable statements
* descriptive statements
* expressions

The language is primarily statement-oriented, with expressions used where values are required.

The language uses indentation as the canonical block structure. A brace form may exist as a transport or serialization variant, but it is not the canonical source form.

Vulgata v0.5 distinguishes three semantic layers:

* **Executable**: ordinary computation and side-effecting action calls
* **Checkable**: runtime-validated assertions, contracts, and examples
* **Descriptive**: intent, meaning, explanation, and traceable step structure

These layers share one source language, but they do not share one execution policy. The execution mode determines whether a construct runs, is enforced, or is treated as metadata only.


# Vulgata Design Specification version 0.5

This document covers sections 5-10 and 15 of the split specification: the surface language and grammar reference.

## 5. Source file and module model

### 5.1 Module structure

Each source file defines one module.

Optional header:

```text
module sales.invoice
```

If omitted, the module name may be derived from the file path by the hosting toolchain.

### 5.2 Imports

```text
import math
import sales.tax
import text.format as fmt
from net.http import get, post
```

Imports are lexical and semantic only. Import resolution must not trigger runtime side effects.

### 5.3 Visibility

For v0.5, top-level declarations are module-public by default. A later version may add explicit visibility control such as `private`.

## 6. Lexical rules

### 6.1 Character set

Source is UTF-8.

### 6.2 Identifiers

Identifiers:

* start with a letter or `_`
* continue with letters, digits, and `_`
* are case-sensitive

Convention:

* module names: dotted lowercase
* actions: snake_case
* records, enums, and type names: PascalCase
* fields and variables: snake_case
* constants: `SCREAMING_SNAKE_CASE` by style, not by grammar

### 6.3 Reserved keywords

Core reserved keywords include:

* `module`, `import`, `from`, `as`
* `const`, `record`, `enum`, `extern`, `pure`, `impure`
* `action`, `test`
* `let`, `var`, `if`, `elif`, `else`, `while`, `for`, `each`, `in`
* `return`, `break`, `continue`, `expect`
* `and`, `or`, `not`, `true`, `false`, `none`

Semantic-layer reserved keywords added in v0.5:

* `intent`, `meaning`, `explain`, `step`
* `requires`, `ensures`, `example`
* `goal`, `constraints`, `assumptions`, `properties`
* `input`, `output`

### 6.4 Comments

```text
# single line comment
```

Block comments are omitted to keep lexing simple.

### 6.5 Literals

Supported literal classes:

* integer
* decimal
* string
* boolean
* `none`
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

## 7. Type system

### 7.1 General approach

Vulgata uses a gradually explicit static type system.

That means:

* all declarations and expressions have types
* type inference exists locally where obvious
* action signatures, extern signatures, record fields, enum payloads, and public constants should generally be explicit
* interpreter and compiler share the same type rules

### 7.2 Built-in primitive types

Primitive built-ins:

* `Bool`
* `Int`
* `Dec`
* `Text`
* `Bytes`
* `None`

The surface alias `Number` is intentionally not used in the core type system.

### 7.3 Composite types

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

### 7.4 Action types

Action types are first-class in the type system, though closures remain deferred.

Syntax:

```text
Action[Int, Int -> Int]
Action[Text -> Bool]
Action[-> None]
```

### 7.5 Type inference

Allowed local inference:

```text
let x = 1
let ok = true
let names = ["a"]
```

Required explicit types in ambiguous situations:

```text
let empty_names: List[Text] = []
let index: Int = 0
```

### 7.6 Structural versus nominal typing

* records are nominally typed
* enums are nominally typed
* tuples, lists, maps, and sets are structural by shape and parameter types

### 7.7 Assignability rules

General rules:

* exact type matches assign
* `T` may assign to `Option[T]`
* `none` may assign only to `None` or `Option[T]`
* `Int -> Dec` widening is allowed only where numeric rules permit
* implicit `Dec -> Int` narrowing is forbidden
* implicit `Text <-> Bytes` conversion is forbidden
* truthiness conversions do not exist

### 7.8 Equality rules

`==` and `!=` are defined only for types with equality semantics.

Actions are not equatable.

Maps and lists are equatable structurally only when their component types are equatable.

## 8. Declaration forms

### 8.1 Constants

```text
const DEFAULT_PORT: Int = 8080
const APP_NAME: Text = "vulgata-demo"
```

Constants are immutable.

### 8.2 Records

```text
record Customer:
  name: Text
  email: Text
    meaning: "Primary contact address"
  active: Bool
```

Records are product types with named fields.

Construction:

```text
let c = Customer(name: "Ana", email: "a@example.com", active: true)
```

`meaning:` is descriptive metadata attached to the immediately preceding field. It is not executable and has no runtime effect.

### 8.3 Enums

```text
enum OrderStatus:
  Pending
  Paid
  Shipped
  Cancelled(reason: Text)
```

Enums participate in phase-1 pattern matching through statement-form `match`. Empty variants are matched as `Pending()`-style patterns, and payload variants may bind nested patterns such as `Cancelled(reason)`.

### 8.4 Actions

```text
action gcd(a: Int, b: Int) -> Int:
  requires a >= 0
  requires b >= 0
  intent:
    goal: "Return the greatest common divisor"
  var x = a
  var y = b
  while y != 0:
    let r = x % y
    x := y
    y := r
  ensures result >= 0
  return x
```

An action body may contain executable statements, checkable statements, and descriptive statements.

### 8.5 Extern declarations

Externs allow the source program to call functions not implemented in Vulgata.

```text
extern action read_file(path: Text) -> Result[Text, Text]
extern pure action sha256(data: Bytes) -> Bytes
extern impure action write_log(level: Text, message: Text) -> None
```

Extern declarations are source-level declarations only. Binding is provided by configuration in interpreter mode or code generation mapping in compiler mode.

### 8.6 Tests

```text
test gcd_basic:
  expect gcd(84, 30) == 6
```

Tests are top-level executable blocks evaluated by the test runner.

## 9. Statement model

Statements in v0.5:

* `let`
* `var`
* assignment
* conditional
* `while`
* `for each`
* `match`
* `return`
* `break`
* `continue`
* `expect`
* `intent:`
* `explain:`
* `step <name>:`
* `requires`
* `ensures`
* `example <name>:`
* expression statement

### 9.1 Variable declaration

```text
let total: Int = 0
let active = true
var count: Int = 0
```

`let` introduces an immutable binding.  
`var` introduces a mutable binding.

Tuple and nominal-record destructuring are also supported in declarations:

```text
let (left, right) = pair
var Customer(name: current_name, active: current_active) = customer
```

Phase-1 declaration destructuring is intentionally narrower than `match` patterns:

* only tuple and nominal record forms are allowed
* destructured outputs must be plain identifiers
* wildcard, nested, and enum-style destructuring are rejected in `let` and `var`
* destructuring in `:=` is rejected

### 9.2 Assignment

```text
count := count + 1
customer.email := "new@example.com"
items[0] := 99
```

Mutation and reassignment are always explicit via `:=`.

Only a target rooted in a `var` binding is writable.

### 9.3 Conditionals

```text
if amount > 0:
  return true
elif amount == 0:
  return false
else:
  return false
```

Conditions must be `Bool`.

### 9.4 While loop

```text
while i < limit:
  i := i + 1
```

### 9.5 For-each loop

```text
for each item in items:
  process(item)
```

The iterator source must be iterable according to the standard semantics.

Pattern matching:

```text
match result:
  Ok(value):
    return value
  Err(_):
    return 0
```

`match` evaluates the scrutinee once, tests arms in source order, executes the first matching arm, and raises `NonExhaustiveMatch` if no arm matches. Phase-1 patterns include wildcard, literals, bindings, tuple patterns, nominal record patterns, and variant patterns such as `Ok(value)`, `Err(message)`, and `None`.

### 9.6 Return

```text
return value
return
```

Bare `return` is valid only in actions returning `None`.

### 9.7 Break and continue

```text
break
continue
```

Only valid inside loops.

### 9.8 Expect

```text
expect total == 10
```

`expect` belongs to the checkable layer.

* in `checked` and `debug` modes it is enforced
* in `release` mode it is skipped
* in `tooling` mode it is preserved for tooling and metadata, not executed as a check

Inside tests, a failed enforced `expect` marks the test as failed. Inside actions, a failed enforced `expect` is a runtime error.

### 9.9 Intent blocks

```text
intent:
  goal: "Return the normalized customer score"
  constraints:
    - "Must remain deterministic"
  assumptions:
    - "Input record is already validated"
  properties:
    - "No side effects"
```

`intent:` is a descriptive statement allowed only inside action bodies. It accepts the optional fields `goal`, `constraints`, `assumptions`, and `properties`.

### 9.10 Explain blocks

```text
explain:
  "Normalize the raw score first"
  "Then clamp it to the accepted range"
```

`explain:` is descriptive only. It contains one or more indented text literal lines.

### 9.11 Step blocks

```text
step normalize:
  let score = value / 10
  return score
```

`step` is a transparent wrapper around its body. The body executes normally in all modes. In `debug` mode the label may be emitted as a trace before execution.

### 9.12 Requires clauses

```text
requires amount >= 0
```

`requires` belongs to the checkable layer. It must contain a boolean expression and is evaluated before the action body in `checked` and `debug` modes.

### 9.13 Ensures clauses

```text
ensures result >= 0
```

`ensures` belongs to the checkable layer. It must contain a boolean expression. Inside the expression, `result` refers to the action return value.

### 9.14 Example blocks

```text
example gcd_basic:
  input:
    a = 84
    b = 30
  output:
    result = 6
```

`example` belongs to the checkable layer.

Rules:

* the example name must be an identifier
* both `input:` and `output:` sub-blocks are required
* each sub-block contains one or more `<name> = <literal>` bindings
* `output:` currently describes expectations for the action result; `result = ...` is the canonical form

In `checked` and `debug` modes, examples execute as embedded tests. In `release` and `tooling` modes they are skipped.

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

### 10.1 Field access

```text
customer.email
```

Certain receiver types also expose built-in member operations resolved by the semantic core rather than by imported modules:

* `Result[T, E]`: `is_ok()`, `is_err()`, `value()`, `error()`
* `Option[T]`: `is_some()`, `is_none()`, `value()`

Examples:

```text
if result.is_ok():
  return result.value()

if maybe_name.is_none():
  return "anonymous"
```

### 10.2 Indexing

```text
items[0]
map["key"]
```

### 10.3 Calls

```text
gcd(84, 30)
format(text: "Hello, {name}", name: customer.name)
```

Named arguments are strongly recommended for public or generated APIs.

### 10.4 Operators

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

### 10.5 Operator precedence

Highest to lowest:

1. postfix: `.`, `[]`, `()`
2. unary: `-`, `not`
3. multiplicative: `*`, `/`, `%`
4. additive: `+`, `-`
5. comparison
6. `and`
7. `or`

## 15. Full grammar proposal

Below is the expanded grammar for v0.5.

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

field_decl      = Identifier ":" type NEWLINE
                  [ INDENT meaning_decl DEDENT ] ;
meaning_decl    = "meaning" ":" STRING_LIT NEWLINE ;

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

statement       = intent_stmt
                | explain_stmt
                | step_stmt
                | requires_stmt
                | ensures_stmt
                | example_stmt
                | let_stmt
                | var_stmt
                | assign_stmt
                | if_stmt
                | while_stmt
                | for_stmt
                | match_stmt
                | return_stmt
                | break_stmt
                | continue_stmt
                | expect_stmt
                | expr_stmt ;

intent_stmt     = "intent" ":" NEWLINE
                  INDENT { intent_field } DEDENT ;
intent_field    = "goal" ":" STRING_LIT NEWLINE
                | "constraints" ":" NEWLINE INDENT { string_item } DEDENT
                | "assumptions" ":" NEWLINE INDENT { string_item } DEDENT
                | "properties" ":" NEWLINE INDENT { string_item } DEDENT ;

string_item     = "-" STRING_LIT NEWLINE ;

explain_stmt    = "explain" ":" NEWLINE
                  INDENT { STRING_LIT NEWLINE } DEDENT ;

step_stmt       = "step" Identifier ":" NEWLINE
                  INDENT block DEDENT ;

requires_stmt   = "requires" expr NEWLINE ;
ensures_stmt    = "ensures" expr NEWLINE ;

example_stmt    = "example" Identifier ":" NEWLINE
                  INDENT example_input example_output DEDENT ;
example_input   = "input" ":" NEWLINE
                  INDENT { example_binding } DEDENT ;
example_output  = "output" ":" NEWLINE
                  INDENT { example_binding } DEDENT ;
example_binding = Identifier "=" literal NEWLINE ;

match_stmt      = "match" expr ":" NEWLINE
                  INDENT { match_arm } DEDENT ;
match_arm       = pattern ":" NEWLINE
                  INDENT block DEDENT ;

let_stmt        = "let" binding_pattern [ ":" type ] "=" expr NEWLINE ;
var_stmt        = "var" binding_pattern [ ":" type ] "=" expr NEWLINE ;
binding_pattern = Identifier
                | tuple_binding
                | record_binding ;
tuple_binding   = "(" Identifier "," Identifier { "," Identifier } ")" ;
record_binding  = Identifier "(" record_binding_field { "," record_binding_field } ")" ;
record_binding_field = Identifier ":" Identifier ;

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

pattern         = "_"
                | literal
                | Identifier
                | tuple_pattern
                | named_pattern ;
tuple_pattern   = "(" pattern "," pattern { "," pattern } ")" ;
named_pattern   = Identifier "(" [ pattern_items | pattern_fields ] ")" ;
pattern_items   = pattern { "," pattern } ;
pattern_fields  = pattern_field { "," pattern_field } ;
pattern_field   = Identifier ":" pattern ;

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


# Vulgata Design Specification version 0.5

This document covers sections 4, 11-14, and 16 of the split specification: execution architecture, semantic layers, runtime behavior, and call semantics.

## 4. Execution architecture

### 4.1 Front-end pipeline

Both interpreter and compiler share the same front-end:

1. lexical analysis
2. parsing
3. AST creation
4. name resolution
5. type resolution and checking
6. semantic lowering to typed IR
7. validation and optimization passes

After typed IR, the pipeline diverges.

### 4.2 Interpreter pipeline

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
* execution-mode handling

### 4.3 Compiler pipeline

Typed IR is lowered to a Rust code generation IR, then emitted as Rust source, then compiled by Rust tooling.

The compiler output must:

* produce normal Rust source
* avoid dependence on the interpreter runtime
* inline or emit only minimal support structures required by the program
* generate explicit Rust types and functions wherever possible
* map Vulgata semantics predictably to Rust semantics

For the v0.5 reference implementation, semantic-layer constructs are part of the source language and front-end, but compiler backend parity for those constructs may still be staged separately from interpreter support.

### 4.4 Semantic consistency rule

Interpreter and compiler must be tested against a shared conformance suite.

Every language feature should have:

* parser tests
* type-check tests
* interpreter execution tests
* compiler execution tests where backend support exists
* equivalence tests where feasible

### 4.5 CLI and REPL model

The implementation exposes both file-oriented commands and an interactive REPL.

File-oriented commands:

```text
vulgata parse [--emit-metadata <path>] <source-file>
vulgata check [--emit-metadata <path>] <source-file>
vulgata run [--mode <release|checked|debug|tooling>] [--emit-metadata <path>] <source-file>
vulgata test [--mode <release|checked|debug|tooling>] [--emit-metadata <path>] <source-file>
vulgata compile [--emit-metadata <path>] <source-file>
```

Interactive command:

```text
vulgata repl [--mode <release|checked|debug|tooling>]
```

Defaults:

* `vulgata run` defaults to `release`
* `vulgata repl` defaults to `tooling`
* `vulgata test` is normally run in a checking mode; `checked` is the expected default

The REPL is backed by a virtual source file for the current session, accepts both declarations and expressions, and supports session-local bindings through `let`, `var`, and `:=`.

Core REPL commands:

* `:help`
* `:show`
* `:parse`
* `:check`
* `:run`
* `:test`
* `:reset`
* `:quit`

### 4.6 Semantic layers and execution modes

Vulgata v0.5 distinguishes three semantic layers:

* **Executable**: ordinary computation and side-effecting statements
* **Checkable**: `expect`, `requires`, `ensures`, `example`
* **Descriptive**: `intent`, `meaning`, `explain`, `step`

The runtime supports four named execution modes:

* `release`
* `checked`
* `debug`
* `tooling`

Behavior matrix:

| Layer       | release | checked | debug   | tooling |
|-------------|---------|---------|---------|---------|
| Executable  | run     | run     | run     | run     |
| Checkable   | skip    | enforce | enforce | expose  |
| Descriptive | skip    | skip    | skip    | expose  |

Notes:

* `step` executes its body in all modes. In `debug`, its label may be emitted as a trace.
* `intent`, `meaning`, and `explain` never affect runtime results.
* `example` executes as an embedded test only in `checked` and `debug`.
* `expect` is mode-governed in v0.5: enforced in `checked` and `debug`, skipped in `release`, exposed for tooling.

### 4.7 Metadata export

The CLI accepts `--emit-metadata <path>`.

Metadata emission:

* occurs after successful parsing and type checking
* does not require program execution
* does not change normal execution output or exit behavior other than reporting ordinary argument or write errors
* must be deterministic for unchanged input

The top-level JSON shape is:

```json
{
  "module": "<module-name>",
  "actions": [ ... ],
  "records": [ ... ]
}
```

Action entries always include `"name"` and may include:

* `"intent"`
* `"contracts"` with `"requires"` and `"ensures"`
* `"steps"`
* `"examples"`
* `"explain"`

Record entries may include field-level `"meaning"` metadata.

## 11. Foreign function and external integration model

### 11.1 Goal

A Vulgata program must be able to call external functions in both interpreter and compiler modes without changing source semantics.

### 11.2 Extern declaration syntax

```text
extern action now_iso() -> Text
extern action http_get(url: Text, headers: Map[Text, Text]) -> Result[Text, Text]
extern action sha256(data: Bytes) -> Bytes
```

### 11.3 Binding strategy

Bindings are supplied by configuration rather than encoded fully in source.

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

### 11.4 Type contract

Every extern declaration must have a fully explicit signature. The binder must validate compatibility at registration time.

### 11.5 Runtime behavior

Interpreter mode:

* extern actions resolve through a registry
* values are converted between runtime values and native Rust values
* type mismatch is a setup error, not a silent runtime error

Compiler mode:

* extern calls become direct Rust calls to configured functions
* conversion code is emitted only where necessary
* if types map directly, the call should compile to near-zero-overhead wrapper code

### 11.6 Purity and side effects

Optional metadata may classify externs:

```text
extern pure action sha256(data: Bytes) -> Bytes
extern impure action write_log(level: Text, message: Text) -> None
```

Purity metadata is advisory for future optimization and analysis.

## 12. Standard library model

The standard library should be small and mostly defined as modules of actions rather than syntax.

Candidate modules:

* `text`
* `math`
* `list`
* `map`
* `set`
* `time`
* `test`
* `console`
* `file`

For v0.5, the initial implemented standard runtime library remains:

* `console`
* `file`

`Result[T, E]` and `Option[T]` inspection/extraction are part of the semantic core, not runtime modules. Their member operations require no import.

Examples:

```text
let printed = console.println("hello")
let line = console.read_line()
let data = file.read_text("notes.txt")
let present = file.exists("config.txt")
```

### 12.1 `console` module

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
* end-of-input is represented as `Err(...)`

### 12.2 `file` module

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

### 12.3 No print statement rule

Vulgata does not introduce a dedicated `print` statement. Console and file operations remain ordinary action calls so side effects stay explicit and the language core stays small.

## 13. Error model

### 13.1 Core rule

Do not introduce exceptions as a language-level control-flow system. Use `Result[T, E]` and `Option[T]` explicitly.

### 13.2 Result, Option, and match handling

`Result[T, E]` exposes built-in member operations:

* `is_ok()`
* `is_err()`
* `value()`
* `error()`

`Option[T]` exposes built-in member operations:

* `is_some()`
* `is_none()`
* `value()`

These operations are resolved by the semantic core with no import declaration.

Examples:

```text
if result.is_ok():
  return result.value()

if maybe_name.is_none():
  return "anonymous"
```

Statement-form `match` is also part of the core language and is the canonical branching form for variant-style handling:

```text
match result:
  Ok(value):
    return value
  Err(message):
    return message
```

If no arm matches, execution fails with the named runtime error `NonExhaustiveMatch`.

### 13.3 Interpreter errors

Interpreter failures should report:

* source span
* module
* action
* operation
* diagnostic message

Named runtime errors should exist for checkable-layer failures such as:

* failed `expect`
* failed `requires`
* failed `ensures`
* failed `example`
* `NonExhaustiveMatch`
* `InvalidResultValueAccess`
* `InvalidResultErrorAccess`
* `InvalidOptionValueAccess`

### 13.4 Compiler errors

Compiler diagnostics should reference original Vulgata spans, even when Rust compilation fails in generated code.

### 13.5 Metadata errors

Metadata emission failures are CLI or filesystem errors, not language semantics errors. They must not require program execution to reproduce.

## 14. Mutability and value model

### 14.1 Variables

Variables declared with `let` are immutable bindings. Variables declared with `var` are mutable bindings.

Examples:

```text
let port: Int = 8080
var retries: Int = 0
retries := retries + 1
```

Action parameters are immutable bindings. If local mutation is required, the value should first be copied into a `var`.

### 14.2 Records and collections

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

Descriptive annotations such as `meaning:` do not weaken or alter mutability rules.

### 14.3 Aliasing

The source-language model is value-oriented:

* assigning a compound value to `let` captures an immutable value
* assigning a compound value to `var` creates mutable storage for that variable
* mutating a `var`-rooted value must not implicitly mutate a value visible through an immutable `let` binding

Implementations may use copying, structural sharing, copy-on-write, or other internal strategies as long as those observable semantics are preserved.

## 16. Semantics of calls

### 16.1 Call categories

A call expression may target:

* a declared Vulgata action
* an extern action
* a record constructor
* an enum variant constructor where supported
* a first-class action value where supported

### 16.2 Named and positional arguments

Both are supported.

Rules:

* positional arguments must come first
* named arguments may follow
* once a named argument is used, all following arguments must be named
* no duplicate names
* unknown names are compile-time errors

### 16.3 Overloading

No function overloading is defined in the core language.

### 16.4 Dispatch

Dispatch is lexical and static by symbol identity, not dynamic multimethod dispatch.


# Vulgata Design Specification version 0.5

This document covers sections 17-24 of the split specification: implementation guidance and the execution contract for the Rust reference implementation.

## 17. Interpreter design guidance for Rust agent

The interpreter should be implemented in layered modules.

Suggested structure:

* `lexer`
* `parser`
* `ast`
* `resolver`
* `types`
* `tir`
* `runtime`
* `externs`
* `metadata`
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

The interpreter must also carry an explicit `ExecutionMode` value rather than a process-global mode.

## 18. Compiler design guidance for Rust agent

The compiler path should lower typed IR into a Rust generation model.

Suggested phases:

1. typed IR validation
2. monomorphization or generic strategy decision
3. Rust AST or structured Rust emission model
4. support code emission
5. Cargo project or inline module emission
6. rustc or cargo compilation

### Compiler output principle

Generated code should be understandable Rust, not opaque low-level output.

### No runtime dependency principle

Do not make compiled output depend on the interpreter runtime crate.

Allowed:

* small emitted helper modules per compiled target
* generated structs, enums, wrappers
* direct calls to configured Rust functions

Not allowed:

* compiled programs requiring a general-purpose Vulgata VM

### Current v0.5 caveat

The language specification includes semantic-layer constructs, but backend parity for those constructs may be staged after front-end and interpreter support. Until parity lands, rejecting unsupported semantic-layer constructs in code generation is preferable to silently miscompiling them.

## 19. Minimal support code in compiled output

The compiler may emit support code only where necessary for language semantics.

Examples:

* helper type aliases
* generated `Result` conversion helpers
* collection construction helpers
* small source-location utilities for diagnostics
* target-local support for metadata comments where desired

This support code should remain target-local and removable by ordinary Rust optimization.

## 20. Conformance examples

### 20.1 Basic algorithm

```text
action gcd(a: Int, b: Int) -> Int:
  requires a >= 0
  requires b >= 0
  var x: Int = a
  var y: Int = b
  while y != 0:
    step iterate:
      let r: Int = x % y
      x := y
      y := r
  ensures result >= 0
  return x
```

### 20.2 Record and extern use

```text
record Customer:
  name: Text
  email: Text
    meaning: "Primary contact address"

extern action send_email(to: Text, subject: Text, body: Text) -> Result[None, Text]

action welcome(customer: Customer) -> Result[None, Text]:
  intent:
    goal: "Send a welcome email"
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

### 20.5 Semantic-layer action

```text
action normalize(value: Int) -> Int:
  intent:
    goal: "Clamp a value into the accepted range"
  explain:
    "Negative values map to zero"
    "Values above one hundred map to one hundred"
  requires value >= -1000
  example below_zero:
    input:
      value = -1
    output:
      result = 0
  if value < 0:
    return 0
  elif value > 100:
    return 100
  ensures result >= 0
  ensures result <= 100
  return value
```

## 21. Open issues to freeze before implementation

The v0.5 reference spec still leaves a few points intentionally explicit:

1. Exact compiler-backend handling for semantic-layer constructs until parity is complete.
2. Whether tuples and `Set[T]` are part of the near-term executable subset or remain reserved.
3. Exact map key equality and hashing rules.
4. Whether decimal uses `f64` initially or a decimal library.
5. Exact module-to-file mapping rules.
6. Whether metadata JSON is frozen as a public stable tooling contract or remains implementation-internal for one release cycle.
7. Whether `example output:` remains general `<name> = <literal>` syntax or narrows to canonical `result = ...` for single-result actions.

## 22. Recommended implementation phases

### Phase 1

* lexer
* parser
* AST
* diagnostics
* canonical formatting or pretty-printing

### Phase 2

* resolver
* type checker
* typed IR
* constants, records, actions, expressions, and control flow

### Phase 3

* interpreter runtime
* execution modes
* semantic-layer enforcement and skipping
* metadata emitter
* extern binding registry
* standard runtime core
* test runner

### Phase 4

* Rust code generator
* Cargo project emission
* extern mapping in compiler mode
* semantic-layer backend parity
* conformance equivalence hardening

### Phase 5

* richer enums
* broader pattern forms beyond the phase-1 `match` subset
* optimization passes
* formatter and linter hardening

## 23. Implementation contract for the Rust coding agent

This section turns the specification into an execution contract for an AI coding agent or human implementation team.

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
   * typed IR
   * validation passes
   * metadata extraction
   * shared semantic test suite

3. **Execution backends**

   * interpreter backend
   * Rust code generation backend

No backend may define language semantics independently of the semantic core.

## 23.2 Frozen v0.5 surface subset

The reference implementation must support at least the following surface:

### Top-level declarations

* `module`
* `import`
* `const`
* `record`
* `enum`
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
* nominal `record` and `enum` types

### Statements

* `let`
* `var`
* `:=`
* `if / elif / else`
* `while`
* `for each`
* `match`
* `return`
* `break`
* `continue`
* `expect`
* `intent`
* `explain`
* `step`
* `requires`
* `ensures`
* `example`
* expression statement

### Phase-1 binding and pattern features

* tuple destructuring in `let` and `var`
* nominal record destructuring in `let` and `var`
* phase-1 `match` patterns: wildcard, literals, bindings, tuple patterns, nominal record patterns, and enum-style variant patterns
* built-in `Result[T, E]` member operations: `is_ok()`, `is_err()`, `value()`, `error()`
* built-in `Option[T]` member operations: `is_some()`, `is_none()`, `value()`

### Metadata-facing annotations

* record-field `meaning`
* action `intent`
* action `explain`
* action `step`
* `requires` / `ensures`
* `example`

### Not yet guaranteed across every backend

* semantic-layer code-generation parity
* nested destructuring and broader pattern forms beyond the phase-1 subset
* closures
* async semantics
* macros
* user-defined syntax

## 23.3 Canonical source formatting contract

Formatting rules:

* indentation uses two spaces
* tabs are rejected in canonical source mode
* one statement per line
* spaces around binary operators
* no trailing whitespace
* double quotes for strings only
* imports grouped at top after optional module declaration
* blank lines between top-level declaration groups when practical

Round-trip tests should aim for:

1. parse source
2. pretty-print canonical form
3. parse canonical form again
4. verify AST equivalence

## 23.4 AST contract

The AST must remain close to surface syntax.

### File and declarations

```text
File
  module: Option<ModuleDecl>
  imports: Vec<ImportDecl>
  items: Vec<TopItem>

TopItem
  Const(ConstDecl)
  Record(RecordDecl)
  Enum(EnumDecl)
  Extern(ExternDecl)
  Action(ActionDecl)
  Test(TestDecl)
```

### Declarations

```text
RecordDecl
  name: Ident
  fields: Vec<RecordField>

RecordField
  name: Ident
  ty: TypeRef
  meaning: Option<Text>

ActionDecl
  name: Ident
  params: Vec<Param>
  ret_ty: TypeRef
  body: Block
```

### Statements

```text
Stmt
  Intent(IntentStmt)
  Explain(ExplainStmt)
  Step(StepStmt)
  Requires(RequiresStmt)
  Ensures(EnsuresStmt)
  Example(ExampleStmt)
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

Expressions remain surface-oriented: literals, names, field access, indexing, calls, unary, binary, list, map, tuple, grouping.

### Required AST metadata

Every AST node must carry:

* stable source span
* optional node id assigned after parsing

## 23.5 Typed IR contract

The typed IR is the real semantic core.

The TIR must:

* resolve names to symbols
* resolve every expression type
* distinguish l-values from r-values
* normalize control structures
* represent extern calls explicitly
* make mutable targets explicit
* preserve semantic-layer nodes explicitly rather than erasing them too early

### Recommended TIR skeleton

```text
TStmt
  Intent { goal, constraints, assumptions, properties }
  Explain { lines }
  Step { label, body }
  Requires { value: TValueExpr }
  Ensures { value: TValueExpr }
  Example { name, inputs, outputs }
  Let { ... }
  Assign { ... }
  If { ... }
  While { ... }
  ForEach { ... }
  Return { ... }
  Break
  Continue
  Expect { value: TValueExpr }
  Expr { value: TValueExpr }
```

Backends may choose to ignore, enforce, export, or reject these nodes depending on capability, but the semantic core must not lose them.

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
* synthetic checkable-layer bindings such as `result` where required by later semantic phases

## 23.7 Type-checking contract

### Required checks

* variable must be declared before use
* assignment target must be rooted in a `var` binding
* assignment value must be assignable to target type
* `if`, `elif`, `while`, `expect`, `requires`, and `ensures` conditions must be `Bool`
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
* `result` must resolve correctly inside `ensures`
* `example` input and output bindings must type-check against the enclosing action signature

### Inference boundaries

Local inference is allowed only where deterministic and simple.

### Numeric rules

* `Int + Int -> Int`
* `Dec + Dec -> Dec`
* `Int + Dec -> Dec`
* `Dec + Int -> Dec`
* implicit narrowing is forbidden

## 23.8 Interpreter runtime contract

### Runtime obligations

* preserve source semantics faithfully
* use explicit runtime values
* support external bindings through a registry
* provide deterministic test execution
* provide source-based diagnostics
* honor execution modes explicitly
* skip descriptive constructs without side effects
* enforce checkable constructs only in the modes that require enforcement

### Recommended runtime modules

* `value`
* `env`
* `heap`
* `call`
* `extern_registry`
* `standard_runtime`
* `repl_session`
* `metadata`
* `test_runner`
* `diagnostics`

### Runtime value categories

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

### Mutation behavior

Interpreter and compiler semantics must preserve the mutability contract:

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

* `Option` and `Result` mapping where needed
* collection helpers
* source-location comments or metadata
* extern-boundary conversions

### Not allowed

* embedding a general-purpose Vulgata VM
* interpreting TIR at runtime inside the compiled program
* silently dropping semantic-layer constructs

If a semantic-layer construct is unsupported in the current backend stage, compilation should fail explicitly.

## 23.10 Extern binding contract

Extern behavior must be identical in principle across interpreter and compiler modes.

### Source form

```text
extern action read_file(path: Text) -> Result[Text, Text]
```

### Interpreter mode

* an extern registry maps the symbol to a Rust host function
* host function signature compatibility is validated during startup or module load
* conversion failures are setup errors

### Compiler mode

* configuration maps the extern symbol to a Rust path
* the code generator emits a direct call or generated wrapper call
* no runtime lookup table should be required for ordinary compiled execution

## 23.11 Diagnostics contract

Every error should include:

* source file
* line and column span
* phase
* a precise message
* where relevant, expected and actual types

Diagnostics must cover parse, resolve, type-check, runtime, codegen, extern, and CLI failures.

## 23.12 Conformance suite contract

The conformance suite should be shared by both backends where backend support exists.

### Test categories

1. lexer tests
2. parser tests
3. round-trip formatting tests
4. resolver tests
5. type-check tests
6. interpreter execution tests
7. compiler execution tests
8. interpreter/compiler equivalence tests
9. extern binding tests
10. metadata emission tests
11. diagnostics snapshot tests

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
* descriptive-layer no-op behavior
* `requires` and `ensures` enforcement
* `example` success and failure
* deterministic metadata output

## 23.13 Milestone plan for implementation agent

### Milestone 1: front-end and semantic core

Required output:

* lexer
* parser
* AST
* resolver
* type checker
* typed IR
* parser and type-check test suite

### Milestone 2: interpreter

Required output:

* runtime value model
* execution engine
* extern registry
* test runner
* execution-mode support
* semantic-layer runtime handling
* metadata emitter

### Milestone 3: compiler to Rust

Required output:

* Rust code generator from TIR
* Cargo project emitter
* extern Rust path mapping
* generated record and action code
* compiled execution harness

### Milestone 4: parity hardening

Required output:

* equivalence harness
* diagnostics improvement
* source mapping for compiler failures
* semantic-layer backend parity
* regression corpus

## 23.14 Final stance

Vulgata is only useful if it remains disciplined:

* one canonical syntax
* one shared semantic core
* explicit mutation
* explicit type rules
* explicit extern binding
* explicit execution modes
* descriptive constructs that never silently alter computation

## 24. Final design stance

Vulgata should remain deliberately constrained.

Its value lies in being compact, formal, auditable, and executable across both an interpreter and a compiler, while also carrying non-executable intent and contract information for humans and tools.
