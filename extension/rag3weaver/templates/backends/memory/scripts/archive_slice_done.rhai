// archive_slice_done — the report of archive_slice_core (one element), and with run how far the run
// is archived: to_turn when the slice is filed without error, from_turn - 1 otherwise (the mark
// advances only after the commit, on an error-free report).
let found = input.value;
let r = if type_of(found) == "array" && found.len() == 1 { found[0] } else { found };
if input.run != "" {
    let clean = type_of(r) == "map" && (r.errors == () || r.errors.len() == 0);
    r.archived_through_turn = if clean { input.to_turn } else { input.from_turn - 1 };
}
r
