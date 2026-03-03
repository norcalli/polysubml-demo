// E2E error: ref defined outside existential, type escapes
// This is the variant where ref is destructured together with
// the existential, but ref.v is still mutable and outside the
// type's scope.
let {type t; a: t; b: t->t; ref: _} = {a=3; b=fun x->x+1; ref={mut v=`None 0}};

ref.v <- `Some a;
match ref.v with
| `Some a -> b a
| `None _ -> 0
