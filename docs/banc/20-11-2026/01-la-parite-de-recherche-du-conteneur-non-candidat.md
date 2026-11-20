# La parité de recherche du conteneur non candidat (2.12)

19 et 20 novembre 2026, nuit, à la demande d'ED et au critère de c7. Le lot 2.12 (everything declarative) retire des
candidats de `search_code` un `namespace` ou `module` replié qui ne porte presque rien à lui. Sur une base déjà indexée,
le geste `set_aside_non_candidates` écarte ces conteneurs au premier index suivant.

**Le critère de c7** : aucun premier résultat perdu sur le jeu commun ; et sur « rag3db » filtré sur `src/include/`,
`rag3db_database` entre dans les 20 premiers.

## Le verdict, sur la tête retenue

**container-not-candidate-4 `6ff7d11cae`** contre son parent **`a542e0f518`** (la pile de codeparsers, règles d'analyse
à 9 ; le lot les monte à 10) : **le critère est tenu, sur base existante comme sur index neuf.**

| | base existante | index neuf |
|---|---|---|
| `rag3db_database`, « rag3db » sur `src/include/` | **rang 8** (absent du top 20 au parent) | **rang 8** (absent du top 20 au parent) |
| premiers résultats perdus (30 cas) | **0** | **0** |
| premiers résultats changés | 1 | 1 |
| le geste au journal d'index | « 3188 versions de conteneurs repliés écartées de la recherche (plein texte et morceaux) » | pas de ligne (rien d'indexé avant la règle) |

- **Le premier résultat changé** est le même dans les deux jeux. « ouvrir une base et enregistrer le schéma », sans
  filtre, passe de `ouvrir` (`tests/e2e_outils_disent_ce_qui_reste.rs`) à `ouvrir` (`tests/e2e_plan_declare.rs`).
  L'ancien premier reste dans le top 10.
- **Le top 10 de « rag3db » sur `src/include/`**, côté lot : `rag3db_state`, `rag3db_data_type_id`,
  `Rag3dbCypherParser`, `AttachedRag3dbDatabase`, `Rag3dbCypherParser`, `rag3db`, `copyToRag3dbString`,
  **`rag3db_database`**, `rag3db_value`, `rag3db_logical_type`. Au parent : `rag3db_state`, `rag3db_data_type_id`, puis
  huit `namespace rag3db` repliés.
- **Le reste du top 10** bouge dans une douzaine de cas sur 30. Ce sont surtout des scores d'un poil (le plein texte
  compte moins de documents) et des rangs qui glissent d'une place. Le détail est dans `juge-apres.txt`.

## Le dispositif

- **La base existante** : une copie reflink neuve, par côté, de `banc-parite/ecr-base-ecrivain-neuf-5-lot-reindex-20261115`
  (indexée avant la règle, en Generations, ouverte avec la lib 6e2476d41).
- **L'index neuf** : une base vide, par côté.
- **La source** : `rag3db-banc-source-2` à `1de1f86d88`, la même des deux côtés.
- **Le jeu** : la sonde `releve_les_reponses_apres_une_reindexation_publiee` sur le jeu commun (k = 10) : un relevé
  avant, l'index de la source sous le tampon par défaut, un relevé après. Puis « rag3db » sur `src/include/` en k = 20.
  Le comparateur `parite_search_code.py`, au régime `reponses`.
- **Les sorties**, à luciepc : `~/.cache/rag3weaver-build/banc-parite/sorties/ns-2.12-20261120-{existante,neuf}-6f/`.
  Les journaux d'index sont sous `~/.cache/rag3weaver/commandes/index/`.

## Le chemin de la nuit, tête par tête

| tête | ce qui s'est passé |
|---|---|
| `19c150a836` (sur `stop-words-locals`) | la réindexation d'une base existante remplit le tampon (8 Gio, puis 24 Gio) en recopiant les arêtes `DEFINED_IN`, des deux côtés : le défaut corrigé sur master par `cd18543ac0`, absent de cette pile. Sur index neuf : 0 premier perdu, mais `rag3db_database` absent du top 20. Les `namespace rag3db` extérieurs gardent `} // namespace x` et le pli `namespace x { …` |
| `3b093717b2` | reposé sur master ; ne compilait pas (le conflit de `catalog.rs`) |
| `85ef7ee084` | les plis reconnus : sur **index neuf**, rang 8. Sur **base existante**, aucun effet |
| `fe46d752c6` | la poignée du plein texte ouverte par le geste ; sur base existante, toujours aucun effet |
| `33d67aec1b` | le geste joué avant l'aiguillage de la source (une source repointée prenait la voie de l'instantané, où le geste n'était pas joué) : sur base existante, **rang 8**, 3 188 conteneurs écartés |
| `6ff7d11cae` | le même code sur la pile de codeparsers, règles à 10 : **le verdict ci-dessus** |

## Ce que le banc a appris

- **Le journal d'index n'est pas dans la sortie de la sonde** : elle n'en garde que les six dernières lignes
  (`fin_index`). Il est sous `~/.cache/rag3weaver/commandes/index/<horodatage>-<pid>-<n>.out`.
- **Un script bash qui tourne lit son fichier au fil de l'eau** : réécrire un script en cours de route décale ce qu'il
  lit ensuite. Une variante va dans un fichier neuf.
- **Un dossier de compilation par worktree** : le garde de cargo refuse un target qui a servi à un autre arbre.
