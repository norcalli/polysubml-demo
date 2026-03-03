// E2E error: existential type escaping through mutable ref
// The existential type t is bound inside the destructuring, but
// ref.v is defined outside, so storing `a: t` in it and then
// using `b` on the retrieved value is unsound - ref could hold
// a value of a different type than what b expects.
let ref = {mut v=`None 0};
let {type t; a: t; b: t->t} = {a=3; b=fun x->x+1};

ref.v <- `Some a;
match ref.v with
| `Some a -> b a
| `None _ -> 0
