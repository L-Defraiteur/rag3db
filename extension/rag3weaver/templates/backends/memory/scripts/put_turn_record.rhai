// put_turn_record — the Turn row of one message: secrets masked in text, params and result (the host
// function mask_secrets), params bounded at 4000 characters and result at 8000, with the original
// size said; how many secrets were masked and whether anything was cut stay on the row.
fn bound(s, max) {
    if s.len() <= max { return #{ text: s, cut: false }; }
    #{ text: s.sub_string(0, max) + `… (${s.len()} characters)`, cut: true }
}
let i = input;
if i.run == "" { throw "put_turn: run is empty"; }
if i.turn < 1 { throw "put_turn: turn starts at 1"; }
if i.seq < 0 { throw "put_turn: seq starts at 0"; }
let text = mask_secrets(i.text);
let params = mask_secrets(i.params);
let result = mask_secrets(i.result);
let p = bound(params.text, 4000);
let r = bound(result.text, 8000);
let rec = #{
    key: `${i.run}#${i.turn}#${i.seq}`,
    run: i.run, turn: i.turn, seq: i.seq, at: i.at, role: i.role,
    text: text.text, status: i.status,
    masked: text.count + params.count + result.count,
    cut: p.cut || r.cut,
    section: `turn ${i.turn}`,
};
if i.tool_name != "" { rec.tool_name = i.tool_name; }
if i.params != "" { rec.params = p.text; }
if i.result != "" { rec.result = r.text; }
rec
