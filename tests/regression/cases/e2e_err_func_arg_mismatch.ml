// E2E error: passing wrong type to function
// Exercises: type_errors.rs (type_mismatch_err for func), reachability.rs (flow graph)
let f = fun (x: int) -> x + 1;
f "not an int"
