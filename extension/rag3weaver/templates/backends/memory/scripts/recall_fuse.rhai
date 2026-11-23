// recall_fuse — the fusion of recall_for_request: a hit is recalled only on evidence — its dense similarity reaches
// min_similarity (never a nearest neighbour recalled to fill). A lexical hit is evidence only for a hit with no dense
// signal: measured on 23 November 2026, bm25 finds an unrelated question by its common words (« how is the weather »:
// 0.68) above a relevant one (0.58), while the similarity separates them (0.29 and 0.44 against 0.69 and 0.82); then each hit scores weight(tier) × decay(its age rank in its tier) /
// (60 + its rank in its tier); the best `top`, deduplicated, rendered in full under the budget as one block labelled
// « Rappel automatique », placed last in the context by the host. Nothing found (or nothing above min_score): no block,
// and it is not a fallback.
fn text_of(tier, d) {
    if tier == "turns" { return if d.text == () { "" } else { d.text }; }
    if tier == "apports" { return if d.texte == () { "" } else { d.texte }; }
    if d.about == () { "" } else { d.about }
}
fn header_of(tier, d) {
    if tier == "turns" { return `[tour ${d.turn}, ${d.at}, ${d.role}]`; }
    if tier == "apports" { return `[apport de la fiche « ${d.fiche} », ${d.section}]`; }
    `[fiche « ${d.name} » (${d.state})]`
}
fn source_of(tier, r) {
    let d = r.data;
    if tier == "turns" {
        if d.line_id != () && d.line_id != "" { return "turn:" + d.line_id; }
        return `turn:${d.run}#${d.turn}#${d.seq}`;
    }
    if tier == "apports" { return "apport:" + r.uuid; }
    "card:" + d.name
}
let i = input;
let weights = if i.weights == () { #{} } else { i.weights };
let recency = if i.recency == () { #{} } else { i.recency };
let min_score = if i.min_score == () { 0.0 } else { 0.0 + i.min_score };
let min_similarity = if i.min_similarity == () { 0.5 } else { 0.0 + i.min_similarity };
// The evidence of a hit: what each signal said of it (label, raw score, rank), as the fusion kept it.
fn evidence(r, min_similarity) {
    let sigs = if r.signals == () { [] } else { r.signals };
    if sigs.is_empty() { return true; }
    let dense = false;
    for s in sigs {
        if s.label == "vector" {
            dense = true;
            if 0.0 + s.score >= min_similarity { return true; }
        }
    }
    // No dense signal for this hit (an entity without vectors): the lexical signals are the only evidence there is.
    !dense
}
let top = if i.top == () { 2 } else { i.top };
let budget_chars = (if i.budget_tokens == () { 1500 } else { i.budget_tokens }) * 4;
let degraded = ();
if i.prepare == "model" {
    degraded = "prepare « model » is not there yet: the recall ran on the request as it is (direct)";
} else if i.prepare != () && i.prepare != "" && i.prepare != "direct" {
    throw `recall_for_request: prepare is « direct » or « model », not « ${i.prepare} »`;
}
let hits = [];
let sources = if i.value == () { [] } else { i.value };
for s in sources {
    if type_of(s) != "map" || s.results == () { continue; }
    let tier = s.tier;
    let w = if weights[tier] == () { 0.0 } else { 0.0 + weights[tier] };
    let rate = if recency[tier] == () { 0.0 } else { 0.0 + recency[tier] };
    let kept = [];
    for r in s.results { if r.data != () && (r.score == () || 0.0 + r.score >= min_score) && evidence(r, min_similarity) { kept.push(r); } }
    // The age rank in the tier: the newest hit is 0 (turns by `at`, apports by `created_at`).
    let ages = [];
    for r in kept {
        let k = if tier == "turns" { r.data.at } else if tier == "apports" { r.data.created_at } else { () };
        ages.push(if k == () { "" } else { k.to_string() });
    }
    let rank = 0;
    for r in kept {
        let newer = 0;
        for a in ages { if a > ages[rank] { newer += 1; } }
        let decay = 1.0 / (1.0 + rate * newer.to_float());
        hits.push(#{ tier: tier, r: r, score: w * decay / (60.0 + rank.to_float() + 1.0) });
        rank += 1;
    }
}
hits.sort(|a, b| if a.score > b.score { -1 } else if a.score < b.score { 1 } else { 0 });
let chosen = [];
let seen = [];
for h in hits {
    if chosen.len() >= top { break; }
    let id = source_of(h.tier, h.r);
    if seen.contains(id) { continue; }
    seen.push(id);
    chosen.push(#{ tier: h.tier, r: h.r, id: id });
}
if chosen.is_empty() {
    return #{ block: (), mode: "recent", sources: [], passages: [], degraded: degraded, swarm: () };
}
let each = budget_chars / chosen.len();
let parts = [];
for c in chosen {
    let t = text_of(c.tier, c.r.data);
    if t.len() > each { t = t.sub_string(0, each) + "…"; }
    parts.push(header_of(c.tier, c.r.data) + "\n" + t);
}
let block = "Rappel automatique (mode recent) : ces passages ont été retrouvés pour ta demande ; ils ne sont pas la demande.\n\n";
let first = true;
for p in parts { if !first { block += "\n\n"; } block += p; first = false; }
let ids = [];
let passages = [];
for c in chosen {
    ids.push(c.id);
    passages.push(#{ id: c.id, tier: c.tier, signals: if c.r.signals == () { [] } else { c.r.signals } });
}
#{ block: block, mode: "recent", sources: ids, passages: passages, degraded: degraded, swarm: () }
