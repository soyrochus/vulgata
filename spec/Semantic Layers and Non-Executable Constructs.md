# Vulgata Design Proposal — Semantic Layers and Non-Executable Constructs (v0.5+)

## 1. Purpose

This document defines an extension to Vulgata v0.4 introducing **three semantic layers**:

1. **Executable layer** — defines runtime behavior
2. **Checkable layer** — defines verifiable correctness constraints
3. **Descriptive layer** — defines human- and AI-oriented intent and meaning

The goal is to extend Vulgata from a **programming language** into a **bidirectional communication medium** between humans and AI, while preserving:

* compilability to Rust
* interpreter execution
* deterministic semantics
* performance guarantees

No construct defined in this document may violate the execution guarantees defined in Vulgata v0.4.

---

## 2. Core Principle

All constructs must belong to exactly one of the following categories:

| Category    | Purpose                   | Runtime effect            |
| ----------- | ------------------------- | ------------------------- |
| Executable  | Defines behavior          | Always executed           |
| Checkable   | Defines correctness       | Optional (mode-dependent) |
| Descriptive | Defines meaning/intention | Never executed            |

This classification is **mandatory** and must be enforced by both interpreter and compiler.

---

## 3. Execution Modes

### 3.1 Release mode

* Executes only executable layer
* Removes all checkable constructs
* Removes all descriptive constructs

### 3.2 Checked mode

* Executes executable layer
* Enforces checkable constructs
* Ignores descriptive constructs

### 3.3 Debug mode

* Executes executable layer
* Enforces checkable constructs
* May expose descriptive constructs for tracing

### 3.4 Tooling mode (REPL, IDE, AI)

* Executes executable layer optionally
* Exposes checkable constructs
* Fully exposes descriptive constructs

---

## 4. Descriptive Layer

Descriptive constructs do not affect execution semantics.

They must:

* attach to existing executable structures
* be structurally defined (not arbitrary free text)
* be preserved optionally in metadata

---

## 4.1 Intent Blocks

```text
intent:
  goal: Text
  constraints: List[Text]
  assumptions: List[Text]
  properties: List[Text]
```

### Example

```text
action gcd(a: Int, b: Int) -> Int:
  intent:
    goal: "Compute greatest common divisor"
    constraints:
      - "Inputs are non-negative"
    properties:
      - "gcd(a, b) == gcd(b, a % b)"
```

### Semantics

* Not executed
* Not compiled into runtime code
* May be emitted as metadata or discarded

---

## 4.2 Meaning Annotations

```text
email: Text
  meaning: "unique contact address"
```

### Semantics

* No runtime effect
* Used for documentation, validation, AI reasoning

---

## 4.3 Explain Blocks

```text
explain:
  "Repeatedly replace (a, b) with (b, a % b)"
```

### Semantics

* No execution effect
* Optional metadata only

---

## 4.4 Step Blocks

```text
step iterate:
  while b != 0:
    ...
```

### Semantics

* Equivalent to inner block
* `step` label is metadata only
* May be used for tracing in debug mode

---

## 5. Checkable Layer

## 5.1 Requires / Ensures

```text
requires a >= 0
ensures result >= 0
```

### Semantics

| Mode    | Behavior |
| ------- | -------- |
| Release | removed  |
| Checked | enforced |
| Debug   | enforced |

---

## 5.2 Expect

Already defined in v0.4, extended with execution-mode semantics.

---

## 5.3 Example Blocks

```text
example gcd_basic:
  input:
    a = 84
    b = 30
  output:
    result = 6
```

### Semantics

* Transformed into tests
* Ignored in release execution
* Used by AI and tooling

---

## 6. Interaction Between Layers

* Non-executable constructs must not alter execution semantics
* Must attach to executable constructs

---

## 7. Compiler Behavior

### Descriptive constructs

* removed in release
* optionally emitted as metadata

### Checkable constructs

* compiled to assertions or removed

---

## 8. Interpreter Behavior

* ignores descriptive constructs
* enforces checkable constructs depending on mode

---

## 9. No-Op Guarantee

Descriptive constructs must not introduce runtime overhead in release mode.

---

## 10. Conformance Requirements

Descriptive constructs must not affect execution output.

---

## 11. Machine-Readable Metadata Model (Extension)

To fully enable AI interaction, Vulgata implementations should support exporting semantic layers into a structured metadata representation.

### 11.1 Purpose

* Enable AI systems to consume intent, contracts, and structure
* Allow round-trip transformation (intent → code → intent)
* Provide a stable interchange format

### 11.2 Metadata Structure

The compiler or interpreter may emit a JSON-like structure:

```json
{
  "module": "example",
  "actions": [
    {
      "name": "gcd",
      "intent": {
        "goal": "Compute greatest common divisor",
        "constraints": ["Inputs are non-negative"]
      },
      "contracts": {
        "requires": ["a >= 0", "b >= 0"],
        "ensures": ["result >= 0"]
      },
      "steps": [
        "normalize",
        "iterate",
        "result"
      ],
      "examples": [
        {
          "input": {"a": 84, "b": 30},
          "output": {"result": 6}
        }
      ]
    }
  ]
}
```

### 11.3 Requirements

* Metadata must not affect execution
* Must be derivable from source
* Must preserve structure of semantic layers
* Must be deterministic

### 11.4 Usage

* AI code generation
* documentation generation
* reverse engineering
* verification pipelines

---

## 12. Final Position

With this extension, Vulgata becomes:

* executable
* verifiable
* communicative

Without sacrificing:

* performance
* determinism
* compilability

The invariant remains:

> The executable layer defines behavior. Everything else must not change it.
