## 1. Lexer, Parser, and AST Surface

- [x] 1.1 Add lexer support for the `match` keyword and any syntax needed by phase-1 match patterns
- [x] 1.2 Extend the AST with explicit `match` statement, match-arm, and pattern node types
- [x] 1.3 Parse `match <expr>:` statements with ordered arms and supported phase-1 pattern forms
- [x] 1.4 Add parser tests covering valid `Result`, `Option`, tuple, enum, and record matches plus invalid syntax cases

## 2. Type-Checking and Scope Rules

- [x] 2.1 Add type-checking for pattern compatibility, including literal assignability, tuple arity, enum-variant validity, and nominal record-type matching
- [x] 2.2 Reject duplicate binding names within a single pattern and ensure arm-local bindings are immutable and scoped only to the selected arm
- [x] 2.3 Add type-checker tests covering invalid patterns, duplicate bindings, and arm-scope visibility rules

## 3. Typed IR and Runtime Semantics

- [x] 3.1 Add explicit typed IR for match statements, match arms, and lowered patterns in `src/tir.rs`
- [x] 3.2 Lower parsed `match` statements into the new typed IR instead of desugaring them ad hoc
- [x] 3.3 Implement interpreter matching for wildcard, literal, binding, enum, tuple, and record patterns with first-match-wins semantics
- [x] 3.4 Raise a named runtime error `NonExhaustiveMatch` when no arm matches

## 4. Compile Mode and Conformance

- [x] 4.1 Lower typed match statements and supported patterns to Rust control flow in `src/codegen.rs`
- [x] 4.2 Add interpreter and codegen tests for `Result`, `Option`, tuple, enum, and record matches
- [x] 4.3 Add conformance fixtures covering successful matches and explicit non-exhaustive runtime failure
