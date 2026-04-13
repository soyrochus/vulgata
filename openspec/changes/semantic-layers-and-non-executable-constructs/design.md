## Context

Vulgata v0.4 has a single flat execution model: every construct in an `action` body either runs or fails. The AST (`src/ast.rs`) has `StmtKind` variants for control flow, mutation, and `Expect`. The lexer (`src/lexer.rs`) defines keywords as a flat `Token` enum. The interpreter (`src/runtime.rs`) executes all statements unconditionally. There is no concept of execution mode and no metadata pipeline.

The proposed v0.5 extension must graft three semantic layers onto this foundation without changing the executable layer's semantics or requiring users to annotate existing code.

---

## Goals / Non-Goals

**Goals:**
- Add descriptive layer constructs (intent, meaning, explain, step) that are parsed, stored in the AST, and silently skipped at runtime
- Add checkable layer constructs (requires, ensures, example) that are enforced or stripped depending on an execution mode flag
- Introduce an `ExecutionMode` enum threaded through the interpreter
- Add an optional JSON metadata emitter that derives from the AST — no runtime side-effects
- Keep the existing executable layer, codegen path, and resolver completely unchanged

**Non-Goals:**
- Changing the Rust codegen backend (deferred to a future pass)
- Formal verification or proof-checking of `requires`/`ensures` expressions
- AI tooling or IDE integration beyond providing the metadata JSON
- User-defined layers or extensible annotation systems

---

## Decisions

### 1. Semantic layer nodes live in `StmtKind` (not a parallel AST)

**Decision**: Add new `StmtKind` variants (`IntentBlock`, `MeaningAnnotation`, `ExplainBlock`, `StepBlock`, `RequiresClause`, `EnsuresClause`, `ExampleBlock`) rather than attaching them as side-fields on existing nodes.

**Rationale**: The parser already emits a `Vec<Stmt>` body everywhere; inserting new statement variants requires the smallest change to the existing pipeline. The resolver and type-checker can skip unknown-to-them variants via a catch-all. The interpreter already dispatches on `StmtKind` — adding cases for no-ops is trivial.

**Alternative considered**: Attach intent/contracts as optional fields on `ActionDecl`. Rejected because it requires threading optional structs through every pipeline stage that touches `ActionDecl`, and it cannot generalise to block-level `step` and `explain` constructs.

### 2. `ExecutionMode` is a parameter on `Interpreter`, not a global

**Decision**: Add `pub mode: ExecutionMode` to the `Interpreter` struct and thread it into `run_main` / `run_action` calls.

**Rationale**: Keeps all execution state self-contained; allows tests to construct interpreters in different modes without global mutation. The CLI and REPL set the mode once at startup from a `--mode` flag.

**Alternative considered**: A thread-local or static mode. Rejected — hard to test and violates the project's explicit-state principle.

### 3. `requires`/`ensures` expressions reuse the existing `Expr` AST

**Decision**: The condition in `requires <expr>` and `ensures <expr>` is parsed as a standard `Expr`. In checked/debug mode the interpreter evaluates it and panics on failure (same mechanism as `expect`).

**Rationale**: Zero new evaluation infrastructure. The existing expression evaluator already handles booleans. `ensures` binds the return value as `result` — a single synthetic binding added to the local scope before evaluating the expression.

**Alternative considered**: A separate contract AST node with richer semantics. Deferred — out of scope for v0.5.

### 4. `step` is a transparent wrapper, not a new scope

**Decision**: `step <name>: <block>` is desugared during parsing into a single `StmtKind::StepBlock { label: String, body: Vec<Stmt> }`. The interpreter in all modes simply executes the inner body; the label is ignored at runtime.

**Rationale**: Preserves the no-op guarantee for the descriptive layer. Debug-mode tracing (printing the label before executing the body) is a low-cost addition that does not affect execution output.

### 5. `meaning` attaches to `FieldDecl` as an optional string, not a statement

**Decision**: `FieldDecl` gains `pub meaning: Option<String>`. The parser reads `meaning: "<text>"` lines inside record and type bodies. This is the only node where a side-field is appropriate — `meaning` has no positional relationship to other statements.

**Alternative considered**: A `StmtKind::MeaningAnnotation` inside record bodies. Rejected — records do not have statement bodies in the current AST; adding one just for this would be more invasive.

### 6. Metadata emitter is a separate `src/metadata.rs` module

**Decision**: A standalone function `emit_metadata(module: &AstModule) -> serde_json::Value` walks the AST and collects all semantic-layer nodes into a structured JSON value. It is invoked only when `--emit-metadata` is passed on the CLI.

**Rationale**: Clean separation of concerns; zero coupling to the interpreter; trivially testable. `serde_json` is already a transitive dependency of the project.

**Alternative considered**: Emit metadata inside the interpreter as a side-channel. Rejected — couples metadata shape to runtime execution order.

### 7. New keywords are added to the `Token` enum

**Decision**: Add `Intent`, `Meaning`, `Explain`, `Step`, `Requires`, `Ensures`, `Example`, `Goal`, `Constraints`, `Assumptions`, `Properties`, `Input`, `Output` as new `Token` variants in `src/lexer.rs`.

**Rationale**: Consistent with the existing keyword approach. Sub-keywords (`goal`, `constraints`, etc.) are context-sensitive — they are only meaningful inside an `intent:` block — but making them full tokens avoids ambiguity with user-defined identifiers of the same name.

**Risk**: `goal`, `input`, `output` are common identifiers. Programs using these as variable names will break.

**Mitigation**: Document as reserved in v0.5. Accept the breakage — the project has no published ecosystem yet.

---

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| Reserved keywords (`goal`, `input`, `output`) shadow common identifiers | Document as breaking; acceptable at current maturity |
| `ensures result >= 0` requires `result` to be in scope | Inject a synthetic `result` binding after evaluating the return expression but before checking `ensures` clauses |
| `example` blocks with incorrect output may cause noisy failures in checked mode | Errors from `example` blocks clearly identify the example name in the diagnostic |
| Metadata JSON shape is unstable | Mark the `--emit-metadata` flag as experimental in v0.5 |
| Resolver currently has no concept of layers | Resolver must be taught to skip new `StmtKind` variants rather than treating them as unresolvable |

---

## Migration Plan

1. Extend `Token` enum and keyword map in `src/lexer.rs`
2. Add new `StmtKind` variants and `FieldDecl.meaning` in `src/ast.rs`
3. Extend parser in `src/parser.rs` to produce new nodes
4. Add `ExecutionMode` enum and `--mode` CLI flag; thread through interpreter in `src/runtime.rs`
5. Add skip/enforce logic in interpreter for new statement kinds
6. Add `src/metadata.rs` emitter and wire `--emit-metadata` CLI flag
7. Update `src/resolver.rs` to skip new statement kinds
8. Extend REPL to default to tooling mode

No migration of existing Vulgata source files is required — all new constructs are additive.

Rollback: revert the above files; the existing executable layer is untouched.

---

## Open Questions

- Should `ensures` be checked before or after the `return` statement executes? (Proposed: after, using the return value bound as `result`)
- Should the REPL print step labels during execution in tooling mode, or only in debug mode?
- Should `example` blocks also run in debug mode, or only in checked mode?
