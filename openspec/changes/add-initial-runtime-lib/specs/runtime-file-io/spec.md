## ADDED Requirements

### Requirement: Standard file module provides basic text file access
The system SHALL provide a standard `file` runtime module with `read_text`, `write_text`, `append_text`, and `exists` actions.

#### Scenario: Read text file
- **WHEN** a program calls `file.read_text(path)`
- **THEN** the runtime reads the full file contents at `path` as UTF-8 text

#### Scenario: Replace file contents
- **WHEN** a program calls `file.write_text(path, content)`
- **THEN** the runtime writes `content` to `path` and replaces any existing contents

#### Scenario: Append file contents
- **WHEN** a program calls `file.append_text(path, content)`
- **THEN** the runtime appends `content` to `path`, creating the file if the implementation supports that behavior

#### Scenario: Check file existence
- **WHEN** a program calls `file.exists(path)`
- **THEN** the runtime returns `true` when a file-system entry exists at `path` and `false` otherwise

### Requirement: File I/O reports failures explicitly
The system SHALL represent fallible file read/write/append operations through `Result` values with human-readable `Text` errors.

#### Scenario: Text file read succeeds
- **WHEN** `file.read_text(path)` succeeds
- **THEN** it returns `Ok(content)`

#### Scenario: Text file read fails
- **WHEN** `file.read_text(path)` cannot read the file, the file is missing, or the file is not valid UTF-8
- **THEN** it returns `Err(message)` where `message` is human-readable `Text`

#### Scenario: Text file write or append fails
- **WHEN** `file.write_text(path, content)` or `file.append_text(path, content)` cannot complete
- **THEN** the runtime returns `Err(message)` where `message` is human-readable `Text`

### Requirement: Paths are represented as text
The system SHALL represent runtime file paths as `Text` values interpreted according to the host environment.

#### Scenario: Relative path behavior
- **WHEN** a program passes a relative path to a standard file action
- **THEN** the runtime resolves it relative to the current working directory of the host process

#### Scenario: Absolute path behavior
- **WHEN** a program passes an absolute path to a standard file action
- **THEN** the runtime interprets it according to host platform path conventions
