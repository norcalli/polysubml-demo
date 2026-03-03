# Phase 1: rpds + ustr Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor compiler_lib to use persistent data structures (rpds) and global string interning (ustr), enabling cheap clone-based forking of type checker state.

**Architecture:** Replace lasso::Rodeo/Spur with ustr::Ustr for string interning (no interner object needed). Replace Vec/HashMap in the reachability graph and bindings with rpds persistent collections. Remove journal-based save/revert mechanism in favor of clone-based forking via rpds structural sharing.

**Tech Stack:** Rust, rpds (persistent data structures), ustr (global string interning), lalrpop (parser generator)

---

### Task 1: Add rpds and ustr dependencies, remove lasso

**Files:**
- Modify: `compiler_lib/Cargo.toml`

**Step 1: Update Cargo.toml**

Replace lasso with rpds and ustr:

```toml
[dependencies]
lalrpop-util = { version = "0.20.2", features = ["lexer"] }
rpds = "1"
ustr = "1"
itertools = "0.14.0"
```

**Step 2: Verify it compiles (it won't yet, but dependencies resolve)**

Run: `cd /tmp/polysubml-demo && cargo check -p compiler_lib 2>&1 | head -5`
Expected: Compilation errors (lasso references), but no dependency resolution errors.

**Step 3: Commit**

```bash
git add compiler_lib/Cargo.toml
git commit -m "chore: replace lasso with rpds + ustr dependencies"
```

---

### Task 2: Convert StringId from lasso::Spur to ustr::Ustr

This is the most pervasive change. It touches nearly every file.

**Files:**
- Modify: `compiler_lib/src/ast.rs` (StringId typedef, ParserContext, helper functions)
- Modify: `compiler_lib/src/grammar.lalr` (Ident rule, string resolution)
- Modify: `compiler_lib/src/lib.rs` (remove Rodeo from State, update process_sub)
- Modify: `compiler_lib/src/typeck.rs` (remove `strings: &mut lasso::Rodeo` params)
- Modify: `compiler_lib/src/core.rs` (remove `strings` params from check_heads/flow)
- Modify: `compiler_lib/src/type_errors.rs` (remove `strings` params)
- Modify: `compiler_lib/src/codegen.rs` (remove Rodeo from Context)
- Modify: `compiler_lib/src/parse_types.rs` (no lasso changes needed, uses StringId)

**Step 1: Update ast.rs**

Change StringId typedef and ParserContext:

```rust
// OLD: pub type StringId = lasso::Spur;
pub type StringId = ustr::Ustr;

// OLD:
// pub struct ParserContext<'a, 'input> {
//     pub span_maker: SpanMaker<'input>,
//     pub strings: &'a mut lasso::Rodeo,
// }

// NEW:
pub struct ParserContext<'input> {
    pub span_maker: SpanMaker<'input>,
}
```

Update helper functions that take `strings: &mut lasso::Rodeo`:

```rust
// enumerate_tuple_fields: replace strings param with direct ustr calls
fn enumerate_tuple_fields<T, R>(
    vals: impl IntoIterator<Item = (T, Span)>,
    mut make_field: impl FnMut(Spanned<StringId>, T) -> R,
) -> Vec<R> {
    vals.into_iter()
        .enumerate()
        .map(|(i, (val, span))| {
            let name = ustr::ustr(&format!("_{}", i));
            make_field((name, span), val)
        })
        .collect()
}

// make_tuple_expr: remove strings param
pub fn make_tuple_expr(mut vals: Vec<SExpr>) -> Expr { ... }

// make_tuple_pattern: remove strings param
pub fn make_tuple_pattern(vals: Spanned<Vec<Spanned<LetPattern>>>) -> LetPattern { ... }

// make_tuple_type: remove strings param
pub fn make_tuple_type(mut vals: Vec<STypeExpr>) -> TypeExpr { ... }
```

**Step 2: Update grammar.lalr**

The grammar uses `ctx.strings` in several rules. With ustr, string interning is global:

```lalrpop
// Change grammar signature - remove 'a lifetime since no more Rodeo reference
grammar(ctx: &mut ast::ParserContext<'input>);

// OLD: Ident: ast::StringId = StringIdent => ctx.strings.get_or_intern(<>);
// NEW:
Ident: ast::StringId = StringIdent => ustr::ustr(<>);

// OLD (SimpleType): match ctx.strings.resolve(&<>) {
// NEW:
SimpleType: ast::TypeExpr = {
    Ident => {
        match <>.as_str() {
            "any" => ast::TypeExpr::Top,
            "never" => ast::TypeExpr::Bot,
            "_" => ast::TypeExpr::Hole,
            _ => ast::TypeExpr::Ident(<>),
        }
    },
    ...
}

// OLD (VarPatName): let name = if ctx.strings.resolve(&name) == "_" {None} else {Some(name)};
// NEW:
VarPatName: (Option<ast::StringId>, spans::Span) = {
    <Spanned<Ident>> => {
        let (name, span) = <>;
        let name = if name.as_str() == "_" {None} else {Some(name)};
        (name, span)
    }
}

// OLD (VarOrLiteral): ast::expr::variable(ctx.strings.get_or_intern(s))
// NEW:
VarOrLiteral: ast::Expr = {
    Spanned<StringIdent> => {
        let (s, span) = <>;
        match s {
            "false" | "true" => ast::expr::literal(ast::Literal::Bool, (String::from(s), span)),
            _ => {
                ast::expr::variable(ustr::ustr(s))
            }
        }
    },
    ...
}

// Update tuple-related rules to not pass ctx.strings:
TupleType: ast::TypeExpr = {
    SepList<Spanned<SimpleType>, "*"> => {
        ast::make_tuple_type(<>)
    }
}

CompareOrTupleExpr: ast::Expr = {
    SepList<SCompareExpr, ","> =>
        ast::make_tuple_expr(<>)
}

TupleOrParensLetPattern: ast::LetPattern = {
    Spanned<("(" <SepList<Spanned<LetPattern>, ",">> ")")> => {
        ast::make_tuple_pattern(<>)
    }
}
```

**Step 3: Update core.rs**

Remove `strings: &mut lasso::Rodeo` from `check_heads` and `flow`. Replace `strings.resolve(&name)` with `name.as_str()`:

```rust
// check_heads signature change:
fn check_heads(
    type_ctors: &[TypeCtor],
    lhs_ind: Value,
    lhs: &VTypeNode,
    rhs_ind: Use,
    rhs: &UTypeNode,
    mut edge_context: TypeEdge,
    out: &mut Vec<(Value, Use, TypeEdge)>,
) -> Result<CheckHeadsResult, PartialTypeError> {
    // ... body: replace strings.resolve(&name) with name.as_str()
}

// flow signature change:
pub fn flow(
    &mut self,
    lhs: Value,
    rhs: Use,
    expl_span: Span,
    scopelvl: ScopeLvl,
) -> Result<(), TypeError> {
    // ... body: remove strings param from check_heads calls
    // ... replace e.add_hole_int(self, strings, ...) with e.add_hole_int(self, ...)
}
```

**Step 4: Update type_errors.rs**

Remove `strings: &mut lasso::Rodeo` from all error functions. Replace `strings.resolve(&x)` with `x.as_str()`:

```rust
// add_hole_int: remove strings param
pub fn add_hole_int(&mut self, core: &TypeCheckerCore, pair: (Value, Use)) {
    // ... replace strings.resolve(&name) with name.as_str()
}

// type_mismatch_err: remove strings param
pub fn type_mismatch_err(
    type_ctors: &[TypeCtor],
    lhs: &VTypeNode,
    rhs: &UTypeNode,
) -> PartialTypeError {
    // ... replace strings.resolve(&x) with x.as_str()
}

// type_escape_error: remove strings param
pub fn type_escape_error(
    ty_ctor: &TypeCtor,
    lhs: &VTypeNode,
    rhs: &UTypeNode,
    scopelvl: ScopeLvl,
) -> PartialTypeError {
    // ... replace strings.resolve(&ty_ctor.name) with ty_ctor.name.as_str()
}
```

**Step 5: Update typeck.rs**

Remove `strings: &mut lasso::Rodeo` from all methods. Replace `strings.get_or_intern_static("x")` with `ustr::ustr("x")`:

```rust
// TypeckState::new no longer takes strings param
pub fn new() -> Self {
    let mut core = TypeCheckerCore::new();
    let TY_BOOL = core.add_builtin_type(ustr::ustr("bool"));
    let TY_FLOAT = core.add_builtin_type(ustr::ustr("float"));
    let TY_INT = core.add_builtin_type(ustr::ustr("int"));
    let TY_STR = core.add_builtin_type(ustr::ustr("str"));
    // ... rest unchanged but remove strings refs
}

// All method signatures: remove strings param
// fn check_expr(&mut self, expr: &ast::SExpr, bound: Use) -> Result<()>
// fn infer_expr(&mut self, expr: &ast::SExpr) -> Result<Value>
// fn check_let_def(&mut self, lhs: &ast::LetPattern, expr: &ast::SExpr) -> Result<()>
// fn check_let_rec_defs(&mut self, defs: &Vec<ast::LetRecDefinition>) -> Result<()>
// fn check_statement(&mut self, def: &ast::Statement, allow_useless_exprs: bool) -> Result<()>
// pub fn check_script(&mut self, parsed: &[ast::Statement]) -> Result<()>

// In check_expr, replace strings.get_or_intern_static("Break") with ustr::ustr("Break")
// In check_expr, replace self.core.flow(strings, ...) with self.core.flow(...)
```

**Step 6: Update codegen.rs**

Replace `&'a lasso::Rodeo` with direct ustr access:

```rust
// OLD: pub struct Context<'a>(pub &'a mut ModuleBuilder, pub &'a lasso::Rodeo);
// NEW:
pub struct Context<'a>(pub &'a mut ModuleBuilder);

impl<'a> Context<'a> {
    fn get(&self, id: StringId) -> &str {
        id.as_str()
    }

    fn get_new(&self, id: StringId) -> String {
        id.as_str().to_owned()
    }
}
```

Update all callers in codegen.rs that pass the Rodeo to Context.

**Step 7: Update lib.rs**

Remove Rodeo from State:

```rust
pub struct State {
    parser: ScriptParser,
    spans: SpanManager,
    checker: TypeckState,
    compiler: ModuleBuilder,
}
impl State {
    pub fn new() -> Self {
        let checker = TypeckState::new();
        State {
            parser: ScriptParser::new(),
            spans: SpanManager::default(),
            checker,
            compiler: ModuleBuilder::new(),
        }
    }

    fn process_sub(&mut self, source: &str) -> Result<String, SpannedError> {
        let span_maker = self.spans.add_source(source.to_owned());
        let mut ctx = ast::ParserContext { span_maker };

        let ast = self
            .parser
            .parse(&mut ctx, source)
            .map_err(|e| convert_parse_error(ctx.span_maker, e))?;
        let _t = self.checker.check_script(&ast)?;

        let mut ctx = codegen::Context(&mut self.compiler);
        let js_ast = codegen::compile_script(&mut ctx, &ast);
        Ok(js_ast.to_source())
    }

    pub fn reset(&mut self) {
        self.checker = TypeckState::new();
        self.compiler = ModuleBuilder::new();
    }
}
```

**Step 8: Build and verify**

Run: `cd /tmp/polysubml-demo && cargo build -p compiler_lib 2>&1`
Expected: Clean build with no errors.

**Step 9: Run regression tests**

Run: `cd /tmp/polysubml-demo && cargo build --release --bin regression && ./runtests.sh`
Expected: All tests pass (or if baseline dir doesn't exist, at minimum `cargo build` succeeds).

**Step 10: Commit**

```bash
git add -A
git commit -m "refactor: replace lasso with ustr for global string interning

Remove lasso::Rodeo threading through all type checking and codegen
functions. ustr provides global string interning so StringId (now
ustr::Ustr) can be created anywhere without an interner reference."
```

---

### Task 3: Convert Reachability to rpds persistent data structures

**Files:**
- Modify: `compiler_lib/src/reachability.rs` (complete rewrite)
- Modify: `compiler_lib/src/core.rs` (remove save/revert/make_permanent, update TypeNode)

**Step 1: Rewrite reachability.rs with rpds**

Replace Vec with rpds::Vector, OrderedMap with rpds::HashTrieMap. Remove journal/rewind mechanism. Remove OrderedMap entirely.

```rust
use rpds::{HashTrieMap, Vector};

pub trait ExtNodeDataTrait {}

pub trait EdgeDataTrait<ExtNodeData>: Clone {
    fn update(&mut self, other: &Self) -> bool;
    fn expand(self, hole: &ExtNodeData, ind: TypeNodeInd) -> Self;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeNodeInd(pub usize);

#[derive(Clone, Debug)]
struct ReachabilityNode<ExtNodeData, ExtEdgeData> {
    data: ExtNodeData,
    flows_from: HashTrieMap<TypeNodeInd, ExtEdgeData>,
    flows_to: HashTrieMap<TypeNodeInd, ExtEdgeData>,
}

#[derive(Clone)]
pub struct Reachability<ExtNodeData, ExtEdgeData> {
    nodes: Vector<ReachabilityNode<ExtNodeData, ExtEdgeData>>,
}
impl<ExtNodeData: ExtNodeDataTrait + Clone, ExtEdgeData: EdgeDataTrait<ExtNodeData>>
    Reachability<ExtNodeData, ExtEdgeData>
{
    pub fn new() -> Self {
        Self {
            nodes: Vector::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, i: TypeNodeInd) -> Option<&ExtNodeData> {
        self.nodes.get(i.0).map(|rn| &rn.data)
    }

    pub fn get_mut(&mut self, i: TypeNodeInd) -> Option<&mut ExtNodeData> {
        self.nodes.get_mut(i.0).map(|rn| &mut rn.data)
    }

    pub fn get_edge(&self, lhs: TypeNodeInd, rhs: TypeNodeInd) -> Option<&ExtEdgeData> {
        self.nodes.get(lhs.0).and_then(|rn| rn.flows_to.get(&rhs))
    }

    pub fn add_node(&mut self, data: ExtNodeData) -> TypeNodeInd {
        let i = self.len();
        let n = ReachabilityNode {
            data,
            flows_from: HashTrieMap::new(),
            flows_to: HashTrieMap::new(),
        };
        self.nodes.push_back_mut(n);
        TypeNodeInd(i)
    }

    fn update_edge_value(
        &mut self,
        lhs: TypeNodeInd,
        rhs: TypeNodeInd,
        val: ExtEdgeData,
    ) {
        // Update flows_to on lhs node
        let lhs_node = self.nodes.get_mut(lhs.0).unwrap();
        lhs_node.flows_to.insert_mut(rhs, val.clone());

        // Update flows_from on rhs node
        let rhs_node = self.nodes.get_mut(rhs.0).unwrap();
        rhs_node.flows_from.insert_mut(lhs, val);
    }

    pub fn add_edge(
        &mut self,
        lhs: TypeNodeInd,
        rhs: TypeNodeInd,
        edge_val: ExtEdgeData,
        out: &mut Vec<(TypeNodeInd, TypeNodeInd, ExtEdgeData)>,
    ) {
        let mut work = vec![(lhs, rhs, edge_val)];

        while let Some((lhs, rhs, mut edge_val)) = work.pop() {
            let old_edge = self.nodes.get(lhs.0).unwrap().flows_to.get(&rhs).cloned();
            match old_edge {
                Some(mut old) => {
                    if old.update(&edge_val) {
                        edge_val = old;
                    } else {
                        continue;
                    }
                }
                None => {}
            };
            self.update_edge_value(lhs, rhs, edge_val.clone());

            // Collect ancestors and descendants before mutating
            let lhs_ancestors: Vec<TypeNodeInd> = self.nodes.get(lhs.0).unwrap()
                .flows_from.keys().copied().collect();
            let rhs_descendants: Vec<TypeNodeInd> = self.nodes.get(rhs.0).unwrap()
                .flows_to.keys().copied().collect();

            let temp = edge_val.clone().expand(&self.nodes.get(lhs.0).unwrap().data, lhs);
            for lhs2 in lhs_ancestors {
                work.push((lhs2, rhs, temp.clone()));
            }

            let temp = edge_val.clone().expand(&self.nodes.get(rhs.0).unwrap().data, rhs);
            for rhs2 in rhs_descendants {
                work.push((lhs, rhs2, temp.clone()));
            }

            out.push((lhs, rhs, edge_val));
        }
    }
}
```

**Step 2: Update core.rs - remove save/revert/make_permanent**

Remove `ExtNodeDataTrait::truncate` (no longer needed). Remove `save()`, `revert()`, `make_permanent()` from TypeCheckerCore. Make TypeCheckerCore derive Clone:

```rust
// Remove truncate from ExtNodeDataTrait impl for TypeNode:
impl ExtNodeDataTrait for TypeNode {}

// Remove from TypeCheckerCore:
// pub fn save(&mut self) { ... }
// pub fn revert(&mut self) { ... }
// pub fn make_permanent(&mut self) { ... }

// Add Clone derive to TypeCheckerCore (rpds types are Clone):
#[derive(Clone)]
pub struct TypeCheckerCore {
    pub r: reachability::Reachability<TypeNode, TypeEdge>,
    pub type_ctors: Vec<TypeCtor>,
    pub flowcount: u32,
    pub varcount: u32,
}
```

Note: TypeNode, VTypeHead, UTypeHead, TypeEdge, BoundPairsSet etc must all derive Clone.
VTypeHead and UTypeHead already derive Clone. TypeEdge needs Clone.
TypeNode needs Clone - add `#[derive(Clone)]` (the Var and Value/Use variants' contents are already Clone).
TypeCtor needs Clone.
InferenceVarData already derives Copy/Clone.

**Step 3: Build and verify**

Run: `cd /tmp/polysubml-demo && cargo build -p compiler_lib 2>&1`
Expected: Compilation errors in typeck.rs (still references save/revert/make_permanent). We fix those in next task.

**Step 4: Commit (may not compile yet)**

Defer commit to after Task 4 where everything compiles together.

---

### Task 4: Convert Bindings and TypeckState to clone-based forking

**Files:**
- Modify: `compiler_lib/src/typeck.rs` (Bindings, TypeckState snapshot logic)
- Modify: `compiler_lib/src/parse_types.rs` (TreeMaterializer uses Bindings)
- Remove: `compiler_lib/src/unwindmap.rs` (no longer needed for type checking)
- Modify: `compiler_lib/src/codegen.rs` (still uses UnwindMap for its own bindings)
- Modify: `compiler_lib/src/lib.rs` (remove unwindmap module if fully unused, or keep for codegen)

**Step 1: Convert Bindings to use rpds::HashTrieMap**

```rust
use rpds::HashTrieMap;

#[derive(Clone)]
pub struct Bindings {
    pub vars: HashTrieMap<StringId, Value>,
    pub types: HashTrieMap<StringId, TypeCtorInd>,
    pub scopelvl: ScopeLvl,
}
impl Bindings {
    fn new() -> Self {
        Self {
            vars: HashTrieMap::new(),
            types: HashTrieMap::new(),
            scopelvl: ScopeLvl(0),
        }
    }
}
```

**Step 2: Update TypeckState to use clone-based snapshots**

Replace all `unwind_point()`/`unwind()`/`make_permanent()` patterns with clone/restore:

```rust
// In check_script:
pub fn check_script(&mut self, parsed: &[ast::Statement]) -> Result<()> {
    let snapshot_core = self.core.clone();
    let snapshot_bindings = self.bindings.clone();

    for (i, item) in parsed.iter().enumerate() {
        let is_last = i == len - 1;
        if let Err(e) = self.check_statement(item, is_last) {
            self.core = snapshot_core;
            self.bindings = snapshot_bindings;
            return Err(e);
        }
    }
    // Success - changes are already in place, no make_permanent needed
    Ok(())
}

// In check_expr (Block case):
Block(e) => {
    let saved_bindings = self.bindings.clone();
    for stmt in e.statements.iter() {
        self.check_statement(stmt, false)?;
    }
    self.check_expr(&e.expr, bound)?;
    self.bindings = saved_bindings;
}

// In check_let_def:
// The mark/unwind pattern for evaluating RHS before LHS bindings stays the same
// conceptually but uses clone instead of unwind_point

// In infer_expr (Block, FuncDef cases): same clone/restore pattern

// In check_let_rec_defs: same pattern
```

**Step 3: Update Bindings usage everywhere**

Replace `self.bindings.vars.insert(name, ty)` with `self.bindings.vars.insert_mut(name, ty)`.
Replace `self.bindings.types.insert(name, ty)` with `self.bindings.types.insert_mut(name, ty)`.
Replace `self.bindings.vars.get(&name)` - works the same on HashTrieMap.

**Step 4: Update parse_types.rs**

The `TreeMaterializer::materialize_and_instantiate_bindings` method inserts into `bindings.vars` and `bindings.types`. Update to use `insert_mut`:

```rust
// Replace bindings.vars.insert(name, ty) with bindings.vars.insert_mut(name, ty)
// Replace bindings.types.insert(alias, ...) with bindings.types.insert_mut(alias, ...)
```

**Step 5: Handle codegen.rs**

Codegen still uses `UnwindMap` for its own JS bindings. This is separate from the type checker bindings. Keep `unwindmap.rs` for codegen use only. (It doesn't need to be persistent since codegen doesn't fork.)

**Step 6: Update TypeckState::new**

```rust
pub fn new() -> Self {
    let mut core = TypeCheckerCore::new();
    let TY_BOOL = core.add_builtin_type(ustr::ustr("bool"));
    // ... etc

    let mut new = Self {
        core,
        bindings: Bindings::new(),
        TY_BOOL, TY_FLOAT, TY_INT, TY_STR,
    };

    for (i, ty) in new.core.type_ctors.iter().enumerate() {
        new.bindings.types.insert_mut(ty.name, TypeCtorInd(i));
    }

    new
}
```

Note: The old code had `bindings.make_permanent(n)` after inserting builtin types. With rpds this is unnecessary - changes persist by default.

**Step 7: Build and verify**

Run: `cd /tmp/polysubml-demo && cargo build -p compiler_lib 2>&1`
Expected: Clean build.

**Step 8: Build full workspace**

Run: `cd /tmp/polysubml-demo && cargo build 2>&1`
Expected: Clean build (wasm and cli_code should still compile since they only use the public API).

**Step 9: Run regression tests**

Run: `cd /tmp/polysubml-demo && cargo build --release --bin regression`
If baselines exist: `./runtests.sh`
Expected: All tests pass with identical behavior.

**Step 10: Commit**

```bash
git add -A
git commit -m "refactor: replace mutable state with rpds persistent data structures

Convert Reachability to use rpds::Vector and rpds::HashTrieMap for
structural sharing. Remove journal-based save/revert/make_permanent
mechanism. TypeCheckerCore and Bindings are now cheaply cloneable,
enabling fork-based state management for incremental type checking.

Uses _mut variants on hot paths for in-place mutation when refcount
is 1, only paying persistent data structure overhead at fork points."
```

---

### Task 5: Verify Clone derives propagate correctly

**Files:**
- Modify: `compiler_lib/src/core.rs` (ensure Clone on all types)
- Modify: `compiler_lib/src/bound_pairs_set.rs` (already Clone)
- Modify: `compiler_lib/src/parse_types.rs` (PolyDeps, SourceLoc)

**Step 1: Audit and add missing Clone derives**

Ensure these types derive Clone (needed for TypeCheckerCore to be Clone):

```rust
// core.rs
#[derive(Clone, Debug)]
pub struct TypeCtor { ... }

#[derive(Clone, Debug)]
pub enum TypeNode { ... }

#[derive(Clone, Debug)]
pub struct TypeEdge { ... }

// FlowReason already derives Copy/Clone

// parse_types.rs - PolyDeps needs Clone (already has it)
// SourceLoc needs Clone (already has Copy)
```

**Step 2: Verify TypeCheckerCore is Clone**

Add a compile-time assertion:

```rust
// Temporary test in core.rs or a test file:
fn _assert_clone() {
    fn assert_clone<T: Clone>() {}
    assert_clone::<TypeCheckerCore>();
}
```

**Step 3: Verify TypeckState can snapshot correctly**

The full typeck state includes `TypeckState { core, bindings, TY_BOOL, ... }`.
TypeCtorInd is Copy, so the TY_* fields are fine. core is Clone, bindings is Clone.
TypeckState itself doesn't need to be Clone - we clone individual fields.

**Step 4: Build and run tests**

Run: `cd /tmp/polysubml-demo && cargo build --release 2>&1 && cargo build --release --bin regression`
Expected: Clean build.

**Step 5: Commit (if any changes were needed)**

```bash
git add -A
git commit -m "fix: ensure all type checker types derive Clone for fork-based state"
```

---

### Task 6: Clean up and verify end-to-end

**Files:**
- Modify: `compiler_lib/src/lib.rs` (clean up imports)
- Possibly modify other files for warnings

**Step 1: Clean up dead imports and warnings**

Run: `cd /tmp/polysubml-demo && cargo build 2>&1 | grep warning`
Fix any warnings about unused imports (e.g., lasso references).

**Step 2: Run full test suite**

Run: `cd /tmp/polysubml-demo && cargo build --release --bin regression`

If a baseline directory is set up, run regression tests:
Run: `./runtests.sh`
Expected: All tests pass.

**Step 3: Verify the CLI still works**

Run: `echo 'let x = 1; print x' | cargo run --bin cli 2>&1`
Expected: Compiles and prints output.

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: clean up imports and warnings after rpds/ustr migration"
```
