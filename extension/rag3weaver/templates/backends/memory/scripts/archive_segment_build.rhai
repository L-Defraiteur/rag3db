// archive_segment_build — les sous-sections d'une section découpée : une par sujet rendu par le modèle,
// de titre « <titre de la section> §from-to » (l'identité d'un L1 reste unique, et le bilan lit toujours
// le premier tour du titre) et de texte ses paragraphes. Un intervalle hors des paragraphes est refusé
// en le nommant ; aucun sujet : la section reste entière.
let s = input.section;
let paragraphs = [];
for p in s.text.split("\n\n") { let t = p; t.trim(); if t != "" { paragraphs.push(t); } }
let topics = if input.value == () || input.value.topics == () { [] } else { input.value.topics };
if topics.len() == 0 { let e = s; e.long = false; return [e]; }
let out = [];
for x in topics {
    if x.from < 1 || x.to > paragraphs.len() || x.from > x.to {
        throw `archive_segment : le sujet « ${x.title} » a l’intervalle ${x.from}-${x.to}, hors des ${paragraphs.len()} paragraphes de « ${s.title} »`;
    }
    let text = "";
    for k in range(x.from - 1, x.to) { text += paragraphs[k] + "\n\n"; }
    text.trim();
    let e = s;
    e.title = `${s.title} §${x.from}-${x.to}`;
    e.text = text;
    e.segment_topic = x.title;
    e.long = false;
    out.push(e);
}
out
