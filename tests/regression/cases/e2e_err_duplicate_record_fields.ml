// E2E error: duplicate fields in record expression
// Exercises: typeck.rs (duplicate detection via HashMap insert)
let r = {x=1; x=2}
