## ADDED Requirements

### Requirement: Meaning annotation syntax
A `record` field declaration MAY include a `meaning: "<text>"` annotation on the following indented line. The annotation SHALL be a single Text literal.

#### Scenario: Field with meaning annotation is parsed
- **WHEN** the parser encounters a field declaration followed by `meaning: "some text"`
- **THEN** it SHALL store the text in `FieldDecl.meaning` as `Some(String)`

#### Scenario: Field without meaning annotation has None
- **WHEN** a field declaration has no `meaning:` line
- **THEN** `FieldDecl.meaning` SHALL be `None`

#### Scenario: Meaning annotation on non-field is rejected
- **WHEN** a `meaning:` annotation appears outside a record field context
- **THEN** the parser SHALL report a syntax error

### Requirement: Meaning annotation has no runtime effect
The interpreter SHALL treat a `meaning:` annotation as metadata only with no effect on execution in any mode.

#### Scenario: Record with meaning behaves identically to record without
- **WHEN** a record field has a `meaning:` annotation and the record is constructed and accessed
- **THEN** the field value SHALL be identical to the same field without an annotation

#### Scenario: Meaning is emitted in metadata
- **WHEN** the metadata emitter processes a record whose fields have `meaning:` annotations
- **THEN** the emitted JSON SHALL include a `"meaning"` string for each annotated field
