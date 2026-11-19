// slice_render — get_slice in clear: {state, chemin, run, from_turn, to_turn, slice_summary,
// l1: [{fiche, section, from_turn, to_turn}]}. No Source contains the turn yet: state open, no L1.
let found = input;
let x = found[0];
let src = x.carried.source;
if src == () {
    return #{ state: "open", run: x.carried.run, turn: x.carried.turn, l1: [] };
}
let l1 = [];
for r in x.rows {
    let d = r.data;
    l1.push(#{ fiche: d.fiche, section: d.section, from_turn: d.from_turn, to_turn: d.to_turn, state: d.state });
}
l1.sort(|a, b| { let fa = if a.from_turn == () { 0 } else { a.from_turn }; let fb = if b.from_turn == () { 0 } else { b.from_turn }; fa - fb });
#{ state: "closed", chemin: src.chemin, run: src.run, from_turn: src.from_turn, to_turn: src.to_turn,
   slice_summary: src.slice_summary, titre: src.titre, l1: l1 }
