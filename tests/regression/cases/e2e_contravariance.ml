// E2E: contravariance and covariance in function types and mutable fields
// Functions are contravariant in arguments, covariant in return.
// Mutable fields are invariant.

// Covariant return: function returning {x; y} where only x is needed
let f = fun b -> if b then {x=1; y=2} else {x=3; y=4; z=5};
print (f true).x;
print (f false).x;

// Passing wider records to functions expecting fewer fields
let show_name = fun r -> r.name;
print show_name {name="Alice"; age=30; job="eng"};
print show_name {name="Bob"};

// Mutable field write-read cycle
let cell = {mut v=0};
cell.v <- 42;
print cell.v;
cell.v <- cell.v + 8;
print cell.v;

// Function that reads and writes a mutable field
let bump = fun r -> (
    r.v <- r.v + 1;
    r.v
);
let c = {mut v=10};
print bump c;
print bump c;
print bump c;
