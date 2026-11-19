// slices_l1_query — after the Sources of the run, its L1: the Apports of the run.
let x = input[0];
let carried = x.carried;
carried.sources = x.rows;
[#{ filter: #{ field: #{ key: "run", value: carried.run } }, carried: carried }]
