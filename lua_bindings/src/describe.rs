use std::collections::HashSet;

use alsub::reachability::TypeNodeInd;
use alsub::{TypeCheckerCore, TypeNode, UTypeHead, VTypeHead, Value, Use};

const MAX_DEPTH: usize = 10;
const NONE: TypeNodeInd = TypeNodeInd(usize::MAX);

/// How to describe inference variables
#[derive(Clone, Copy, PartialEq)]
enum VarMode {
    /// Normal: follow flows_from to find Value nodes (what IS it)
    Supply,
    /// Demanded: follow flows_to to find Use nodes (what's REQUIRED of it)
    Demand,
}

pub fn describe_value(core: &TypeCheckerCore, val: Value) -> String {
    let mut visited = HashSet::new();
    desc(core, val.0, &mut visited, 0, VarMode::Supply)
}

pub fn describe_use(core: &TypeCheckerCore, use_: Use) -> String {
    let mut visited = HashSet::new();
    desc(core, use_.0, &mut visited, 0, VarMode::Supply)
}

/// Describe what is *demanded* of a value — for Var nodes, follows flows_to
/// to find Use constraints. Recurses through structure so e.g. a function
/// `#|r| r.x + 1` shows `{x: int} -> int` instead of `? -> int`.
pub fn describe_demanded(core: &TypeCheckerCore, val: Value) -> String {
    let mut visited = HashSet::new();
    desc(core, val.0, &mut visited, 0, VarMode::Demand)
}

fn desc(
    core: &TypeCheckerCore,
    ind: TypeNodeInd,
    visited: &mut HashSet<usize>,
    depth: usize,
    mode: VarMode,
) -> String {
    if ind == NONE {
        return "_".to_string();
    }
    if depth > MAX_DEPTH || !visited.insert(ind.0) {
        return "...".to_string();
    }

    let result = match core.r.get(ind) {
        Some(node) => match node {
            TypeNode::Var(_) => desc_var(core, ind, visited, depth, mode),
            TypeNode::Value((head, _, _)) => desc_vhead(core, head, visited, depth, mode),
            TypeNode::Use((head, _, _)) => desc_uhead(core, head, visited, depth, mode),
            TypeNode::Placeholder => "placeholder".to_string(),
        },
        None => "?".to_string(),
    };

    visited.remove(&ind.0);
    result
}

fn desc_var(
    core: &TypeCheckerCore,
    ind: TypeNodeInd,
    visited: &mut HashSet<usize>,
    depth: usize,
    mode: VarMode,
) -> String {
    match mode {
        VarMode::Supply => {
            // Collect concrete Value types flowing INTO this variable
            let sources = core.r.flows_from_keys(ind);
            let mut parts = Vec::new();
            for src_ind in sources {
                if let Some(TypeNode::Value(_)) = core.r.get(src_ind) {
                    let d = desc(core, src_ind, visited, depth + 1, mode);
                    if !parts.contains(&d) {
                        parts.push(d);
                    }
                }
            }
            if parts.is_empty() {
                "?".to_string()
            } else if parts.len() == 1 {
                parts.pop().unwrap()
            } else {
                parts.join(" | ")
            }
        }
        VarMode::Demand => {
            // Collect Use nodes this variable flows TO (what's demanded of it)
            let targets = core.r.flows_to_keys(ind);
            let mut parts = Vec::new();
            for tgt_ind in targets {
                if let Some(TypeNode::Use(_)) = core.r.get(tgt_ind) {
                    let d = desc(core, tgt_ind, visited, depth + 1, mode);
                    if !parts.contains(&d) {
                        parts.push(d);
                    }
                }
            }
            if parts.is_empty() {
                "?".to_string()
            } else if parts.len() == 1 {
                parts.pop().unwrap()
            } else {
                parts.join(" & ")
            }
        }
    }
}

