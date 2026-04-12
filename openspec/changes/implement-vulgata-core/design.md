## Context

The repository currently contains a Rust crate with a minimal CLI entrypoint and no language implementation beyond the project scaffold. The Vulgata specification defines a shared front-end, two execution targets, explicit extern integration, and a conformance rule that requires interpreter and compiler semantics to match.

This change is cross-cutting because it introduces the language pipeline, runtime model, Rust code generation, test harnesses, and the CLI surface that ties them together. The design needs to establish a modular architecture so the implementation can grow with the language while keeping parser, typing, runtime, and code generation concerns separated.

Constraints:

- The canonical source form is indentation-based and must round-trip through a deterministic parser.
- Interpreter and compiler modes must consume the same typed IR and preserve the same observable behavior.
- Compiler output must be ordinary, auditable Rust source with only minimal support code.
- Extern bindings must remain explicit and type-checked at registration time rather than resolved dynamically at arbitrary call sites.

## Goals / Non-Goals

**Goals:**

- Establish a layered implementation architecture that mirrors the specification: lexer, parser, AST, resolver, type checker, typed IR, interpreter runtime, extern registry, Rust code generation, diagnostics, and tests.
- Define a single typed IR as the semantic contract between front-end analysis and both execution modes.
- Support the v0.2 core language surface described in the specification, including declarations, statements, expressions, tests, and explicit mutation.
- Provide a CLI structure that can parse, check, interpret, test, and compile Vulgata programs without duplicating semantic logic.
- Build conformance tests that exercise both execution modes against the same fixtures and expected outcomes.

**Non-Goals:**

- Implementing every future language feature mentioned as deferred in the spec, such as full pattern matching, closures, or advanced optimization passes.
- Designing a large standard library beyond the minimum support needed to exercise core semantics and tests.
- Building incremental compilation, IDE integration, or bytecode/VM infrastructure.
- Hiding semantic differences behind separate interpreter-only or compiler-only language behavior.

## Decisions

### 1. Organize the crate around language pipeline boundaries

The crate should be split into focused modules that match the specification's phase model: `lexer`, `parser`, `ast`, `resolver`, `types`, `tir`, `runtime`, `externs`, `codegen`, `diagnostics`, `cli`, and `tests`.

This keeps syntax concerns out of runtime code and prevents the compiler backend from depending on parser-specific structures. The CLI layer becomes a thin orchestrator that wires phases together for commands like `check`, `run`, `test`, and `compile`.

Alternatives considered:

- A single monolithic module tree would be faster to start, but it would make semantic drift between interpreter and compiler more likely.
- Separate interpreter and compiler front-ends would duplicate parsing and typing logic and directly violate the spec's shared pipeline requirement.

### 2. Introduce a typed IR as the only execution-facing representation

AST nodes should capture source structure faithfully, but all execution and code generation should operate on a validated typed IR. The typed IR should encode resolved symbols, concrete types, explicit mutability targets, and normalized control flow constructs.

This gives the interpreter and Rust emitter one shared semantic model and centralizes validation. It also simplifies diagnostics because source spans can be preserved while removing parse-only syntax distinctions before execution.

Alternatives considered:

- Executing directly from the AST would reduce initial code, but it would push name and type logic into runtime paths and make code generation harder to reason about.
- Lowering separately for interpreter and compiler would make equivalence testing more difficult and create two semantic sources of truth.

### 3. Use an explicit runtime value model with managed reference semantics

The interpreter runtime should represent language values with a `Value` enum and heap-backed containers for records and collections where aliasing matters. Mutable operations such as `set customer.email = ...` and `set items[1] = ...` should mutate shared managed values so interpreter behavior matches the high-level language model described in the spec.

This matches the requirement that the language behave like a managed language rather than an implicit copy-by-value system. It also creates a clear boundary for extern marshaling and test assertions.

Alternatives considered:

