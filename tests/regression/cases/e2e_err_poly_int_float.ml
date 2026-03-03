// E2E error: polymorphic function used at conflicting types
// The type parameter t is instantiated to int by `f 3.2` usage context
// but the result is added to an int, requiring float + int.
let f = fun (type t) (x: t) : t -> x;
let _ = 1 + f 3.2
