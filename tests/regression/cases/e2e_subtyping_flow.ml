// E2E: Subtyping and flow analysis
// Exercises: reachability.rs (flow graph with HashMap/Vector), bound_pairs_set.rs, type_errors.rs

// Width subtyping - pass record with extra fields
let get_x = fun r -> r.x;
print get_x {x=42; y=100};
print get_x {x=99; y=0; z="extra"};

// Function subtyping
let apply_to_five = fun f -> f 5;
let double = fun x -> x * 2;
let as_str = fun x -> "got it";
print apply_to_five double;
print apply_to_five as_str;

// Variant subtyping (narrowing)
let handle_ab = fun v ->
    match v with
    | `A x -> x
    | `B x -> x * 10;

print handle_ab (`A 3);
print handle_ab (`B 4);

// If-else with different variant subsets
let make_val = fun b ->
    if b then `Ok 42
    else `Err "bad";

let r1 = make_val true;
let r2 = make_val false;
let extract = fun v ->
    match v with
    | `Ok n -> n
    | `Err _ -> 0;
print extract r1;
print extract r2;
