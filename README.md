# Vulgata 

> a lingua franca for humans and AI systems

[![Rust](https://img.shields.io/badge/rust-workspace-orange.svg)](./rust/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![FOSS Pluralism](https://img.shields.io/badge/FOSS-Pluralism-purple.svg)](FOSS_PLURALISM_MANIFESTO.md)
[![OpenSpec](https://img.shields.io/badge/OpenSpec-Used-black.svg)](https://openspec.dev/)

## 1. Purpose

Vulgata is a compact, human-readable, executable language designed as a lingua franca for humans and AI systems collaborating on software design, algorithm specification, workflow definition, and lightweight application logic.

It is not intended to compete with Python, Rust, or Java as a general-purpose systems language. Its design target is different:

* readable enough for non-specialist technical users to verify
* regular enough for AI systems to generate reliably
* formal enough to compile and interpret unambiguously
* expressive enough to describe real algorithms and structured software behavior
* restricted enough to remain compact and auditable

Vulgata must support two equally valid execution modes:

1. **Interpreter mode** — source is parsed, analyzed, and executed in a managed runtime environment.
2. **Compiler mode** — source is translated to Rust and then compiled to native code with no dependency on the interpreter runtime other than minimal emitted support code strictly required by the program itself.

The canonical source language is the same in both cases. Semantics must match.


## Principles of Participation

Everyone is invited and welcome to contribute: open issues, propose pull requests, share ideas, or help improve documentation. Participation is open to all, regardless of background or viewpoint.  

This project follows the [FOSS Pluralism Manifesto](./FOSS_PLURALISM_MANIFESTO.md),  
which affirms respect for people, freedom to critique ideas, and space for diverse perspectives.  

## License and Copyright

Copyright (c) 2026, Iwan van der Kleijn

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
