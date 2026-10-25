// archive_candidates — les fiches candidates d'une section, pour le Decide (M4, page 06 §2).
// input : #{ section: {title, text, chemin, …}, candidates: n, allow_create: bool,
//           value: les résultats de la recherche de fiches (recall_fiches, search_structured) }
// Rend l'élément du Decide : la section, ses `candidates` (les noms des fiches trouvées, étiquettes
// de `labels_from`) et `prompt`, ce que le modèle lit : la section, puis chaque candidate par son nom
// et son `about`.
let section = input.section;
let found = input.value;
if found == () { found = []; }
let mut names = [];
let mut presentation = "";
for r in found {
    if names.len() >= input.candidates { break; }
    let d = r.data;
    if d == () { continue; }
    let name = d.name;
    if name == () || names.contains(name) { continue; }
    names.push(name);
    let about = if d.about == () { "" } else { d.about };
    presentation += `- « ${name} » : ${about}\n`;
}
let choix = if input.allow_create { "none, ou create si le sujet est net et qu'aucune fiche ne le porte" } else { "none" };
let texte = section.text;
if texte.len() > 4000 { texte = texte.sub_string(0, 4000) + "…"; }
let liste = if presentation == "" { "(aucune)\n" } else { presentation };
let prompt = `Section « ${section.title} » :\n${texte}\n\nFiches candidates :\n${liste}\nRéponds par le nom exact d'une candidate, ou ${choix}.`;
let out = section;
out.candidates = names;
out.prompt = prompt;
out
