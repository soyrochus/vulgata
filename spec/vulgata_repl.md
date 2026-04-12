# Vulgata REPL Specification

## 1. Purpose

This document specifies an interactive REPL interface for Vulgata that is consistent with the current implementation and with the language model described in `spec/vulgata_spec_v0.2.md`.

The current crate already supports a file-oriented CLI with the following subcommands:

* `parse`
* `check`
* `run`
* `test`
* `compile`

Those commands operate on a complete source file and route through a shared front-end pipeline, then either execute through the interpreter runtime or emit Rust source.

The REPL extends that model with an interactive shell for interpreter-oriented development. It does not define a second language. It provides a session-oriented way to build and execute ordinary Vulgata source incrementally.

---

## 2. Design goals

The REPL should provide:

* a direct interactive entrypoint for the existing interpreter
* reuse of the same lexer, parser, resolver, type checker, typed IR, and runtime used by file-based execution
* deterministic behavior that matches file execution semantics
* source-span diagnostics that remain readable in an interactive context
* a practical workflow for experimenting with declarations, actions, and tests
* a minimal MVP that fits the current implementation constraints

---

## 3. Non-goals

The initial REPL does not aim to provide:

* a dynamic interpreter mode with semantics different from ordinary Vulgata modules
* ad hoc runtime mutation that bypasses parsing, type checking, or typed IR lowering
* compile-mode execution inside the interactive loop
* debugger-grade stepping, breakpoints, or variable inspection in the first version

---

## 4. Relationship to the existing implementation

## 4.1 Existing execution model

The current crate exposes shared front-end entrypoints:

1. `parse_source`
2. `check_source`
3. `lower_source`
4. `run_source`
5. `test_source`
6. `compile_source`

Interpreter execution currently requires a fully formed module and runs through `runtime::Interpreter`.

## 4.2 Current CLI limitations

The current CLI is file-oriented and expects:

```text
vulgata <parse|check|run|test|compile> <source-file>
```

There is no interactive mode, no persistent session state, and no command for incremental source entry.

## 4.3 Consequence for the REPL design

Because the current parser and interpreter are module-oriented, the REPL should be specified as a **session that maintains a virtual source file** together with expression evaluation against that source.

Accepted declaration blocks become part of that virtual module. Expression input is parsed, checked, lowered, and executed against the current session buffer without mutating it.

This keeps the REPL aligned with the v0.2 language rather than introducing a special-case interactive grammar.

---

## 5. Command-line surface

The CLI shall gain a new subcommand:

```text
vulgata repl
```

Optional future flags may be added, but the MVP command surface is:

```text
vulgata repl
```

If the command succeeds, the process enters an interactive session.

If the command fails to initialize terminal I/O or session state, it shall report a CLI diagnostic and exit nonzero.

---

## 6. Session model

## 6.1 Virtual source file

A REPL session maintains an in-memory source buffer representing one Vulgata module.

The session shall assign that buffer a synthetic path for diagnostics, such as:

```text
<repl>
```

or

```text
<repl>/session.vg
```

The exact spelling is an implementation choice, but it must be stable within a session and readable in diagnostics.

## 6.2 Source truth

The accumulated session buffer is the canonical source of truth for top-level declarations in the REPL.

In addition, the REPL may maintain session-local bindings introduced through interactive `let` and `var` input.

Those bindings are not top-level declarations and therefore are not required to appear in `:show`.

## 6.3 Persistence within a session

Top-level declarations entered successfully remain available for later `run`, `test`, `check`, and inspection commands until the session is reset or exited.

Session-local bindings entered successfully through `let` or `var` remain available to later interactive expressions and assignments until the session is reset or exited.

## 6.4 No persistence across sessions in MVP

The initial REPL does not need automatic save/load behavior between process invocations.

---

## 7. Input model

## 7.1 Accepted unit

The REPL accepts one **input block** at a time.

An input block is either:

* ordinary Vulgata source text containing one or more top-level declarations, for example:

