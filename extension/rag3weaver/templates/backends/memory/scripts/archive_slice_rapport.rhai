// archive_slice_rapport — the report of archive_slice: the decisions report (as archive_rapport does,
// with usage_total), plus the slice meta carried by the elements (slice_summary, open). `results`
// stays only under trace.
// The decisions report, then the usage of the extraction call (Collect, in that order).
let v = input.value;
let r = if type_of(v) == "array" { v[0] } else { v };
if type_of(v) == "array" && v.len() > 1 && type_of(v[1]) == "map" { r.extraction_usage = v[1]; }
let summary = ();
let open = ();
if type_of(r.results) == "array" {
    for x in r.results {
        if type_of(x) == "map" && x.slice_summary != () { summary = x.slice_summary; open = x.slice_open; break; }
    }
}
r.slice_summary = summary;
r.open = if open == () { [] } else { open };
// The path of the slice Source, written even when a topic failed (the host keeps it only when the
// slice is archived).
r.source = input.chemin;
if type_of(r.results) == "array" {
    let t = #{ calls: 0, without_usage: 0, prompt_tokens: 0, cached_prompt_tokens: 0, completion_tokens: 0, ms: 0 };
    for x in r.results {
        t.calls += 1;
        let u = if type_of(x) == "map" { x.usage } else { () };
        if u == () { t.without_usage += 1; continue; }
        if u.ms != () { t.ms += u.ms; }
        if u.prompt_tokens == () { t.without_usage += 1; } else {
            t.prompt_tokens += u.prompt_tokens; t.cached_prompt_tokens += u.cached_prompt_tokens; t.completion_tokens += u.completion_tokens;
        }
    }
    r.usage_total = t;
}
if input.trace != true { r.remove("results"); }
r
