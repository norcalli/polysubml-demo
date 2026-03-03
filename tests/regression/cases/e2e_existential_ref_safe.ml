// E2E: existential type with mutable ref - SAFE version
// The ref is created INSIDE the existential scope (via loop trick)
// so storing a: t is safe.
let {type t; a: t; b: t->t} = {a=3; b=fun x->x+1};
let ref = loop `Break {mut v=`None 0};

ref.v <- `Some a;
match ref.v with
| `Some (a: t) -> b a
| `None _ -> 0