* `const`
* `record`
* `enum`
* `extern`
* `action`
* `test`
* optional `module`
* optional `import`
* or an ordinary Vulgata expression, such as:
* `1 + 2`
* `add(20, 22)`
* `file.exists("README.md")`
* or a REPL-local statement, such as:
* `let p = Point(x: 0.0, y: 0.0)`
* `var total = 1`
* `total := total + 1`

## 7.2 Multi-line entry

Since Vulgata is indentation-based, the REPL must support multi-line input.

The MVP should treat a submitted block as complete when the user terminates entry explicitly, for example by:

* entering an empty line after the block, or
* using a delimiter command such as `.end`, or
* using a terminal line editor that submits a buffered block

The exact input UX may vary, but block completion must be explicit and predictable.

In a reasonably modern terminal, the CLI REPL should provide basic line editing behavior:

* left/right cursor movement within the current line
* history navigation for previous inputs
* ordinary character insertion and deletion

The specification does not require custom terminal emulation logic beyond that baseline.

## 7.3 Successful append semantics

When a declaration block parses and checks successfully in the context of the existing session buffer, the REPL appends it to the session source.

If the block fails validation, it shall not mutate session state.

This prevents a broken block from partially corrupting the current interactive module.

## 7.4 Bare expressions

Bare expressions are part of the MVP.

When an input block is not a declaration block, the REPL shall attempt to parse it as an expression.

If expression parsing and checking succeed, the REPL shall evaluate that expression against the current session module and print the resulting value.

Expression evaluation shall not mutate the accumulated session source.

## 7.5 REPL-local statements

The MVP REPL also supports interactive local statements:

* `let`
* `var`
* `:=`

Those statements are checked against the current session module plus any already-established REPL-local bindings.

Successful `let` and `var` statements create bindings that persist for the rest of the REPL session unless reset.

Successful `:=` statements update existing mutable REPL-local bindings using the same mutability rules as ordinary Vulgata action bodies.

---

## 8. Built-in REPL commands

Lines beginning with `:` are REPL meta-commands rather than Vulgata source.

The MVP shall provide the following commands.

## 8.1 `:help`

Displays available REPL commands and a short usage summary.

## 8.2 `:show`

Prints the current accumulated session source exactly as the REPL will analyze it.

## 8.3 `:reset`

Clears the session buffer after confirmation or explicit immediate reset behavior.

After reset, subsequent commands operate on an empty module.

## 8.4 `:check`

Runs the shared front-end through semantic checking on the current session buffer.

On success it should print a concise success message.

On failure it should print diagnostics using the same formatting style as the ordinary CLI.

## 8.5 `:run`

Runs interpreter execution for the current session buffer.

This command shall behave like the existing file-based `run` command:

* it requires an action named `main`
* `main` must take no parameters
* the result is displayed using the same runtime formatting already used by the CLI

## 8.6 `:test`

Runs interpreter-backed test execution for the current session buffer.

This command shall behave like the existing file-based `test` command:

* each test prints `PASS <name>` or `FAIL <name>`
* failed expectations include source location and message

## 8.7 `:parse`

Runs the parser on the current buffer and prints the parsed module representation, matching the style of the existing `parse` command where practical.

## 8.8 `:quit`

Exits the session cleanly.

`Ctrl-D` may also terminate the session if supported by the terminal environment.

---

## 9. Execution semantics

## 9.1 Shared pipeline rule

The REPL shall use the same front-end stages as file execution:

1. lexing
2. parsing
3. resolution
4. type checking
5. typed IR lowering
6. interpreter execution for `:run`, `:test`, and evaluated expressions

No REPL-only semantic shortcuts are permitted.

## 9.2 Session updates

Adding a declaration block should follow this flow:

1. concatenate the current session buffer with the candidate block
2. parse and check the combined source
3. if successful, commit the new source buffer
4. if unsuccessful, keep the previous source buffer unchanged and print diagnostics

Evaluating an expression block should follow this flow:

1. keep the current session buffer unchanged
2. parse the candidate input as an expression
3. type-check that expression against the current session module
4. lower the expression using the shared typed IR machinery
5. evaluate it with a fresh interpreter built from the current session source
6. print the resulting value

