## ADDED Requirements

### Requirement: Reassignment and in-place update use `:=`
The system SHALL use `:=` as the only reassignment and in-place mutation syntax for writable targets.

#### Scenario: Local reassignment uses `:=`
- **WHEN** a program mutates a writable local binding
- **THEN** the source uses the form `target := expr` rather than `set target = expr`

#### Scenario: Field and index updates use `:=`
- **WHEN** a program mutates a writable record field or indexed collection element
- **THEN** the source uses the form `target := expr` with the field or index expression as the writable target

### Requirement: Legacy `set` mutation syntax is not part of v0.3
The system SHALL reject `set target = value` as canonical v0.3 mutation syntax.

#### Scenario: Legacy `set` form is rejected
- **WHEN** a v0.3 source file contains `set total = total + 1`
- **THEN** parsing or earlier syntax validation fails instead of accepting the statement as a writable update

### Requirement: Writability is rooted in declaration-time mutability
The system SHALL permit `:=` only when the target root is declared writable through `var`.

#### Scenario: Mutable root allows field update
- **WHEN** a program declares `var customer = Customer(email: "before")` and later evaluates `customer.email := "after"`
- **THEN** semantic analysis accepts the update because the target root is mutable

#### Scenario: Immutable root rejects syntactically valid update
- **WHEN** a program declares `let customer = Customer(email: "before")` and later evaluates `customer.email := "after"`
- **THEN** parsing succeeds but semantic analysis rejects the update because the target root is immutable

### Requirement: Semantic examples and frozen subset use `var` plus `:=`
The system SHALL express the v0.3 mutable subset using `var` declarations and `:=` updates in its canonical examples and conformance surface.

#### Scenario: Mutable algorithm example uses canonical syntax
- **WHEN** the specification or conformance suite includes an algorithm that updates loop state, accumulator values, or partition indices
- **THEN** the example uses `var` for mutable locals and `:=` for each reassignment
