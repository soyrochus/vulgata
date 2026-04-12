## ADDED Requirements

### Requirement: Local bindings define mutability explicitly
The system SHALL treat `let` as an immutable local binding form and SHALL treat `var` as a mutable local binding form.

#### Scenario: Mutable local can be reassigned
- **WHEN** a program declares `var retries = 0` and later evaluates `retries := retries + 1`
- **THEN** semantic analysis accepts the program and execution observes the updated value

#### Scenario: Immutable local cannot be reassigned
- **WHEN** a program declares `let retries = 0` and later evaluates `retries := retries + 1`
- **THEN** semantic analysis rejects the program because the assignment target is not rooted in a mutable binding

### Requirement: Action parameters behave as immutable bindings
The system SHALL treat action parameters as immutable bindings unless the program copies them into mutable locals explicitly.

#### Scenario: Parameter cannot be reassigned directly
- **WHEN** an action parameter `count` is used as the root of `count := count + 1`
- **THEN** semantic analysis rejects the program because parameters are not writable roots

#### Scenario: Parameter can be copied into mutable local
- **WHEN** an action copies parameter `count` into `var current = count` and later evaluates `current := current + 1`
- **THEN** semantic analysis accepts the program because the writable root is the mutable local `current`

### Requirement: Compound values through `let` are strictly non-writable
The system SHALL reject writes through a root declared with `let`, including record-field mutation and indexed collection updates.

#### Scenario: Record field update through immutable binding is rejected
- **WHEN** a program declares `let customer = Customer(email: "before")` and later evaluates `customer.email := "after"`
- **THEN** semantic analysis rejects the program because the writable path is rooted in an immutable binding

#### Scenario: Indexed collection update through immutable binding is rejected
- **WHEN** a program declares `let items = [1, 2, 3]` and later evaluates `items[0] := 99`
- **THEN** semantic analysis rejects the program because the writable path is rooted in an immutable binding

### Requirement: Immutable bindings do not observe hidden mutation through aliasing
The system SHALL preserve strict immutability for values visible through `let` even when the implementation uses internal sharing or heap-backed storage.

#### Scenario: Mutable update does not silently rewrite immutable view
- **WHEN** a program binds a compound value through `let frozen = ...` and also derives a mutable value that is later updated
- **THEN** execution preserves the observable value seen through `frozen` rather than exposing hidden mutation through aliasing
