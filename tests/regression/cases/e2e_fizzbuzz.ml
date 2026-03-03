// E2E: FizzBuzz - exercises union types (int | str), recursion, and print
// The if-else returns either a string or an int, testing subtype joins.
let rec fizzbuzz = fun i -> (
  print (if i % 3 == 0 then
    if i % 5 == 0 then
      "FizzBuzz"
    else
      "Fizz"
  else
    if i % 5 == 0 then
      "Buzz"
    else
      i
  );
  if i < 20 then
    fizzbuzz (i+1)
  else
    0
);
fizzbuzz 0
