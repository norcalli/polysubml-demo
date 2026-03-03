// E2E: complex polymorphic function with nested type parameters
// Tests inference with higher-rank polymorphism and multiple instantiations.
let _ = fun (type t) (x: t, f: type u. t * u->int * u) : int * t -> (
  let (a, b) = f (x, 23);
  let (c, d) = f (x, {x=a+b});
  let _ = c + d.x;

  f (x, x)
);

// Polymorphic identity used at multiple types in one expression
let id = fun (type t) (x: t): t -> x;
print id 1, id "two", id 3.0;

// Explicit instantiation with multiple type params
let const = fun (type a b) (x: a, y: b): a -> x;
print const (42, "ignored");
print const ("kept", 99);
