// gestes_plan — le plan d’un geste (scinder, rejoindre) : un élément par groupe, avec tout ce que les
// étages écrivent dans l’ordre. Les apports de l’ancienne fiche sont recopiés sous la fiche du groupe
// (l’identité d’un apport porte le nom de sa fiche : il ne se déplace pas), les originaux passent
// recopied, l’ancienne est retirée. input : #{ element, value: les apports de l’ancienne fiche }.
fn phrase_suffisante(p) {
    if type_of(p) != "string" { return false; }
    let n = 0;
    for m in p.split(" ") { if m != "" && m != "-" && m != "*" { n += 1; } }
    n >= 6
}
let e = input.element;
let found = input.value;
if type_of(found) != "array" { found = []; }
let apports = [];
for r in found { if r.data != () && r.data.state == "current" { apports.push(r.data); } }
let sections = apports.map(|a| a.section);
let vues = [];
for g in e.groupes {
    if g.sections == "toutes" { continue; }
    for s in g.sections {
        if !sections.contains(s) { throw `${e.mode}_fiche : « ${e.ancienne.name} » n’a pas de section « ${s} »`; }
        if vues.contains(s) { throw `${e.mode}_fiche : la section « ${s} » est dans deux groupes`; }
        vues.push(s);
    }
    if g.creer && !phrase_suffisante(g.about) { throw `${e.mode}_fiche : l’about de « ${g.name} » a moins de six mots`; }
}
if e.mode == "scinder" {
    let oubliees = sections.filter(|s| !vues.contains(s));
    if !oubliees.is_empty() { throw `scinder_fiche : des sections ne sont dans aucun groupe : ${oubliees}`; }
}
let plan = [];
for g in e.groupes {
    let siens = if g.sections == "toutes" { apports } else { apports.filter(|a| g.sections.contains(a.section)) };
    let copies = []; let links = []; let citations = []; let recopies = [];
    for a in siens {
        let cle = #{ fiche: g.name, source: a.source, section: a.section };
        copies.push(#{ record: #{ fiche: g.name, source: a.source, section: a.section, texte: a.texte, niveau: a.niveau, state: "current" }, expected_revision: 0 });
        links.push(#{ from: #{ name: g.name }, to: cle });
        citations.push(#{ apport_key: cle, issue_valeur: `${e.ancienne.name} — ${a.source}#${a.section}`, doc_valeur: `${a.source}#${a.section}` });
        recopies.push(#{ record: #{ fiche: a.fiche, source: a.source, section: a.section, texte: a.texte, niveau: a.niveau, state: "recopied" }, expected_revision: a.revision });
    }
    plan.push(#{
        mode: e.mode, nom: g.name, genre: e.genre,
        ancien: e.ancienne.name, fiche_id: #{ name: g.name },
        fiches: if g.creer { [ #{ record: #{ name: g.name, about: g.about, state: "proposed" }, expected_revision: 0 } ] } else { [] },
        copies: copies, links: links, citations: citations, recopies: recopies,
        ancienne: #{ record: #{ name: e.ancienne.name, about: e.ancienne.about, state: "retired" }, expected_revision: e.ancienne.revision }
    });
}
plan
