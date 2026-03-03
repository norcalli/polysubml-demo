// E2E: Variant types and pattern matching
// Exercises: core.rs (case maps with HashTrieMap -> HashMap), typeck.rs (duplicate tag detection)

let shape1 = `Circle 5.0;
let shape2 = `Rect {w=3.0; h=4.0};
let shape3 = `Point 0;

let describe = fun s ->
    match s with
    | `Circle r -> "circle"
    | `Rect {w; h} -> "rect"
    | `Point _ -> "point";

print describe shape1;
print describe shape2;
print describe shape3;

// Nested match
let eval = fun expr ->
    match expr with
    | `Num n -> n
    | `Add {l; r} -> l + r
    | `Neg n -> 0 - n;

print eval (`Num 42);
print eval (`Add {l=10; r=20});
print eval (`Neg 7);

// Wildcard patterns
let is_some = fun opt ->
    match opt with
    | `Some _ -> true
    | `None _ -> false;

print is_some (`Some 1);
print is_some (`None 0);
