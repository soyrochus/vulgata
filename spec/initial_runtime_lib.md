# Initial Runtime Library Specification

## 1. Purpose

This document defines the first small standard runtime library for Vulgata.

It is intended to be consistent with `spec/vulgata_spec_v0.2.md`, especially these constraints:

* the standard library should be small
* it should be defined mostly as modules of actions rather than syntax
* side effects should remain explicit
* exceptions should not be introduced
* fallible operations should use `Result[T, E]`

The goal of this initial library is modest:

* provide basic console I/O
* provide basic file I/O
* keep the API easy to read
* avoid cleverness that hides side effects

This document does not define a large general-purpose runtime.

---

## 2. Design principles

The initial runtime library should follow these principles:

1. **No new syntax.**
   Console and file access should be ordinary action calls, not statements or special forms.
2. **Small surface area.**
   The initial library should include only the minimum useful operations.
3. **Explicit side effects.**
   Console output, console input, and file access are impure operations and should look like ordinary calls to clearly named actions.
4. **Simple error model.**
   Fallible operations should return `Result[..., Text]`.
5. **Readable names over compressed names.**
   `file.read_text` is preferable to abbreviated APIs or overloaded forms.

---

## 3. Library shape

The initial runtime library consists of two standard modules:

* `console`
* `file`

These are standard runtime modules, not keywords.

Source programs use them like ordinary module actions:

```text
console.println("hello")
let line = console.read_line()
let data = file.read_text("notes.txt")
```

The implementation may internally bind these actions through the same host-integration machinery used for externs, but source-level programs should treat them as part of the standard runtime library.

---

## 4. Console module

## 4.1 Scope

The `console` module covers only basic terminal-style text I/O.

It does not attempt to define:

* formatting mini-languages
* rich terminal control
* cursor positioning
* colors
* prompts with hidden input
* asynchronous or streaming APIs

Those can be added later if needed.

## 4.2 Actions

The initial `console` module shall provide:

```text
console.print(value: Text) -> Result[None, Text]
console.println(value: Text) -> Result[None, Text]
console.eprint(value: Text) -> Result[None, Text]
console.eprintln(value: Text) -> Result[None, Text]
console.read_line() -> Result[Text, Text]
```

## 4.3 Semantics

### `console.print`

Writes `value` to standard output without appending a newline.

Returns:

* `Result[None, Text]`
* `Ok(None)` on success
* `Err(message)` on failure

### `console.println`

Writes `value` to standard output followed by one line terminator.

At the source-language level, this should be treated as writing one newline-terminated line.

Returns:

* `Result[None, Text]`
* `Ok(None)` on success
* `Err(message)` on failure

### `console.eprint`

Writes `value` to standard error without appending a newline.

Returns:

* `Result[None, Text]`
* `Ok(None)` on success
* `Err(message)` on failure

### `console.eprintln`

Writes `value` to standard error followed by one line terminator.

Returns:

* `Result[None, Text]`
* `Ok(None)` on success
* `Err(message)` on failure

### `console.read_line`

Reads one line of text from standard input.

The returned `Text` should not include the trailing newline.

Returns:

* `Result[Text, Text]`
* `Ok(line)` when a line is read successfully
* `Err(message)` on I/O failure

End-of-input behavior should be represented as:

* `Ok("")` only when an actual empty line was read
* `Err("end of input")` or equivalent implementation-defined message when input is closed before a line can be read

This avoids introducing hidden sentinel values.

## 4.4 Examples

```text
let _ = console.println("Vulgata")
let name = console.read_line()
```

With explicit result handling:

```text
let written = console.println("Enter name:")
let line = console.read_line()
```

---

## 5. File module

## 5.1 Scope

The initial `file` module covers simple text file access.

It deliberately excludes:

* directory traversal
* file metadata inspection beyond existence checks
* random access I/O
* streaming readers and writers
* binary I/O in the first version
* file locking
* globbing

The first version should be easy to understand and easy to implement.

## 5.2 Actions

The initial `file` module shall provide:

```text
file.read_text(path: Text) -> Result[Text, Text]
file.write_text(path: Text, content: Text) -> Result[None, Text]
file.append_text(path: Text, content: Text) -> Result[None, Text]
file.exists(path: Text) -> Bool
```

