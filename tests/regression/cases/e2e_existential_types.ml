// E2E: Existential types and abstract type hiding
// Exercises: parse_types.rs (HashSet for poly deps, HashMap for bindings), core.rs (abstract type Vector)

// Basic existential
let r = if true then
  {a=5; b=fun x -> x + 1}
else
  {a=10; b=fun x -> x * 2};

let {type t; a: t; b: t -> t} = r;
print b a;
print b (b a);

// Existential with different branches
let pair = if false then
  {val="hello"; show=fun x -> x ^ "!"}
else
  {val="world"; show=fun x -> x ^ "?"};

let {type s; val: s; show: s -> s} = pair;
print show val;
print show (show val);
