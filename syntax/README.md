# Vulgata Syntax Coloring

This directory contains the current syntax-coloring assets for Vulgata and a minimal VS Code extension skeleton that uses them.

## Contents

- `vulgata.tmLanguage.json`: TextMate grammar for `.vg` files
- `language-configuration.json`: editor behavior for `#` comments, bracket matching, and auto-closing pairs
- `vscode-extension/`: minimal installable VS Code extension layout with `package.json`, copied syntax assets, and packaging notes

## What The Syntax Coloring Supports

The grammar is aligned with the current lexer in `src/lexer.rs` and with the checked-in examples. It provides lexical highlighting for:

- top-level declarations such as `module`, `import`, `record`, `enum`, `action`, `test`, and `const`
- modifiers such as `extern`, `pure`, and `impure`
- control-flow and binding forms such as `let`, `var`, `if`, `elif`, `else`, `while`, `for`, `each`, `in`, `return`, `break`, `continue`, and `expect`
- semantic-layer constructs such as `intent`, `meaning`, `explain`, `step`, `requires`, `ensures`, `example`, `goal`, `constraints`, `assumptions`, `properties`, `input`, and `output`
- string literals, supported escapes, integer and decimal literals, booleans, and `none`
- builtin type names such as `Bool`, `Int`, `Dec`, `Text`, `Bytes`, `List`, `Map`, `Set`, `Option`, and `Result`
- user-defined type names, function-call names, field labels, dotted member access, and operators such as `:=`, `->`, `==`, `!=`, `<=`, `>=`, `+`, `-`, `*`, `/`, and `%`

This is grammar-only editor support. It does not provide parser-backed diagnostics, completion, hover text, rename, go-to-definition, or semantic tokens.

## How To Use It

If you just want the raw syntax assets, use:

- `syntax/vulgata.tmLanguage.json`
- `syntax/language-configuration.json`

If you want the ready-made VS Code extension layout, use:

- `syntax/vscode-extension/`

That folder already contains the structure VS Code expects:

```text
vscode-extension/
  package.json
  language-configuration.json
  syntaxes/
    vulgata.tmLanguage.json
  README.md
  CHANGELOG.md
```

## Permanent Local Install (No Packaging Required)

The quickest way to get syntax highlighting in every workspace is to symlink the extension folder into VS Code's extensions directory:

```sh
ln -s /path/to/vulgata/syntax/vscode-extension \
  ~/.vscode/extensions/vulgata-language
```

Restart VS Code once. The extension will be active globally, and because it is a symlink any edits to the grammar or configuration take effect on the next window reload — no reinstall needed.

## Quick Local Check (Development Host)

To test without a permanent install:

1. Open `syntax/vscode-extension` as the VS Code workspace root — either via **File → Open Folder…** or `code syntax/vscode-extension` from the terminal.
2. Press `F5` to launch an Extension Development Host.
3. In the new window, open a `.vg` file such as `examples/sort.vg`.
4. Confirm that declaration heads, semantic-layer keywords, strings, numbers, comments, and operators like `:=` are colored distinctly.

## What Is Left To Do

Nothing else is required for a minimal local grammar-only extension beyond opening and testing `syntax/vscode-extension` in VS Code.

If you want to go further, the remaining optional work is:

- package it as a `.vsix` using `vsce`
- add marketplace-oriented metadata such as an icon, gallery banner, and publisher details
- tune scopes based on how the grammar looks across more `.vg` examples
- add richer tooling later by reusing the Rust frontend in `src/lexer.rs` and `src/parser.rs`

## Limits

- Vulgata is indentation-based, but TextMate grammars do not validate indentation correctness.
- Highlighting is lexical and heuristic, not parser-backed.
- Some identifiers may share the same scope where a semantic engine would eventually distinguish them more precisely.
