// gestes_rejoindre — les deux fiches d’une jonction, par leurs uuid : A rejoint B (les gestes de Lucie,
// 15 novembre 2026). input : #{ uuid_a, uuid_b, value: toutes les fiches }.
let found = input.value;
if type_of(found) != "array" { found = []; }
let a = ();
let b = ();
for r in found {
    if r.data == () { continue; }
    if r.uuid == input.uuid_a || r.data._uuid == input.uuid_a { a = r.data; }
    if r.uuid == input.uuid_b || r.data._uuid == input.uuid_b { b = r.data; }
}
if a == () { throw `rejoindre_fiches : aucune fiche d’uuid ${input.uuid_a}`; }
if b == () { throw `rejoindre_fiches : aucune fiche d’uuid ${input.uuid_b}`; }
if a.name == b.name { throw "rejoindre_fiches : une fiche ne se rejoint pas elle-même"; }
if a.state == "retired" || b.state == "retired" { throw "rejoindre_fiches : une fiche retirée ne se rejoint pas"; }
[ #{ mode: "rejoindre", ancienne: #{ name: a.name, about: a.about, state: a.state, revision: a.revision },
     filtre_apports: #{ field: #{ key: "fiche", value: [a.name] } },
     groupes: [ #{ name: b.name, about: b.about, sections: "toutes", creer: false } ], genre: "alias" } ]
