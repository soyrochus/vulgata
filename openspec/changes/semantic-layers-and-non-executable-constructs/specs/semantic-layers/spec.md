## ADDED Requirements

### Requirement: Three-layer taxonomy
Every Vulgata construct SHALL belong to exactly one of three semantic layers: executable, checkable, or descriptive. The interpreter and compiler SHALL enforce this classification and SHALL NOT allow a construct to span layers.

#### Scenario: Executable construct always runs
- **WHEN** the interpreter executes any program in any mode
- **THEN** all executable constructs in the program SHALL be evaluated

#### Scenario: Descriptive construct never runs
- **WHEN** the interpreter executes any program in release, checked, or debug mode
- **THEN** all descriptive constructs SHALL be skipped without producing any side-effect or output

#### Scenario: Checkable construct is mode-dependent
- **WHEN** the interpreter executes a program in checked or debug mode
- **THEN** all checkable constructs SHALL be enforced

#### Scenario: Checkable construct removed in release
- **WHEN** the interpreter executes a program in release mode
- **THEN** all checkable constructs SHALL be skipped

### Requirement: Four execution modes
The Vulgata runtime SHALL support four named execution modes: `release`, `checked`, `debug`, and `tooling`.

#### Scenario: Default mode for CLI
- **WHEN** the `vulgata run` command is invoked without a `--mode` flag
- **THEN** the interpreter SHALL run in `release` mode

#### Scenario: Default mode for REPL
- **WHEN** the `vulgata repl` command is invoked
- **THEN** the interpreter SHALL run in `tooling` mode

#### Scenario: Explicit mode selection
- **WHEN** the user invokes `vulgata run --mode checked <file>`
- **THEN** the interpreter SHALL run in `checked` mode

#### Scenario: Invalid mode is rejected
- **WHEN** the user passes an unrecognised value to `--mode`
- **THEN** the CLI SHALL report an error and exit with a non-zero status

### Requirement: Layer behaviour matrix
The runtime SHALL implement the following behaviour for each layer/mode combination:

| Layer       | release  | checked  | debug    | tooling  |
|-------------|----------|----------|----------|----------|
| Executable  | run      | run      | run      | run      |
| Checkable   | skip     | enforce  | enforce  | expose   |
| Descriptive | skip     | skip     | skip     | expose   |

#### Scenario: Release strips all non-executable constructs
- **WHEN** a program containing intent blocks, requires clauses, and example blocks runs in release mode
- **THEN** none of those constructs SHALL produce output, side-effects, or failures

#### Scenario: Checked mode enforces contracts
- **WHEN** a program containing `requires`/`ensures` clauses runs in checked mode and a clause evaluates to false
- **THEN** the interpreter SHALL raise a runtime error identifying the violated clause
