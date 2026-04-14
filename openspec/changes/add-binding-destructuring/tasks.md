## 1. Parser and AST Surface

- [ ] 1.1 Extend `let` and `var` syntax to accept tuple and nominal record binding patterns
- [ ] 1.2 Add dedicated AST nodes for declaration binding patterns instead of overloading single-name declarations
- [ ] 1.3 Reject unsupported declaration forms in phase 1, including enum destructuring, nested destructuring, wildcard destructuring, and destructuring in `:=`
- [ ] 1.4 Add parser tests for valid tuple/record destructuring and invalid unsupported forms

## 2. Type-Checking and Lowering

- [ ] 2.1 Add type-checking for tuple destructuring arity, nominal record-type matching, record field existence, and duplicate binding names
- [ ] 2.2 Ensure destructured names inherit mutability from `let` versus `var`, and that whole-pattern annotations apply to the initializer type
- [ ] 2.3 Add typed lowering for destructuring declarations that preserves one-time initializer evaluation

## 3. Runtime and Compile Semantics

- [ ] 3.1 Implement interpreter support for destructuring declarations with one-time evaluation of the initializer
- [ ] 3.2 Ensure destructured bindings are values rather than writable aliases back into the original tuple or record
- [ ] 3.3 Implement Rust codegen for tuple and record destructuring declarations with equivalent value semantics

## 4. Verification and Conformance

- [ ] 4.1 Add interpreter tests for tuple and record destructuring in both `let` and `var`
- [ ] 4.2 Add tests proving mutation of a destructured `var` binding does not mutate the original source value
- [ ] 4.3 Add conformance fixtures covering valid destructuring and rejected `:=` destructuring
