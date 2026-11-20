// archive_candidates — les fiches candidates d'une section, pour le Decide (M4, page 06 §2).
// input : #{ section: {title, text, chemin, …}, candidates: n, allow_create: bool,
//           value: les résultats de la recherche de fiches (recall_fiches, search_structured) }
// Rend des étiquettes courtes pour le Decide et leur table de résolution candidate_names.
// Les noms exacts restent les identités des fiches, présentées avec leur about dans le prompt.
let section = input.section;
let found = input.value;
if found == () { found = []; }
// Par archive_recall (la requête bornée) : [#{results, query_chars}], un élément.
let query_chars = ();
if type_of(found) == "array" && found.len() == 1 && type_of(found[0]) == "map" && found[0].results != () {
    query_chars = found[0].query_chars;
    found = found[0].results;
    if found == () { found = []; }
}
let names = [];
let aliases = [];
let candidate_names = [];
for r in found {
    if names.len() >= input.candidates { break; }
    let d = r.data;
    if d == () { continue; }
    let name = d.name;
    if name == () || names.contains(name) { continue; }
    names.push(name);
    let alias = "candidate_" + names.len().to_string();
    aliases.push(alias);
    let about = if d.about == () { "" } else { d.about };
    candidate_names.push(#{ alias: alias, name: name, about: about });
}
// Les fiches proposées plus haut dans ce document : pas encore cherchables (la transaction de l'outil
// ne les a pas validées), elles sont ajoutées aux candidates pour qu'une section puisse les rejoindre.
let proposed = if input.proposed == () { [] } else { input.proposed };
for p in proposed {
    let name = p.name;
    if name == () || names.contains(name) { continue; }
    names.push(name);
    let alias = "candidate_" + names.len().to_string();
    aliases.push(alias);
    let about = if p.about == () { "" } else { p.about };
    candidate_names.push(#{ alias: alias, name: name, about: about, proposed: true });
}
let texte = section.text;
let titre_contexte = if input.context_title == () || input.context_title == "" { "Ce qui précède (déjà rangé, ne pas ranger) :" } else { input.context_title };
// The Decide in parts (lot B4): what precedes (stable for the whole document) first, then the section, then the
// candidates by their declared template. No prose line: the labels and their meaning are in the Decide's instruction.
let parts = [
    #{ name: "context", title: titre_contexte, value: if input.context == () { "" } else { input.context } },
    #{ name: "section", title: "Section « " + section.title + " »", value: texte, max_tokens: 1000 },
    #{ name: "candidates", title: "Fiches candidates", value: if candidate_names.is_empty() { "(aucune)" } else { candidate_names }, format: #{ template: input.candidates_template } },
];
let out = section;
out.candidates = aliases;
out.candidate_names = candidate_names;
out.parts = parts;
if query_chars != () { out.query_chars = query_chars; }
out
