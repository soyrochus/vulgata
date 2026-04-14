# Vulgata Design Specification version 0.5

This specification is split into a minimal concern-oriented set of subdocuments rather than a single monolithic reference.

Subdivision heuristic:

* `01_vulgata_spec_intro.md` covers purpose, goals, and language overview.
* `02_vulgata_spec_language_reference.md` covers the surface language: source files, lexical rules, types, declarations, statements, expressions, and grammar.
* `03_vulgata_spec_execution_model.md` covers execution architecture, semantic layers, runtime behavior, externs, standard library, errors, mutability, and call semantics.
* `04_vulgata_spec_implementation_contract.md` covers the implementation-facing contract for the Rust reference implementation.

The files preserve the original section numbering so citations remain stable.

## 1. Purpose

Vulgata is a compact, human-readable, executable language designed as a lingua franca for humans and AI systems collaborating on software design, algorithm specification, workflow definition, and lightweight application logic.

It is not intended to compete with Python, Rust, or Java as a general-purpose systems language. Its design target is different:

* readable enough for non-specialist technical users to verify
* regular enough for AI systems to generate reliably
* formal enough to parse, analyze, and execute unambiguously
* expressive enough to describe real algorithms and structured software behavior
* restricted enough to remain compact and auditable

Vulgata supports two long-term execution backends:

1. **Interpreter mode**: source is parsed, analyzed, and executed in a managed runtime environment.
2. **Compiler mode**: source is translated to Rust and then compiled to native code with no dependency on the interpreter runtime beyond target-local emitted support code.

The canonical source language is shared across backends. Divergence in meaning is a bug.

Version 0.5 keeps the v0.4 mutability model and the implemented `console`, `file`, and `repl` surfaces, and adds semantic layers:

* a **descriptive layer** for `intent:`, `meaning:`, `explain:`, and `step`
* a **checkable layer** for `requires`, `ensures`, `example`, and mode-governed `expect`
* four execution modes: `release`, `checked`, `debug`, and `tooling`
* optional JSON metadata export through `--emit-metadata`

Version 0.5 also includes a first phase of richer value handling and structural control flow:

* statement-form `match`
* phase-1 match patterns: wildcard, literals, bindings, tuple patterns, nominal record patterns, and enum-style variant patterns
* tuple and nominal-record destructuring in `let` and `var`
* built-in `Result[T, E]` operations: `is_ok()`, `is_err()`, `value()`, `error()`
* built-in `Option[T]` operations: `is_some()`, `is_none()`, `value()`

The semantic-layer additions are specified as source-language features even where backend parity is still being hardened in the reference implementation.

## 2. Design goals

### 2.1 Primary goals

Vulgata should provide:

* a small surface grammar
* explicit semantics
* predictable execution
* strong readability
* explicit mutation
* explicit type model
* straightforward foreign function calls
* deterministic formatting
* semantic room for intent, contracts, and documentation
* easy implementation in Rust

### 2.2 Non-goals

Vulgata does not aim to provide:

* complex metaprogramming
* user-defined syntax
* inheritance-heavy object orientation
* hidden control flow
* implicit coercion culture
* magical dynamic dispatch everywhere
* a large standard library in the language core

### 2.3 Guiding principles

1. **One canonical readable form.** Indentation-based source is canonical.
2. **No hidden meaning.** Mutation, conversion, branching, and failure must be explicit.
3. **Minimal grammar, rich semantic core.** Keep syntax small; place richness in typed IR, execution modes, and metadata.
4. **Same source, stable meaning.** Interpreter and compiler should converge on the same semantics; gaps in parity are implementation debt, not a license for divergence.
5. **External integration is first-class.** Calls to configured external functions must be easy.
6. **Generated code must be auditable.** Canonical formatting and predictable lowering are mandatory.
7. **Descriptive information must not silently change execution.** Intent, explanation, and meaning are first-class, but they are not executable semantics.

## 3. Language overview

Vulgata consists of:

* modules
* imports
* constants
* type declarations
* record declarations
* enum declarations
* action declarations
* extern declarations
* tests
* executable statements
* checkable statements
* descriptive statements
* expressions

The language is primarily statement-oriented, with expressions used where values are required.

The language uses indentation as the canonical block structure. A brace form may exist as a transport or serialization variant, but it is not the canonical source form.

Vulgata v0.5 distinguishes three semantic layers:

* **Executable**: ordinary computation and side-effecting action calls
* **Checkable**: runtime-validated assertions, contracts, and examples
* **Descriptive**: intent, meaning, explanation, and traceable step structure

These layers share one source language, but they do not share one execution policy. The execution mode determines whether a construct runs, is enforced, or is treated as metadata only.
