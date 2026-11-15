// Le contexte d'après une compaction (page du 15 nov. 2026, « la compaction est
// l'archiviste ») : composé dans un budget, par priorité — l'identité, la fiche
// de tâche, le bilan ordonné, les L1 de la session dans l'ordre où ils ont été
// pris (la fiche en marge), puis les L1 des autres sur les mêmes fiches, les plus
// récents d'abord. Un L1 entre en entier ou reste au rappel ; il n'est jamais
// coupé. Les jetons sont estimés : ceil(caractères / 4), et la règle est rendue.
fn jetons(s) { (s.len() + 3) / 4 }
fn premier_tour(section) {
    let mots = section.split(" ");
    if mots.len() < 2 { return 1000000000; }
    let bornes = mots[1].split("-");
    let n = parse_int(bornes[0]);
    if type_of(n) != "i64" { 1000000000 } else { n }
}
fn noms(a) {
    let out = [];
    if a.fiches != () { for f in a.fiches { out.push(f.name); } }
    // Un apport posé sans relier_apport n'a que son champ `fiche`.
    if out.len() == 0 && a.fiche != () && a.fiche != "" { out.push(a.fiche); }
    out
}
let p = input;
let rows = p.value;
let sources = p.sources;
let session = [];
let autres = [];
let identite = "";
let identite_at = -1;
let tache_about = "";
let tache_plan = "";
let tache_plan_at = -1;
for a in rows {
    if a.state == "recopied" { continue; }
    let n = noms(a);
    if sources.contains(a.source) {
        a.premier = premier_tour(a.section);
        session.push(a);
    } else if p.identity != "" && n.contains(p.identity) {
        if a.section == "identity" && a.created_at > identite_at { identite = a.texte; identite_at = a.created_at; }
    } else if p.task_fiche != "" && n.contains(p.task_fiche) {
        for f in a.fiches { if f.name == p.task_fiche { tache_about = f.about; } }
        if a.section == "plan" && a.created_at > tache_plan_at { tache_plan = a.texte; tache_plan_at = a.created_at; }
    } else {
        autres.push(a);
    }
}
session.sort(|x, y| if x.premier != y.premier { x.premier - y.premier } else { x.created_at - y.created_at });

let tete = "";
if p.identity != "" {
    tete += "## Identité : agent:" + p.identity.sub_string(if p.identity.starts_with("agent:") { 6 } else { 0 }) + "\n\n" + identite + "\n\n";
}
if p.task_fiche != "" {
    tete += "## Tâche : fiche:" + p.task_fiche + "\n\n" + tache_about + "\n";
    if tache_plan != "" { tete += "\n" + tache_plan + "\n"; }
    tete += "\n";
}
let bilan = [];
for a in session {
    let n = noms(a);
    let f = if n.len() > 0 { n[0] } else { "(sans fiche)" };
    bilan.push(a.section + " → fiche:" + f);
}
let texte_bilan = "## Bilan, dans l'ordre de la session\n\n";
let i = 1;
for l in bilan { texte_bilan += "" + i + ". " + l + "\n"; i += 1; }
texte_bilan += "\n";

let texte = tete + texte_bilan;
let parts = #{identity_task: jetons(tete), bilan: jetons(texte_bilan), session_l1: 0, others_l1: 0};
let reste = [];
let budget = p.budget_tokens;
let entete_s = "## Ce que la session a rangé, dans l'ordre\n\n";
let pris_s = 0;
for a in session {
    let n = noms(a);
    let bloc = "### " + a.section + " — fiche:" + (if n.len() > 0 { n[0] } else { "(sans fiche)" }) + "\n\n" + a.texte + "\n\n";
    let cout = jetons(bloc) + (if pris_s == 0 { jetons(entete_s) } else { 0 });
    if jetons(texte) + cout <= budget {
        if pris_s == 0 { texte += entete_s; }
        texte += bloc; parts.session_l1 += cout; pris_s += 1;
    } else {
        reste.push(#{fiche: if n.len() > 0 { n[0] } else { "" }, section: a.section, source: a.source});
    }
}
let entete_a = "## Sur les mêmes fiches, d'autres sessions (les plus récents d'abord)\n\n";
let pris_a = 0;
for a in autres {
    let n = noms(a);
    let bloc = "### fiche:" + (if n.len() > 0 { n[0] } else { "(sans fiche)" }) + " — " + a.source + "#" + a.section + "\n\n" + a.texte + "\n\n";
    let cout = jetons(bloc) + (if pris_a == 0 { jetons(entete_a) } else { 0 });
    if jetons(texte) + cout <= budget {
        if pris_a == 0 { texte += entete_a; }
        texte += bloc; parts.others_l1 += cout; pris_a += 1;
    } else {
        reste.push(#{fiche: if n.len() > 0 { n[0] } else { "" }, section: a.section, source: a.source});
    }
}
#{
    text: texte,
    tokens: jetons(texte),
    budget_tokens: budget,
    over_budget: jetons(texte) > budget,
    estimate: "ceil(caractères / 4)",
    parts: parts,
    bilan: bilan,
    session_l1: pris_s,
    others_l1: pris_a,
    left_to_recall: reste
}
