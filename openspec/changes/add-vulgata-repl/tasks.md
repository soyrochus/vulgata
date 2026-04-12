## 1. CLI and Session Foundation

- [x] 1.1 Add a `repl` subcommand to the CLI entrypoint
- [x] 1.2 Implement a REPL session type that owns the accumulated virtual source buffer and synthetic diagnostic path
- [x] 1.3 Implement multi-line block collection and `:`-prefixed command dispatch for the interactive loop

## 2. Shared-Pipeline Integration

- [x] 2.1 Implement transactional block submission that validates combined session source before committing it
- [x] 2.2 Implement `:parse`, `:check`, `:run`, and `:test` by routing through the existing parse/check/lower/interpreter pipeline
- [x] 2.3 Implement `:show`, `:reset`, `:help`, and `:quit` session-control commands

## 3. REPL Validation

- [x] 3.1 Add integration tests covering accepted blocks, rejected blocks, and unchanged session state after failure
- [x] 3.2 Add integration tests for `:run` and `:test` behavior against the current session buffer
- [x] 3.3 Add diagnostics or explicit MVP behavior for unsupported extern-backed REPL execution
