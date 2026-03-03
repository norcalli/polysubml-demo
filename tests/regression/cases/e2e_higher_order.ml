// E2E: Higher-order functions and closures
// Exercises: core.rs (function type nodes), parse_types.rs (closure scoping with HashMap)

let apply = fun (f, x) -> f x;
let compose = fun (f, g, x) -> f (g x);

let add1 = fun x -> x + 1;
let mul2 = fun x -> x * 2;

print apply (add1, 10);
print apply (mul2, 10);
print compose (add1, mul2, 5);
print compose (mul2, add1, 5);

// Closure capturing
let make_adder = fun n -> fun x -> x + n;
let add10 = make_adder 10;
let add20 = make_adder 20;
print add10 5;
print add20 5;

// Function returning function
let choose = fun b -> if b then add1 else mul2;
print (choose true) 7;
print (choose false) 7;

// Pipeline operator
print 5 |> add1 |> mul2 |> add1;
print 10 |> mul2 |> mul2;
