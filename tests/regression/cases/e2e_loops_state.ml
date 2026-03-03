// E2E: Loops with mutable state - exercises snapshot/rollback via persistent data structures
// Exercises: unwindmap.rs (HashMap snapshots), reachability.rs (Vector for graph)

// Simple loop with accumulator
let acc = {mut sum=0; mut i=1};
loop if acc.i > 10 then `Break 0 else (
    acc.sum <- acc.sum + acc.i;
    acc.i <- acc.i + 1;
    `Continue 0
);
print acc.sum;

// Nested loop
let state = {mut result=0; mut x=1};
loop if state.x > 3 then `Break 0 else (
    let inner = {mut y=1};
    loop if inner.y > 3 then `Break 0 else (
        state.result <- state.result + state.x * inner.y;
        inner.y <- inner.y + 1;
        `Continue 0
    );
    state.x <- state.x + 1;
    `Continue 0
);
print state.result;

// Loop building a string
let buf = {mut s=""; mut i=0};
loop if buf.i >= 5 then `Break 0 else (
    buf.s <- buf.s ^ "x";
    buf.i <- buf.i + 1;
    `Continue 0
);
print buf.s;
