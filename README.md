# Vulgata 

> a lingua franca for humans and AI systems

[![Rust](https://img.shields.io/badge/rust-workspace-orange.svg)](./rust/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![FOSS Pluralism](https://img.shields.io/badge/FOSS-Pluralism-purple.svg)](FOSS_PLURALISM_MANIFESTO.md)
[![OpenSpec](https://img.shields.io/badge/OpenSpec-Used-black.svg)](https://openspec.dev/)

![Vulgata logo](./images/vulgata-logo-small.png)

## 1. Purpose

Vulgata is a compact, human-readable, executable language designed as a lingua franca for humans and AI systems collaborating on software design, algorithm specification, workflow definition, and lightweight application logic. Its syntax is whitespace-delimited and indentation-based, inspired by Python, making it easy to read and write without visual noise.

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


## Installation

**Prerequisites:** Rust toolchain (stable, 1.85+). Install via [rustup](https://rustup.rs).

```sh
git clone https://github.com/soyrochus/vulgata.git
cd vulgata
cargo build --release
```

The binary will be at `target/release/vulgata`. You can also install it into your Cargo bin path:

```sh
cargo install --path .
```

---

## Usage

```text
vulgata <parse|check|run|test|compile> [options] <source-file>
vulgata repl [--mode <release|checked|debug|tooling>]
```

| Command | Description |
| ------- | ----------- |
| `parse` | Parse a source file and print the AST |
| `check` | Type-check and lower to typed IR |
| `run` | Interpret and execute the module |
| `test` | Run all `test` blocks in the module |
| `compile` | Emit Rust source code to `<file>.rs` |
| `repl` | Start an interactive source-persistent REPL session |

Useful options:

* `--mode <release|checked|debug|tooling>` for `run`, `test`, and `repl`
* `--emit-metadata <path>` to write semantic-layer metadata JSON without requiring execution

### Examples

```sh
vulgata run    hello.vg       # interpret and print the return value
vulgata run --mode checked hello.vg
vulgata test   math.vg        # run inline tests, report PASS/FAIL
vulgata check  sales.vg       # type-check only, no execution
vulgata check --emit-metadata sales.json sales.vg
vulgata compile invoice.vg    # emit invoice.rs, then: rustc invoice.rs
vulgata parse  foo.vg         # dump AST for debugging
vulgata repl                  # interactive Vulgata session
```

---

## Running the test suite

```sh
cargo test
```

This runs the conformance suite under [tests/conformance/](tests/conformance/). Each fixture pairs a `.vg` source file with a `fixture.conf` that declares the expected outcome (pass, type error, runtime failure, etc.).

---

## Quick tutorial

The full language reference is now split into a small set of spec documents:

* [docs/01_vulgata_spec_intro.md](docs/01_vulgata_spec_intro.md)
* [docs/02_vulgata_spec_language_reference.md](docs/02_vulgata_spec_language_reference.md)
* [docs/03_vulgata_spec_execution_model.md](docs/03_vulgata_spec_execution_model.md)
* [docs/04_vulgata_spec_implementation_contract.md](docs/04_vulgata_spec_implementation_contract.md)

What follows is still just a short taste.

The current v0.6 surface includes built-in `Result`/`Option` member operations, statement-form `match`, and phase-1 tuple/record destructuring in declarations.

### Hello, Vulgata

Save this as `hello.vg`:

```text
action greet_world() -> Text:
  return "Hello, world!"

action main() -> Text:
  return greet_world()
```

Run it:

```sh
vulgata run hello.vg
# Text("Hello, world!")
```

### Standard runtime modules

The initial standard runtime library exposes `console` and `file` as ordinary module actions:

```text
action main() -> Bool:
  let printed = console.println("Checking whether README.md exists...")
  return file.exists("README.md")
```

```sh
vulgata run examples/runtime_io.vg
# Bool(true)
```

Note that runtime I/O actions are ordinary action calls and most of them are fallible:

```text
action announce() -> Result[None, Text]:
  return console.println("Hello from Vulgata")
```

### Types and mutability

Bindings are immutable by default (`let`). Use `var` when you need mutation, and `:=` to reassign:

```text
action countdown(n: Int) -> Int:
  var i = n
  var total = 0
  while i > 0:
    total := total + i
    i := i - 1
  return total
```

### Result, Option, match, and destructuring

`Result[T, E]` and `Option[T]` expose built-in member operations with no import:

```text
action choose_name(result: Result[Text, Text], fallback: Option[Text]) -> Text:
  if result.is_ok():
    return result.value()
  if fallback.is_some():
    return fallback.value()
  return result.error()
```

Statement-form `match` handles variants, tuples, and records, while declaration destructuring stays narrower:

```text
record Customer:
  name: Text
  active: Bool

action score(pair: (Int, Int), customer: Customer, result: Result[Int, Text]) -> Int:
  let (left, right) = pair
  let Customer(active: active) = customer

  match result:
    Ok(value):
      if active:
        return value + left + right
      return value
    Err(_):
      return 0
```

In phase 1, declaration destructuring supports tuple and nominal record forms only. Wildcards, nested patterns, enum destructuring in `let`/`var`, and destructuring in `:=` are rejected.

### Intent, contracts, and execution modes

Vulgata v0.6 also has a first semantic layer system. Some constructs are descriptive only, and some are checkable depending on the active mode:

```text
action normalize(value: Int) -> Int:
  intent:
    goal: "Clamp a score into the accepted range"
  requires value >= -100
  if value < 0:
    return 0
  ensures result >= 0
  return value
```

In `release` mode, descriptive and checkable constructs are stripped from execution. In `checked` and `debug`, contracts are enforced.

```sh
vulgata run --mode release score.vg
vulgata run --mode checked score.vg
```

### More semantic-layer examples

Field-level `meaning:` attaches descriptive metadata to records:

```text
record Customer:
  email: Text
    meaning: "Primary contact address"
  active: Bool
    meaning: "Whether the customer can receive notifications"
```

`explain:` gives a human-readable description inside an action body without changing runtime behavior:

```text
action shipping_band(total: Int) -> Text:
  explain:
    "Orders below 50 stay in the standard band"
    "Orders from 50 upward become priority"
  if total < 50:
    return "standard"
  return "priority"
```

`step` labels wrap ordinary executable code. In `debug` mode the label can be emitted as a trace:

```text
action sum_to(n: Int) -> Int:
  var i = 0
  var total = 0
  step accumulate:
    while i < n:
      i := i + 1
      total := total + i
  return total
```

`example` embeds checkable examples directly in the action:

```text
action clamp(value: Int) -> Int:
  example below_zero:
    input:
      value = -1
    output:
      result = 0

  example already_valid:
    input:
      value = 12
    output:
      result = 12

  if value < 0:
    return 0
  return value
```

Run those checks in a checking mode:

```sh
vulgata run --mode checked clamp.vg
```

You can also export semantic metadata without executing the program:

```sh
vulgata check --emit-metadata clamp.json clamp.vg
cat clamp.json
```

The emitted JSON includes the module name plus semantic-layer data such as action intent, contracts, steps, examples, and field meanings when present.

### Records and enums

```text
record Point:
  x: Dec
  y: Dec

enum Direction:
  North
  South
  East
  West

action origin() -> Point:
  return Point(x: 0.0, y: 0.0)
```

### Tests

Inline tests live at the top level of any module:

```text
action gcd(a: Int, b: Int) -> Int:
  var x = a
  var y = b
  while y != 0:
    let r = x % y
    x := y
    y := r
  return x

test gcd_basic:
  expect gcd(84, 30) == 6

test gcd_coprime:
  expect gcd(13, 7) == 1
```

```sh
vulgata test math.vg
# PASS gcd_basic
# PASS gcd_coprime
```

### Compiling to Rust

```sh
vulgata compile math.vg   # writes math.rs
rustc math.rs -o math
./math
```

The emitted Rust is plain, readable code — no VM, no runtime dependency.

### REPL

Vulgata also provides a small interactive REPL:

```sh
vulgata repl
# or:
vulgata repl --mode tooling
```

The REPL keeps an in-memory virtual source file for declarations and also maintains session-local `let`/`var` bindings. Submit a declaration block with a trailing empty line to extend the session source, submit an expression block to evaluate it against the current session, or submit `let`/`var`/`:=` statements to work with REPL-local bindings. In a modern terminal the CLI REPL now provides basic line editing, cursor movement, and input history. Commands such as `:show`, `:check`, `:run`, `:test`, `:reset`, and `:quit` remain available.

Example session:

```text
> action add(a: Int, b: Int) -> Int:
|   return a + b
|
ok: block added
> let answer = add(20, 22)
answer = Int(42)
> answer
Int(42)
```

---

## Principles of Participation

Everyone is invited and welcome to contribute: open issues, propose pull requests, share ideas, or help improve documentation. Participation is open to all, regardless of background or viewpoint.  

This project follows the [FOSS Pluralism Manifesto](./FOSS_PLURALISM_MANIFESTO.md),  
which affirms respect for people, freedom to critique ideas, and space for diverse perspectives.  

## License and Copyright

Copyright (c) 2026, Iwan van der Kleijn

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
