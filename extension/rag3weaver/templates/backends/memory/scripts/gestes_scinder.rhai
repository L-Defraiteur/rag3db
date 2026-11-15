// gestes_scinder — la fiche à scinder, trouvée par son uuid parmi toutes les fiches, et ses groupes
// vérifiés dans leur forme (les gestes de Lucie, 15 novembre 2026). Le plan (gestes_plan) vérifie le
// partage des sections. input : #{ uuid, groupes, value: toutes les fiches }.
let u = input.uuid;
let found = input.value;
if type_of(found) != "array" { found = []; }
let d = ();
let noms = [];
for r in found {
    if r.data == () { continue; }
    noms.push(r.data.name);
    if r.uuid == u || r.data._uuid == u { d = r.data; }
}
if d == () { throw `scinder_fiche : aucune fiche d’uuid ${u}`; }
if d.state == "retired" { throw `scinder_fiche : « ${d.name} » est retirée`; }
let groupes = input.groupes;
if type_of(groupes) != "array" || groupes.is_empty() { throw "scinder_fiche : groupes est une liste de {name, about, sections}"; }
let sortie = [];
for g in groupes {
    if type_of(g.name) != "string" || g.name == "" { throw "scinder_fiche : un groupe sans name"; }
    if type_of(g.sections) != "array" || g.sections.is_empty() { throw `scinder_fiche : le groupe « ${g.name} » n’a pas de sections`; }
    if g.name == d.name || noms.contains(g.name) { throw `scinder_fiche : « ${g.name} » est déjà une fiche — rejoindre_fiches la rejoint`; }
    sortie.push(#{ name: g.name, about: g.about, sections: g.sections, creer: true });
}
[ #{ mode: "scinder", ancienne: #{ name: d.name, about: d.about, state: d.state, revision: d.revision },
     filtre_apports: #{ field: #{ key: "fiche", value: [d.name] } }, groupes: sortie, genre: "derived_from" } ]
