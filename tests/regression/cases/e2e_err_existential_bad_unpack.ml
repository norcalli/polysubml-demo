// E2E error: existential unpack with inconsistent fields
// Record has a: int, b: int->int, c: float, d: float->float
// but we try to unpack with a single type t for both a and c.
let r = {a=1; b=fun x->x+1; c=1.2; d=fun x->x+.2.1};
let {type t; a: t; b: t->t} = r;
let _ = 4 +. 3
