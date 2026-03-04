// Internal modules
pub mod ast;
pub mod bound_pairs_set;
pub mod core;
pub mod instantiate;
pub mod parse_types;
pub mod reachability;
pub mod spans;
pub mod type_errors;
pub mod typeck;

// ---- Public API re-exports ----
//
// alsub is a type checking library implementing polynomial-time type inference
// with structural subtyping via bidirectional flow constraints.
//
// ## Quick Start
//
// The type system uses two dual representations:
// - `Value` (supply/covariant) — what a value provides
// - `Use` (demand/contravariant) — what a consumer requires
//
// Constraints are added via `TypeCheckerCore::flow(value, use_)`, which
// propagates through a reachability graph and checks type compatibility.
//
// The state is fully cloneable (backed by persistent data structures),
// so you can fork, try a constraint, and discard or keep the result.
//
// ## Layers
//
// 1. **Constraint graph** (`TypeCheckerCore`) — create types, add flow constraints
// 2. **Type materialization** (`TreeMaterializer`) — parse type expressions into Value/Use
// 3. **Expression checker** (`TypeckState`) — walk an AST and type-check expressions
//
// Most users will work at layer 1 (constraint graph) for ad-hoc type checking.

// Layer 1: Constraint graph — core types for building and solving constraints
pub use crate::bound_pairs_set::BoundPairsSet;
pub use crate::core::{
    FlowReason, InferenceVarData, PolyHeadData, ScopeLvl, TypeCheckerCore, TypeCtor, TypeCtorInd, TypeEdge, TypeNode,
    UTypeHead, UTypeNode, Use, VTypeHead, VTypeNode, Value, VarSpec,
};
pub use crate::reachability::TypeNodeInd;

// Layer 2: Type materialization — convert type expressions into constraint graph nodes
pub use crate::parse_types::{PolyDeps, SourceLoc, TreeMaterializer, TreeMaterializerState};

// Layer 3: Expression-level type checking
pub use crate::typeck::{Bindings, TypeckState};

// Span and error types
pub use crate::spans::{Span, SpanMaker, SpanManager, Spanned, SpannedError};

// String interning (re-export for convenience)
pub use crate::ast::StringId;
pub use ustr::{Ustr, ustr};
