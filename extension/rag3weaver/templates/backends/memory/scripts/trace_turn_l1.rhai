// trace_turn_l1 — third step: the L1 of the run whose range covers the turn.
let x = input[0];
let c = x.carried;
c.source = if type_of(x.rows) == "array" && x.rows.len() > 0 { x.rows[0].data } else { () };
[#{ filter: #{ must: [
    #{ field: #{ key: "run", value: c.run } },
    #{ field: #{ key: "from_turn", value: [#{ op: "lte", value: c.turn }] } },
    #{ field: #{ key: "to_turn", value: [#{ op: "gte", value: c.turn }] } },
] }, carried: c }]
