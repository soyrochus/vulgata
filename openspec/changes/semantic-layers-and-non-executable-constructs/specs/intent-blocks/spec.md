## ADDED Requirements

### Requirement: Intent block syntax
An `action` declaration MAY contain an `intent:` block as the first element of its body. The block SHALL accept four optional sub-fields: `goal` (Text), `constraints` (List[Text]), `assumptions` (List[Text]), and `properties` (List[Text]).

#### Scenario: Well-formed intent block is parsed
- **WHEN** the parser encounters an `intent:` block inside an `action` body with valid sub-fields
- **THEN** it SHALL produce an `IntentBlock` AST node attached to the enclosing `action`

#### Scenario: Intent block with only goal is valid
- **WHEN** an `intent:` block contains only a `goal:` field and no other sub-fields
- **THEN** the parser SHALL accept it without error

#### Scenario: Intent block outside action is rejected
- **WHEN** an `intent:` block appears outside an `action` body (e.g. at module level)
- **THEN** the parser SHALL report a syntax error

### Requirement: Intent block has no runtime effect
The interpreter SHALL skip `IntentBlock` nodes in all modes except tooling.

#### Scenario: Intent block does not affect return value
- **WHEN** an action containing an `intent:` block is called in release mode
- **THEN** the return value and side-effects SHALL be identical to the same action without the block

#### Scenario: Intent block visible in tooling mode
- **WHEN** the metadata emitter processes an action with an `intent:` block
- **THEN** the emitted JSON SHALL contain an `"intent"` object with all present sub-fields

### Requirement: Intent sub-field types
`goal` SHALL be a single Text literal. `constraints`, `assumptions`, and `properties` SHALL be lists of Text literals using the `- "..."` list syntax.

#### Scenario: List fields accept multiple entries
- **WHEN** `constraints:` contains three `- "..."` entries
- **THEN** the parser SHALL produce a list of three strings in the AST node

#### Scenario: Unknown sub-field is rejected
- **WHEN** an `intent:` block contains a sub-field name not in {goal, constraints, assumptions, properties}
- **THEN** the parser SHALL report a syntax error
