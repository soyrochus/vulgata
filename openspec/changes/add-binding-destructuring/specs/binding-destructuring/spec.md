## ADDED Requirements

### Requirement: Tuple destructuring in let and var
The system SHALL allow tuple destructuring in `let` and `var` declarations. The initializer SHALL be evaluated exactly once, and the tuple arity SHALL match the number of declared names.

#### Scenario: Let tuple destructuring binds immutable names
- **WHEN** a program evaluates `let (a, b) = pair`
- **THEN** the system binds `a` and `b` as immutable names to the corresponding tuple elements from a single evaluation of `pair`

#### Scenario: Var tuple destructuring binds mutable names
- **WHEN** a program evaluates `var (x, y) = get_coordinates()`
- **THEN** the system binds `x` and `y` as mutable names to the corresponding tuple elements from a single evaluation of the initializer

### Requirement: Record destructuring in let and var
The system SHALL allow name-based nominal record destructuring in `let` and `var` declarations. The right-hand side SHALL be a value of the named record type, and only the named fields SHALL be extracted.

#### Scenario: Let record destructuring extracts selected fields
- **WHEN** a program evaluates `let Customer(name: n, email: e) = customer`
- **THEN** the system binds `n` and `e` to the selected fields of the `Customer` value

#### Scenario: Record destructuring rejects wrong nominal type
- **WHEN** a record destructuring declaration names a record type different from the initializer’s actual nominal type
- **THEN** the type-checker rejects the declaration

### Requirement: Destructuring declarations remain narrower than match patterns
The system SHALL restrict declaration destructuring to tuple and record binding forms with identifier outputs only. It SHALL reject enum destructuring, nested destructuring, wildcard destructuring, and destructuring in `:=`.

#### Scenario: Assignment destructuring is rejected
- **WHEN** a program uses `(a, b) := pair` or `Customer(name: n) := customer`
- **THEN** parsing or earlier syntax validation rejects the statement instead of treating it as a writable target

#### Scenario: Enum destructuring in let is rejected
- **WHEN** a program attempts to use an enum-style declaration pattern such as `let Some(value) = maybe`
- **THEN** the frontend rejects the declaration as unsupported in phase 1

### Requirement: Destructured bindings are values, not writable aliases
The system SHALL treat names introduced by destructuring declarations as ordinary bindings containing extracted values. Mutating a destructured `var` binding SHALL NOT mutate the original tuple or record value.

#### Scenario: Mutating destructured var does not update source record
- **WHEN** a program evaluates `var Customer(name: n) = customer` and later mutates `n`
- **THEN** the original `customer.name` value remains unchanged
