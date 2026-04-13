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
* optional pattern matching
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

### Metadata-facing annotations

* record-field `meaning`
* action `intent`
* action `explain`
* action `step`
* `requires` / `ensures`
* `example`

### Not yet guaranteed across every backend

* semantic-layer code-generation parity
* pattern matching
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
