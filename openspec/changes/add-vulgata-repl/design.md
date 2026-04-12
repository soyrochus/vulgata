## Context

Vulgata already has a shared front-end pipeline (`parse`, `check`, `lower`, `run`, `test`, `compile`) and an interpreter that operates on a fully formed module, but the CLI only exposes file-based commands. The REPL must fit that architecture rather than introducing a second language or a heap-persistent interactive evaluator.

The source specification in `spec/vulgata_repl.md` intentionally defines the REPL as a session over a virtual source file. That choice keeps semantics aligned with ordinary Vulgata modules and makes diagnostics, checking, testing, and execution reuse the same front-end/runtime path the crate already uses.

## Goals / Non-Goals

**Goals:**
- Add a `vulgata repl` subcommand for interactive interpreter-backed sessions.
- Keep REPL behavior source-persistent by accumulating accepted top-level Vulgata declarations in a virtual module buffer.
- Reuse the same lexer, parser, resolver, type checker, typed IR lowering, and interpreter used by file execution.
- Provide a minimal set of REPL commands for help, display, reset, parse, check, run, test, and quit.
- Ensure invalid submitted blocks are rejected without mutating session state.

**Non-Goals:**
- Designing a separate expression-only REPL language.
- Preserving runtime locals or heap state between `:run` invocations.
- Supporting compile-mode execution or emitted Rust inspection in the MVP.
- Adding sophisticated terminal history, completion, debugger features, or persistent session save/load in the first version.
- Fully solving extern configuration in the MVP if it would significantly expand the CLI surface.

## Decisions

### 1. The REPL is a virtual source file, not a special evaluator

The REPL should maintain one in-memory source buffer with a stable synthetic path such as `<repl>` or `<repl>/session.vg`. Each accepted source block is appended to that buffer, and commands such as `:check`, `:run`, and `:test` operate on the whole accumulated module.

This preserves semantic alignment with the existing implementation and avoids adding REPL-only parser or runtime rules.

Alternatives considered:
- Evaluate expressions or fragments directly against interpreter state. This would feel more REPL-like, but it would diverge from the module-oriented parser and runtime model.
- Persist typed IR or runtime heap state incrementally. This could reduce repeated work later, but it introduces a more complex and less transparent execution model.

### 2. Session state is source-persistent, not runtime-persistent

Successful submitted declarations should remain in the session buffer, but each `:run` and `:test` invocation should rebuild the pipeline and execute from a fresh interpreter state.

This matches the current architecture and keeps behavior deterministic. It also avoids subtle interactions where runtime locals or mutated state survive in ways that are not visible in source.

Alternatives considered:
- Preserve runtime state across runs. This is more interactive, but it introduces hidden state outside the source buffer.
- Rebuild only partial pipeline layers incrementally. This may be a future optimization, but it should not define the semantics of the first REPL.

### 3. REPL meta-commands are distinguished by a `:` prefix

Lines beginning with `:` should be treated as REPL commands rather than Vulgata source. The MVP command set should include `:help`, `:show`, `:reset`, `:parse`, `:check`, `:run`, `:test`, and `:quit`.

This is a conventional interactive shell pattern and cleanly separates session control from Vulgata declarations.

Alternatives considered:
- Use dot-prefixed commands such as `.run` and `.quit`. This is also workable, but `:` is a more common REPL convention and visually distinct from Vulgata syntax.
- Avoid meta-commands entirely. This would make basic inspection and control awkward.

### 4. Block acceptance must be transactional

When the user submits a candidate source block, the REPL should concatenate it with the current session buffer, parse and check the combined source, and only commit the new buffer if validation succeeds.

This keeps the session coherent and ensures failed blocks do not partially corrupt later analysis.

Alternatives considered:
- Append first and tolerate a “broken” session until reset. This is simpler to implement, but it degrades the interactive workflow quickly.
- Parse only the candidate block in isolation. This misses interactions such as duplicate declarations or unresolved references against accumulated state.

### 5. Extern support should be deferred unless session configuration is added explicitly

The MVP should either reject extern-backed execution in REPL mode with a clear diagnostic or add an explicit session-level extern configuration command. The preferred first step is to keep extern-backed REPL execution out of scope unless configuration support lands with the feature.

This avoids expanding the MVP into a larger CLI configuration project while keeping the extension point explicit.

Alternatives considered:
- Automatically infer extern configuration. This is convenient but ambiguous and inconsistent with the current CLI.
- Fully support extern configuration from day one. This is possible, but it broadens the first implementation beyond the core interactive loop.

## Risks / Trade-offs

- [Repeated parse/check/lower per command may be less efficient than incremental state] → Prefer semantic clarity first and optimize later only if usage shows it is necessary.
- [A declaration-oriented REPL is less flexible than expression-evaluation shells] → Keep the MVP aligned with the current module-based architecture and defer expression wrappers to a later extension.
- [Multi-line block submission can be awkward in an indentation-sensitive language] → Make block completion explicit and predictable rather than trying to infer too much from partially entered text.
- [Deferring extern support limits some interactive use cases] → Emit a clear REPL diagnostic and reserve a future `:externs` command as the extension path.

## Migration Plan

1. Extend the CLI command parser with a `repl` subcommand.
2. Introduce a REPL session type that owns the virtual source buffer and command loop.
3. Implement transactional block submission over the shared front-end pipeline.
4. Implement the MVP REPL commands and output formatting.
5. Add integration tests covering valid sessions, rejected blocks, `:run`, and `:test`.

Rollback is straightforward because the change is additive: the `repl` subcommand and session machinery can be removed without affecting the existing file-based commands.

## Open Questions

- What exact block-submission UX should the MVP use: blank-line termination, a delimiter command, or a lightweight prompt state machine that infers block completion?
- Should `:parse` print the full debug representation already used by the file-based command, or a shorter REPL-oriented rendering?
- Should `:reset` require confirmation in the MVP, or should it immediately clear session state?
- If a module declaration is allowed in the session, should later attempts to redefine it be rejected with the same rule as ordinary source files?
