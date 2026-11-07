// Les paramètres de whois vers ceux du Plan (plans/whois.json). On demande
// par un seul des trois : la clé de l'agent (le nom de sa fiche d'identité),
// un handle (`session:<uuid>`, `uds:<socket>`, `task:<nom>`, la valeur nue de
// la Ref de genre `handle`) ou un alias (un nom affiché, genre `alias`).
// Les sections lues sont toujours les trois de la page 04 : alias, handle,
// identity — passées en paramètre, le Plan n'écrit pas de littérale.
let p = input;
let donnes = [];
if p.agent != "" { donnes.push("agent"); }
if p.handle != "" { donnes.push("handle"); }
if p.alias != "" { donnes.push("alias"); }
if donnes.len() != 1 {
    throw "whois prend un seul de agent, handle, alias (reçu : " + donnes.len() + ")";
}
let out = #{sections: ["alias", "handle", "identity"], limit: 200};
if p.agent != "" {
    out.agent = p.agent;
} else if p.handle != "" {
    out.cite_section = "handle"; out.cite_genre = "handle"; out.cite_valeur = p.handle;
} else {
    out.cite_section = "alias"; out.cite_genre = "alias"; out.cite_valeur = p.alias;
}
out
