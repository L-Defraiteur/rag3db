// Les apports alias, handle et identity des fiches qui répondent (le Plan
// whois, les plus récents d'abord) vers la réponse de whois (page 04 §8,
// pièce 3) :
// - agent : la clé, le nom de la fiche d'identité ; si plusieurs fiches
//   répondent (un alias porté par deux agents), celle dont l'apport est le
//   plus récent, et `said` nomme les autres ;
// - alias, aliases : le dernier alias, puis tous avec leur date ;
// - handles : les handles cités par les apports handle, avec leur date ;
// - identite, identity_at : le texte du dernier apport identity ;
// - tache, autres : les Refs de genre `fiche` de ce dernier apport, dans
//   l'ordre de la ligne `doing:` (chaque valeur cherchée bornée, les plus
//   longues d'abord et masquées une fois trouvées) ; une fiche que la ligne
//   ne nomme pas vient en dernier, dans l'ordre de citation. La première est
//   la tâche en cours.
// Un caractère qui borne un nom : ni lettre, ni chiffre, ni « - _ . / : »
// (les noms de fiche en portent : « tache-a », « lot-96-m4 »).
fn borne(c) {
    !((c >= '0' && c <= '9') || c.to_upper() != c.to_lower()
      || c == '-' || c == '_' || c == '.' || c == '/' || c == ':')
}

// La première occurrence bornée de `v` dans `m` (son indice en caractères),
// ou -1 : le caractère d'avant et celui d'après bornent un nom. Un jeton
// entre deux séparateurs (« , », « ; », « puis », « et ») est borné, donc
// l'égalité de jeton est le cas commun de cette recherche.
fn trouve(m, v) {
    let depuis = 0;
    loop {
        let i = m.index_of(v, depuis);
        if i < 0 { return -1; }
        let j = i + v.len();
        if (i == 0 || borne(m[i - 1])) && (j >= m.len() || borne(m[j])) { return i; }
        depuis = i + 1;
    }
}

let rows = input;
let vide = #{agent: (), identite: (), alias: (), aliases: [], handles: [], tache: (),
             autres: [], identity_at: (), said: ()};
if rows.len() == 0 {
    vide.said = "aucune fiche d'identité ne répond : ni apport alias, handle ou identity qui cite la valeur demandée, ni fiche de ce nom qui en porte";
    return vide;
}
let noms = [];
for r in rows {
    for f in r.fiches {
        if !noms.contains(f.name) { noms.push(f.name); }
    }
}
let agent = noms[0];
let out = vide;
out.agent = agent;
let dits = [];
if noms.len() > 1 {
    dits.push("plusieurs fiches répondent, la plus récente est rendue ; aussi : " + noms.extract(1).reduce(|s, n| if s == "" { n } else { s + ", " + n }, ""));
}
let identity = ();
for r in rows {
    let de_l_agent = false;
    for f in r.fiches { if f.name == agent { de_l_agent = true; } }
    if !de_l_agent { continue; }
    if r.section == "alias" {
        for x in r.refs { if x.genre == "alias" { out.aliases.push(#{nom: x.valeur, at: r.created_at}); } }
    } else if r.section == "handle" {
        for x in r.refs { if x.genre == "handle" { out.handles.push(#{valeur: x.valeur, at: r.created_at}); } }
    } else if r.section == "identity" && identity == () {
        identity = r;
    }
}
if out.aliases.len() > 0 { out.alias = out.aliases[0].nom; }
if identity == () {
    dits.push("aucun apport identity");
} else {
    out.identite = identity.texte;
    out.identity_at = identity.created_at;
    let doing = "";
    for l in identity.texte.split("\n") {
        let t = l;
        t.trim();  // trim() change la chaîne en place et rend ()
        if t.starts_with("doing:") { doing = t.sub_string(6); }
    }
    // Les fiches citées, une fois chacune, les plus longues d'abord : « a »
    // ne se lit pas dans « tache-a », déjà masquée quand on la cherche.
    let valeurs = [];
    for x in identity.refs {
        if x.genre == "fiche" && !valeurs.contains(x.valeur) { valeurs.push(x.valeur); }
    }
    let cite_avant = valeurs;  // l'ordre de citation, pour celles que la ligne ne nomme pas
    valeurs.sort(|a, b| b.len() - a.len());
    let masque = doing;
    let rangees = [];
    for v in valeurs {
        let i = trouve(masque, v);
        if i >= 0 {
            let blanc = "";
            for k in 0..v.len() { blanc += " "; }
            masque = masque.sub_string(0, i) + blanc + masque.sub_string(i + v.len());
            rangees.push(#{nom: v, pos: i});
        } else {
            rangees.push(#{nom: v, pos: 1000000 + cite_avant.index_of(v)});
        }
    }
    rangees.sort(|a, b| a.pos - b.pos);
    if rangees.len() > 0 {
        out.tache = rangees[0].nom;
        for x in rangees.extract(1) { out.autres.push(x.nom); }
    } else {
        dits.push("l'apport identity ne cite aucune fiche");
    }
}
if dits.len() > 0 { out.said = dits.reduce(|s, d| if s == "" { d } else { s + " ; " + d }, ""); }
out
