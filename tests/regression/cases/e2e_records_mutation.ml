// E2E: Records with mutable fields, field access, destructuring
// Exercises: typeck.rs (HashMap for duplicate detection), core.rs (record field maps)

let point = {x=1; y=2};
print point.x;
print point.y;

// Destructuring
let {x; y} = {x=10; y=20};
print x;
print y;

// Mutable fields
let counter = {mut val=0; label="counter"};
print counter.val;
counter.val <- counter.val + 1;
print counter.val;
counter.val <- counter.val + 5;
print counter.val;

// Nested records
let nested = {inner={a=1; b=2}; outer=3};
print nested.inner.a;
print nested.inner.b;
print nested.outer;

// Record with many fields (stress HashMap)
let big = {a=1; b=2; c=3; d=4; e=5; f=6; g=7; h=8};
print big.a;
print big.d;
print big.h;
