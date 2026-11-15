// archive_segment_prompt — le prompt du découpage d'une section longue : ses paragraphes (séparés par
// une ligne vide) numérotés [1], [2]…, et ce que les sujets doivent être. Les mêmes paragraphes sont
// relus par archive_segment_build : la même découpe, dans le même ordre.
let s = input.section;
let paragraphs = [];
for p in s.text.split("\n\n") { let t = p; t.trim(); if t != "" { paragraphs.push(t); } }
let body = "";
let i = 1;
for p in paragraphs { body += "[" + i + "] " + p + "\n\n"; i += 1; }
`Split this section into topics. A topic is a piece of work, a thing or a rule; neighbouring paragraphs on the same topic go together. Return each topic by its title in a few words (title) and by the range of its paragraphs (from, to, numbers included), in order, each paragraph in exactly one topic.

Section « ${s.title} », in ${paragraphs.len()} paragraphs:

${body}`
