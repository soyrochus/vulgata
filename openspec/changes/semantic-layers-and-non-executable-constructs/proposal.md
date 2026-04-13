## Why

Vulgata v0.4 is purely executable — every construct either runs or errors. To fulfill its role as a bidirectional communication medium between humans and AI, it needs a way to express *intent*, *contracts*, and *meaning* that does not alter execution semantics. This change introduces three formal semantic layers so that Vulgata source can carry rich information for AI tooling, verification, and documentation while guaranteeing that release-mode execution is unaffected.

## What Changes

- Add a **descriptive layer** with four new non-executing constructs: `intent:` blocks, `meaning:` annotations, `explain:` blocks, and `step` labels.
- Add a **checkable layer** with `requires`/`ensures` pre/post-condition contracts and `example` blocks (joining the existing `expect` construct).
- Define four **execution modes** — release, checked, debug, tooling — governing which layers are active.
- Extend the lexer, parser, and AST to represent all new constructs as first-class nodes.
- Extend the interpreter to skip descriptive constructs and enforce checkable constructs according to the active execution mode.
- Add an optional **metadata export** (JSON) that emits the full semantic layer structure without affecting execution output.
- No changes to the existing executable layer syntax or semantics. **No breaking changes.**

## Capabilities

### New Capabilities

- `semantic-layers`: Core classification model — the three-layer taxonomy (executable / checkable / descriptive) and the four execution modes (release / checked / debug / tooling).
- `intent-blocks`: `intent:` block syntax attached to `action` declarations; fields `goal`, `constraints`, `assumptions`, `properties`; parsed but never executed.
- `meaning-annotations`: `meaning:` inline annotation on record fields and type aliases; descriptive only, no runtime effect.
- `explain-blocks`: `explain:` block containing free-form text; descriptive only, strips cleanly in all execution modes.
- `step-labels`: `step <name>:` wrapper around inner blocks; semantically equivalent to the inner block; label is metadata only, usable for debug tracing.
- `requires-ensures`: `requires <expr>` / `ensures <expr>` pre/post-condition contracts on actions; enforced in checked and debug modes, removed in release.
- `example-blocks`: `example <name>:` blocks with `input:` / `output:` sub-blocks; transformed into tests in checked/debug mode, ignored in release.
- `metadata-export`: Optional structured JSON emission of all semantic-layer data (intent, contracts, steps, examples) for AI and tooling consumption.

### Modified Capabilities

- `expect`: Execution-mode semantics clarified — `expect` is now formally part of the checkable layer. Behavior unchanged, but it must obey mode rules (enforced in checked/debug, removed in release).

## Impact

- **Lexer** (`src/lexer.rs` or equivalent): new keywords — `intent`, `meaning`, `explain`, `step`, `requires`, `ensures`, `example`, `goal`, `constraints`, `assumptions`, `properties`, `input`, `output`.
- **Parser** (`src/parser.rs`): new grammar rules for all descriptive and checkable constructs; must attach them to the correct parent AST nodes.
- **AST** (`src/ast.rs`): new node variants for `IntentBlock`, `MeaningAnnotation`, `ExplainBlock`, `StepBlock`, `Requires`, `Ensures`, `ExampleBlock`.
- **Interpreter** (`src/interpreter.rs`): execution-mode parameter; skip or enforce nodes based on layer classification.
- **CLI / REPL** (`src/main.rs`, `src/repl.rs`): `--mode` flag (release | checked | debug | tooling); tooling mode is the default for REPL.
- **Metadata emitter** (new module `src/metadata.rs`): walks AST and serialises semantic-layer nodes to JSON.
- No changes to the Rust code-generation backend in this phase.
