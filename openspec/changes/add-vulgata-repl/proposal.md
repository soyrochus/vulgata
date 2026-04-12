## Why

Vulgata currently exposes only file-oriented CLI commands, which makes quick interpreter-driven experimentation awkward even though the front-end and runtime pipeline already exist. A REPL would provide a practical interactive entrypoint without inventing a second execution model.

## What Changes

- Add a `vulgata repl` CLI subcommand for interactive interpreter-backed sessions.
- Define the REPL as a virtual source file that accumulates ordinary Vulgata declarations.
- Add a small set of REPL meta-commands for help, showing session source, checking, parsing, running, testing, resetting, and quitting.
- Keep REPL execution aligned with the existing parse/check/lower/runtime pipeline rather than introducing REPL-only semantics.
- Define clear failure behavior so invalid submitted blocks do not mutate session state.

## Capabilities

### New Capabilities
- `interactive-repl`: An interactive CLI mode for source-persistent Vulgata interpreter sessions.

### Modified Capabilities

## Impact

- Affects the CLI surface by adding an interactive subcommand.
- Will require session-management logic layered over the existing parser, resolver, type checker, typed IR, and interpreter.
- Creates the canonical interactive workflow for trying declarations, running `main`, and executing tests from a live session.
