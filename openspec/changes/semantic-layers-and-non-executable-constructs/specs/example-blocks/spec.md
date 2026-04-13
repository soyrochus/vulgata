## ADDED Requirements

### Requirement: Example block syntax
An `action` body MAY contain one or more `example <name>:` blocks. Each block SHALL contain an `input:` sub-block and an `output:` sub-block. Each sub-block SHALL contain one or more `<param> = <literal>` bindings.

#### Scenario: Well-formed example block is parsed
- **WHEN** the parser encounters an `example gcd_basic:` block with `input:` and `output:` sub-blocks
- **THEN** it SHALL produce an `ExampleBlock` AST node with the name, input bindings, and expected output bindings

#### Scenario: Example block requires both input and output
- **WHEN** an `example` block contains only `input:` and no `output:`
- **THEN** the parser SHALL report a syntax error

#### Scenario: Example name must be an identifier
- **WHEN** an `example` block has a name that is not a valid identifier
- **THEN** the parser SHALL report a syntax error

### Requirement: Example block execution by mode
In `checked` and `debug` modes, `example` blocks SHALL be executed as tests: the interpreter SHALL call the enclosing action with the `input:` bindings and compare the result to the `output:` bindings. In `release` and `tooling` modes, `example` blocks SHALL be skipped.

#### Scenario: Passing example produces no output in checked mode
- **WHEN** an `example` block's expected output matches the actual return value in checked mode
- **THEN** the interpreter SHALL continue without error

#### Scenario: Failing example raises error in checked mode
- **WHEN** an `example` block's expected output does not match the actual return value in checked mode
- **THEN** the interpreter SHALL raise a runtime error naming the example and showing expected vs actual values

#### Scenario: Example block is skipped in release mode
- **WHEN** an action containing `example` blocks runs in release mode
- **THEN** none of the example inputs SHALL be evaluated and no comparison SHALL occur

#### Scenario: Examples appear in metadata
- **WHEN** the metadata emitter processes an action with `example` blocks
- **THEN** the emitted JSON SHALL include an `"examples"` array with each example's name, inputs, and outputs
