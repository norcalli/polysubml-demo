// E2E: Scope and binding stress test
// Exercises: spans.rs (HashMap for name resolution), unwindmap.rs (scope snapshots)

// Shadowing
let x = 1;
let x = x + 1;
let x = x * 3;
print x;

// Nested scopes via let blocks
let result = (
    let a = 10;
    let b = 20;
    a + b
);
print result;

// Many bindings
let a = 1; let b = 2; let c = 3; let d = 4; let e = 5;
let f = 6; let g = 7; let h = 8; let i = 9; let j = 10;
print a + b + c + d + e + f + g + h + i + j;

// Closures capturing different scope levels
let outer = 100;
let f1 = fun x -> x + outer;
let f2 = (
    let inner = 50;
    fun x -> x + inner + outer
);
print f1 1;
print f2 1;

// Recursive in nested scope
let result2 = (
    let rec go = fun n -> if n <= 0 then 0 else n + go (n - 1);
    go 10
);
print result2;
