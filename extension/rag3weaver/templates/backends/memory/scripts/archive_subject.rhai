// archive_subject — une fiche à créer qu'une fiche déjà là porte sous un nom voisin : la section la
// rejoint (13 novembre 2026, le second archivage à blanc : « Hooks Claude Code », « Hooks de Claude
// Code », « Hooks Claude Code de rag3weaver », trois fiches nées de trois documents).
// input : #{ element: l'élément décidé (archive_decided), value: les fiches que la recherche rend pour
//           element.subject_query (search_structured) }
// Hors de kind create, l'élément passe tel quel. En create, la première fiche rendue dont le nom est le
// même ou voisin (sujets_voisins) devient la fiche de la section : kind fiche, et l'apport, sa clé et son
// lien la nomment. La recherche ne voit que les fiches validées : celles d'un document précédent, pas
// celles que ce document propose (archive_decided les rejoint déjà, portées par le carry).
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

let e = input.element;
if e.kind != "create" { return e; }
let found = input.value;
if type_of(found) != "array" { found = []; }
let nom = e.fiche;
let retenu = "";
for r in found {
    let d = r.data;
    if d == () || type_of(d.name) != "string" { continue; }
    if d.name == nom { retenu = d.name; break; }
}
if retenu == "" {
    for r in found {
        let d = r.data;
        if d == () || type_of(d.name) != "string" { continue; }
        if sujets_voisins(d.name, nom) { retenu = d.name; break; }
    }
}
if retenu == "" { return e; }
e.kind = "fiche";
e.fiche = retenu;
e.verdict = retenu;
e.subject_query = retenu;
e.apport_record.fiche = retenu;
e.apport_key.fiche = retenu;
e.links = [ #{ from: #{ name: retenu }, to: e.apport_key } ];
e.joined_from = nom;
e
