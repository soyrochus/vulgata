## Why

Vulgata currently requires compound values to be unpacked one field or element at a time after binding, which makes tuple- and record-oriented code verbose and obscures intent. Narrow destructuring in `let` and `var` declarations improves readability and local data flow while preserving the language's simple mutation model.

## What Changes

- Extend `let` and `var` declarations to accept binding destructuring patterns instead of only a single identifier.
- Support first-phase tuple destructuring in declarations, with arity checks and one-time evaluation of the initializer.
- Support first-phase name-based nominal record destructuring in declarations, extracting only the named fields.
- Keep declaration destructuring intentionally narrower than general pattern matching: no enum destructuring, no nested destructuring, no wildcard `_`, and no destructuring in `:=`.
- Preserve current value semantics so destructured names are ordinary bindings, not writable aliases back into tuples or records.
- Add frontend, runtime/codegen lowering, and conformance coverage for tuple and record destructuring in declarations.

## Capabilities

### New Capabilities
- `binding-destructuring`: Tuple and nominal record destructuring in `let` and `var` declarations, including scope, mutability of introduced bindings, type-checking, and value-copy semantics.

### Modified Capabilities

## Impact

- `src/parser.rs` and `src/ast.rs`: extend declaration syntax and add binding-pattern nodes for tuple and record destructuring.
- `src/types.rs`: validate tuple arity, record type/field compatibility, duplicate names, and declaration-local binding mutability.
- `src/tir.rs`: lower destructuring declarations to explicit binding-pattern IR or equivalent normalized statements.
- `src/runtime.rs`: evaluate initializers once and bind extracted values without creating writable aliases into the original compound value.
- `src/codegen.rs`: lower destructuring declarations to Rust bindings while preserving Vulgata’s copy/value semantics.
- `tests/` and `tests/conformance/`: add tuple and record destructuring coverage plus rejection cases for unsupported forms and `:=` destructuring.
