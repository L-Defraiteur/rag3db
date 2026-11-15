// archive_segment_prompt — le prompt du découpage d'une section longue : ses paragraphes (séparés par
// une ligne vide) numérotés [1], [2]…, et ce que les sujets doivent être. Les mêmes paragraphes sont
// relus par archive_segment_build : la même découpe, dans le même ordre.
let s = input.section;
let paras = [];
for p in s.text.split("\n\n") { let t = p; t.trim(); if t != "" { paras.push(t); } }
let corps = "";
let i = 1;
for p in paras { corps += "[" + i + "] " + p + "\n\n"; i += 1; }
`Découpe cette section en sujets. Un sujet est un chantier, une chose ou une règle ; des paragraphes voisins sur le même sujet vont ensemble. Rends chaque sujet par son titre en quelques mots et par l’intervalle de ses paragraphes (de, a, numéros inclus), dans l’ordre, chaque paragraphe dans un seul sujet.

Section « ${s.title} », en ${paras.len()} paragraphes :

${corps}`
