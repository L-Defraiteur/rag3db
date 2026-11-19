// trace_turn_query — trace_turn, first step: the Turn rows of the turn.
let i = input;
if i.run == "" || i.turn < 1 { throw "trace_turn: give run and turn"; }
[#{ filter: #{ must: [#{ field: #{ key: "run", value: i.run } }, #{ field: #{ key: "turn", value: i.turn } }] },
    carried: #{ run: i.run, turn: i.turn } }]
