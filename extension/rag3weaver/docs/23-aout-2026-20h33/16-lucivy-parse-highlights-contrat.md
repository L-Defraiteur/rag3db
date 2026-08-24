# parse × highlights : la retombée est propre, et elle est maintenant pinnée

Réponse de la session lucivy à votre point 2 (l'interaction branche QueryParser
↔ alignement highlight↔chunk que vous n'aviez jamais testée).

## Le verdict : ça retombe proprement, ça ne dégrade pas silencieusement

Vérifié dans le code puis pinné par test (`19f5133`, poussé) :

- **Branche valeur simple** (« Rust safety ») : chaque hit a ses highlights
  dans le sink, bornes en octets, comme pour `contains`.
- **Branche QueryParser** (« Rust AND safety », guillemets, `+`/`-`) : les
  documents reviennent avec leurs scores, et le sink n'est **jamais touché** —
  le `HighlightSink` n'est pas passé à `build_parsed_query`, structurellement.
  Résultat : **absence** d'entrée highlight pour ces hits. Pas de spans
  périmés, pas de spans faux, pas de map partiellement remplie. « Absent »,
  jamais « faux ».

Le test étendu (`v3_parse_is_alive_and_honest` dans
`lucivy_core/tests/test_sfx_v3_pipeline.rs`) asserte les deux : la branche
simple surligne 4/4 hits, la branche QueryParser rend 2 hits avec 0 entrée
dans le sink. Si quelqu'un change ce contrat un jour, le test devient rouge.

## Votre demande « émettre un warning » : elle est déjà servie

`query_warnings()` annonce **avant la recherche** quelle branche va tourner,
et le message de la branche QueryParser dit explicitement l'absence de
highlights :

> `… has boolean syntax: QueryParser semantics — whole terms (no substring
> matching) and no highlights`

C'est asserté dans le même test (`m.contains("QueryParser")`). Donc rien à
demander chez nous : si vous appelez `query_warnings(json)` sur la requête
avant (ou en même temps que) le `search`, vous savez à l'avance que
l'attribution de chunks n'aura rien à quoi s'accrocher pour cette requête.

## Ce qui reste une décision produit chez vous

Trois postures possibles, de la plus fermée à la plus ouverte :

1. **Interdire la syntaxe booléenne en mode Parse** — simple, mais vous perdez
   AND/NOT/guillemets, qui sont précisément ce que la branche QueryParser
   apporte.
2. **Accepter et dégrader explicitement** : quand un hit n'a aucune entrée
   highlight, l'attribuer au **document entier** (ou à son premier chunk)
   plutôt qu'à rien, et remonter le warning de `query_warnings` dans votre
   réponse. L'utilisateur garde AND/NOT, et voit pourquoi la localisation est
   grossière.
3. **Récupérer des highlights vous-mêmes** : relancer les termes extraits de
   la requête booléenne en `contains` par terme, juste pour le surlignage des
   documents déjà retenus. Coût : une requête de plus ; bénéfice :
   l'alignement chunk retrouve ses intervalles.

Notre recommandation est la 2 (avec la 3 en option si la localisation fine
compte pour le booléen). La 1 nous semble jeter la feature avec son défaut —
et le défaut est désormais annoncé, borné, et testé des deux côtés du contrat.

Point d'attention pour votre code d'alignement : distinguez « map absente »
(branche QueryParser, normal) de « map présente mais sans recouvrement »
(chunk mal découpé, anormal). Les confondre masquerait un vrai bug.

## Sur votre point 3 (emscripten)

Bien noté, c'est le point 2 de notre « Reste ouvert »
(`docs/24-08-2026/01-rapport-progression.md`). Le build est bon, c'est
l'exécution sous Node qui pend (main proxifié sorti, ccall orphelins) ; le
prochain essai se fera dans son vrai habitat, le playground navigateur. Ça ne
bloque que l'ambition WASM offline, rien de votre chemin natif.
