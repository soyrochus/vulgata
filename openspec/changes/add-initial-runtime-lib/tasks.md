## 1. Standard Runtime Surface

- [x] 1.1 Add standard runtime declarations for the `console` module actions in the front-end or built-in binding layer
- [x] 1.2 Add standard runtime declarations for the `file` module actions in the front-end or built-in binding layer
- [x] 1.3 Ensure the declared runtime action signatures match the specified `Result[..., Text]` and `Bool` return contracts

## 2. Runtime Execution Support

- [x] 2.1 Implement interpreter-backed console bindings for `print`, `println`, `eprint`, `eprintln`, and `read_line`
- [x] 2.2 Implement interpreter-backed file bindings for `read_text`, `write_text`, `append_text`, and `exists`
- [x] 2.3 Route standard runtime actions through the existing host-binding architecture or an equivalent shared dispatch layer without introducing syntax-level special cases

## 3. Verification and Surface Updates

- [x] 3.1 Add tests covering successful console/file calls and explicit failure behavior for fallible actions
- [x] 3.2 Update CLI-facing examples or docs to use `console` and `file` as the canonical runtime modules
- [x] 3.3 Verify the standard runtime modules are available consistently through normal parse/check/run flows
