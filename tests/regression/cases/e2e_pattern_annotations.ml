// E2E: type annotations in patterns - exercises parse_types.rs binding resolution

// Tuple pattern with annotations
let (a: int, b: str, c, d: float) = 1, "", "", 3.2;
print a;
print b;
print d;

// Record pattern with annotations and renaming
let {a; b: int; c=x: int; d={e: float}} = {a=1; b=2; c=3; d={e=4.4}};
print a;
print x;

// Variant match with annotation
let v = match `Foo {x=32} with
| `Foo {x: int} -> x;
print v;

// Annotated function parameter
let add = fun {a: int; b: int} -> a + b;
print add {a=10; b=20};
