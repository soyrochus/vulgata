## Why

Vulgata now has `Result`, `Option`, tuples, and records, but branching on their shape is still awkward and forces users back into ad hoc helper calls and nested `if` chains. A statement-form `match` is the smallest addition that makes variant-oriented control flow ergonomic without committing the language to a full expression-pattern system yet.

## What Changes

- Add a statement-level `match <expr>:` form with ordered arms and no fallthrough.
- Add a first-phase pattern language for `match`: wildcard `_`, literals, binding identifiers, enum-variant patterns, tuple patterns, and name-based record patterns.
- Define arm-local binding scope and immutable pattern bindings.
- Define runtime failure semantics for non-exhaustive matches with a named runtime error.
- Extend the typed frontend and IR so `match` and its patterns are represented explicitly instead of remaining parser-only sugar.
- Add parser, type-check, runtime, codegen, and conformance coverage for `Result`, `Option`, tuple, enum, and record matches.

## Capabilities

### New Capabilities
- `match-statements`: Statement-form pattern matching over `Result`, `Option`, enums, tuples, and nominal records, including ordered arm evaluation, arm-local bindings, and non-exhaustive runtime failure.

### Modified Capabilities

## Impact

- `src/lexer.rs`: add the `match` keyword and any pattern-related token handling required by the chosen surface grammar.
- `src/parser.rs` and `src/ast.rs`: add `match` statements, arm nodes, and pattern AST nodes.
- `src/types.rs`: validate pattern compatibility, tuple arity, variant/type matching, duplicate bindings, and arm-local scopes.
- `src/tir.rs`: introduce explicit typed IR for match statements and lowered patterns.
- `src/runtime.rs`: execute ordered arm matching and raise `NonExhaustiveMatch` when no arm matches.
- `src/codegen.rs`: lower match statements and supported patterns to Rust control flow.
- `tests/` and `tests/conformance/`: add coverage for `Result`, `Option`, tuple, enum, and record matching.
