// archive_candidates — les fiches candidates d'une section, pour le Decide (M4, page 06 §2).
// input : #{ section: {title, text, chemin, …}, candidates: n, allow_create: bool,
//           value: les résultats de la recherche de fiches (recall_fiches, search_structured) }
// Rend des étiquettes courtes pour le Decide et leur table de résolution candidate_names.
// Les noms exacts restent les identités des fiches, présentées avec leur about dans le prompt.
let section = input.section;
let found = input.value;
if found == () { found = []; }
let names = [];
let presentation = "";
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
    candidate_names.push(#{ alias: alias, name: name });
    let about = if d.about == () { "" } else { d.about };
    presentation += `- ${alias} : « ${name} » : ${about}\n`;
}
let choix = if input.allow_create { "none, ou create si le sujet est net et qu'aucune fiche ne le porte" } else { "none" };
let texte = section.text;
if texte.len() > 4000 { texte = texte.sub_string(0, 4000) + "…"; }
let liste = if presentation == "" { "(aucune)\n" } else { presentation };
let prompt = `Section « ${section.title} » :\n${texte}\n\nFiches candidates :\n${liste}\nRéponds par l’identifiant court d’une candidate, ou ${choix}.`;
let out = section;
out.candidates = aliases;
out.candidate_names = candidate_names;
out.prompt = prompt;
out
