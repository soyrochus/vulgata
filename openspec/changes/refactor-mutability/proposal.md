## Why

The current Vulgata mutability model makes mutation explicit with `set`, but it does not provide a true immutable-by-default programming model. This creates an awkward middle ground where `let` looks lightweight and stable while compound values remain broadly mutable, making the language harder to reason about and less aligned with a specification-oriented style.

## What Changes

- **BREAKING** Replace `set target = value` mutation syntax with `target := value`.
- **BREAKING** Introduce `var` for mutable local bindings and make `let` strictly immutable.
- **BREAKING** Make compound values bound through `let` non-writable, including record fields and indexed collection elements.
- Treat action parameters like immutable bindings; code that needs local mutation must copy parameter values into `var` locals first.
- Redefine aliasing expectations so mutations through `var` do not silently affect values visible through immutable `let` bindings.
- Update grammar, type-checking rules, semantic contracts, and conformance examples to reflect the new mutability model.

## Capabilities

### New Capabilities
- `immutable-bindings`: Define `let` as immutable, `var` as mutable, and require strict non-writability for compound values rooted in `let`.
- `explicit-mutation-syntax`: Define `:=` as the only reassignment and in-place update form, with writable targets restricted to roots declared with `var`.

### Modified Capabilities
<!-- None yet; there are no existing OpenSpec capability specs in openspec/specs/ to update. -->

## Impact

- Affected specs: the core language mutability, assignment, grammar, and semantic rules described in `spec/vulgata_spec_v0.3.md`.
- Affected code: lexer, parser, AST, type checker, TIR lowering, interpreter runtime, code generation, CLI examples, and conformance tests.
- Affected user APIs: all source code using `set` or relying on mutation through `let` bindings becomes incompatible and must be rewritten to `var` plus `:=`.
- Affected semantics: aliasing and writable-target behavior for records and collections become stricter and must be preserved consistently in interpreter and compiler modes.
