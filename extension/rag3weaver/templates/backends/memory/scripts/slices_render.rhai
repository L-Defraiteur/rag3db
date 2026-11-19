// slices_render — the slices of a run in order of from_turn, each with its count of L1.
let x = input[0];
let count = #{};
for r in x.rows { let k = r.data.source; if count[k] == () { count[k] = 0; } count[k] += 1; }
let slices = [];
for r in x.carried.sources {
    let d = r.data;
    let n = if count[d.chemin] == () { 0 } else { count[d.chemin] };
    slices.push(#{ chemin: d.chemin, from_turn: d.from_turn, to_turn: d.to_turn, state: "closed", l1: n, slice_summary: d.slice_summary });
}
slices.sort(|a, b| a.from_turn - b.from_turn);
#{ run: x.carried.run, slices: slices }
