## 1. Syntax and Front-End Surface

- [x] 1.1 Update the lexer to recognize `var` declarations and the `:=` assignment operator
- [x] 1.2 Update the parser and AST to represent `let` declarations, `var` declarations, and `:=` assignments as distinct statement forms
- [x] 1.3 Remove `set` from the canonical v0.3 syntax and ensure legacy `set` statements are rejected with clear diagnostics
- [x] 1.4 Update parser tests and syntax fixtures to cover valid `var` / `:=` usage and invalid legacy `set` usage

## 2. Mutability Semantics

- [x] 2.1 Add declaration-time mutability metadata for locals and parameters in the resolver and semantic model
- [x] 2.2 Update the type checker so assignment targets are writable only when rooted in a `var` binding
- [x] 2.3 Reject field and index updates rooted in `let` bindings, including compound values bound immutably
- [x] 2.4 Treat action parameters as immutable roots and require copy-into-`var` patterns for local mutation
- [x] 2.5 Add semantic tests for immutable `let`, mutable `var`, immutable parameters, and invalid field/index writes through `let`

## 3. Typed IR and Execution Semantics

- [x] 3.1 Update the typed IR to carry explicit `var`/writable-root semantics for assignments and writable targets
- [x] 3.2 Refactor interpreter execution to preserve the v0.3 rule that values visible through `let` do not observe hidden mutation through aliasing
- [x] 3.3 Update compiler lowering and emitted Rust expectations to preserve the same mutability and aliasing semantics as the interpreter
- [x] 3.4 Add interpreter and codegen tests covering mutable updates through `var` and non-writable compound values through `let`

## 4. Specs, Examples, and Conformance

- [x] 4.1 Update language-facing examples and docs from `set` to `var` plus `:=` where mutation is required
- [x] 4.2 Update conformance fixtures and test cases so mutable algorithms use canonical v0.3 syntax
- [x] 4.3 Add negative conformance coverage for rejected `set` syntax and rejected writes rooted in `let`
- [x] 4.4 Verify interpreter and compiler parity for the refactored mutability model, including aliasing-sensitive cases
