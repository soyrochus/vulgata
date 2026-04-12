## ADDED Requirements

### Requirement: Extern declarations require explicit signatures
The system SHALL require extern actions to declare fully explicit parameter and return types before they can be used by interpreter or compiler workflows.

#### Scenario: Untyped extern declaration is rejected
- **WHEN** a source file declares an extern action without a complete signature required by the language specification
- **THEN** front-end validation fails before execution or compilation begins

### Requirement: Extern bindings are validated before use
The system SHALL validate configured extern bindings against declared extern signatures during setup and SHALL reject incompatible bindings before program execution.

#### Scenario: Incompatible binding fails during setup
- **WHEN** an extern declaration expects one signature and the configured binding provides an incompatible callable target
- **THEN** setup fails with a diagnostic describing the extern name and signature mismatch

### Requirement: Extern calls behave consistently across modes
The system SHALL preserve the same source-level calling semantics for extern actions in interpreter mode and compiler mode.

#### Scenario: Extern call returns equivalent result in both modes
- **WHEN** a program calls a configured extern action from valid source in interpreter mode and in compiled mode
- **THEN** both execution paths observe the same returned value shape and failure behavior for that call