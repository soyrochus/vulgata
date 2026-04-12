# Vulgata Mutability Note

## 1. Context

The current Vulgata v0.2 design chooses **explicit mutation** rather than a stronger immutable-by-default model.

The current spec states:

* mutation is a primary language goal
* assignment is always explicit via `set`
* `let` bindings may later be reassigned with `set`
* records and collections are mutable at the language level through explicit `set`
* standard-library collection helpers may later return new values rather than mutating in place

That gives the language a clear current position:

* constants are immutable
* ordinary local bindings are potentially mutable
* mutation must be visible in surface syntax

This note documents the tradeoff and compares four viable directions, with the first family split into two syntax variants.

---

## 2. Problem statement

The `set` form is intentionally noisy:

```text
set total = total + 1
set customer.email = "new@example.com"
set items[0] = 99
```

That noise is useful because it makes side effects explicit, but it also creates friction:

* common loops become visually repetitive
* simple local rebinding feels heavier than the underlying operation
* the syntax suggests a mutability discipline, but the language is not actually immutable-by-default

The question is whether that tradeoff remains the right one for Vulgata.

---

## 3. Evaluation criteria

Any mutability model should be judged against the language’s broader goals:

* readability for non-specialist technical users
* regularity for AI generation
* semantic clarity
* parity between interpreter and compiler
* easy implementation in Rust
* explicit visibility of side effects

---

## 4. Option 1a: Keep `set`

## 4.1 Summary

Retain the current model:

* `let` introduces a binding
* later reassignment requires `set`
* record field and indexed updates also require `set`
* records and collections remain mutable at the language level

## 4.2 Example

```text
let x = 0
while x < 10:
  set x = x + 1
```

## 4.3 Advantages

* Mutation is always visually obvious.
* There is one mutation form for variables, fields, and indexes.
* The parser and type checker remain simple.
* The current implementation and tests already follow this model.
* AI-generated code is less likely to hide assignment accidentally inside visually similar syntax.

## 4.4 Disadvantages

* The syntax is noisier than many users expect.
* It does not deliver a true immutable-by-default programming model.
* It can feel inconsistent that `let` introduces bindings that later mutate without a separate mutability marker.
* Repeated `set` statements in loops and accumulator code reduce visual smoothness.

## 4.5 Assessment

This is the lowest-risk option. It is coherent if the design goal is not immutability, but rather explicitness.

---

## 5. Option 1b: Keep explicit mutation semantics, replace `set` with `:=`

## 5.1 Summary

Preserve the current semantics of explicit mutation, but replace the keyword form with a symbolic assignment operator:

* `let` introduces a binding
* later reassignment requires `:=`
* record field and indexed updates also require `:=`
* records and collections remain mutable at the language level

This changes the surface syntax, not the execution model.

## 5.2 Example

```text
let x = 0
while x < 10:
  x := x + 1
```

Field and index updates would follow the same rule:

```text
customer.email := "new@example.com"
items[0] := 99
```

## 5.3 Advantages

* Mutation remains visually distinct from declaration and value construction.
* The syntax is less noisy than `set`.
* The semantic model can remain identical to the current implementation.
* It avoids overloading plain `=` with both declaration and reassignment.
* It is a plausible compromise between explicitness and concision.

## 5.4 Disadvantages

* `:=` is less word-like and may be less readable for non-programmer audiences.
* Vulgata already uses `:` for block syntax, type annotations, and named arguments, so another colon-based construct increases visual density.
* The syntax may suggest declaration-assignment to users coming from languages where `:=` means introduction rather than reassignment.
* The language and tooling would still need coordinated lexer, parser, spec, and test updates.

## 5.5 Assessment

This is the strongest alternative if the project wants to keep explicit mutation semantics but reduce keyword noise.

---

## 6. Option 2: Immutable `let`, add `var`

## 6.1 Summary

Adopt a more conventional mutability split:

* `let` means immutable binding
* `var` means mutable binding
* assignment uses ordinary `=`
* field and index mutation either also use ordinary assignment or retain an explicit mutation keyword

## 6.2 Example

```text
var x = 0
while x < 10:
  x = x + 1
```

Possible field form:

```text
customer.email = "new@example.com"
items[0] = 99
```

## 6.3 Advantages

