// turns_where — the conditions of list_turns and search_turns from their filters (run, role,
// tool_name, status, since and until on `at`, ISO 8601); an empty filter adds nothing. Returns
// the list of `must` conditions.
fn conditions(i) {
    let must = [];
    for k in ["run", "role", "tool_name", "status"] {
        let v = i[k];
        if v != () && v != "" { must.push(#{ field: #{ key: k, value: v } }); }
    }
    // The author (23 November 2026): « me » is the agent the call is made for (`caller`, given by the host).
    let author = i["author"];
    if author != () && author != "" && author != "*" {
        if author == "me" {
            let caller = i["caller"];
            if caller == () || caller == "" { throw "search_turns: author « me » needs an agent behind the call; give an agent key, or « * »"; }
            author = caller;
        }
        must.push(#{ field: #{ key: "author", value: author } });
    }
    // `until` is a reserved word of Rhai: both bounds are read by index.
    let since = i["since"];
    let until_ = i["until"];
    if since != () && since != "" { must.push(#{ field: #{ key: "at", value: [#{ op: "gte", value: since }] } }); }
    if until_ != () && until_ != "" { must.push(#{ field: #{ key: "at", value: [#{ op: "lte", value: until_ }] } }); }
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
