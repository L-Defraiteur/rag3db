// archive_slice_input — the slice archive_slice files, as one element for archive_slice_core:
// with run, the turns rendered by render_turns (input.value), titled and placed by default; without
// run, contenu as given. Refused: neither; or run and contenu both; or a range with no turn.
let i = input;
if i.run != "" {
    if i.contenu != "" { throw "archive_slice: give run (the turns are rendered from their rows) or contenu, not both"; }
    if i.from_turn < 1 || i.to_turn < i.from_turn { throw `archive_slice: the range ${i.from_turn}-${i.to_turn} is not a range of turns`; }
    let r = i.value;
    if r == () || r.messages == 0 { throw `archive_slice: run ${i.run} has no turn between ${i.from_turn} and ${i.to_turn}`; }
    let slug = "";
    for c in i.run.chars() {
        slug += if (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '.' || c == '_' || c == '-' { c.to_string() } else { "-" };
    }
    let titre = if i.titre == "" { `run ${i.run}, turns ${i.from_turn}-${i.to_turn}` } else { i.titre };
    let chemin = if i.chemin == "" { `sources/${slug}-turns-${i.from_turn}-${i.to_turn}.md` } else { i.chemin };
    return [#{ titre: titre, contenu: r.text, chemin: chemin }];
}
if i.contenu == "" { throw "archive_slice: give run and a turn range, or contenu"; }
if i.titre == "" || i.chemin == "" { throw "archive_slice: with contenu, titre and chemin are required"; }
[#{ titre: i.titre, contenu: i.contenu, chemin: i.chemin }]
