// trace_turn_render — « d'où vient ce tour et qu'en a-t-on retenu » : the turn (indexed or gone), its
// slice (closed, or open when no Source contains it yet), the L1 that cover it and their fiches.
let x = input[0];
let c = x.carried;
let turn = #{ turn: c.turn, state: if c.messages.len() > 0 { "indexed" } else { "gone" }, messages: c.messages };
let s = c.source;
let slice = if s == () { #{ state: "open" } } else {
    #{ state: "closed", chemin: s.chemin, from_turn: s.from_turn, to_turn: s.to_turn, slice_summary: s.slice_summary }
};
let l1 = [];
let fiches = [];
for r in x.rows {
    let d = r.data;
    l1.push(#{ fiche: d.fiche, section: d.section, source: d.source, from_turn: d.from_turn, to_turn: d.to_turn, texte: d.texte });
    if !fiches.contains(d.fiche) { fiches.push(d.fiche); }
}
#{ run: c.run, turn: turn, slice: slice, l1: l1, fiches: fiches }
