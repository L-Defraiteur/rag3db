# Les versions orphelines du chemin d'avant faussent le BM25 d'une base réindexée

- **État** : ouvert ; constat, pas de correctif.
- **Famille** : produit
- **Gravité** : moyenne. Les réponses lexicales de toute base à versions sans générations
  réindexée sont classées sur des statistiques gonflées.
- **Atteignable en service** : oui, à chaque réindexation d'une base à versions sans générations,
  quand une édition ou une suppression laisse un contenu sans occurrence.
- **Touche rag3weaver** : oui. L'effacement à la fin d'instantané (`apply_snapshot_finish`) exclut
  les versions à identité de contenu (`ScopeContent`).
- **Pour** : à attribuer par l'orchestration. Everything declarative met fin aux orphelines pour les
  bases à générations, par l'écrivain de 2a′-2 ; les autres bases restent.
- **Signalé par** : banc, départage de la parité de l'écrivain, nuit du 15 novembre (au mot
  d'everything declarative).
- **Ouvert le** : 15 novembre 2026

## Le constat

Une copie de la base migrée (`1b-copie/migree-20261114-1641`), passée aux générations, est
réindexée sur une source plus récente (835d5298f), au commit 1105a2590b (le chemin d'avant
l'écrivain). Elle est ensuite lue sans rien réindexer :

- Scope : 77 149 lignes ;
- ScopeContent : **84 211**, dont **14 201 sans aucune origine** (`NOT EXISTS` sur
  `ScopeContent_DERIVED_FROM`) ;
- la même copie réindexée par l'écrivain (7ddedb026) n'en a aucune.

## Pourquoi c'est un défaut

- La fin d'instantané efface les occurrences que la source n'a plus, mais pas leurs versions de
  contenu. Elles restent donc, sans origine.
- Le masque des versions les écarte des résultats : le court-circuit sans filtre ne vaut que sans
  orpheline (ticket d'arbre principal
  `2026-11-14-la-recherche-a-versions-masque-toutes-les-occurrences-sans-filtre`).
- Mais elles restent au plein texte des versions. Le BM25 compte leurs documents et leurs termes :
  le nombre de documents, les fréquences et la longueur moyenne bougent. Sur la copie ci-dessus,
  un document sur six est orphelin.
- Elles forcent aussi le masque même sans filtre, donc le chemin lent (ticket ci-dessus).

## Ce qui reste à faire

- reprendre une version quand sa dernière occurrence part, à la fin d'instantané (sous la forme à
  versions sans générations) ; ou la retirer du plein texte avec elle ;
- un témoin rouge : une base à versions, un fichier édité puis réindexé ; après la fin, aucune
  `ScopeContent` sans origine, et les comptes du plein texte des versions égaux aux lignes.

## La preuve

- La sonde `releve_des_lignes_par_cypher`, dans
  `extension/rag3weaver/tests/sonde_parite_search_code.rs`, branche `banc-parite-search-code`
  1927bfd463.
- Le relevé : `banc-parite/sorties/ecrivain-2/depart-parent.json`, à luciepc.
- Le contexte : `docs/banc/14-11-2026/02-le-juge-de-parite-de-search-code.md` § 6, et le ticket
  d'everything declarative
  `2026-11-15-l-ecrivain-de-2a-prime-2-rougit-a-la-parite-lexicale.md`.

Fin : le témoin vert. Les orphelines d'une base réindexée retombent à zéro.
