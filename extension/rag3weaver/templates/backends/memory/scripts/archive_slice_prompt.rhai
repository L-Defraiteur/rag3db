// archive_slice_prompt — the extraction prompt of a slice: the slice as it is (its turns are already
// numbered by « ## turn <n> »), and what the topics must be. No base, no candidate: step 1 only says
// what the slice talks about.
let s = input.contenu;
`Split this slice of a conversation into topics. A topic is a piece of work, a thing or a rule, never a date or a moment; neighbouring turns on the same topic go together, and a topic may come back later in the slice (give it several ranges). For each topic give: its title in a few words (title); the ranges of turns that carry it (turns: from, to, by the « ## turn <n> » numbers, inclusive); what is said about it, in a few sentences: the fact, the decision, what is left to do (said); whether it is still open at the end of the slice (open); free attributes as (dimension, value) pairs in plain words, without deciding anything (attributes, may be empty). Put in not_filed, with a reason, the turns that carry no topic (an acknowledgement, a politeness). Give also slice_summary: what happened in the whole slice, in a few sentences. Every turn must be in a topic or in not_filed.

${s}`
