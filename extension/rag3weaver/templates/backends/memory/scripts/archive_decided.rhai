// archive_decided — l'élément étiqueté par le Decide devient l'élément des écritures (M4, page 06 §3).
// input : #{ allow_create, value: la section étiquetée {title, text, chemin, candidates, verdict, …} }
// Rend : kind (none | create | fiche), fiche, et tout ce que les sous-graphes d'écriture lisent par
// $element.<champ> : fiche_record, apport_record, apport_key, links, doc_value.
let s = input.value;
let verdict = s.verdict;
let mut kind = "fiche";
if verdict == "none" { kind = "none"; }
else if verdict == "create" { kind = if input.allow_create { "create" } else { "none" }; }
else if !s.candidates.contains(verdict) { throw `archive : « ${verdict} » n'est ni une candidate ni none ni create`; }
let fiche = if kind == "create" { s.title } else if kind == "fiche" { verdict } else { "" };
let texte = s.text;
let mut about = texte;
let point = about.index_of(". ");
if point > 0 { about = about.sub_string(0, point + 1); }
if about.len() > 200 { about = about.sub_string(0, 200) + "…"; }
let key = #{ fiche: fiche, source: s.chemin, section: s.title };
#{
    kind: kind,
    fiche: fiche,
    verdict: verdict,
    title: s.title,
    chemin: s.chemin,
    index: s.index,
    fiche_record: #{ name: fiche, about: about, state: "proposed", via: "archiviste" },
    apport_record: #{ fiche: fiche, source: s.chemin, section: s.title, texte: texte, niveau: "L1", state: "current", via: "archiviste" },
    apport_key: key,
    links: [ #{ from: #{ name: fiche }, to: key } ],
    doc_value: `${s.chemin}#${s.title}`
}