- Purely immutable runtime values would conflict with the explicit mutation model unless every assignment rewrote large structures.
- Reusing Rust ownership semantics directly in the interpreter would leak implementation details into language behavior and complicate consistent diagnostics.

### 4. Emit Rust from typed IR through a small codegen IR and support module

Compiler mode should lower typed IR into a Rust-oriented representation before rendering source files. The emitter should prefer explicit Rust structs, enums, functions, and helper modules rather than generating opaque runtime calls. A small generated support layer is acceptable for shared helpers such as diagnostics metadata, result conversions, or collection helpers when direct Rust mapping is insufficient.

This keeps generated code auditable and aligns with the specification's requirement to avoid depending on the interpreter runtime. The intermediate Rust-oriented representation also allows deterministic formatting and clearer source-span annotations.

Alternatives considered:

- Emitting Rust directly from typed IR is possible, but a small codegen IR will make formatting, helper insertion, and source mapping easier to maintain.
- Compiling through a bytecode VM would be simpler in the short term, but it violates the stated compiler architecture.

### 5. Resolve externs through a validated registry and configuration model

Extern declarations should live in the source language, while actual bindings come from configuration loaded by the CLI. During interpreter startup, configuration should be validated against the declared extern signatures and stored in a registry that exposes typed call adapters. During compilation, the same binding metadata should be used to emit direct Rust paths or wrappers.

This preserves explicit signatures and ensures incompatibilities fail during setup rather than surfacing as ad hoc runtime surprises.

Alternatives considered:

- Hard-coding extern bindings in source would reduce flexibility and conflict with the spec.
- Treating externs as untyped dynamic hooks would undermine both diagnostics and compiler guarantees.

### 6. Build the CLI around shared workflows and fixture-driven conformance tests

The CLI should expose subcommands for parse/check/run/test/compile, all of which route through the same front-end pipeline. Test fixtures should include source files plus expected parser, type-check, interpreter, and compiler outcomes. Conformance tests should compare interpreter results with compiled binary results wherever feasible.

This is the most direct way to enforce the semantic consistency rule while keeping the initial implementation observable and debuggable.

Alternatives considered:

- Testing interpreter and compiler separately would be easier initially, but it would not protect against semantic divergence.
- Deferring compile-path tests until later would make the compiler backend harder to trust once implemented.

## Risks / Trade-offs

- [Shared typed IR may be underspecified early] → Start with a minimal but explicit IR covering only v0.2 constructs and evolve it behind tests instead of exposing parser structures to later phases.
- [Interpreter and compiler parity can drift as features are added] → Add fixture-based equivalence tests for each implemented language feature before expanding the surface area.
- [Indentation-sensitive parsing can create fragile diagnostics] → Keep lexing and indentation tokenization separate, and invest early in source-span-rich parser errors.
- [Extern bindings can complicate both execution modes] → Define one binding schema and one validation path reused by interpreter startup and compiler emission.
- [Generated Rust may become hard to audit if helper code grows] → Keep support code small, deterministic, and isolated from the core interpreter runtime.

## Migration Plan

1. Replace the placeholder CLI with subcommands for parse, check, run, test, and compile.
2. Implement the shared front-end through typed IR and land parser/type-check tests first.
3. Add interpreter execution for the core subset and validate it with executable fixtures.
4. Add Rust emission and compiled execution tests against the same fixtures.
5. Introduce extern configuration support and expand conformance coverage before broadening the language surface.

Rollback is straightforward at this stage because the project has no released language runtime or external users. If a backend proves unstable, the front-end and tests can remain while the affected command is temporarily gated behind an unfinished status.

## Open Questions

- Should the initial CLI read single source files only, or support module-root directory execution from the first implementation?
- What configuration format should extern bindings use in practice for this repo: TOML matching the spec examples, or a simpler Rust-native configuration layer?
- How much Rust support code is acceptable in compiler mode before the output no longer feels minimal and auditable?
- Should the first implementation include canonical formatting/pretty-printing, or treat that as a follow-on capability once parsing and code generation are stable?