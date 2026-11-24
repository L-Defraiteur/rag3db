// recall_where — the sources of recall_for_request, one element per source: the turns of the summarised windows (only
// with a run and active_since: never a passage still in clear), the L1 apports (not the recopied ones), the cards (not
// the retired ones), and the calling agent's own turns from its other conversations. `recent` is for the preparation (prepare « model »), not read here. The author filter: « me » is the agent the call is made for (`caller`, given by the host).
let i = input;
if i.request == () || i.request == "" { throw "recall_for_request: request is empty"; }
let limit = if i.limit == () { 10 } else { i.limit };
let author = i.author;
let by_author = [];
if author != () && author != "" && author != "*" {
    if author == "me" {
        if i.caller == () || i.caller == "" { throw "recall_for_request: author « me » needs an agent behind the call; give an agent key, or « * »"; }
        author = i.caller;
    }
    by_author.push(#{ field: #{ key: "author", value: author } });
}
let elements = [];
if i.run != () && i.run != "" && i.active_since != () && i.active_since != "" {
    let must = [#{ field: #{ key: "run", value: i.run } }, #{ field: #{ key: "at", value: [#{ op: "lt", value: i.active_since }] } }];
    must += by_author;
    elements.push(#{ tier: "turns", target: "Turn", query: i.request, options: #{ limit: limit, filter_condition: #{ must: must } } });
}
// The calling agent's own turns from its other conversations (24 November 2026, « mes tours d'abord »): signed by the
// agent that calls (`caller`, the key the host gave, the same `writes.author` signs), in runs other than the one given —
// the current run is the source above, so nothing comes twice and its active window never. No agent: no such source.
let own = if i.own_turns == () { true } else { i.own_turns };
if own && i.caller != () && i.caller != "" {
    let o = #{ must: [#{ field: #{ key: "author", value: i.caller } }] };
    if i.run != () && i.run != "" { o.must_not = [#{ field: #{ key: "run", value: i.run } }]; }
    elements.push(#{ tier: "own_turns", target: "Turn", query: i.request, options: #{ limit: limit, filter_condition: o } });
}
let apports = #{ must_not: [#{ field: #{ key: "state", value: ["recopied"] } }] };
if !by_author.is_empty() { apports.must = by_author; }
elements.push(#{ tier: "apports", target: "Apport", query: i.request, options: #{ limit: limit, filter_condition: apports } });
let cards = #{ must_not: [#{ field: #{ key: "state", value: ["retired"] } }] };
if !by_author.is_empty() { cards.must = by_author; }
elements.push(#{ tier: "cards", target: "Fiche", query: i.request, options: #{ limit: limit, filter_condition: cards } });
elements
