// turns_found — the hits of search_turns, in their order, each turn row with its score.
let out = [];
// Keyed (`swarm`), the hits arrive under `value`.
let found = if type_of(input) == "map" && input.contains("value") { input.value } else { input };
if type_of(found) == "array" {
    for batch in found {
        if type_of(batch) == "array" {
            for r in batch { if r.data != () { let d = r.data; d.score = r.score; out.push(d); } }
        }
    }
}
// `swarm` is accepted before the swarms exist (23 November 2026): it filtered nothing, and the answer says so.
#{ turns: out, swarm: () }