* The mutability contract becomes clearer at the point of declaration.
* Local rebinding becomes less noisy.
* The model aligns more closely with user expectations from mainstream languages.
* It supports a stronger “immutable unless declared mutable” story.

## 6.4 Disadvantages

* Mutation becomes less visually distinct at the point of update.
* The language gains another top-level keyword and a more complex binding model.
* The compiler and interpreter must track declaration-time mutability explicitly.
* AI-generated code may produce accidental mutation more easily because assignment syntax becomes visually lightweight.
* The distinction between mutable variable assignment and mutable field/index updates still needs a policy decision.

## 6.5 Assessment

This option is a reasonable middle ground if Vulgata wants a stronger mutability discipline without committing to persistent data structures.

---

## 7. Option 3: Persistent-by-default values and update expressions

## 7.1 Summary

Move the language toward an immutable-by-default model:

* `let` bindings are immutable
* records and collections are persistent values
* updates produce new values
* mutation-like syntax is replaced by explicit value transformation

## 7.2 Example

Possible forms:

```text
let total = total + 1
let customer = customer.with(email: "new@example.com")
let items = list.set(items, 0, 99)
```

or, with dedicated update syntax in a future design:

```text
let customer = customer update email = "new@example.com"
let items = items update [0] = 99
```

## 7.3 Advantages

* The language earns a genuine immutability story.
* Aliasing becomes easier to reason about.
* Compiler and interpreter semantics may become simpler in some areas because explicit heap aliasing is reduced.
* The language becomes more declarative and closer to a specification language.

## 7.4 Disadvantages

* This is a major semantic shift from the current spec and implementation.
* It increases surface and semantic complexity unless update syntax is designed very carefully.
* Efficient implementation may require structural sharing or optimized lowering strategies.
* Generated Rust may become less direct and less obvious for common imperative code.
* Many ordinary algorithm examples become more verbose unless the language grows richer update constructs.

## 7.5 Assessment

This is the strongest option if Vulgata wants to optimize for specification-style programming over imperative clarity, but it is also the most disruptive.

---

## 8. Comparison

## 8.1 Visibility of mutation

* Option 1a is strongest in a word-like form because every update uses `set`.
* Option 1b is also explicit, though more symbolic than verbal.
* Option 2 is weaker because assignment becomes visually ordinary.
* Option 3 is strongest in a different way because state changes become value replacement rather than mutation.

## 8.2 Ergonomics

* Option 1a is the noisiest in imperative code.
* Option 1b is more compact while keeping explicit mutation.
* Option 2 is the most familiar and lightweight.
* Option 3 can be elegant for declarative code but cumbersome for update-heavy algorithms.

## 8.3 Implementation risk

* Option 1a is minimal risk.
* Option 1b is low-to-moderate risk because semantics can stay the same while syntax changes.
* Option 2 is moderate risk.
* Option 3 is high risk.

## 8.4 Alignment with current codebase

* Option 1a aligns directly with the current parser, type checker, TIR, interpreter, codegen, and tests.
* Option 1b aligns with the current semantic model but still requires coordinated syntax changes across the parser, lexer, tests, examples, and specs.
* Option 2 requires language and implementation changes but preserves the general imperative model.
* Option 3 requires revisiting the language’s value model and aliasing semantics substantially.

---

## 9. Recommendation

The practical recommendation is:

1. Keep the explicit-mutation model for v0.2 and near-term implementation work.
2. Treat Option 1a and Option 1b as two syntax choices for the same semantic model.
3. Do not describe the current model as “striving for immutability,” because that is not what the current spec actually defines.
4. If the project wants less noise without changing semantics, Option 1b is the most direct alternative to evaluate.
5. If the language wants a stronger mutability discipline later, evaluate Option 2 before considering a full persistent-value redesign.

This recommendation follows from the current state of the project:

* the existing spec is explicit-mutation-first
* the implementation already matches that design
* changing mutability semantics now would create broad parser, checker, runtime, codegen, and conformance churn

---

## 10. Proposed wording clarification for the main spec

If the project keeps an explicit mutation form such as `set` or `:=`, the main spec should describe the model more directly:

> Vulgata v0.2 is not immutable-by-default. Instead, it uses explicit mutation syntax so state changes are always visible in source.

That would better match the current design than implying the language is trying to avoid mutation altogether.
