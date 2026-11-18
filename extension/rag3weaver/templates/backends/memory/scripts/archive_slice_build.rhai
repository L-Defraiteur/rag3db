// archive_slice_build — the elements of step 2 from the extraction: one per topic, titled
// « turns a-b · <title> » (a, b: its first and last turn), whose text follows step2_input (the
// extracted « said », its raw turns bounded by raw_turns_max_tokens, or both); then one residual
// « turns a-b · reste » per maximal run of turns that no topic covers and no not_filed sets aside.
// Each element carries the slice meta (slice_summary, open) for the report.
fn turn_blocks(text) {
    let blocks = #{};
    let order = [];
    let current = ();
    let buf = "";
    for line in text.split("\n") {
        if line.starts_with("## turn ") {
            if current != () { blocks[current] = buf; }
            let n = parse_int(line.sub_string(8).trim_text());
            current = n.to_string();
            order.push(n);
            buf = line + "\n";
        } else if current != () {
            buf += line + "\n";
        }
    }
    if current != () { blocks[current] = buf; }
    #{ blocks: blocks, order: order }
}
fn trim_text(s) { let t = s; t.trim(); t }
let x = input.value;
if x == () || x.topics == () { throw "archive_slice : the extraction returned no topics"; }
let tb = turn_blocks(input.contenu);
let blocks = tb.blocks;
let order = tb.order;
let budget = input.raw_turns_max_tokens * 4;
let covered = #{};
let elements = [];
let open = [];
let i = 0;
for t in x.topics {
    let lo = 1000000000;
    let hi = -1;
    let raw = "";
    for r in t.turns {
        if r.from > r.to { throw `archive_slice : the topic « ${t.title} » has the range ${r.from}-${r.to}`; }
        for n in range(r.from, r.to + 1) {
            let k = n.to_string();
            if blocks[k] == () { throw `archive_slice : the topic « ${t.title} » cites turn ${n}, which is not in the slice`; }
            covered[k] = true;
            if n < lo { lo = n; }
            if n > hi { hi = n; }
            if raw.len() + blocks[k].len() <= budget { raw += blocks[k] + "\n"; }
        }
    }
    let text = if input.step2_input == "extracted" { t.said } else if input.step2_input == "raw_turns" { raw } else { t.said + "\n\n" + raw };
    text.trim();
    if t.open == true { open.push(t.title); }
    elements.push(#{ title: `turns ${lo}-${hi} · ${t.title}`, level: 2, text: text, anchor: "", index: i, chemin: input.chemin, titre: input.titre, origine: input.origine });
    i += 1;
}
let set_aside = #{};
if x.not_filed != () {
    for r in x.not_filed { for n in range(r.from, r.to + 1) { set_aside[n.to_string()] = true; } }
}
// The residual: maximal runs of turns neither covered nor set aside.
let run_lo = -1;
let run_hi = -1;
let residuals = [];
for n in order {
    let k = n.to_string();
    let loose = covered[k] == () && set_aside[k] == ();
    if loose {
        if run_lo < 0 { run_lo = n; }
        run_hi = n;
    } else if run_lo >= 0 {
        residuals.push([run_lo, run_hi]); run_lo = -1;
    }
}
if run_lo >= 0 { residuals.push([run_lo, run_hi]); }
for r in residuals {
    let raw = "";
    for n in range(r[0], r[1] + 1) { let k = n.to_string(); if raw.len() + blocks[k].len() <= budget { raw += blocks[k] + "\n"; } }
    raw.trim();
    elements.push(#{ title: `turns ${r[0]}-${r[1]} · reste`, level: 2, text: raw, anchor: "", index: i, chemin: input.chemin, titre: input.titre, origine: input.origine });
    i += 1;
}
let summary = if x.slice_summary == () { "" } else { x.slice_summary };
for j in 0..elements.len() { elements[j].slice_summary = summary; elements[j].slice_open = open; }
elements
