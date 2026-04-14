## ADDED Requirements

### Requirement: Statement-form match
The system SHALL support a statement-form `match <expr>:` construct. A match statement SHALL evaluate its target expression exactly once, test arms in source order, execute the first matching arm, and provide no fallthrough between arms.

#### Scenario: First matching arm executes
- **WHEN** a match statement has multiple arms and more than one pattern could match the target
- **THEN** the system executes only the first matching arm and skips the remaining arms

### Requirement: Phase-1 match patterns
The system SHALL support the following match pattern forms in phase 1: wildcard `_`, literal patterns, binding identifiers, enum-variant patterns, tuple patterns, and nominal name-based record patterns.

#### Scenario: Result variant pattern binds payload
- **WHEN** a `Result[T, E]` value is matched against `Ok(value)` or `Err(error)`
- **THEN** the system matches the correct variant and binds the payload names only inside the selected arm

#### Scenario: Tuple pattern matches by arity
- **WHEN** a tuple value is matched against a tuple pattern such as `(left, right)`
- **THEN** the system matches only when the tuple arity equals the pattern arity

#### Scenario: Record pattern matches nominal type and named fields
- **WHEN** a record value is matched against `Customer(name: n, active: false)`
- **THEN** the system matches only when the value is a `Customer` record and each named field pattern matches the corresponding field value

### Requirement: Match type-checking and arm bindings
The type-checker SHALL validate match statements. It SHALL reject incompatible literal patterns, tuple arity mismatches, unknown enum variants, wrong nominal record types, unknown record fields, and duplicate binding names within a single pattern. Names introduced by a pattern SHALL be immutable and scoped only to the selected arm body.

#### Scenario: Duplicate binding names are rejected
- **WHEN** a single match pattern introduces the same binding name more than once
- **THEN** the type-checker rejects the program with a type error

#### Scenario: Arm binding does not escape arm scope
- **WHEN** a name is introduced by a pattern in one match arm
- **THEN** that name is available only inside that arm body and is not visible after the match statement

### Requirement: Non-exhaustive match is a defined runtime failure
The runtime SHALL raise a named runtime error `NonExhaustiveMatch` when no arm matches the evaluated target value. The generated Rust code SHALL preserve the same behavior.

#### Scenario: Match with no matching arm fails explicitly
- **WHEN** a match statement has no wildcard or otherwise matching arm for the target value
- **THEN** execution fails with `NonExhaustiveMatch` instead of falling through silently or producing undefined behavior
