## MODIFIED Requirements

### Requirement: Expect belongs to the checkable layer
The `expect` statement is formally classified as a **checkable layer** construct. Its execution SHALL obey the execution-mode rules defined for the checkable layer: enforced in `checked` and `debug` modes, removed in `release` mode.

Previously `expect` was always enforced regardless of mode. With v0.5 it becomes mode-dependent.

#### Scenario: Expect is enforced in checked mode
- **WHEN** `expect false` appears in an action body and the program runs in `checked` mode
- **THEN** the interpreter SHALL raise a runtime error

#### Scenario: Expect is enforced in debug mode
- **WHEN** `expect false` appears in an action body and the program runs in `debug` mode
- **THEN** the interpreter SHALL raise a runtime error

#### Scenario: Expect is silent in release mode
- **WHEN** `expect false` appears in an action body and the program runs in `release` mode
- **THEN** the interpreter SHALL skip the expect statement without raising an error

#### Scenario: Expect behaviour unchanged in checked mode vs pre-v0.5
- **WHEN** an existing program using `expect` is run in `checked` mode
- **THEN** its behaviour SHALL be identical to running the same program in pre-v0.5 (where expect was always enforced)