## 5.3 Semantics

### `file.read_text`

Reads the full contents of the file at `path` as UTF-8 text.

Returns:

* `Ok(content)` on success
* `Err(message)` if the file cannot be read, does not exist, is not valid UTF-8, or another file-system error occurs

### `file.write_text`

Writes `content` to `path`, replacing any existing file contents.

Returns:

* `Ok(None)` on success
* `Err(message)` on failure

### `file.append_text`

Appends `content` to the file at `path`.

If the file does not exist, the implementation may create it.

Returns:

* `Ok(None)` on success
* `Err(message)` on failure

### `file.exists`

Returns `true` if a file-system entry exists at `path`, otherwise `false`.

This action is intentionally simpler than a full metadata API and does not distinguish between files and directories in the initial version.

## 5.4 Path model

Paths are represented as `Text`.

The runtime library does not define a separate path type in the initial version.

Paths are interpreted according to the host environment:

* relative paths are resolved relative to the program’s current working directory
* absolute paths use the host platform’s conventions

The source language should not normalize or reinterpret path strings beyond passing them to the runtime implementation.

## 5.5 Examples

```text
let content = file.read_text("notes.txt")
let _ = file.write_text("out.txt", "done\n")
let present = file.exists("config.txt")
```

---

## 6. Purity and side effects

All `console` and `file` actions except `file.exists` should be treated as impure operations.

The language specification already allows purity metadata for extern actions. The runtime library should follow the same semantic expectation:

* `console.print` is impure
* `console.println` is impure
* `console.eprint` is impure
* `console.eprintln` is impure
* `console.read_line` is impure
* `file.read_text` is impure
* `file.write_text` is impure
* `file.append_text` is impure
* `file.exists` may be treated as impure in implementation terms because it touches the file system, even though its surface behavior is query-like

The important requirement is not a static purity proof in v0.2, but obvious source-level naming and explicit calls.

---

## 7. Error model

The initial runtime library shall follow the v0.2 error model:

* no exceptions
* no hidden implicit failures
* use `Result[T, Text]` for fallible operations

This leads to a simple initial convention:

* success values are returned directly in `Ok(...)`
* failures use human-readable `Text` messages

The library does not introduce a structured error record type in the first version.

That may be added later if the language grows better pattern matching and richer result handling.

---

## 8. Why no `print` statement

The initial runtime library should not define a `print` statement.

Instead, output remains an ordinary action call:

```text
console.println("hello")
```

This keeps the language aligned with the principle that the standard library should mostly be modules of actions rather than syntax.

It also keeps side effects explicit without adding a special statement category.

---

## 9. Relationship to externs and implementation

The standard runtime library may be implemented in either of these ways:

1. as predeclared standard modules wired directly into the interpreter and compiler
2. as built-in bindings implemented through the same registry mechanism used for extern actions

The source-level behavior should be the same either way.

For the initial implementation, reusing the existing binding model is preferred because it:

* keeps the implementation small
* matches the current host integration architecture
* avoids introducing a separate special-case runtime dispatch path too early

---

## 10. Example usage

Example interactive-style program:

```text
action main() -> None:
  let _ = console.println("Enter file name:")
  let name = console.read_line()
  let _ = console.println("Reading...")
  return
```

Example file copy with explicit operations:

```text
action copy_notes() -> Result[None, Text]:
  let content = file.read_text("notes.txt")
  # explicit result-handling helpers would be used here in a fuller program
  return file.write_text("notes-copy.txt", "copy pending")
```

These examples are intentionally simple. The goal of the initial runtime library is clarity, not completeness.

---

## 11. Initial action summary

The initial runtime library is therefore:

```text
console.print(value: Text) -> Result[None, Text]
console.println(value: Text) -> Result[None, Text]
console.eprint(value: Text) -> Result[None, Text]
console.eprintln(value: Text) -> Result[None, Text]
console.read_line() -> Result[Text, Text]

file.read_text(path: Text) -> Result[Text, Text]
file.write_text(path: Text, content: Text) -> Result[None, Text]
file.append_text(path: Text, content: Text) -> Result[None, Text]
file.exists(path: Text) -> Bool
```

This is a sufficient first step:

* output can be shown clearly
* input can be read simply
* text files can be read and written
* the library stays small and obvious
