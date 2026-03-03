// E2E error: abstract type escaping its scope
// Exercises: type_errors.rs (type_escape_error), core.rs (scope levels)
let leak = fun r -> (
    let {type t; val: t; show: t -> str} = r;
    val
)
