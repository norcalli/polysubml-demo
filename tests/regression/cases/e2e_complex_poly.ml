// E2E: Complex polymorphism - multiple type params, partial instantiation
// Exercises: instantiate.rs (multi-param HashMap), parse_types.rs (PolyDeps HashSet)

// Multiple type parameters
let fst = fun (type a b) (x: a, y: b): a -> x;
let snd = fun (type a b) (x: a, y: b): b -> y;

print fst (1, "hello");
print snd (1, "hello");

// Partial instantiation
let map_pair = fun (type a b c) (f: a -> b, g: a -> c, x: a): b * c -> (f x, g x);
let inc = fun x -> x + 1;
let dbl = fun x -> x * 2;
print map_pair (inc, dbl, 10);

// Polymorphic with records
let make_box = fun (type t) (x: t): {val: t} -> {val=x};
print (make_box 42).val;
print (make_box "test").val;

// Reuse poly function across types in same expression
let id = fun (type t) (x: t): t -> x;
print id 1, id "a", id true;
