// Test boolean expressions, patterns, and matching

// Boolean literals
let x = true;
let y = false;
print x;
print y;

// Boolean from comparison
let z = 3 > 1;
print z;

// if/then/else with literals
print (if true then "yes" else "no");
print (if false then "yes" else "no");

// if/then/else with comparison
print (if 5 > 3 then "greater" else "not greater");

// Pattern matching on booleans with match
let describe = fun b -> match b with
    | true -> "it's true"
    | false -> "it's false";
print describe true;
print describe false;

// let-binding with boolean pattern (rhs must be known true)
let true = true;

// let-binding with boolean pattern via variable
let x2 = true;
let true = x2;

// Boolean in records
let r = {flag = true; value = 42};
print r.flag;
print r.value;

// Boolean as function argument and return
let negate = fun b -> if b then false else true;
print negate true;
print negate false;

// Nested if
let classify = fun n ->
    if n > 0 then "positive"
    else if n == 0 then "zero"
    else "negative";
print classify 5;
print classify 0;
print classify (-3);

// Boolean with polymorphic variants (direct `t/`f usage)
let a = `t {};
let b = `f {};
print a;
print b;
print (if a then "also true" else "also false");

// Match mixing true/false with wildcard
let check = fun x -> match x with
    | true -> "got true"
    | _ -> "got something else";
print check true;
print check false;

// Equality on booleans
print true == true;
print true == false;
print false == false;
