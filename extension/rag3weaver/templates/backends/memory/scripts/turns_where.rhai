// turns_where — the conditions of list_turns and search_turns from their filters (run, role,
// tool_name, status, since and until on `at`, ISO 8601); an empty filter adds nothing. Returns
// the list of `must` conditions.
fn conditions(i) {
    let must = [];
    for k in ["run", "role", "tool_name", "status"] {
        let v = i[k];
        if v != () && v != "" { must.push(#{ field: #{ key: k, value: v } }); }
    }
    if i.since != () && i.since != "" { must.push(#{ field: #{ key: "at", value: [#{ op: "gte", value: i.since }] } }); }
    if i.until != () && i.until != "" { must.push(#{ field: #{ key: "at", value: [#{ op: "lte", value: i.until }] } }); }
    must
}
let must = conditions(input);
if input.query == () {
    [#{ filter: if must.is_empty() { #{} } else { #{ must: must } } }]
} else {
    if input.query == "" { throw "search_turns: query is empty; list_turns lists the turns by time"; }
    let options = input.options;
    if options == () { options = #{}; }
    if !must.is_empty() { options.filter_condition = #{ must: must }; }
    [#{ query: input.query, options: options }]
}
