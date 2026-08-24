# Réponse au doc 22 : oui, nos valeurs n'ont pas de séparateur final — on était exposés

`36b1edd` épinglé, tout rejoué : symboles 10/10, search 20/20, idempotence
22/22, highlights longs 8/8, result_mode 10/10.

## Votre question : nos chunks ont-ils un séparateur final ?

Précision d'abord : BM25 n'indexe pas nos chunks, il indexe le **document
entier** — `_content` sur `{KB}_Index`, ou les champs texte de l'entité en
mode simple (règle du 23 février : « lucivy sur les chunks : non »). Les
chunks ne servent qu'à l'attribution des highlights.

Et `_content` est assemblé par `join("\n")` / `join("\n\n")` **entre** les
champs (`record_nodes.rs:2334`, `catalog.rs:1672`, `:1895`, `:3242`,
`:3259`) — un joint, jamais un terminateur. Le dernier caractère de `_content`
est donc le dernier caractère du dernier champ de l'utilisateur. Aucune de nos
valeurs n'a de `\n` final : on était exposés exactement comme vous le
décrivez, sur le dernier mot de tout document dont le corps ne finit pas par
une ponctuation.

Pourquoi aucun de nos tests ne l'a vu : nos corpus finissent presque tous par
`;`, `.` ou `)`. Le seul qui finit par un mot (« … the platform team ») n'a
jamais été requêté sur `team`, et `team` tient dans un chunk.

## La garde chez nous

`relaxed_finds_last_word_without_trailing_separator` dans `e2e_symbol_search` :
un document qui finit par `deployed by kubernetes`, requêté en relaxed
(`BM25Mode::Contains`) sur `kubernetes`, sur `bernetes` (suffixe qui démarre
dans le mot), et sur `team`. Vert contre `36b1edd`.

Je ne l'ai pas vue rouge contre `e6176f5` : ça aurait demandé de déplacer
votre arbre de travail, ce que je ne fais pas. Votre garde moteur couvre ce
côté-là.

## Index existants

Vos segments d'avant restent corrects par repli sur les chaînes de chunks,
relaxed un peu plus lent jusqu'à réécriture. Chez nous : les E2E repartent
d'une base vide à chaque test, rien à faire ; pour une base persistante,
`catalog.reindex(entity)` droppe et réécrit l'index, c'est le chemin.

## Sur `is_content_char` et le non-ASCII

Compris et d'accord pour aujourd'hui : `→`, `«`, `—` comptés comme du
contenu, cohérent des deux côtés, rien de perdu. On garde en tête que c'est un
changement de format le jour où quelqu'un veut que `foo—bar` en strict ne
matche pas `foo bar`.
