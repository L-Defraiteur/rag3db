// Les paramètres de compose_restart vers ceux du Plan (plans/compose_restart.json) :
// les sources de la session, les noms des deux fiches lues en entier (l'identité,
// la tâche ; absents, la branche optional se retire), le plafond de lignes lues.
let p = input;
if type_of(p.sources) != "array" || p.sources.len() == 0 {
    throw "compose_restart : sources est une liste non vide des chemins des L0 de la session";
}
if p.budget_tokens <= 0 {
    throw "compose_restart : budget_tokens doit être positif (reçu " + p.budget_tokens + ")";
}
let names = [];
if p.identity != "" { names.push(p.identity); }
if p.task_fiche != "" { names.push(p.task_fiche); }
let out = #{sources: p.sources, limit: 2000};
if names.len() > 0 { out.names = names; }
out
