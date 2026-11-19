// trace_turn_slice — second step: the slice Source of the run whose range contains the turn.
let x = input[0];
let c = x.carried;
let m = [];
for r in x.rows { m.push(r.data); }
m.sort(|a, b| a.seq - b.seq);
c.messages = m;
[#{ filter: #{ must: [
    #{ field: #{ key: "run", value: c.run } },
    #{ field: #{ key: "from_turn", value: [#{ op: "lte", value: c.turn }] } },
    #{ field: #{ key: "to_turn", value: [#{ op: "gte", value: c.turn }] } },
] }, carried: c }]
