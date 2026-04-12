## ADDED Requirements

### Requirement: Standard console module provides basic text output
The system SHALL provide a standard `console` runtime module with `print`, `println`, `eprint`, and `eprintln` actions for basic text output.

#### Scenario: Standard output without newline
- **WHEN** a program calls `console.print(value)`
- **THEN** the runtime writes `value` to standard output without appending a newline

#### Scenario: Standard output with newline
- **WHEN** a program calls `console.println(value)`
- **THEN** the runtime writes `value` to standard output followed by a line terminator

#### Scenario: Standard error output
- **WHEN** a program calls `console.eprint(value)` or `console.eprintln(value)`
- **THEN** the runtime writes `value` to standard error, with `eprintln` appending a line terminator and `eprint` not appending one

### Requirement: Console output reports failures explicitly
The system SHALL represent console output failure through `Result[None, Text]` values.

#### Scenario: Console write succeeds
- **WHEN** a console output action completes successfully
- **THEN** it returns `Ok(None)`

#### Scenario: Console write fails
- **WHEN** a console output action cannot write to its target stream
- **THEN** it returns `Err(message)` where `message` is human-readable `Text`

### Requirement: Standard console module provides line input
The system SHALL provide `console.read_line() -> Result[Text, Text]` for basic line-oriented input.

#### Scenario: Line read succeeds
- **WHEN** a program calls `console.read_line()` and one line of input is available
- **THEN** the runtime returns `Ok(line)` where `line` excludes the trailing newline

#### Scenario: End of input is explicit
- **WHEN** a program calls `console.read_line()` after input has closed before a line is read
- **THEN** the runtime returns `Err(message)` rather than using an implicit sentinel value for end-of-input
