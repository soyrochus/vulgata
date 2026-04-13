## ADDED Requirements

### Requirement: Explain block syntax
An `explain:` block MAY appear as a statement inside an `action` body. It SHALL contain one or more Text literal lines forming a human-readable description of the surrounding logic.

#### Scenario: Explain block inside action is parsed
- **WHEN** the parser encounters `explain:` followed by an indented block of Text literals
- **THEN** it SHALL produce an `ExplainBlock` AST node containing the joined text

#### Scenario: Multiple text lines are joined
- **WHEN** an `explain:` block contains three indented Text lines
- **THEN** the `ExplainBlock` node SHALL store all three lines

#### Scenario: Explain block at module level is rejected
- **WHEN** an `explain:` block appears at module top level outside any action
- **THEN** the parser SHALL report a syntax error

### Requirement: Explain block has no runtime effect
The interpreter SHALL skip `ExplainBlock` nodes in all execution modes.

#### Scenario: Action output unchanged by explain block
- **WHEN** an action contains one or more `explain:` blocks and is executed in any mode
- **THEN** return value, side-effects, and stdout output SHALL be identical to the same action without explain blocks

#### Scenario: Explain text appears in metadata
- **WHEN** the metadata emitter processes an action containing `explain:` blocks
- **THEN** the emitted JSON SHALL include an `"explain"` array with the text of each block in source order
