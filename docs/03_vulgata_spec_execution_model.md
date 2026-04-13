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
* `result`
* `option`
* `test`
* `console`
* `file`

For v0.5, the initial implemented standard runtime library remains:

* `console`
* `file`

Examples:

```text
let _ = console.println("hello")
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

### 13.2 Result handling

Without full pattern matching, initial programs may use helper actions:

```text
if result.is_ok():
  ...
```

Long term, controlled `match` may be added.

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
