## Context

Vulgata now has nominal records, enums, tuples, `Result`, and `Option`, but its executable control flow is still limited to `if`, loops, and direct calls. The parser and AST represent statements and expressions directly, and the typed pipeline lowers them into TIR before the interpreter and Rust codegen backends execute them. Adding `match` therefore affects every stage of the executable pipeline: lexer, parser, AST, type-checker, TIR, runtime, codegen, and conformance tests.

The source proposal is intentionally narrow. It asks for a statement-level `match`, not an expression form; a limited first-phase pattern language; and runtime-defined non-exhaustive behavior instead of a full static exhaustiveness engine.

## Goals / Non-Goals

**Goals:**
- Add a statement-form `match <expr>:` with ordered arms and no fallthrough.
- Support the first-phase pattern set: wildcard, literal, binding identifier, enum variant, tuple, and nominal record pattern.
- Introduce arm-local immutable bindings for names produced by a matching pattern.
- Lower `match` into explicit typed IR rather than leaving it as parser sugar.
- Define runtime `NonExhaustiveMatch` behavior when no arm matches.
- Make the feature available in both interpreter and compile mode.

**Non-Goals:**
- Expression-form `match`.
- Full static exhaustiveness analysis for arbitrary enums and nested patterns.
- Flow-sensitive narrowing outside a chosen arm.
- Guards, or-patterns, nested declaration destructuring, or backtracking pattern languages.
- Record destructuring in declarations; that belongs to the separate binding-destructuring change.

## Decisions

### 1. Add a dedicated pattern AST and TIR shape

**Decision:** Represent patterns explicitly in the AST and TIR instead of desugaring them immediately into `if` chains.

**Rationale:** Type-checking duplicate bindings, tuple arity, record field validity, and variant compatibility is easier when the frontend can inspect a structured pattern tree. The runtime and codegen backends also need a stable lowered shape to share semantics.

**Alternative considered:** Desugar directly in the parser to nested `if`/`let` statements. Rejected because it would make diagnostics, scoped bindings, and codegen parity substantially harder.

### 2. `match` remains a statement, not an expression

**Decision:** Add a new statement kind for `match`, with each arm owning a normal block.

**Rationale:** This aligns with the source proposal’s “minimum expressive power” goal and avoids needing branch-value unification rules, temporary storage, or fallthrough-expression semantics in the type-checker and codegen.

**Alternative considered:** Implement expression `match` immediately. Rejected as too much semantic surface for the first pass.

### 3. Pattern bindings are created only after a full arm matches

**Decision:** Pattern matching produces a temporary per-arm binding map. Only when the full arm pattern succeeds are those names inserted into the arm-local execution scope.

**Rationale:** This avoids partially bound names from failed subpatterns and keeps arm-local scope predictable.

**Alternative considered:** Bind eagerly while descending the pattern tree. Rejected because failed subpatterns would require rollback logic and complicate both interpreter and codegen.

### 4. Built-in `Result` and `Option` patterns are treated like enum-variant patterns

**Decision:** `Ok(...)`, `Err(...)`, `Some(...)`, and `None` are matched through the same pattern machinery as user-defined enum variants, with explicit handling for the existing runtime representations of `Result` and `Option`.

**Rationale:** This gives one user mental model for sum-type matching while still accommodating the language’s built-in algebraic forms.

**Alternative considered:** Reserve separate pattern kinds for `Result` and `Option`. Rejected because it would duplicate matching logic and complicate future enum work.

### 5. Non-exhaustive matches are runtime errors in phase 1

**Decision:** If no arm matches, interpreter and generated Rust both raise `NonExhaustiveMatch`.

**Rationale:** The proposal explicitly allows runtime-defined failure before full exhaustiveness analysis exists. This yields deterministic behavior without blocking the feature on a much larger static-analysis change.

**Alternative considered:** Require a wildcard arm in all cases. Rejected because it is stricter than the proposal and would make simple closed-world `Result` / `Option` matches more awkward.

### 6. Record patterns are nominal and partial by field selection

**Decision:** Record patterns name the record type explicitly and only inspect the listed fields. Unmentioned fields are ignored.

**Rationale:** That matches current Vulgata record construction semantics and avoids inventing structural matching rules that do not exist elsewhere in the language.

**Alternative considered:** Structural or positional record matching. Rejected because it conflicts with the existing nominal, name-based record model.

## Risks / Trade-offs

- [Risk] Pattern matching touches parser, typing, runtime, and codegen simultaneously. → Mitigation: keep the first pattern set intentionally small and use explicit TIR to centralize semantics.
- [Risk] Runtime matching for built-in and user-defined variants may diverge. → Mitigation: funnel both through the same pattern matcher abstraction and add shared conformance coverage.
- [Risk] Lack of full exhaustiveness analysis may hide missing arms until runtime. → Mitigation: define `NonExhaustiveMatch` clearly and leave room for trivial static checks later.
- [Risk] Duplicate binding names inside nested patterns can produce confusing diagnostics. → Mitigation: validate duplicate names during type-checking before runtime or codegen.

## Migration Plan

1. Add lexer and parser support for `match` syntax and pattern grammar.
2. Extend AST and TIR with explicit match and pattern nodes.
3. Add type-checking for pattern compatibility, duplicate bindings, and arm-local scopes.
4. Implement interpreter matching and `NonExhaustiveMatch`.
5. Add Rust codegen lowering for supported patterns.
6. Add parser, typing, runtime, codegen, and conformance tests for `Result`, `Option`, tuple, enum, and record matches.

Rollback is straightforward: remove the new lexer/parser branches, AST/TIR nodes, and backend handling. The change is additive and does not require data migration.

## Open Questions

- Whether phase 1 should include trivial static exhaustiveness checks for `Option` and `Result`, or defer all exhaustiveness to runtime.
- Whether tuple matching should reuse the current tuple runtime representation directly or be normalized to a dedicated tuple value later.
- How much enum payload support should be exposed immediately if user-defined enums are still evolving separately from this change.
