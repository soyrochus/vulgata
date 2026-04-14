## Context

Vulgata currently permits only single-name `let` and `var` declarations. Compound values can be accessed later through indexing or field access, but the declaration itself cannot unpack them. The match-and-destructuring proposal intentionally separates declaration destructuring from general pattern matching: declarations only need deterministic unpacking, not branching semantics. That narrower scope makes it reasonable to implement as a dedicated change.

This feature still cuts across parser, AST, type-checking, TIR, runtime, codegen, and tests because destructuring changes the meaning of declaration syntax and introduces multiple bindings from one source-level declaration.

## Goals / Non-Goals

**Goals:**
- Extend `let` and `var` to support tuple destructuring and name-based nominal record destructuring.
- Preserve one-time evaluation of the initializer for destructuring declarations.
- Ensure destructured names inherit mutability from the declaration form: immutable for `let`, mutable for `var`.
- Keep destructured values as ordinary bindings rather than references back into the original compound value.
- Reject unsupported forms such as enum destructuring in declarations, nested destructuring, wildcard destructuring, and destructuring in `:=`.

**Non-Goals:**
- General pattern syntax in declarations.
- Enum / `Result` / `Option` destructuring in declarations.
- Per-element type annotations inside a destructuring pattern.
- Writable aliasing from a destructured name back into a record field or tuple element.
- Reusing declaration destructuring as assignment syntax.

## Decisions

### 1. Use a dedicated binding-pattern AST for declarations

**Decision:** Extend `let` and `var` declarations to carry a binding-pattern node rather than overloading the existing single-name field ad hoc.

**Rationale:** Tuple and record destructuring create multiple names, and the type-checker/runtime need explicit access to the pattern structure. A dedicated node also keeps declaration destructuring separate from full `match` patterns.

**Alternative considered:** Desugar in the parser to several synthetic `let` / `var` statements. Rejected because it makes one-time initializer evaluation and source-span diagnostics harder to preserve.

### 2. Declaration destructuring remains narrower than match patterns

**Decision:** Allow only identifier outputs inside destructuring declarations: tuple positions map to identifiers, and record fields map from field name to identifier. No nested patterns, wildcard, enum patterns, or literal subpatterns.

**Rationale:** This fits the “simple unpacking” goal and avoids turning declarations into a second full pattern language.

**Alternative considered:** Reuse the entire match-pattern grammar in declarations. Rejected because it would blur the line between binding and branching and complicate error reporting.

### 3. Initializers are evaluated once, then unpacked

**Decision:** The runtime and codegen must evaluate the initializer exactly once, store the resulting value in a temporary if needed, and then extract the destructured values from that single result.

**Rationale:** This preserves correct semantics when the initializer performs work or has observable effects.

**Alternative considered:** Re-evaluate field/index expressions during lowering. Rejected because it changes semantics and risks double execution.

### 4. Destructured bindings are values, not aliases

**Decision:** Extracted names receive copied/snapshotted values using the same value semantics as ordinary bindings. Mutating a `var` destructured name does not mutate the original tuple or record.

**Rationale:** This matches the existing Vulgata rule that `let`-visible values do not observe hidden mutable aliasing and keeps destructuring compatible with the refactored mutability model.

**Alternative considered:** Destructure to live references for record fields. Rejected because it would introduce partial writable aliases that the language otherwise avoids.

### 5. Optional declaration type annotations apply to the whole initializer

**Decision:** If a destructuring declaration carries an explicit type annotation, it constrains the right-hand-side value as a whole, not the individual extracted names.

**Rationale:** This is the narrowest extension and matches the proposal text.

**Alternative considered:** Per-element or per-field annotations. Deferred because they would require more syntax and more type-checking rules.

## Risks / Trade-offs

- [Risk] Destructuring declarations may overlap conceptually with future `match` patterns. → Mitigation: keep the declaration pattern grammar explicitly narrower and document the difference in specs.
- [Risk] Lowering a single declaration into multiple runtime bindings can accidentally evaluate the initializer more than once. → Mitigation: require explicit one-time evaluation in runtime and codegen design.
- [Risk] Users may expect `var Customer(name: n) = customer` to alias back into the record. → Mitigation: specify value-copy semantics clearly and add tests that prove mutation does not flow back.

## Migration Plan

1. Extend parser and AST for tuple and record binding patterns in `let` / `var`.
2. Add type-checking for arity, nominal record matching, duplicate names, and supported-form validation.
3. Lower destructuring declarations into explicit typed binding forms or normalized temporaries in TIR.
4. Implement runtime and codegen extraction while preserving one-time evaluation and value semantics.
5. Add tests and conformance coverage for success and rejection cases.

Rollback is additive: remove the new parser branches, binding-pattern nodes, and lowering/runtime/codegen support. No stored data or on-disk format migration is involved.

## Open Questions

- Whether phase 1 should allow `_` in tuple destructuring declarations or keep declarations strictly identifier-only.
- Whether a whole-pattern type annotation should be required for some ambiguous cases, or remain optional everywhere.
- Whether future enum destructuring in declarations should reuse these binding-pattern nodes or remain exclusive to `match`.
