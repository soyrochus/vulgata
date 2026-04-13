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

Pattern matching is still deferred. Helper actions and predicates may be used instead.

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