fn desc_vhead(
    core: &TypeCheckerCore,
    head: &VTypeHead,
    visited: &mut HashSet<usize>,
    depth: usize,
    mode: VarMode,
) -> String {
    match head {
        VTypeHead::VAbstract { ty } => core
            .type_ctors
            .get(ty.0)
            .map(|tc| tc.name.as_str().to_string())
            .unwrap_or_else(|| format!("type#{}", ty.0)),
        VTypeHead::VFunc { arg, ret } => {
            let arg_s = desc(core, arg.0, visited, depth + 1, mode);
            let ret_s = desc(core, ret.0, visited, depth + 1, mode);
            format!("{} -> {}", wrap_if_func(&arg_s), ret_s)
        }
        VTypeHead::VObj { fields } => {
            let mut parts = Vec::new();
            for (name, (read_val, _, _)) in fields.iter() {
                let ty = desc(core, read_val.0, visited, depth + 1, mode);
                parts.push(format!("{}: {}", name.as_str(), ty));
            }
            format!("{{{}}}", parts.join(", "))
        }
        VTypeHead::VCase { case: (tag, val) } => {
            let payload = desc(core, val.0, visited, depth + 1, mode);
            format!("`{} {}", tag.as_str(), payload)
        }
        VTypeHead::VTop => "top".to_string(),
        VTypeHead::VUnion(values) => {
            let parts: Vec<_> = values
                .iter()
                .map(|v| desc(core, v.0, visited, depth + 1, mode))
                .collect();
            parts.join(" | ")
        }
        VTypeHead::VPolyHead(_, sub, _) => desc(core, sub.0, visited, depth + 1, mode),
        VTypeHead::VTypeVar(spec) => spec.name.as_str().to_string(),
        VTypeHead::VDisjointIntersect(_, sub) => sub
            .map(|v| desc(core, v.0, visited, depth + 1, mode))
            .unwrap_or_else(|| "top".to_string()),
        VTypeHead::VInstantiateExist { target, .. } => {
            desc(core, target.0, visited, depth + 1, mode)
        }
    }
}

fn desc_uhead(
    core: &TypeCheckerCore,
    head: &UTypeHead,
    visited: &mut HashSet<usize>,
    depth: usize,
    mode: VarMode,
) -> String {
    match head {
        UTypeHead::UAbstract { ty } => core
            .type_ctors
            .get(ty.0)
            .map(|tc| tc.name.as_str().to_string())
            .unwrap_or_else(|| format!("type#{}", ty.0)),
        UTypeHead::UFunc { arg, ret } => {
            let arg_s = desc(core, arg.0, visited, depth + 1, mode);
            let ret_s = desc(core, ret.0, visited, depth + 1, mode);
            format!("{} -> {}", wrap_if_func(&arg_s), ret_s)
        }
        UTypeHead::UObj { fields } => {
            let mut parts = Vec::new();
            for (name, (read_use, _, _)) in fields.iter() {
                let ty = desc(core, read_use.0, visited, depth + 1, mode);
                parts.push(format!("{}: {}", name.as_str(), ty));
            }
            format!("{{{}}}", parts.join(", "))
        }
        UTypeHead::UCase { cases, wildcard } => {
            let mut parts = Vec::new();
            for (tag, use_) in cases.iter() {
                let ty = desc(core, use_.0, visited, depth + 1, mode);
                parts.push(format!("`{} {}", tag.as_str(), ty));
            }
            if let Some(w) = wildcard {
                let ty = desc(core, w.0, visited, depth + 1, mode);
                parts.push(format!("_ {}", ty));
            }
            format!("[{}]", parts.join(" | "))
        }
        UTypeHead::UBot => "bot".to_string(),
        UTypeHead::UIntersection(uses) => {
            let parts: Vec<_> = uses
                .iter()
                .map(|u| desc(core, u.0, visited, depth + 1, mode))
                .collect();
            parts.join(" & ")
        }
        UTypeHead::UPolyHead(_, sub, _) => desc(core, sub.0, visited, depth + 1, mode),
        UTypeHead::UTypeVar(spec) => spec.name.as_str().to_string(),
        UTypeHead::UDisjointUnion(_, sub) => sub
            .map(|u| desc(core, u.0, visited, depth + 1, mode))
            .unwrap_or_else(|| "bot".to_string()),
        UTypeHead::UInstantiateUni { target, .. } => {
            desc(core, target.0, visited, depth + 1, mode)
        }
    }
}

fn wrap_if_func(s: &str) -> String {
    if s.contains(" -> ") {
        format!("({})", s)
    } else {
        s.to_string()
    }
}
