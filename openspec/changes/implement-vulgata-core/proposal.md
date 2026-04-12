## Why

Vulgata has a detailed language specification, but the repository does not yet implement the executable toolchain that makes the language usable or testable. This change turns the design into a working baseline so the project can validate semantics, run example programs, and evolve the language against a concrete implementation.

## What Changes

- Implement the shared front-end pipeline for Vulgata source files, covering lexing, parsing, AST construction, name resolution, type checking, and lowering into a typed intermediate representation.
- Add interpreter-mode execution for the v0.2 core language, including values, statements, declarations, tests, diagnostics, and module loading.
- Add compiler-mode translation that emits auditable Rust source from the same typed IR and supports building native executables from Vulgata programs.
- Introduce explicit extern binding support so interpreter and compiler modes can call configured external Rust functions with consistent semantics.
- Establish a conformance-oriented test suite that checks parser behavior, type checking, interpreter execution, compiler execution, and semantic equivalence across both execution modes.

## Capabilities

### New Capabilities
- `core-frontend`: Parse canonical Vulgata source and produce validated typed IR for modules, declarations, statements, and expressions defined in the v0.2 spec.
- `interpreter-runtime`: Execute typed IR in interpreter mode with the defined value model, control flow, diagnostics, module loading, and top-level test execution.
- `rust-codegen`: Lower typed IR into readable Rust source that preserves Vulgata semantics and can be compiled without depending on the interpreter runtime.
- `extern-bindings`: Resolve and validate configured external function bindings in both interpreter and compiler modes using explicit signatures.
- `conformance-testing`: Run shared language conformance coverage across parsing, typing, interpreter execution, compiler execution, and cross-mode equivalence.

### Modified Capabilities
None.

## Impact

- Affected code: [src/main.rs](/home/iwk/src/vulgata/src/main.rs), new language implementation modules under [src](/home/iwk/src/vulgata/src), and new OpenSpec capability specs under [openspec/specs](/home/iwk/src/vulgata/openspec/specs).
- APIs: Introduces the first CLI and internal compiler/interpreter APIs for loading, validating, running, and compiling Vulgata programs.
- Dependencies: Continues to rely on Rust and Cargo tooling; compiler mode will additionally depend on emitted Rust building cleanly through the standard Rust toolchain.
- Systems: Establishes the baseline architecture and validation suite that later language features will build on.