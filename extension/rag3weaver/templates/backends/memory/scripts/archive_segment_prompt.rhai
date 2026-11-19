// archive_segment_prompt — the context part of the split of a long section: its paragraphs (separated by a
// blank line) numbered [1], [2]… The same paragraphs are read again by archive_segment_build: the same cut, in the
// same order. No prose: the rules of a topic are in the descriptions of the schema (lot B3).
let s = input.section;
let paragraphs = [];
for p in s.text.split("\n\n") { let t = p; t.trim(); if t != "" { paragraphs.push(t); } }
let body = [];
let i = 1;
for p in paragraphs { body.push("[" + i + "] " + p); i += 1; }
#{ name: "section", title: "The section: " + s.title, value: body.reduce(|acc, x| if acc == "" { x } else { acc + "\n\n" + x }, "") }
