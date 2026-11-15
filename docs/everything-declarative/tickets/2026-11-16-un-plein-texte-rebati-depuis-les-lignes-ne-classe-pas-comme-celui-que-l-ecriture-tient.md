# Un plein texte rebâti depuis les lignes ne classe pas comme celui que l'écriture tient

- **État** : ouvert (16 novembre 2026)
- **Famille** : produit (le plein texte, son rebâti)
- **Gravité** : défaut. Un rebâti du plein texte, qu'il vienne d'une réparation, d'un élagage ou d'un `rebuild_fts_step`,
  rend une recherche lexicale moins juste que l'index qu'il remplace, alors qu'il en a les comptes exacts
- **Atteignable en service** : oui, partout où le plein texte se rebâtit depuis les lignes : dossier `.fts` jeté ou perdu,
  réparation, montée de schéma
- **Touche rag3weaver** : oui (le rebâti du plein texte depuis `ScopeContent_Chunk`, `node_id_cache`, les offsets lucivy)
  et peut-être **lucivy**
- **Pour** : ED, avec lucivy
- **Signalé par** : banc, la preuve par élagage du ticket BM25 (16 novembre 2026, 0 h 20) ; ouvert sur décision de c7

## Ce que c'est

Pour prouver la cause du ticket `2026-11-16-l-histoire-gardee-au-plein-texte-deplace-le-classement-bm25-de-la-vue-courante.md`,
banc a élagué la copie du lot. Il a retiré les 14 935 versions fermées, jeté le dossier du plein texte et rebâti celui-ci
depuis les lignes. L'élagué a **les comptes exacts de l'index neuf**, mais il classe **plus loin** de lui que le lot non
élagué. Rappel à 10, sur les 30 cas :

| Paire | Rappel à 10 |
|---|---:|
| index neuf / lot | 0,900 |
| index neuf / élagué | 0,668 |
| lot / élagué | 0,585 |

Un cas rend même **vide** sur l'élagué : « read_text », filtré sur `src/include/`.

Le plein texte rebâti n'est donc pas celui que l'écriture tient au fil de l'eau, à contenu égal. Pistes, aucune n'est
prouvée :

- **la correspondance offsets ↔ lignes** : le rebâti numérote les documents autrement que `node_id_cache`, et un hit
  remonte alors vers une autre ligne, ou vers aucune (le cas vide) ;
- **ce que le rebâti oublie** : un champ indexé par l'écriture (titre, chemin, le boost du titre) que le rebâti ne
  repose pas, ou des morceaux qu'il saute ;
- **le filtre `src/include/`** : il passe par `allowed_ids`, résolu en offsets. Si les offsets du rebâti ne sont plus
  ceux des lignes, le filtre n'admet rien.

## Les copies

À luciepc, sous `~/.cache/rag3weaver-build/banc-parite/` ; des copies reflink, jamais des bases de service :

- `ecr-base-ecrivain-neuf-5-lot-reindex-20261115/` : le lot, plein texte tenu par l'écriture ;
- `preuve-elague-copie-20261116/` et `preuve-elague2-20261116/` : l'élagué, plein texte rebâti ;
- `ecr-base-ecrivain-neuf-5-parent-reindex-20261115/` : l'index neuf du parent ;
- les sorties sous `sorties/ecrivain-neuf-5/`.

À compléter par banc : **la commande exacte de l'élagage et du rebâti**, puisque les provenances des deux copies ne la
disent pas.

## Comment le départager

1. Sur l'élagué, pour « read_text » filtré sur `src/include/`, comparer les offsets que résout le filtre aux offsets que
   le plein texte rend pour les morceaux de ces lignes.
2. Pour un cas qui classe autrement, prendre les dix premiers hits bruts de lucivy (offset, score), puis les lignes vers
   lesquelles ils remontent, des deux côtés.
3. Un témoin, rouge d'abord : écrire quelques scopes, chercher, jeter le plein texte, le rebâtir, chercher de nouveau.
   Les deux classements doivent être les mêmes au bit près, filtre compris.

## Comment le fermer

Par ce témoin vert. Puis la preuve par élagage de banc se refait sur un rebâti juste, pour le ticket BM25.
