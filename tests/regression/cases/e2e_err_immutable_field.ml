// E2E error: mutation of immutable field
// Exercises: type_errors.rs (immutable_field_err), core.rs (record field mutability)
let r = {x=1; y=2};
r.x <- 5
