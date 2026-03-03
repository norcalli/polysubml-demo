// E2E: Polymorphic functions with explicit and implicit instantiation
// Exercises: instantiate.rs (HashMap for type params), core.rs (HashSet for type vars)

let id = fun (type t) (x: t): t -> x;
print id 42;
print id "hello";
print id true;

// Explicit instantiation
let id_int: int -> int = id[t=int];
print id_int 100;

// Polymorphic pair swap
let swap = fun (type a b) (x: a, y: b): b * a -> (y, x);
print swap (1, "two");
print swap ("alpha", 99);

// Nested polymorphic usage
let apply = fun (type a b) (f: a -> b, x: a): b -> f x;
let double = fun x -> x * 2;
print apply (double, 21);

// Composition with polymorphism
let compose = fun (type a b c) (f: b -> c, g: a -> b, x: a): c -> f (g x);
let inc = fun x -> x + 1;
let triple = fun x -> x * 3;
print compose (triple, inc, 10);
