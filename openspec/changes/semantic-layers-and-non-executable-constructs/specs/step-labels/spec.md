## ADDED Requirements

### Requirement: Step block syntax
A `step <name>:` construct MAY appear as a statement inside an `action` body. It SHALL introduce an indented block of statements. `<name>` SHALL be an identifier.

#### Scenario: Step block with body is parsed
- **WHEN** the parser encounters `step iterate:` followed by an indented statement block
- **THEN** it SHALL produce a `StepBlock` AST node with `label = "iterate"` and the inner statements as `body`

#### Scenario: Step block must have a body
- **WHEN** a `step <name>:` line has no indented body
- **THEN** the parser SHALL report a syntax error

#### Scenario: Nested step blocks are allowed
- **WHEN** a `step` block body contains another `step` block
- **THEN** the parser SHALL accept it without error

### Requirement: Step block executes its body transparently
The interpreter SHALL execute the inner body of a `StepBlock` identically to the same statements without the step wrapper. The label SHALL NOT affect execution in release, checked, or tooling modes.

#### Scenario: Step block produces same result as bare block
- **WHEN** an action uses `step` to wrap statements that compute a value
- **THEN** the return value SHALL equal the value produced by the same statements without the step wrapper

#### Scenario: Step label is emitted as trace in debug mode
- **WHEN** an action containing `step` blocks runs in debug mode
- **THEN** the interpreter SHALL emit the step label to stderr before executing the block's statements

#### Scenario: Step labels appear in metadata
- **WHEN** the metadata emitter processes an action with `step` blocks
- **THEN** the emitted JSON SHALL include a `"steps"` array listing all step labels in source order
