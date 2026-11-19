// read_turns — each turn of the range, in order, with its state: `indexed` (its Turn rows, in seq
// order) or `gone` (no row any more: past the TTL, or never indexed). The host completes `gone`
// from its chat file (from_chat_file). Never a silent gap.
let i = input;
if i.from_turn < 1 || i.to_turn < i.from_turn { throw `read_turns: ${i.from_turn}-${i.to_turn} is not a range of turns`; }
if i.to_turn - i.from_turn >= 1000 { throw "read_turns: at most 1000 turns at once"; }
let by = #{};
let found = i.value;
if type_of(found) == "array" {
    for batch in found { if type_of(batch) == "array" { for r in batch { if r.data != () {
        let k = r.data.turn.to_string();
        if by[k] == () { by[k] = []; }
        by[k].push(r.data);
    } } } }
}
let turns = [];
for n in range(i.from_turn, i.to_turn + 1) {
    let m = by[n.to_string()];
    if m == () {
        turns.push(#{ turn: n, state: "gone", messages: [] });
    } else {
        m.sort(|a, b| a.seq - b.seq);
        turns.push(#{ turn: n, state: "indexed", messages: m });
    }
}
#{ run: i.run, turns: turns }
