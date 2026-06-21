# Antimatter Language
A high-performance systems language with **Mutable Value Semantics** and **Copy-on-Write (CoW)** memory management.

## Status
![Lexer](https://img.shields.io/badge/Lexer-Done-brightgreen)
![Parser](https://img.shields.io/badge/Parser-In_Progress-yellow)
![CodeGen](https://img.shields.io/badge/CodeGen-Planned-blue)

## The Core Concept
Antimatter eliminates the garbage collector while maintaining the ease of use of high-level languages.
- **Native Primitives:** No heap overhead for `Int`, `Float`, `Bool`.
- **CoW Memory:** Automatic safety via reference counting and deep-copy-on-mutation.
- **Zero-Cost Borrows:** Immutable, stack-only pointers for function arguments.

## Technical Architecture
- **Runtime:** Rust-based memory manager using `AmHeader` for ref-counting.
- **Backend:** Emits QBE IR for native performance.
- **Memory Model:** [Link to your Engineering Log/Discussion]

*Note: This is an active research project. Design is subject to change.*
