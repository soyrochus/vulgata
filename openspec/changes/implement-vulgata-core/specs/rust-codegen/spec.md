## ADDED Requirements

### Requirement: Typed programs compile to auditable Rust
The system SHALL lower validated typed intermediate representation into ordinary Rust source code that represents Vulgata types, declarations, and control flow explicitly enough for a developer to inspect the generated program.

#### Scenario: Compilation emits Rust source for a valid module
- **WHEN** a valid Vulgata module is compiled in compiler mode
- **THEN** the compiler writes Rust source that includes generated Rust items corresponding to the Vulgata program structure

### Requirement: Generated Rust preserves runtime semantics
The system SHALL generate Rust code whose observable execution behavior matches interpreter-mode execution for supported language features.

#### Scenario: Compiled program matches interpreter output
- **WHEN** the same valid Vulgata program is run through interpreter mode and through compiler mode followed by Rust compilation and execution
- **THEN** both execution paths produce the same program result and test outcomes

### Requirement: Generated code avoids interpreter runtime dependency
The system SHALL emit only the minimal support code required by the compiled program and SHALL NOT require linking against the interpreter runtime to execute compiled output.

#### Scenario: Compiled output builds without interpreter runtime
- **WHEN** compiler mode emits Rust for a supported program
- **THEN** the emitted Rust can be built and run using the Rust toolchain without importing the interpreter execution engine