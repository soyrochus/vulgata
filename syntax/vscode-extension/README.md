# Vulgata Language Support for VS Code

This extension provides lightweight editor support for Vulgata source files:

- syntax highlighting for `.vg`
- `#` line comments
- bracket matching
- auto-closing pairs for `"`, `()`, `[]`, and `{}`

It is intentionally grammar-only. It does not yet provide diagnostics, completion, hover, rename, or go-to-definition.

## Included Files

```text
vscode-extension/
  package.json
  language-configuration.json
  syntaxes/
    vulgata.tmLanguage.json
```

## Run Locally In VS Code

1. Open this folder in VS Code:

   ```text
   syntax/vscode-extension
   ```

2. Press `F5` to launch an Extension Development Host.
3. In the new window, open any `.vg` file such as `examples/sort.vg` from the main repo.

## Package As A `.vsix`

If you want a distributable VS Code package, use `vsce` from this directory:

```sh
npm install -g @vscode/vsce
cd syntax/vscode-extension
vsce package
```

That produces a `.vsix` file you can install in VS Code with `Extensions: Install from VSIX...`.

## Current Scope

The grammar follows the current Vulgata lexer and examples. It highlights:

- declarations such as `module`, `import`, `record`, `enum`, `action`, `test`, and `const`
- modifiers such as `extern`, `pure`, and `impure`
- control-flow and binding forms such as `let`, `var`, `if`, `while`, `return`, and `expect`
- semantic-layer keywords such as `intent`, `meaning`, `explain`, `step`, `requires`, `ensures`, `example`, `goal`, `constraints`, `assumptions`, `properties`, `input`, and `output`
- strings, numbers, booleans, `none`, types, field labels, function calls, and operators such as `:=` and `->`

## Limits

- Vulgata is indentation-based, but TextMate grammars do not validate indentation.
- Scope assignment is lexical and heuristic.
- Richer editor features should be built later on top of the Rust frontend in `src/lexer.rs` and `src/parser.rs`.
