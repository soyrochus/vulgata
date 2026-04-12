## Context

Vulgata currently has a language core, a file-oriented CLI, and host integration via extern bindings, but it does not yet define a canonical standard runtime library. The new runtime surface must stay aligned with the language design: explicit side effects, ordinary action calls instead of syntax, and a small API that can be implemented by the current interpreter and future compiler paths without introducing a second dispatch model.

The initial runtime library is intentionally small. It covers only basic console text I/O and basic file text I/O so examples and early programs have an obvious standard way to interact with the outside world.

## Goals / Non-Goals

**Goals:**
- Define a standard `console` module for straightforward text input and output.
- Define a standard `file` module for basic text reads, writes, appends, and existence checks.
- Keep runtime calls as ordinary module actions rather than introducing special syntax.
- Use `Result[..., Text]` for fallible operations so failures remain explicit and consistent with the language model.
- Keep the implementation compatible with the existing interpreter, CLI, and extern-style host binding architecture.

**Non-Goals:**
- Designing a large general-purpose standard library.
- Adding binary file I/O, directory traversal, metadata APIs, or streaming interfaces in the first version.
- Introducing exceptions, hidden implicit failure, or language-level `print` statements.
- Standardizing a fully portable path abstraction beyond plain `Text` paths.

## Decisions

### 1. Standard runtime modules are language-defined but action-shaped

The runtime surface should be expressed as standard modules named `console` and `file`, with ordinary action calls such as `console.println(...)` and `file.read_text(...)`.

This preserves the language’s preference for library actions over syntax. It also keeps side effects obvious in source and avoids making I/O a parser-level feature.

Alternatives considered:
- Add a `print` statement or other built-in syntax. This is simpler to teach initially, but it cuts against the “small core, actions over syntax” direction.
- Treat all runtime I/O only as user-configured externs. This keeps implementation minimal, but it leaves common programs without a canonical standard API.

### 2. The initial library should be explicitly small and text-first

The first runtime library should include only:
- console text output and line input
- text file read/write/append
- file existence checks

This keeps the first version obvious and implementable. It also avoids prematurely committing to broader I/O abstractions before the language grows more result handling and module ecosystem maturity.

Alternatives considered:
- Add binary I/O and directory APIs immediately. That would expand usefulness, but it would also widen the spec and implementation surface substantially.
- Add formatting helpers and richer terminal behavior up front. That adds convenience, but it risks turning the initial runtime library into a grab bag instead of a crisp first step.

### 3. Fallible I/O returns `Result[..., Text]`

Console writes, console reads, and file read/write/append operations should return `Result` values rather than raising exceptions or silently failing. Error payloads should use human-readable `Text` in the first version.

This is consistent with the language’s explicit-failure direction and avoids introducing a structured error hierarchy before the language is ready to handle it well.

Alternatives considered:
- Use exceptions or implicit runtime failures. This would hide failure semantics and does not fit the language direction.
- Introduce a structured runtime error record immediately. That may become desirable later, but it would add surface area and design pressure too early.

### 4. Built-in runtime bindings should reuse the current host dispatch model where possible

Implementation should prefer wiring standard runtime actions through the same host-integration concepts already used for extern dispatch, while still presenting them as standard library modules at the source level.

This minimizes new architectural surface and keeps interpreter/compiler integration conceptually aligned.

Alternatives considered:
- Create a separate special-case runtime dispatch path unrelated to externs. This is feasible, but it duplicates infrastructure too early.
- Ship the runtime library purely as source modules. This is attractive long-term, but it does not solve the need for host-backed side effects by itself.

## Risks / Trade-offs

- [The first runtime library may feel too small] → Keep the API deliberately narrow and extend it incrementally once real usage clarifies the next missing operations.
- [Human-readable `Text` errors are less structured than richer error values] → Treat this as a first-step error model and defer structured error types until the language has stronger result-handling patterns.
- [Built-in runtime modules could blur the line between standard library and externs] → Keep source-level naming canonical and document that implementation reuse of extern-style bindings is an internal detail.
- [Path handling is host-dependent] → Specify that paths are `Text` interpreted by the host environment and avoid stronger portability claims in the first version.

## Migration Plan

1. Add standard capability specs for console and file runtime actions.
2. Implement interpreter-visible built-in bindings or equivalent runtime registration for `console` and `file`.
3. Expose the runtime modules consistently through CLI-driven execution paths.
4. Update examples and future docs to prefer the standard runtime modules over ad hoc host-specific approaches.

Rollback is straightforward before broad adoption: the runtime modules can be removed and the project can continue relying on core-language-only examples and explicit extern bindings.

## Open Questions

- Should `file.exists` be surfaced as pure-looking `Bool`-returning behavior only, or explicitly documented as operationally impure despite its simple return type?
- Should end-of-input for `console.read_line` use a fixed required message such as `end of input`, or remain implementation-defined while still being an `Err(...)`?
- When compiler mode grows beyond Rust source emission, should built-in runtime actions lower through the same abstraction layer as interpreter extern dispatch or through a dedicated standard-runtime binding registry?
