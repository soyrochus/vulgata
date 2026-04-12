## Context

The current Vulgata implementation and v0.2 language model treat mutation as explicit but broadly available: `set` performs reassignment and in-place updates, `let` bindings may later be reassigned, and compound values behave like managed mutable values with observable aliasing. The proposed v0.3 change is intentionally more opinionated: `let` becomes strictly immutable, `var` becomes the only mutable local binding form, and all mutation is spelled with `:=`.

This change is cross-cutting because it affects the lexer, parser, AST, type checker, TIR contracts, interpreter runtime semantics, compiler lowering expectations, examples, and conformance tests. It is also breaking at the language surface, so the design must define a single coherent semantic model rather than treating the syntax change and the mutability change as separate implementation details.

Constraints:

- The language must remain explicit about state changes; replacing `set` with plain `=` is out of scope.
- `let` must be strictly immutable even for records and collections, not just non-rebindable at the top level.
- Interpreter and compiler modes must preserve the same writable-target and aliasing behavior.
- The implementation should minimize ad hoc special cases and fit the existing shared front-end and typed-IR architecture.

## Goals / Non-Goals

**Goals:**

- Define one consistent v0.3 mutability model: immutable `let`, mutable `var`, explicit mutation via `:=`.
- Ensure writable-target checks are rooted in declaration-time mutability rather than inferred from later usage.
- Preserve strict immutability for compound values visible through `let`, including record fields and indexed collection elements.
- Keep source-level mutation visually explicit while reducing the keyword noise of `set`.
- Make the change implementable across parser, semantic analysis, interpreter, and code generation without semantic drift.

**Non-Goals:**

- Introducing a full persistent standard library or richer immutable update syntax beyond `:=`.
- Adding backward-compatible dual syntax for both `set` and `:=` in the long term.
- Redesigning unrelated language areas such as pattern matching, module loading, or runtime I/O.
- Defining optimization strategy in detail beyond what is needed to preserve the observable mutability contract.

## Decisions

### 1. Treat `let` and action parameters as immutable roots, and `var` as the only mutable root

Writable-target checks should be rooted in binding declarations. A local declared with `let` is never writable. A local declared with `var` is writable. Action parameters should behave like `let`, so mutation of parameter-derived state must start by copying into a mutable local.

This yields a simple rule for users and for the type checker: a target is writable only if its root symbol is a mutable binding. It also avoids the confusing halfway model where `let` looks immutable but still permits field mutation.

Alternatives considered:

- Make `let` immutable only for rebinding while allowing field and index mutation through it. This is easier to implement but weakens the immutability story and is surprising for compound values.
- Introduce separate mutability markers on record and collection types. This is more expressive but substantially more complex than the intended v0.3 change.

### 2. Replace `set target = value` with `target := value` and do not overload `=`

The language should keep `=` for declarations and other non-mutation forms, and use `:=` for all reassignment and in-place mutation.

This preserves explicit mutation while making imperative code less noisy than the `set` keyword form. It also keeps declaration and mutation visually distinct without requiring plain assignment syntax to carry both meanings.

Alternatives considered:

- Keep `set` and only add `var`. This preserves current syntax but does not address the surface-noise concern that motivated the change.
- Use plain `=` for reassignment. This would reduce explicitness and weaken the “no hidden meaning” principle.

### 3. Define source-level compound semantics as immutable-through-`let`, regardless of runtime storage strategy

The source-language rule should be value-oriented: if a compound value is visible through a `let` binding, it is not writable. Mutating a value through a `var` binding must not cause hidden mutation of a value observable through an immutable `let` binding.

This semantic contract should be fixed before choosing implementation mechanics. The runtime and compiler may use copying, structural sharing, or copy-on-write internally, but those strategies are implementation details, not language semantics.

Alternatives considered:

- Preserve the current managed-aliasing model and forbid only direct writes through `let`. That would allow surprising hidden mutation and would not satisfy strict immutability for compound types.
- Require eager deep copying on every `let` assignment. This is semantically valid but may be unnecessarily expensive and should remain an implementation choice rather than a language requirement.

### 4. Represent mutability explicitly in semantic data structures

The AST should distinguish `let` declarations, `var` declarations, and `:=` assignment statements. The resolver and type checker should annotate symbols and writable places with mutability metadata so later phases do not reconstruct the rule indirectly from syntax.

The typed IR should treat writable places as targets rooted in mutable symbols. This keeps interpreter and compiler backends aligned and reduces late-stage string-based or syntax-based writable-target logic.

Alternatives considered:

- Desugar `var` into `let` plus a side table only in the type checker. This keeps AST changes smaller but pushes an important semantic distinction into later phases.
- Treat `:=` as a parser-only alias for the old `set` node shape. This is feasible short-term, but still requires mutability-root checks to change, so the design should acknowledge the semantic change explicitly.

### 5. Make the language change breaking and mechanical rather than transitional

The specification should define `set` as removed in v0.3 rather than supporting two mutation syntaxes indefinitely. Migration should be mechanical: introduce `var` where mutation is needed, replace `set ... = ...` with `:=`, and reject writes rooted in `let`.

This keeps the language simpler and avoids long-term ambiguity in examples, tooling, and diagnostics.

Alternatives considered:

- Support both `set` and `:=` for one or more versions. This would soften migration but would complicate the grammar, formatter expectations, and conformance surface without adding semantic value.

## Risks / Trade-offs

- [Strict immutability for compound values is semantically stronger than the current runtime model] → Define source-level behavior first and allow implementation freedom via copying or copy-on-write until a stable optimization strategy emerges.
- [The change is source-breaking for all update-heavy examples and tests] → Keep migration mechanical and document direct rewrites from `set` to `var` plus `:=`.
- [Compiler and interpreter may diverge on aliasing if mutability metadata is not carried through IR] → Make mutability part of semantic contracts early, especially writable-place representation and symbol metadata.
- [Users may expect `:=` to mean declaration in some languages] → Keep `let` and `var` as the only declaration forms and reserve `:=` exclusively for mutation to make diagnostics unambiguous.
- [Some imperative algorithms become more verbose because parameters are immutable] → Standardize the pattern of copying parameters into local `var` bindings when mutation is needed.

## Migration Plan

1. Update the language specification and OpenSpec requirements to define immutable `let`, mutable `var`, and `:=`.
2. Update the lexer and parser to recognize `var` and `:=`, and remove `set` from the canonical syntax.
3. Update AST, resolver, type checker, and TIR contracts to track binding mutability and writable roots explicitly.
4. Change interpreter and compiler semantics so immutable bindings do not observe hidden mutation through aliasing.
5. Rewrite examples, tests, and conformance fixtures from `set` to `var` plus `:=`, and add negative coverage for mutation through `let`.

Rollback is possible before implementation lands broadly because the change is still at the specification and codebase-development stage. If the refactor proves too disruptive, the project can revert to the v0.2 mutability model by restoring `set` and removing `var`, but mixed semantics should not be carried forward indefinitely.

## Open Questions

- Should the implementation choose eager copying or copy-on-write first for compound values moving from mutable to immutable visibility?
- Should the parser emit a targeted migration diagnostic for legacy `set` usage, or simply reject it as invalid syntax in v0.3 mode?
- Do top-level `const` and local `let` need distinct diagnostic wording for attempted writes, or is one generic immutable-binding error sufficient?
