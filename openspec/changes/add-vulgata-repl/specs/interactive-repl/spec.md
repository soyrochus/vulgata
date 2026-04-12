## ADDED Requirements

### Requirement: CLI provides an interactive REPL mode
The system SHALL provide a `vulgata repl` CLI subcommand that starts an interactive Vulgata session.

#### Scenario: REPL starts successfully
- **WHEN** a user runs `vulgata repl` and session initialization succeeds
- **THEN** the process enters an interactive session rather than requiring a source file path

#### Scenario: REPL initialization fails
- **WHEN** REPL terminal or session initialization cannot be completed
- **THEN** the CLI reports a diagnostic and exits with a nonzero status

### Requirement: REPL sessions are backed by a virtual source buffer
The system SHALL model a REPL session as one accumulated virtual Vulgata module with a stable synthetic path used for diagnostics.

#### Scenario: Accepted block extends session source
- **WHEN** a user submits a valid top-level source block
- **THEN** the block is appended to the session’s accumulated source buffer

#### Scenario: Session source is inspectable
- **WHEN** a user runs `:show`
- **THEN** the REPL prints the current accumulated source exactly as it will be analyzed

### Requirement: REPL block acceptance is transactional
The system SHALL validate candidate source blocks against the full accumulated session source before mutating session state.

#### Scenario: Invalid block is rejected
- **WHEN** a submitted source block causes parse or semantic failure in the combined session source
- **THEN** the REPL prints diagnostics and leaves the prior session buffer unchanged

#### Scenario: Valid block is committed
- **WHEN** a submitted source block parses and checks successfully in the context of the current session source
- **THEN** the REPL updates the session buffer to include the new block

### Requirement: REPL commands reuse the shared execution pipeline
The system SHALL run REPL parse, check, run, and test operations through the same shared Vulgata front-end and interpreter pipeline used by file-based commands.

#### Scenario: Check command uses shared semantics
- **WHEN** a user runs `:check`
- **THEN** the REPL performs semantic checking on the current session source and reports success or diagnostics using the ordinary pipeline

#### Scenario: Run command uses main action contract
- **WHEN** a user runs `:run`
- **THEN** the REPL requires a zero-parameter `action main()` and displays the result using the same runtime formatting as file-based execution

#### Scenario: Test command uses interpreter-backed tests
- **WHEN** a user runs `:test`
- **THEN** the REPL executes tests against the current session source and reports `PASS`/`FAIL` results in the same style as the existing test command

### Requirement: REPL provides core session control commands
The system SHALL provide REPL meta-commands for help, parse, check, run, test, show, reset, and quit.

#### Scenario: Help command lists commands
- **WHEN** a user runs `:help`
- **THEN** the REPL displays the available meta-commands and a short usage summary

#### Scenario: Reset command clears session source
- **WHEN** a user runs `:reset`
- **THEN** the REPL clears the accumulated session source and returns to an empty session state

#### Scenario: Quit command exits cleanly
- **WHEN** a user runs `:quit`
- **THEN** the REPL terminates the session cleanly
