# Vulgata 

> a lingua franca for humans and AI systems

[![Rust](https://img.shields.io/badge/rust-workspace-orange.svg)](./rust/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![FOSS Pluralism](https://img.shields.io/badge/FOSS-Pluralism-purple.svg)](FOSS_PLURALISM_MANIFESTO.md)
[![OpenSpec](https://img.shields.io/badge/OpenSpec-Used-black.svg)](https://openspec.dev/)

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


## Installation

**Prerequisites:** Rust toolchain (stable, 1.85+). Install via [rustup](https://rustup.rs).

```sh
git clone https://github.com/your-org/vulgata.git
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
vulgata <parse|check|run|test|compile> <source-file>
vulgata repl
```

| Command | Description |
| ------- | ----------- |
| `parse` | Parse a source file and print the AST |
| `check` | Type-check and lower to typed IR |
| `run` | Interpret and execute the module |
| `test` | Run all `test` blocks in the module |
| `compile` | Emit Rust source code to `<file>.rs` |
| `repl` | Start an interactive source-persistent REPL session |

### Examples

```sh
vulgata run    hello.vg       # interpret and print the return value
vulgata test   math.vg        # run inline tests, report PASS/FAIL
vulgata check  sales.vg       # type-check only, no execution
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

The full language is specified in [spec/vulgata_spec_v0.4.md](spec/vulgata_spec_v0.4.md). What follows is a short taste.

### Hello, Vulgata

Save this as `hello.vg`:

```text
action greet(name: Text) -> Text:
  return "Hello, " + name + "!"

action main() -> Text:
  return greet("world")
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
  let _ = console.println("Checking whether README.md exists...")
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
```

The REPL keeps an in-memory virtual source file for declarations and also maintains session-local `let`/`var` bindings. Submit a declaration block with a trailing empty line to extend the session source, submit an expression block to evaluate it against the current session, or submit `let`/`var`/`:=` statements to work with REPL-local bindings. Commands such as `:show`, `:check`, `:run`, `:test`, `:reset`, and `:quit` remain available.

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
