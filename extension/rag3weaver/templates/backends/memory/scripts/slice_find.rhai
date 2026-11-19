// slice_find — the selection of the slice Source of get_slice: by its path, or the Source of the run
// whose range contains the turn.
let i = input;
let f = if i.chemin != "" {
    #{ field: #{ key: "chemin", value: i.chemin } }
} else {
    if i.run == "" || i.turn < 1 { throw "get_slice: give chemin, or run and turn"; }
    #{ must: [
        #{ field: #{ key: "run", value: i.run } },
        #{ field: #{ key: "from_turn", value: [#{ op: "lte", value: i.turn }] } },
        #{ field: #{ key: "to_turn", value: [#{ op: "gte", value: i.turn }] } },
    ] }
};
[#{ filter: f, carried: #{ chemin: i.chemin, run: i.run, turn: i.turn } }]
