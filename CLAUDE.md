# polysubml-demo

Polynomial-time type inference with structural subtyping via bidirectional flow constraints.

## Build & Test

```bash
cargo build                      # build all crates
cargo test                       # run all tests (133+ regression tests)
cargo build -p lua_bindings      # build Lua module (.so)
cargo build -p cli_code          # build CLI
cargo build -p wasm              # build WASM target
```

## Workspace Crates

| Crate | Purpose |
|-------|---------|
| `alsub` | Core type system: constraint graph, type materialization, expression checker, parser (LALRPOP) |
| `compiler_lib` | Compilation pipeline: parse → typecheck → codegen (JS + Lua targets) |
| `cli_code` | CLI binary |
| `wasm` | WASM binary (wasm-bindgen) |
| `lua_bindings` | mlua (LuaJIT) bindings exposing alsub + compiler_lib to Lua |

## Architecture

### Type System (alsub)

Three layers:

1. **Constraint graph** (`core.rs`): `TypeCheckerCore` manages `Value` (covariant/supply) and `Use` (contravariant/demand) nodes connected by flow constraints. Uses reachability graph for transitive closure.

2. **Type materialization** (`parse_types.rs`): Converts constraint graph to readable types. Handles polymorphism via `PolyHeadData`, `PolyDeps`, `SourceLoc`.

3. **Expression checker** (`typeck.rs`): `TypeckState` walks AST, creates constraints. `Bindings` tracks variable/type scopes with `ScopeLvl` for escape prevention.

### Key Types

- `Value(TypeNodeInd)` / `Use(TypeNodeInd)` — opaque indices into constraint graph (Copy)
- `VTypeHead` — value type constructors: VFunc, VObj, VCase, VUnion, VTop, VAbstract, VPolyHead, etc.
- `UTypeHead` — use type constructors: UFunc, UObj, UCase, UIntersection, UBot, UAbstract, UPolyHead, etc.
- `ScopeLvl(u32)` — scope nesting depth for type escape checking
- `TypeCtorInd(usize)` — index for abstract/builtin types
- `Span(usize)` — source location handle managed by `SpanManager`
- `StringId = ustr::Ustr` — interned strings (identity-hashed via `UstrBuildHasher`)
- `StringIdMap<V> = im_rc::HashMap<StringId, V, UstrBuildHasher>` — persistent hash map

### AST (`alsub/src/ast.rs`, `alsub/src/ast/expr.rs`)

- `Statement`: Empty, Expr, LetDef, LetRecDef, Println
- `Expr` (15 variants): BinOp, Block, Call, Case, FieldAccess, FieldSet, FuncDef, InstantiateExist, InstantiateUni, Literal, Loop, Match, Record, Typed, Variable
- `LetPattern`: Case, Record, Var
- `SExpr = (Expr, Span)` — spanned expression
- Parser: LALRPOP grammar at `alsub/src/grammar.lalrpop`

### Compilation Pipeline (`compiler_lib`)

`State` holds parser + spans + checker + JS codegen + Lua codegen.

- `process(source) -> CompilationResult` — full JS pipeline
- `process_lua(source) -> CompilationResult` — full Lua pipeline
- Split API: `parse()`, `check()`, `generate_js()`, `generate_lua()`, `format_error()`

### Lua Bindings (`lua_bindings`)

mlua module loadable via `require("alsub")`. crate-type = `cdylib`.

**Exports:** Core, SpanManager, Compiler, Bindings, TypeckState, ScopeLvl, SourceLoc, VarSpec, PolyHeadData

**Files:**
- `core.rs` — LuaCore (TypeCheckerCore), opaque handles (LuaValue, LuaUse, LuaSpan, etc.), span management, bindings, TypeckState
- `types.rs` — VTypeHead/UTypeHead constructors registered on LuaCore (val_func, use_func, etc.)
- `ast.rs` — LuaScript, LuaStatement, LuaExpr, LuaLetPattern with per-variant accessors
- `compiler.rs` — LuaCompiler wrapping compiler_lib::State

**Pattern:** FromLua implemented via `impl_from_lua!` (Copy types) and `impl_from_lua_clone!` macros. IntoLua is provided by mlua's blanket impl for UserData.

## Important Patterns

- **Persistent data structures** (`im-rc`): `StringIdMap` enables cheap clone/snapshot of `TypeCheckerCore` and `Bindings`
- **String interning** (`ustr`): All identifiers are `Ustr` with O(1) equality and hashing
- **Span management**: `SpanManager` owns sources, `SpanMaker` creates spans with source index, `Span(usize)` is a lightweight handle
- **Error reporting**: `SpannedError` accumulates spans + messages, rendered by `SpanManager::print()`

## Tests

Regression tests in `tests/regression/` — input `.ml` files with expected output. Run via `cargo test`.

## Edition

Rust edition 2024. Resolver 3.
