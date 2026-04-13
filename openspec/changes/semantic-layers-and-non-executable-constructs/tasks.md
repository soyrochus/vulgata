## 1. Execution Mode

- [x] 1.1 Add `ExecutionMode` enum (`Release`, `Checked`, `Debug`, `Tooling`) to `src/runtime.rs`
- [x] 1.2 Add `mode: ExecutionMode` field to the `Interpreter` struct
- [x] 1.3 Add `--mode <release|checked|debug|tooling>` CLI flag in `src/cli.rs` and wire it to `Interpreter`
- [x] 1.4 Set default mode to `Release` for `vulgata run` and `Tooling` for `vulgata repl` in `src/main.rs` / `src/repl.rs`
- [x] 1.5 Report an error and exit non-zero when an unrecognised `--mode` value is passed

## 2. Lexer — New Keywords

- [x] 2.1 Add token variants to `Token` enum in `src/lexer.rs`: `Intent`, `Meaning`, `Explain`, `Step`, `Requires`, `Ensures`, `Example`, `Goal`, `Constraints`, `Assumptions`, `Properties`, `Input`, `Output`
- [x] 2.2 Map each new keyword string to its token variant in the lexer keyword table
- [x] 2.3 Verify that existing identifier tests still pass after keyword additions

## 3. AST — New Nodes

- [x] 3.1 Add `IntentBlock` variant to `StmtKind` with fields `goal: Option<String>`, `constraints: Vec<String>`, `assumptions: Vec<String>`, `properties: Vec<String>`
- [x] 3.2 Add `ExplainBlock` variant to `StmtKind` with field `lines: Vec<String>`
- [x] 3.3 Add `StepBlock` variant to `StmtKind` with fields `label: String`, `body: Vec<Stmt>`
- [x] 3.4 Add `RequiresClause` variant to `StmtKind` with field `condition: Expr`
- [x] 3.5 Add `EnsuresClause` variant to `StmtKind` with field `condition: Expr`
- [x] 3.6 Add `ExampleBlock` variant to `StmtKind` with fields `name: String`, `inputs: Vec<(String, Expr)>`, `outputs: Vec<(String, Expr)>`
- [x] 3.7 Add `meaning: Option<String>` field to `FieldDecl` in `src/ast.rs`

## 4. Parser — New Syntax

- [x] 4.1 Parse `intent:` block inside action body producing `StmtKind::IntentBlock`; accept `goal`, `constraints`, `assumptions`, `properties` sub-fields; reject unknown sub-fields
- [x] 4.2 Parse `meaning: "<text>"` line after a record field declaration and store in `FieldDecl.meaning`
- [x] 4.3 Parse `explain:` block inside action body producing `StmtKind::ExplainBlock`; collect indented Text lines
- [x] 4.4 Parse `step <name>:` block inside action body producing `StmtKind::StepBlock`; require non-empty body
- [x] 4.5 Parse `requires <expr>` statement inside action body producing `StmtKind::RequiresClause`
- [x] 4.6 Parse `ensures <expr>` statement inside action body producing `StmtKind::EnsuresClause`; allow `result` as a valid identifier within the expression
- [x] 4.7 Parse `example <name>:` block inside action body producing `StmtKind::ExampleBlock`; require both `input:` and `output:` sub-blocks with `<name> = <literal>` bindings
- [x] 4.8 Reject `intent:`, `explain:`, `step`, `requires`, `ensures`, `example` outside action bodies with a syntax error

## 5. Resolver — Skip New Nodes

- [x] 5.1 Add catch-all arms in `src/resolver.rs` to skip `IntentBlock`, `ExplainBlock`, `StepBlock`, `RequiresClause`, `EnsuresClause`, `ExampleBlock` without resolving them as executable statements
- [x] 5.2 Type-check the condition expression in `RequiresClause` and `EnsuresClause` — must resolve to `Bool`; report a type error otherwise
- [x] 5.3 Ensure `result` is accepted as a valid identifier inside `EnsuresClause` expressions during type resolution

## 6. Interpreter — Layer Dispatch

- [x] 6.1 Skip `IntentBlock`, `ExplainBlock`, and `MeaningAnnotation` nodes unconditionally in all modes
- [x] 6.2 Execute `StepBlock` body transparently in all modes; in `Debug` mode emit the step label to stderr before executing the body
- [x] 6.3 In `Release` mode skip `RequiresClause`, `EnsuresClause`, and `ExampleBlock`
- [x] 6.4 In `Checked` and `Debug` modes evaluate `RequiresClause` conditions before the action body; raise a named runtime error if any evaluates to false
- [x] 6.5 In `Checked` and `Debug` modes inject a `result` binding with the action's return value and evaluate each `EnsuresClause`; raise a named runtime error if any evaluates to false
- [x] 6.6 In `Checked` and `Debug` modes execute `ExampleBlock` tests: call the enclosing action with `input:` bindings and compare to `output:` bindings; raise a named error on mismatch showing expected vs actual
- [x] 6.7 Change `expect` handling: skip in `Release` mode, enforce in `Checked` and `Debug` modes (matches existing behaviour for `Checked`/`Debug`; new behaviour for `Release`)

## 7. Metadata Emitter

- [x] 7.1 Create `src/metadata.rs` with a public function `emit_metadata(module: &AstModule) -> serde_json::Value`
- [x] 7.2 Emit top-level `"module"` and `"actions"` keys
- [x] 7.3 For each action, emit `"name"`; conditionally emit `"intent"`, `"contracts"` (`requires`/`ensures` as string arrays), `"steps"`, `"examples"` only when present
- [x] 7.4 Include `"meaning"` for record fields that have the annotation, under a `"records"` top-level key
- [x] 7.5 Add `--emit-metadata <path>` flag to the CLI; invoke `emit_metadata` and write JSON to the specified path; do not require program execution
- [x] 7.6 Ensure repeated invocations on unchanged source produce byte-identical output (deterministic key order)

## 8. Tests

- [x] 8.1 Add parser tests for each new construct (intent, explain, step, requires, ensures, example, meaning) — valid and invalid inputs
- [x] 8.2 Add interpreter tests confirming descriptive constructs have no effect on return values in all modes
- [x] 8.3 Add interpreter tests for `requires` enforcement: passes in release, fails correctly in checked mode
- [x] 8.4 Add interpreter tests for `ensures` enforcement with `result` binding
- [x] 8.5 Add interpreter tests for `example` block: passing case, failing case, skipped in release
- [x] 8.6 Add interpreter test confirming `expect false` is silent in release mode
- [x] 8.7 Add metadata emitter tests: action with all constructs, action with none, determinism check
