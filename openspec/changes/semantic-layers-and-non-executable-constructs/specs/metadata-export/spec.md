## ADDED Requirements

### Requirement: Metadata emit flag
The `vulgata` CLI SHALL accept a `--emit-metadata <path>` flag. When present, after parsing and type-checking (but before or independent of execution), the tool SHALL write a JSON metadata document to `<path>`.

#### Scenario: Metadata is written to the specified path
- **WHEN** `vulgata run --emit-metadata out.json src/foo.vulgata` is invoked
- **THEN** the file `out.json` SHALL be created (or overwritten) with the metadata JSON

#### Scenario: Metadata emission does not affect execution
- **WHEN** `--emit-metadata` is specified alongside normal execution
- **THEN** the program's stdout, exit code, and side-effects SHALL be identical to running without the flag

#### Scenario: Missing metadata path is an error
- **WHEN** `--emit-metadata` is specified without a path argument
- **THEN** the CLI SHALL report an argument error and exit with a non-zero status

### Requirement: Metadata document structure
The metadata JSON document SHALL conform to the following top-level shape:

```json
{
  "module": "<module-name>",
  "actions": [ ... ]
}
```

Each action entry SHALL include `"name"`, and MAY include `"intent"`, `"contracts"` (`requires`/`ensures`), `"steps"`, and `"examples"` — only for constructs present in the source.

#### Scenario: Action with all semantic-layer constructs
- **WHEN** an action has intent, requires, ensures, step labels, and example blocks
- **THEN** the emitted JSON action entry SHALL contain all four keys (`intent`, `contracts`, `steps`, `examples`)

#### Scenario: Action with no semantic-layer constructs
- **WHEN** an action has no descriptive or checkable constructs
- **THEN** the emitted JSON action entry SHALL contain only `"name"` with no additional keys

### Requirement: Metadata is deterministic
Given the same source file, the metadata emitter SHALL always produce byte-identical JSON output.

#### Scenario: Repeated emission is stable
- **WHEN** `--emit-metadata` is run twice on the same unchanged source file
- **THEN** both output files SHALL be byte-identical

### Requirement: Metadata does not require execution
The metadata emitter SHALL operate on the AST alone and SHALL NOT require the interpreter to execute the program.

#### Scenario: Metadata emitted from program that would fail at runtime
- **WHEN** a syntactically and type-valid program that would panic at runtime is passed with `--emit-metadata`
- **THEN** the metadata SHALL be written successfully without executing the program
