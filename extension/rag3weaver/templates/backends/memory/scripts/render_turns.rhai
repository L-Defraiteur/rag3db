// render_turns — the single form of a slice (archi and mémoire, 18 November 2026), written here
// only: a « # run <run>, turns <a>-<b> » title, then one « ## turn <n> » per turn and one
// « ### <role> · <at> » per message in seq order; a tool message adds its name and status to the
// heading and carries its params and result in fenced blocks. The banc sends the same form.
let rows = [];
let found = input.value;
if type_of(found) == "array" {
    for batch in found {
        if type_of(batch) == "array" { for r in batch { if r.data != () { rows.push(r.data); } } }
    }
}
rows.sort(|a, b| if a.turn != b.turn { a.turn - b.turn } else { a.seq - b.seq });
let text = `# run ${input.run}, turns ${input.from_turn}-${input.to_turn}\n\n`;
let turns = [];
for m in rows {
    if !turns.contains(m.turn) {
        turns.push(m.turn);
        text += `## turn ${m.turn}\n\n`;
    }
    if m.role == "tool" {
        let name = if m.tool_name == () { "" } else { m.tool_name };
        text += `### tool · ${m.at} · ${name} · ${m.status}\n\n`;
        if m.text != () && m.text != "" { text += m.text + "\n\n"; }
        if m.params != () { text += "params:\n```json\n" + m.params + "\n```\n\n"; }
        if m.result != () { text += "result:\n```\n" + m.result + "\n```\n\n"; }
    } else {
        text += `### ${m.role} · ${m.at}\n\n${m.text}\n\n`;
    }
}
#{ text: text, turns: turns, messages: rows.len() }
