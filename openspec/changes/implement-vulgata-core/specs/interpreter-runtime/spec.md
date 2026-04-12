## ADDED Requirements

### Requirement: Interpreter executes typed programs
The system SHALL execute validated typed intermediate representation in interpreter mode for supported declarations, statements, expressions, and tests without requiring compilation to Rust.

#### Scenario: Action execution returns computed value
- **WHEN** a valid program defines an action that performs arithmetic and control flow and the interpreter runs that action
- **THEN** the interpreter returns the value required by the typed program semantics

### Requirement: Interpreter preserves explicit mutation semantics
The system SHALL preserve explicit mutation for variables, record fields, and indexed collection targets according to the language's managed value model.

#### Scenario: Record field mutation is observable
- **WHEN** a program mutates a record field with `set` and later reads that same field in the same execution
- **THEN** the interpreter observes the updated value rather than the original value

### Requirement: Interpreter executes top-level tests
The system SHALL execute top-level `test` blocks and SHALL mark expectations as passed or failed based on evaluated program results.

#### Scenario: Failed expectation is reported as a test failure
- **WHEN** a test block contains an `expect` expression that evaluates to `false`
- **THEN** the interpreter reports the test as failed with the expectation's source location