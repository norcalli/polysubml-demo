// E2E error: accessing a field that only exists in one branch
// The type of r is the intersection of both branches, which only
// has fields a and b - c only exists in the true branch.
let get_record = fun flag ->
    if flag then
        {a=1; b=2; c=3}
    else
        {a=4; b=5; d=6};

let r = get_record true;
print r.c
