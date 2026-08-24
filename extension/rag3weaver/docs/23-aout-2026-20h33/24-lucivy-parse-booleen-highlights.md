# `parse` booléen : highlights désormais, et une seule sémantique — vos options 2 et 3 du doc 16 deviennent inutiles

Le doc 16 vous laissait choisir quoi faire de la branche booléenne de `parse`
(syntaxe `AND`/`OR`/`NOT`, guillemets) qui ne rendait pas de highlights.
Cette branche n'existe plus sous cette forme (`8f14edc`, poussé).

## Ce qui change

La syntaxe booléenne n'est plus envoyée au `QueryParser` (termes entiers sur
l'index inversé, sans sink). Elle est **traduite chez nous** en composite
`boolean` de `contains` :

| syntaxe | devient |
|---|---|
| `a AND b` | `must: [a, b]` |
| `a OR b`, `a b` | `should: [a, b]` (mots côte à côte = OR, comme la branche simple) |
| `NOT a`, `-a` | `must_not: [a]` |
| `+a b` | `must: [a], should: [b]` (règle Lucene : dès qu'un terme est requis, les autres ne font que scorer) |
| `"a b"` | un seul `contains` de la phrase (adjacence SFX, séparateurs relaxed) |
| `(a OR b) AND c` | parenthèses de groupement — **seulement** quand elles sont autonomes ou ouvrent/ferment un mot ; `f(x)` reste littéral |

Précédence : NOT > AND > OR. Chaque feuille est un `contains` sous-chaîne
étalé sur `fields` (un `should` par champ si plusieurs). Conséquences :

- **Highlights sur toutes les formes de `parse`.** Votre invariant
  highlight↔chunk tient partout ; plus de cas « map absente ».
- **Une seule sémantique.** Avant, la branche simple faisait de la
  sous-chaîne et la branche booléenne des termes entiers — `Rust safety` et
  `Rust AND safety` ne matchaient pas les mêmes documents pour des raisons
  sans rapport avec l'opérateur. C'est fini. (Le mot entier reste disponible
  explicitement : `term`, ou `contains` + `anchor_start` + `exact_match`.)
- **Refus explicites** au lieu de résultats vides : `NOT rust` seul → « a
  query cannot be only a negation » ; `rust AND` → « expected a term » ;
  `(rust AND safety` → « unbalanced '(' ». À remonter tels quels.
- `query_warnings` dit toujours quelle forme tourne ; le message de la forme
  booléenne ne parle plus d'absence de highlights.

Gardes : `v3_parse_is_alive_and_honest` (le contrat highlights asserte
maintenant que les DEUX formes remplissent le sink) et
`v3_parse_boolean_syntax_is_composite` (opérateurs, précédence, `+`/`-`,
phrase, parenthèses littérales vs groupantes, multi-champs, refus).

## Chez vous

- Épingler `8f14edc`. Aucun changement d'API ni de config.
- Si vous aviez retenu l'option 1 du doc 16 (interdire la syntaxe booléenne
  en mode Parse), elle n'a plus de raison d'être. Les options 2 et 3 non
  plus.
- Ce que le `QueryParser` savait et que la traduction ne fait pas : la
  syntaxe `champ:terme`, les plages, les boosts. Si l'un de vos utilisateurs
  s'en servait via `BM25Mode::Parse`, `champ:terme` est maintenant un
  littéral (`std::sync` aussi, ce qui est plutôt une bonne nouvelle pour du
  code). Dites-le si ça vous manque, ça se rajoute.

## Et la précision promise sur `is_content_char`

La conséquence pratique du choix « tout non-ASCII est contenu » n'est pas en
strict (les octets diffèrent de toute façon), elle est en **relaxed** :
`foo bar` matche `foo-bar` (le `-` est un séparateur, dépouillé) mais pas
`foo—bar` (le `—` est du contenu, il reste). Le jour où le tiret cadratin
doit se comporter comme le tiret ASCII, c'est le changement de format dont
on a parlé.
