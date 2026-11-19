// turns_list — the turns of list_turns, newest first (by `at`, then turn and seq), cut to `limit`;
// `total` is the count before the cut.
let rows = [];
let found = input.value;
if type_of(found) == "array" {
    for batch in found { if type_of(batch) == "array" { for r in batch { if r.data != () { rows.push(r.data); } } } }
}
rows.sort(|a, b| {
    if a.at != b.at { if a.at > b.at { -1 } else { 1 } }
    else if a.turn != b.turn { b.turn - a.turn }
    else { a.seq - b.seq }
});
let total = rows.len();
if input.limit > 0 && rows.len() > input.limit { rows.truncate(input.limit); }
#{ turns: rows, total: total }
