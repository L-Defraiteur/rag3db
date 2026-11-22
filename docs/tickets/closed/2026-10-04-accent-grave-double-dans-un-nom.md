# Un accent grave doublé dans un nom n'est pas réduit

- **État** : fermé (22 novembre 2026, hotfixs), corrigé sur la branche `escaped-backtick-in-names`. L'analyseur
  (`transformSymbolicName`) réduit l'accent grave doublé, et le Cypher que le moteur génère le double, par
  `StringUtils::quoteIdentifier`, à 22 endroits : le `toCypher` des tables, des relations, des séquences, des
  propriétés et des macros ; l'EXPORT et l'IMPORT, clés primaires comprises, qui étaient écrites sans accents graves ;
  les fonctions de graphe ; `QUERY_VECTOR_INDEX`. La preuve : `UpstreamFixes.EscapedBacktickInAName` (sorti de
  known_red) et `BacktickNameRoundTripTest.NamesWithABacktickSurviveExportAndImport` (table, clé, propriétés et
  relation à accent grave, EXPORT, DROP, IMPORT), **rouges sur master 10b5dc12ac**, **verts avec le correctif**. Ce qui
  reste hors du lot est au ticket
  `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-22-des-noms-ecrits-dans-du-cypher-genere-sans-echappement.md`.
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non, par lecture : ses noms de tables et de colonnes (Scope_Chunk, embedding__<modèle>, _embed_hash__…) n'ont jamais d'accent grave
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : analyseur — à attribuer
- **Famille** : moteur

## Ce que c'est

Dans `` `a``b` ``, la double apostrophe inverse n'est pas réduite : le nom stocké est ``a``b`` au lieu de ``a`b``, et un EXPORT régénère un nom différent.

## Recette minimale

```cypher
CREATE NODE TABLE `a``b`(id INT64 PRIMARY KEY);
CALL show_tables() RETURN name;   -- attendu a`b, obtenu a``b
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — EscapedBacktickInAName

## Cause

`src/parser/transformer.cpp:193`.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `f03139f95` (12 août 2026).

## Pourquoi il n'est pas corrigé avec les autres (4 octobre)

Réduire l'accent grave doublé dans l'analyseur (`transformSymbolicName`) ne suffit pas :
un nom qui contient alors un accent grave s'exporte entre accents graves sans le doubler,
et l'EXPORT écrit du Cypher que l'IMPORT ne relit pas. Ladybug corrige aussi tous les
sites qui écrivent un identifiant dans du Cypher généré, par une fonction qui double
l'accent grave : 17 fichiers, dont le catalogue (`toCypher` des tables, des index, des
séquences, des propriétés), `export_db`, les fonctions de graphe et un fichier de
stockage. Ce n'est plus un correctif borné à l'analyseur ; arrêté là.

## Pour le fermer

le témoin passe au vert.
