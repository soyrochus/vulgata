## ADDED Requirements

### Requirement: Canonical source parsing pipeline
The system SHALL parse canonical indentation-based Vulgata source files into an abstract syntax tree and SHALL lower valid programs into a typed intermediate representation through explicit lexical analysis, parsing, name resolution, and type checking phases.

#### Scenario: Valid module lowers successfully
- **WHEN** a source file contains valid module declarations, imports, declarations, statements, and expressions from the supported v0.2 subset
- **THEN** the front-end produces a typed intermediate representation with resolved symbols, concrete types, and source spans for each lowered construct

### Requirement: Front-end diagnostics are phase-specific
The system SHALL reject invalid source during the earliest failing front-end phase and SHALL report diagnostics that identify the source span, failing phase, and error message.

#### Scenario: Type error is reported with phase context
- **WHEN** a program parses successfully but assigns a `Text` value to a variable declared as `Int`
- **THEN** the front-end fails during type checking and reports a diagnostic that includes the source location and a message describing the type mismatch