// E2E error: accessing nonexistent field
// Exercises: type_errors.rs (missing_field_err), core.rs (record field HashMap)
let r = {x=1; y=2};
r.z
