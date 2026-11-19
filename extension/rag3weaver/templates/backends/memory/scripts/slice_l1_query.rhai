// slice_l1_query — after the slice Source is found (or not), the selection of its L1: the Apports
// whose source is its path. No Source: a selection that finds nothing, and the slice is open.
let found = input;
let x = if type_of(found) == "array" && found.len() > 0 { found[0] } else { #{ rows: [], carried: #{} } };
let src = ();
if type_of(x.rows) == "array" && x.rows.len() > 0 { src = x.rows[0].data; }
let path = if src == () { "" } else { src.chemin };
let carried = x.carried;
carried.source = src;
[#{ filter: #{ field: #{ key: "source", value: path } }, carried: carried }]
