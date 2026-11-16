# Un plein texte rebâti depuis les lignes ne classe pas comme celui que l'écriture tient

- **État** : fermé (16 novembre 2026, ED) — ce n'était pas le rebâti. Le juge cherchait avec 794b14232, où une base rouverte se croyait « tout visible » : la remontée vers les occurrences admettait les `Scope` fermées. Rejoué sur master 11878bf6d (qui porte le correctif, 209bc299b), l'élagué rend le classement de l'index neuf au rang près. La preuve est en fin de page
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

## La preuve (banc, 16 novembre 2026)

Le juge `releve_les_reponses_de_search_code` (bm25 seul, 30 cas, même manifeste), rejoué sur master 11878bf6d, sur des
copies neuves des trois bases :

| | rappel à 10 | même premier | même classement |
|---|---|---|---|
| neuf contre élagué | 289/289 = 1,000 | 30/30 | 30/30 |
| neuf contre lot | 260/289 = 0,900 | 25/30 | 8/30 |
| lot contre élagué | 260/289 = 0,900 | 25/30 | 8/30 |

- Le cas vide, « read_text » sur `src/include/`, a disparu.
- Entre le neuf et l'élagué, 257 scores sur 289 sont égaux au bit près. Les autres diffèrent d'au plus 7,6e-6, sans
  changer un rang.
- Le plein texte rebâti depuis les lignes classe donc comme celui que l'écriture tient. Le 0,668 venait de
  `tout_visible`, posé vrai à toute ouverture avant 209bc299b.
- **Ce qui reste entre le lot et le neuf (0,900) vient tout entier des versions fermées** : les retirer rend le
  classement du neuf. C'est la longueur moyenne du ticket BM25, que lucivy 4.3.9 corrige sans élaguer.

Sorties à luciepc : `banc-parite/sorties/preuve-bm25-master/`, script `~/.cache/rag3weaver-build/preuve-bm25-master.sh`.
