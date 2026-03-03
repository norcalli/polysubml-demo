// E2E error: duplicate tags in match
// Exercises: typeck.rs (duplicate tag detection via HashMap)
let f = fun v ->
    match v with
    | `A x -> x
    | `A y -> y
