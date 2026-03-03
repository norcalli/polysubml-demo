// E2E: Recursive functions - exercises type checker state management
// Exercises: core.rs (persistent maps for scope), typeck.rs (scope levels via HashMap)

let rec factorial = fun n ->
    if n <= 1 then 1
    else n * factorial (n - 1);

print factorial 1;
print factorial 5;
print factorial 10;

// Mutual-style recursion via records
let rec fib = fun n ->
    if n <= 1 then n
    else fib (n - 1) + fib (n - 2);

print fib 0;
print fib 1;
print fib 5;
print fib 10;

// Recursive with accumulator
let rec sum_to = fun (n, acc) ->
    if n <= 0 then acc
    else sum_to (n - 1, acc + n);

print sum_to (100, 0);
