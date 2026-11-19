// turns_filter — the selection of the Turn rows of a run between two turns, as one element for
// turns_select (a cited graph receives its filter by its configuration only).
let i = input;
[#{ filter: #{ must: [
    #{ field: #{ key: "run", value: i.run } },
    #{ field: #{ key: "turn", value: [#{ op: "gte", value: i.from_turn }] } },
    #{ field: #{ key: "turn", value: [#{ op: "lte", value: i.to_turn }] } },
] } }]
