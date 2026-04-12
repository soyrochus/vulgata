## Why

Vulgata currently has a core language and host extern mechanism, but it does not yet define a first-class standard runtime library for ordinary console and file work. That leaves simple programs without an obvious, canonical way to print output, read input, or work with text files.

## What Changes

- Introduce a standard `console` runtime module for basic text-oriented terminal I/O.
- Introduce a standard `file` runtime module for simple text file reads, writes, appends, and existence checks.
- Define the initial runtime library as ordinary module actions rather than new syntax.
- Define a small, explicit error model for runtime I/O using `Result[..., Text]`.
- Specify how these runtime actions relate to the existing extern and host-binding architecture.

## Capabilities

### New Capabilities
- `runtime-console-io`: Standard console actions for line-oriented output and input.
- `runtime-file-io`: Standard file actions for basic text file access.

### Modified Capabilities

## Impact

- Affects the language-facing runtime surface for simple interactive and file-based programs.
- Will require CLI/interpreter/compiler support for standard runtime bindings or equivalent built-in extern wiring.
- Establishes the first canonical non-core modules that examples and future features can rely on.
