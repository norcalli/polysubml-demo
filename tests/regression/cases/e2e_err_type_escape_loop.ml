// E2E error: existential type escaping through loop iteration
// Each iteration unpacks a different existential type, so applying
// b to a value from a *different* iteration's existential is unsound.
let x = {mut v=`None 0; mut t=`None 0};
x.v <- `Some (x.v, {a=0; b=fun x->x+1});
x.v <- `Some (x.v, {a=0.2; b=fun x->x+.9.1});
x.v <- `Some (x.v, {a={q=1}; b=fun {q}->{q}});

loop match x.v with
| `None _ -> `Break 0
| `Some (t, h) -> (
  x.v <- t;

  let {type t; a: t; b: t->t} = h;
  print (match x.t <- `Some a with
  | `None _ -> "missing"
  | `Some t -> b t
  );

  `Continue 0
)
