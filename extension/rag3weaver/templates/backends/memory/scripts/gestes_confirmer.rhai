// gestes_confirmer — la fiche d'un uuid parmi toutes les fiches, jugée : seule une fiche proposed se confirme
// (la transition accept du cycle de vie de Fiche, proposed → current). Les gestes de Lucie, 14 novembre 2026.
// input : #{ uuid, value: toutes les fiches (select_structured, limit 0) }
// Rend #{ lignes: [ {record, expected_revision} ] pour l'écriture, rendu: {uuid, name, state, revision} }.
let u = input.uuid;
if type_of(u) != "string" || u == "" { throw "confirmer_fiche : uuid manquant"; }
let found = input.value;
if type_of(found) != "array" { found = []; }
let d = ();
for r in found { if r.uuid == u || (r.data != () && r.data._uuid == u) { d = r.data; break; } }
if d == () { throw `confirmer_fiche : aucune fiche d’uuid ${u}`; }
if d.state != "proposed" {
    throw `confirmer_fiche : « ${d.name} » est ${d.state}, pas proposed — seule une fiche proposed se confirme (proposed → current)`;
}
#{
    lignes: [ #{ record: #{ name: d.name, about: d.about, state: "current" }, expected_revision: d.revision } ],
    rendu: #{ uuid: u, name: d.name, state: "current", revision: d.revision + 1 }
}
