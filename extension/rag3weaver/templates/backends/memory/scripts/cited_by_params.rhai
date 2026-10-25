// Les paramètres de cited_by vers ceux du Plan (plans/cited_by.json). La
// valeur citée arrive sous `cite` : `value` est l'entrée du RhaiNode.
// `under` cherche aussi sous un dossier (valeur + "/") et dans un fichier
// (valeur + "#", ses symboles) ; sans lui, la valeur seule. Les listes vides
// et les chaînes vides tombent d'elles-mêmes au liage (optional) ; un entier
// n'est jamais vide, donc `since` à 0 se retire ici.
let p = input;
let out = #{value: p.cite, limit: p.limit, offset: p.offset,
            genres: p.genres, states: p.states, authors: p.authors};
if p.under {
    out.value_dir = p.cite + "/";
    out.value_sym = p.cite + "#";
}
if p.since > 0 {
    out.since = p.since;
}
out
