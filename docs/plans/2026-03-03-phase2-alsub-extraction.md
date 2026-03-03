# Phase 2: Extract alsub Library

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract the type checking logic into a standalone `alsub` crate that can be used independently for interactive constraint solving.

**Architecture:** Move all type system modules (core, reachability, instantiate, bound_pairs_set, type_errors, parse_types, typeck, spans, ast, unwindmap) into a new `alsub` crate. compiler_lib keeps only the parser (grammar), codegen (codegen, js), and orchestration (lib.rs), depending on alsub.

**Tech Stack:** Rust workspace with two library crates.

---

### Task 1: Create alsub crate and move files

**Step 1:** Create `alsub/` directory with `Cargo.toml` and `src/lib.rs`.

alsub/Cargo.toml:
```toml
[package]
name = "alsub"
version = "0.1.0"
edition = "2024"

[dependencies]
rpds = "1"
ustr = "1"
itertools = "0.14.0"
```

**Step 2:** Move these files from compiler_lib/src/ to alsub/src/:
- spans.rs
- ast.rs, ast/ directory (contains expr.rs)
- core.rs
- reachability.rs
- instantiate.rs
- bound_pairs_set.rs
- type_errors.rs
- parse_types.rs
- typeck.rs
- unwindmap.rs

**Step 3:** Create alsub/src/lib.rs with module declarations and public re-exports.

**Step 4:** Update all `use crate::` references in moved files to still work (they now reference the alsub crate, so `use crate::` is fine since they're in the same crate).

**Step 5:** Update compiler_lib/Cargo.toml:
- Add `alsub = { path = "../alsub" }` dependency
- Remove rpds, ustr, itertools (now in alsub)
- Keep lalrpop-util

**Step 6:** Update compiler_lib/src/lib.rs:
- Remove module declarations for moved modules
- Add `pub use alsub::ast;` and `pub use alsub::spans;` so grammar.lalr's `use super::ast/spans` still resolves
- Import from alsub what's needed

**Step 7:** Update compiler_lib/src/grammar.lalr:
- Should work unchanged since `super::ast` and `super::spans` resolve via re-exports

**Step 8:** Update compiler_lib/src/codegen.rs:
- Change `use crate::ast` to `use alsub::ast` (or it can use the re-export)
- Change `use crate::unwindmap` to `use alsub::unwindmap`
- Change `use crate::js` stays (js.rs stays in compiler_lib)

**Step 9:** Update root Cargo.toml workspace members to include alsub.

**Step 10:** Build and verify. Commit.

---

### Task 2: Design and expose public API on alsub

Add documentation and organize public exports in alsub/src/lib.rs so users can:
1. Create a TypeCheckerCore
2. Build types (Value, Use)
3. Add flow constraints
4. Clone state for forking
5. Optionally use TypeckState for expression-level checking
6. Use TreeMaterializer for type materialization

Commit.

---

### Task 3: Clean up and verify

Fix warnings, run tests, verify CLI works. Commit.
