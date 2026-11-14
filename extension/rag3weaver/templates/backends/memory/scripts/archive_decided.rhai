// archive_decided — l'élément étiqueté par le Decide devient l'élément des écritures (M4, page 06 §3).
// input : #{ allow_create, value: la section étiquetée {title, text, chemin, candidates, verdict, …} }
// Rend : kind (none | create | fiche), fiche, et tout ce que les sous-graphes d'écriture lisent par
// $element.<champ> : fiche_record, apport_record, apport_key, links, doc_value, subject_query.
// **Le voisinage de deux noms de sujet** : en minuscules, la ponctuation et les mots vides ôtés, deux
// noms sont voisins si leurs jeux de mots sont égaux, ou si l'un contient l'autre et que le plus petit
// compte au moins deux mots (« Hooks de Claude Code » et « Hooks Claude Code de rag3weaver » rejoignent
// « Hooks Claude Code » ; « Archiviste » ne rejoint pas « Archiviste de la mémoire »).
// La même fonction est dans archive_decided et archive_subject : un script Rhai n'en importe pas.
fn sujet_mots(nom) {
    let s = nom.to_lower();
    for p in [",", ".", ":", ";", "«", "»", "'", "’", "(", ")", "-", "/", "\"", "!", "?", "`", "[", "]", "\n", "\t"] { s.replace(p, " "); }
    let vides = ["de", "du", "des", "la", "le", "les", "l", "d", "et", "à", "au", "aux", "pour", "sur", "en", "un", "une"];
    let mots = [];
    for m in s.split(" ") {
        if m == "" || vides.contains(m) || mots.contains(m) { continue; }
        mots.push(m);
    }
    mots
}
// **Un nom daté** : un mois (ou un jour de la semaine) et un nombre (« Matinée du 8 novembre 2026 »,
// « Relance du 7 novembre 2026 »). Une fiche neuve n'est pas nommée par un moment : un journal donne
// une fiche par chantier (Lucie, 14 novembre 2026). Une fiche déjà là reste joignable.
fn nom_date(nom) {
    let mois = ["janvier", "février", "fevrier", "mars", "avril", "mai", "juin", "juillet", "août", "aout", "septembre", "octobre", "novembre", "décembre", "decembre", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];
    let mots = sujet_mots(nom);
    let a_mois = false;
    let a_nombre = false;
    for m in mots {
        if mois.contains(m) { a_mois = true; }
        if m.len() > 0 && "0123456789".contains(m.sub_string(0, 1)) { a_nombre = true; }
    }
    a_mois && a_nombre
}
fn sujets_voisins(a, b) {
    if type_of(a) != "string" || type_of(b) != "string" { return false; }
    let ma = sujet_mots(a);
    let mb = sujet_mots(b);
    if ma.is_empty() || mb.is_empty() { return false; }
    let petit = if ma.len() <= mb.len() { ma } else { mb };
    let grand = if ma.len() <= mb.len() { mb } else { ma };
    for m in petit { if !grand.contains(m) { return false; } }
    petit.len() == grand.len() || petit.len() >= 2
}
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
// **Le nom d'une fiche créée** : le sujet que le modèle donne (Decide name_as='subject'), sinon le
// titre de la section. Un sujet qui nomme une candidate ou une fiche proposée plus haut, ou qui en est
// voisin (sujets_voisins), la rejoint : le nom exact d'abord, puis la première voisine dans l'ordre des
// candidates. (Decide rogne déjà le nom.)
let sujet = if type_of(s.subject) == "string" { s.subject } else { "" };
if kind == "create" && sujet != "" && type_of(s.candidate_names) == "array" {
    for c in s.candidate_names { if c.name == sujet { kind = "fiche"; resolved_name = sujet; } }
    if kind == "create" {
        for c in s.candidate_names {
            if sujets_voisins(c.name, sujet) { kind = "fiche"; resolved_name = c.name; break; }
        }
    }
}
let rejoint = #{};
let a_rejoint = false;
if kind == "fiche" {
    for p in proposed { if p.name == resolved_name { rejoint = p; a_rejoint = true; } }
    if a_rejoint { kind = "create"; }
}
let fiche = if a_rejoint { resolved_name } else if kind == "create" { if sujet != "" { sujet } else { s.title } } else if kind == "fiche" { resolved_name } else { "" };
if kind == "create" && !a_rejoint && nom_date(fiche) {
    throw `archive : le sujet « ${fiche} » est une date ou un moment, pas un chantier — un journal donne une fiche par chantier`;
}
let texte = s.text;
let about = texte;
let point = about.index_of(". ");
if point > 0 { about = about.sub_string(0, point + 1); }
if about.len() > 200 { about = about.sub_string(0, 200) + "…"; }
let key = #{ fiche: fiche, source: s.chemin, section: s.title };
let fiche_record = if a_rejoint { rejoint } else { #{ name: fiche, about: about, state: "proposed", via: "archiviste" } };
if kind == "create" && !a_rejoint { proposed.push(fiche_record); }
#{
    // Le filtre d'archive (Route sur stage) : seuls les éléments décidés passent à l'étage du sujet ; les
    // échecs du Decide ({error, element, index}) restent dans son rapport.
    stage: "decided",
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
    doc_value: `${s.chemin}#${s.title}`,
    // Ce que l'étage archive_subject cherche parmi les fiches validées : le nom de la fiche, sinon le titre.
    subject_query: if fiche != "" { fiche } else if s.title != "" { s.title } else { s.chemin }
}
