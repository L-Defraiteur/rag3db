// archive_segment_build — les sous-sections d'une section découpée : une par sujet rendu par le modèle,
// de titre « <titre de la section> §de-a » (l'identité d'un L1 reste unique, et le bilan lit toujours
// le premier tour du titre) et de texte ses paragraphes. Un intervalle hors des paragraphes est refusé
// en le nommant ; aucun sujet : la section reste entière.
let s = input.section;
let paras = [];
for p in s.text.split("\n\n") { let t = p; t.trim(); if t != "" { paras.push(t); } }
let sujets = if input.value == () || input.value.sujets == () { [] } else { input.value.sujets };
if sujets.len() == 0 { let e = s; e.long = false; return [e]; }
let out = [];
for x in sujets {
    if x.de < 1 || x.a > paras.len() || x.de > x.a {
        throw `archive_segment : le sujet « ${x.titre} » a l’intervalle ${x.de}-${x.a}, hors des ${paras.len()} paragraphes de « ${s.title} »`;
    }
    let texte = "";
    for k in range(x.de - 1, x.a) { texte += paras[k] + "\n\n"; }
    texte.trim();
    let e = s;
    e.title = `${s.title} §${x.de}-${x.a}`;
    e.text = texte;
    e.segment_sujet = x.titre;
    e.long = false;
    out.push(e);
}
out
