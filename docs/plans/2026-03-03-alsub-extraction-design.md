# alsub: Type Checking Library Extraction

## Goal

Extract the type checking logic from compiler_lib into a standalone library (`alsub`)
that can be used interactively: infer types, build constraints, propagate relationships,
and incrementally add constraints with fork-based state management.

## Two-Phase Plan

### Phase 1: Refactor compiler_lib to rpds + ustr (this commit)

Convert internal data structures to persistent/immutable variants so the type checker
state is cheaply cloneable (fork-based). Replace lasso string interning with ustr.

### Phase 2: Extract into alsub crate (separate commit)

Move type-checking modules into a new `alsub` crate. Design public API.
Wire compiler_lib to depend on alsub.

---

## Phase 1 Design

### String Interning: lasso -> ustr

- `lasso::Spur` (StringId) + `lasso::Rodeo` -> `ustr::Ustr`
- ustr is globally interned, Copy, Eq, Hash, Ord -- no interner object needed
- Remove all `strings: &mut lasso::Rodeo` parameters from type checking code
- `strings.get_or_intern("foo")` -> `ustr("foo")`
- `strings.resolve(&name)` -> `name.as_str()`
- Parser/grammar produces Ustr instead of Spur

### Reachability Graph: Vec -> rpds

Before:
```rust
struct ReachabilityNode<N, E> {
    data: N,
    flows_from: OrderedMap<TypeNodeInd, E>,
    flows_to: OrderedMap<TypeNodeInd, E>,
}
struct Reachability<N, E> {
    nodes: Vec<ReachabilityNode<N, E>>,
    rewind_mark: TypeNodeInd,
    journal: Vec<(TypeNodeInd, TypeNodeInd, Option<E>)>,
}
```

After:
```rust
struct ReachabilityNode<N, E> {
    data: N,
    flows_from: rpds::HashTrieMap<TypeNodeInd, E>,
    flows_to: rpds::HashTrieMap<TypeNodeInd, E>,
}
struct Reachability<N, E> {
    nodes: rpds::Vector<ReachabilityNode<N, E>>,
    // No rewind_mark, no journal -- forking is .clone()
}
```

- Remove OrderedMap entirely
- Remove save(), revert(), make_permanent()
- Remove ExtNodeDataTrait::truncate() (journal revert only)
- Use _mut variants (push_back_mut, insert_mut) for in-place mutation on hot path
- Structural sharing only pays cost at fork (.clone()) points

### TypeCheckerCore: Clone-based Forking

Before: save/revert/make_permanent journal mechanism
After: TypeCheckerCore derives Clone (cheap via rpds structural sharing)

```rust
let snapshot = core.clone();
// try operations...
// on error: drop modified, restore from snapshot
```

### Bindings: UnwindMap -> rpds::HashTrieMap

Before:
```rust
pub struct Bindings {
    pub vars: UnwindMap<StringId, Value>,
    pub types: UnwindMap<StringId, TypeCtorInd>,
    pub scopelvl: ScopeLvl,
}
```

After:
```rust
#[derive(Clone)]
pub struct Bindings {
    pub vars: rpds::HashTrieMap<StringId, Value>,
    pub types: rpds::HashTrieMap<StringId, TypeCtorInd>,
    pub scopelvl: ScopeLvl,
}
```

- unwind_point()/unwind() -> let saved = bindings.clone() / bindings = saved
- make_permanent() -> no-op

### BoundPairsSet: Minimal changes

Already uses Rc with clone-on-write. Internal HashMap could optionally convert
to rpds::HashTrieMap for consistency but is low priority.

### Error Handling

No structural changes to error types. PartialTypeError and SpannedError stay as-is.
The backtrack_hole_list_sub function traverses the reachability graph read-only,
so it works with rpds without changes.

### Cargo.toml Changes

```toml
[dependencies]
rpds = "1"
ustr = "1"
# Remove lasso
```

## Decisions

- Fork-based state management (clone to fork, discard to revert)
- rpds for persistent data structures with _mut optimization
- ustr for global string interning (replaces lasso)
- Built-in type constructors (func, obj, case, abstract) -- not extensible
- All three layers exposed: constraint graph, materialization, expression-level checker
- Refactor in-place first, extract into library second
