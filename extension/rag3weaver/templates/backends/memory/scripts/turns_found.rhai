// turns_found — the hits of search_turns, in their order, each turn row with its score.
let out = [];
let found = input;
if type_of(found) == "array" {
    for batch in found {
        if type_of(batch) == "array" {
            for r in batch { if r.data != () { let d = r.data; d.score = r.score; out.push(d); } }
        }
    }
}
#{ turns: out }
