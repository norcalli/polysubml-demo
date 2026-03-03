// E2E error: unhandled variant in match
// Exercises: type_errors.rs (unhandled_variant_err), core.rs (case map)
let f = fun v ->
    match v with
    | `A x -> x;
f (`B 42)
