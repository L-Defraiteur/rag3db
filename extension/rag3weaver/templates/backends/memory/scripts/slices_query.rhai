// slices_query — list_slices: the slice Sources of a run, then its L1 (by run), carried together.
if input.run == "" { throw "list_slices: run is empty"; }
[#{ filter: #{ field: #{ key: "run", value: input.run } }, carried: #{ run: input.run } }]
