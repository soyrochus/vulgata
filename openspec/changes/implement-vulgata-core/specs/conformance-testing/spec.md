## ADDED Requirements

### Requirement: Shared conformance fixtures cover language phases
The system SHALL maintain conformance fixtures that exercise parsing, type checking, interpreter execution, compiler execution, and semantic equivalence for each supported language feature.

#### Scenario: New feature adds fixture coverage
- **WHEN** support for a new language feature is added to the implementation
- **THEN** the conformance suite includes at least one fixture that validates that feature through the relevant phases

### Requirement: Conformance suite checks cross-mode equivalence
The system SHALL compare interpreter-mode and compiler-mode results for fixtures where both execution paths are supported.

#### Scenario: Execution mismatch fails conformance
- **WHEN** interpreter mode and compiled execution produce different outputs or test results for the same conformance fixture
- **THEN** the conformance suite reports the fixture as failed due to semantic divergence

### Requirement: Diagnostics fixtures are testable artifacts
The system SHALL support fixtures that assert expected diagnostics for invalid programs, including the failing phase and source location.

#### Scenario: Invalid fixture expects a type diagnostic
- **WHEN** a conformance fixture contains an invalid program with a declared expected type-check failure
- **THEN** the test harness verifies that type checking fails and that the reported diagnostic identifies the expected location or phase