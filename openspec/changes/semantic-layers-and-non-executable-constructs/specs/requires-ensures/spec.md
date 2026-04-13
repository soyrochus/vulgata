## ADDED Requirements

### Requirement: Requires clause syntax
An `action` body MAY begin with one or more `requires <expr>` statements before any executable statements. `<expr>` SHALL be any valid boolean `Expr`.

#### Scenario: Requires clause is parsed
- **WHEN** the parser encounters `requires a >= 0` inside an action body
- **THEN** it SHALL produce a `RequiresClause` AST node containing the expression

#### Scenario: Multiple requires clauses are allowed
- **WHEN** an action has two `requires` lines
- **THEN** both SHALL be parsed as separate `RequiresClause` nodes

#### Scenario: Requires with non-boolean expression is a type error
- **WHEN** a `requires` clause contains an expression that does not resolve to Bool
- **THEN** the type-checker SHALL report a type error

### Requirement: Ensures clause syntax
An `action` body MAY contain one or more `ensures <expr>` statements. Within an `ensures` expression, the identifier `result` SHALL refer to the value returned by the action.

#### Scenario: Ensures clause is parsed
- **WHEN** the parser encounters `ensures result >= 0` in an action body
- **THEN** it SHALL produce an `EnsuresClause` AST node containing the expression

#### Scenario: Result binding in ensures
- **WHEN** an `ensures` expression references `result`
- **THEN** the interpreter SHALL bind `result` to the action's return value before evaluating the expression

### Requirement: Requires/ensures enforcement by mode
In `checked` and `debug` modes, `requires` clauses SHALL be evaluated before the action body executes. `ensures` clauses SHALL be evaluated after the action body completes and before the return value is yielded to the caller. In `release` mode both SHALL be skipped.

#### Scenario: Violated requires raises error in checked mode
- **WHEN** an action with `requires a >= 0` is called with `a = -1` in checked mode
- **THEN** the interpreter SHALL raise a runtime error identifying the failed requires clause

#### Scenario: Violated ensures raises error in checked mode
- **WHEN** an action returns a value that causes an `ensures` expression to evaluate to false in checked mode
- **THEN** the interpreter SHALL raise a runtime error identifying the failed ensures clause

#### Scenario: Violated requires is silent in release mode
- **WHEN** an action with `requires a >= 0` is called with `a = -1` in release mode
- **THEN** the interpreter SHALL NOT raise an error and SHALL execute the action body normally

#### Scenario: Requires/ensures appear in metadata
- **WHEN** the metadata emitter processes an action with requires/ensures clauses
- **THEN** the emitted JSON SHALL include `"requires"` and `"ensures"` arrays of expression strings
