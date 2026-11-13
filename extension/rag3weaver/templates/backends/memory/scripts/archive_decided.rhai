// archive_decided — l'élément étiqueté par le Decide devient l'élément des écritures (M4, page 06 §3).
// input : #{ allow_create, value: la section étiquetée {title, text, chemin, candidates, verdict, …} }
// Rend : kind (none | create | fiche), fiche, et tout ce que les sous-graphes d'écriture lisent par
// $element.<champ> : fiche_record, apport_record, apport_key, links, doc_value.
let s = input.value;
let verdict = s.verdict;
let kind = "fiche";
if verdict == "none" { kind = "none"; }
else if verdict == "create" { kind = if input.allow_create { "create" } else { "none" }; }
let resolved_name = "";
if kind == "fiche" {
    if type_of(s.candidates) != "array" || !s.candidates.contains(verdict) {
        throw `archive : alias inconnu « ${verdict} »`;
    }
    if type_of(s.candidate_names) != "array" {
        throw `archive : alias inconnu « ${verdict} » : table de résolution absente`;
    }
    let matches = 0;
    for candidate in s.candidate_names {
        if candidate.alias == verdict {
            matches += 1;
            resolved_name = candidate.name;
        }
    }
    if matches == 0 { throw `archive : alias inconnu « ${verdict} »`; }
    if matches > 1 { throw `archive : alias ambigu « ${verdict} »`; }
    if type_of(resolved_name) != "string" || resolved_name == "" {
        throw `archive : alias « ${verdict} » sans nom de fiche valide`;
    }
}
// Une fiche proposée plus haut dans ce document : la section la rejoint par la voie create, avec le
// même enregistrement (une écriture identique), pour que la fiche existe avant son lien.
let proposed = if input.proposed == () { [] } else { input.proposed };
let rejoint = #{};
let a_rejoint = false;
if kind == "fiche" {
    for p in proposed { if p.name == resolved_name { rejoint = p; a_rejoint = true; } }
    if a_rejoint { kind = "create"; }
}
let fiche = if a_rejoint { resolved_name } else if kind == "create" { s.title } else if kind == "fiche" { resolved_name } else { "" };
let texte = s.text;
let about = texte;
let point = about.index_of(". ");
if point > 0 { about = about.sub_string(0, point + 1); }
if about.len() > 200 { about = about.sub_string(0, 200) + "…"; }
let key = #{ fiche: fiche, source: s.chemin, section: s.title };
let fiche_record = if a_rejoint { rejoint } else { #{ name: fiche, about: about, state: "proposed", via: "archiviste" } };
if kind == "create" && !a_rejoint { proposed.push(fiche_record); }
#{
    kind: kind,
    fiche: fiche,
    verdict: if kind == "fiche" { fiche } else { verdict },
    title: s.title,
    chemin: s.chemin,
    index: s.index,
    fiche_record: fiche_record,
    proposed: proposed,
    apport_record: #{ fiche: fiche, source: s.chemin, section: s.title, texte: texte, niveau: "L1", state: "current", via: "archiviste" },
    apport_key: key,
    links: [ #{ from: #{ name: fiche }, to: key } ],
    doc_value: `${s.chemin}#${s.title}`
}
