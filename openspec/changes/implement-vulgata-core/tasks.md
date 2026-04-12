## 1. Module Scaffolding

- [x] 1.1 Replace placeholder `main.rs` entrypoint with a minimal CLI dispatcher that routes to subcommands
- [x] 1.2 Add `lexer`, `parser`, `ast`, `resolver`, `types`, `tir`, `runtime`, `externs`, `codegen`, `diagnostics`, and `cli` as top-level modules in `lib.rs`
- [x] 1.3 Define a `Diagnostic` struct with source span, phase tag, and message fields in `diagnostics`
- [x] 1.4 Define a `SourceSpan` type covering file path, line, and column for use across all phases

## 2. Lexer

- [x] 2.1 Implement a `Token` enum covering all v0.2 keyword, operator, literal, and delimiter variants
- [x] 2.2 Implement indentation tokenization that emits `Indent` / `Dedent` / `Newline` tokens from raw source bytes
- [x] 2.3 Implement the `Lexer` struct that attaches `SourceSpan` to each emitted token
- [x] 2.4 Add lexer unit tests for keywords, operators, string and integer literals, and indentation transitions

## 3. Parser and AST

- [x] 3.1 Define AST node types in `ast` for modules, imports, declarations, statements, and expressions
- [x] 3.2 Implement a recursive-descent parser that consumes the token stream and produces an `AstModule`
- [x] 3.3 Implement `set` mutation statement parsing for variables, record fields, and indexed targets
- [x] 3.4 Implement `test` block and `expect` expression parsing
- [x] 3.5 Add parser unit tests for valid programs and verify early diagnostic emission for malformed inputs

## 4. Name Resolution and Type Checking

- [x] 4.1 Implement a `Resolver` that builds a symbol table from module and import declarations
- [x] 4.2 Implement type representation in `types` for scalar, record, collection, action, and extern types
- [x] 4.3 Implement a `TypeChecker` that annotates AST nodes with concrete types and reports phase-tagged diagnostics for mismatches
- [x] 4.4 Add type checker tests for type mismatch, unresolved symbol, and invalid mutation target scenarios

## 5. Typed IR

- [x] 5.1 Define `tir` node types that encode resolved symbols, concrete types, source spans, and normalized control flow
- [x] 5.2 Implement an AST-to-TIR lowering pass that runs after type checking succeeds
- [x] 5.3 Verify that `set` mutation targets, `test` blocks, and `expect` expressions lower to explicit TIR nodes
- [x] 5.4 Add TIR lowering tests that confirm the output matches expected resolved structures for representative programs

## 6. CLI Subcommands

- [x] 6.1 Implement the `parse` subcommand that runs lexing and parsing and reports diagnostics
- [x] 6.2 Implement the `check` subcommand that runs the full front-end pipeline through TIR lowering
- [x] 6.3 Add `run` and `test` subcommand stubs that invoke the front-end and report "not yet implemented"
- [x] 6.4 Add a `compile` subcommand stub that invokes the front-end and reports "not yet implemented"

## 7. Interpreter Runtime

- [x] 7.1 Define a `Value` enum in `runtime` covering scalar, record, collection, and callable variants
- [x] 7.2 Implement heap-backed containers for records and indexed collections to support aliased mutation
- [x] 7.3 Implement an `Interpreter` that evaluates TIR expressions and statements, including `set` mutation
- [x] 7.4 Implement `test` block execution that runs `expect` expressions and marks pass or fail with source location
- [x] 7.5 Wire the `run` subcommand to invoke the interpreter and report the result or diagnostics
- [x] 7.6 Wire the `test` subcommand to execute test blocks and report per-expectation outcomes
- [x] 7.7 Add interpreter tests covering arithmetic, control flow, record mutation, and failed expectation reporting

## 8. Extern Bindings

- [x] 8.1 Define an `ExternRegistry` in `externs` keyed by extern name with validated signature entries
- [x] 8.2 Implement a TOML-based extern configuration loader that reads declared binding targets
- [x] 8.3 Implement signature validation that compares configuration binding types against declared extern signatures and fails setup on mismatch
- [x] 8.4 Expose typed call adapters on the registry for use by the interpreter dispatch path
- [x] 8.5 Add extern validation tests for compatible bindings, signature mismatch, and missing declaration scenarios

## 9. Rust Code Generation

- [x] 9.1 Define a codegen IR in `codegen` with Rust-oriented node types for items, types, expressions, and statements
- [x] 9.2 Implement a TIR-to-codegen-IR lowering pass that maps Vulgata constructs to their Rust equivalents
- [x] 9.3 Implement a source emitter that renders the codegen IR as formatted Rust source text
- [x] 9.4 Implement a minimal support module emitter for shared helpers that generated programs may reference
- [x] 9.5 Wire the `compile` subcommand to invoke the full pipeline through source emission and write the output file
- [x] 9.6 Add codegen tests that verify emitted Rust compiles and matches expected structure for core language constructs

## 10. Conformance Testing

- [x] 10.1 Create a `tests/conformance` directory with a fixture format that includes source, expected diagnostics, expected interpreter output, and expected compile result
- [x] 10.2 Implement a fixture runner that executes parse, check, run, and compile phases against each fixture and reports mismatches
- [x] 10.3 Add fixtures covering valid module execution, type error diagnostics, record field mutation, and a failed `expect`
- [x] 10.4 Add a cross-mode equivalence check that runs interpreter and compiled binary against the same fixture and fails if results differ
- [x] 10.5 Add fixtures covering extern declaration validation and binding mismatch error reporting
- [x] 10.6 Verify the full conformance suite passes and all phases produce diagnostics with correct source locations