Executing a REPL-local statement should follow this flow:

1. keep the current session source unchanged
2. parse the candidate input as a statement
3. type-check that statement against the current session module plus existing REPL-local bindings
4. lower the statement using the shared typed IR machinery
5. execute it against the persistent REPL-local environment
6. if it succeeds, commit any new or updated REPL-local bindings

## 9.3 Runtime state

The MVP REPL shall be **source-persistent for declarations and binding-persistent for interactive local state**.

That means:

* declarations remain in the accumulated source buffer
* `:run` and `:test` build a fresh typed IR and a fresh interpreter from that buffer
* REPL-local `let` and `var` bindings survive across later interactive inputs
* runtime locals from a prior `:run` invocation do not survive into the next invocation unless they are represented in source or in REPL-local bindings

This keeps declaration execution aligned with the ordinary module model while still allowing practical interactive exploration.

## 9.4 Main action rule

`:run` requires a valid `action main() -> ...` with zero parameters, matching the current interpreter CLI contract.

If no suitable `main` exists, the REPL shall report the same runtime-class error the file-based command would report.

---

## 10. Diagnostics

## 10.1 Formatting

Diagnostics should preserve the existing format:

```text
<path>:<line>:<column>: <phase>: <message>
```

## 10.2 Interactive source spans

Because the REPL works against a virtual source buffer, reported line and column positions shall correspond to the accumulated session source as presented by `:show`.

## 10.3 Failed block behavior

If a submitted source block fails, the REPL should:

1. print diagnostics
2. reject the block
3. leave prior session state unchanged

This behavior is required to keep the session coherent.

---

## 11. Interpreter scope in the REPL

The REPL is an interpreter-oriented feature.

The initial version should support:

* checking source incrementally
* running `main`
* running tests
* inspecting the accumulated source
* evaluating expressions and action calls against the current session
* maintaining REPL-local `let` and `var` bindings for interactive work

The initial version does not need to support:

* `compile` from inside the REPL
* emitted Rust display
* cross-mode equivalence checking

Those can be layered later as optional inspection commands if needed.

---

## 12. Extern bindings

The current runtime supports extern registries and extern configuration, but the current CLI does not expose extern configuration flags.

For the REPL MVP, two acceptable behaviors exist:

1. disallow extern-backed execution in REPL sessions and emit a clear diagnostic
2. add a dedicated REPL command such as `:externs <path>` that associates an extern configuration file with the session

The preferred MVP behavior is to define the extension point explicitly and keep extern execution out of scope unless configuration support is implemented together with the REPL.

This keeps the first iteration aligned with the current CLI surface.

---

## 13. User interaction model

An example session might look like:

```text
$ vulgata repl
vulgata repl
type :help for commands

> action main() -> Int:
|   let x = 40
|   return x + 2
|
ok: block added

> main()
Int(42)

> test smoke:
|   expect 1 == 1
|
ok: block added

> :test
PASS smoke

> :show
action main() -> Int:
  let x = 40
  return x + 2

test smoke:
  expect 1 == 1

> :quit
```

The exact prompt text is not normative. The semantics are.

---

## 14. Future extensions

After the MVP, the REPL may grow to support:

* `:ast` to display parsed AST
* `:tir` to display typed IR
* `:compile` to emit Rust for the current session buffer
* `:load <file>` to seed the session from an existing source file
* `:save <file>` to write the current session buffer
* `:externs <file>` to attach extern bindings
* undo of the most recently accepted block

None of those extensions should violate the central rule that the REPL remains a structured interface over ordinary Vulgata source and the shared front-end pipeline.

---

## 15. Summary

The Vulgata REPL should be specified as an interpreter-backed, session-oriented interface over a virtual Vulgata module.

That design:

* fits the current implementation
* preserves the shared-pipeline rule from the v0.2 language spec
* allows interactive evaluation without inventing a second language semantics
* provides a practical interactive workflow
* leaves room for richer commands later without compromising consistency
