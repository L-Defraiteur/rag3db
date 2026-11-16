# La parité de la migration (16 novembre 2026)

La bascule passe les bases de Lucie aux générations. Avant de la faire, c7 a demandé si la copie
migrée rend d'aussi bonnes réponses que la base d'avant, et si elle rend les mêmes qu'un index neuf.
Cette page dit ce qui a été mesuré, à luciepc, la nuit du 15 au 16 novembre.

## 1. Les trois bases

- **A**, la copie migrée : la copie de `code-principal` faite par la répétition de la bascule
  (réparation, purge, 1b, générations, réécriture avec ses vecteurs). Forme `Generations`.
- **B**, la référence d'avant : l'instantané de 00 h 51, réparé et purgé de la même façon, sans
  réindexation. Forme `SansVersions`.
- **B′**, un index neuf : l'arbre 284cc00c0b, sous-module `codeparsers` compris, indexé dans une
  base vide par master 767c7234e3. Il est embarqué par le seul service 7878 (granite-278m) et
  porte le manifeste de Lucie. Forme `Generations`.

Le même code relit les trois (master du moment). Les 30 cas du jeu `jeu-recherche-2a2.json` sont
joués tous signaux, avec vector et bm25 exigés : un relevé refuse si l'un d'eux ne contribue à
aucun résultat.

## 2. A et B contre les bonnes réponses

Le jugement ne compare pas A à B. Pour chaque cas, les résultats des deux côtés sont mis en
commun. Gemini 3.8 Flash, à température 0, note chacun 0, 1 ou 2, sur ses 25 lignes de code lues
dans l'arbre 284cc00c0b, sans savoir de quel côté vient le résultat. Une note de 2 est une bonne
réponse.

| | rang 1, A | rang 1, B | bonnes réponses à 10, A | à 10, B | bonnes réponses en commun |
|---|---|---|---|---|---|
| tous signaux | 4 / 30 | 4 / 30 | 29 | 31 | 38 |
| dense seul | 8 / 30 | 3 / 30 | 33 | 27 | 40 |

- **Au rang 1 : neutre.**
- **À 10 : A perd 2 bonnes réponses sur 38**, sur deux cas filtrés sur `src/include/` :
  `rag3db_database` (`c_api/rag3db.h:146`) et `getValue` (`value.h:214`). C'est de l'ordre du
  bruit d'une seule passe de notes, qui n'a pas été mesuré.
- **Le dense seul gagne nettement** : A est découpée par le découpeur d'aujourd'hui, avec un contenu
  borné aux octets du nœud.
- Les rangs 1 sont bas des deux côtés : le jeu porte des requêtes d'un mot (`read_text`, `value`,
  `rag3db`), et le juge est strict.

**Les deux pertes ne sont pas des scopes manquants.** Les deux scopes existent dans A, en rangées
ouvertes. Ce sont des reclassements :
- dans `rag3db · src/include/`, des scopes `namespace rag3db {` d'en-têtes (lignes 5 à 10)
  passent devant la déclaration ;
- dans `value · src/include/`, `getValue` n'était qu'au rang 10 dans B.

## 3. A contre B′

| | rappel à 10 | même premier | même classement | mêmes scores aussi |
|---|---|---|---|---|
| tous signaux | 293 / 300 = 0,977 | 28 / 30 | 21 / 30 | 13 / 30 |
| bm25 seul | 270 / 279 = 0,968 | 27 / 30 | 14 / 30 | 11 / 30 |
| vector seul | 292 / 300 = 0,973 | 28 / 30 | 23 / 30 | 2 / 30 |

- Les écarts sont tous en fin de liste (rangs 8 à 10) ou dans l'ordre.
- `rag3db_database` et `getValue` ne sont dans le top 10 ni de A ni de B′, et `rag3db.h` a 25
  scopes des deux côtés. **Un index neuf d'aujourd'hui rend les mêmes deux pertes : elles ne viennent
  pas de la migration.** Elles viennent du classement d'un scope conteneur contre une déclaration.
- **Le bm25 seul diffère** alors que la requête et l'arbre sont les mêmes : le plein texte de A ne
  compte pas les mêmes documents. A garde l'histoire, 113 570 `ScopeContent_Chunk` contre 96 884
  dans B′. C'est ce que lucivy 4.3.9 doit corriger, ce qui reste à prouver : rejouer le bm25 seul
  sur les mêmes copies une fois le pointeur sur master. Il doit passer à l'égalité.
- **Le vector seul diffère dans les scores, presque pas dans le classement** (voir § 4).

## 4. Les vecteurs, jugés par tolérance

Les vecteurs de morceaux communs, liés par `_text_hash` hors des 31 morceaux longs (§ 5), sur les
hashes qui commencent par « 00 » :

- 412 morceaux communs, dont 217 identiques au bit près (53 %) et 195 qui diffèrent ;
- **cosinus minimal 0,99999991, écart absolu maximal 5,86e-5** ; aucun sous 0,99999, aucun au-dessus
  de 1e-4.

La sonde d'embarquement sur le 7878 a donné la cause. Un même texte, seul puis seul, ou dans un même
lot deux fois, rend les mêmes bits. Seul contre dans un lot de textes plus longs, il diffère de
3,77e-5. Le calcul est déterministe pour une même forme de lot : l'écart vient du remplissage et de
la composition du lot, pas du binaire.

**La règle** : une parité de vecteurs entre deux bases embarquées à des moments différents se juge
par tolérance (cosinus au-dessus de 0,99999, écart absolu sous 1e-4), et par la stabilité du
classement. Pas au bit près.

## 5. Le défaut trouvé en route : un texte trop long tuait l'embarquement

Le premier B′ (master bc554c2cee) n'a eu **aucun vecteur sur ses 92 708 `ScopeContent_Chunk`**.
- 31 morceaux font plus de 12 000 caractères, jusqu'à 53 294. Ils viennent de fichiers de données
  de test : `copy_edge_case.test`, `demo.test`, `duplicated.test`, `q24.benchmark`,
  `large_list_of_struct_answer.txt`.
- Le 7878, posé le 15 novembre à 20 h 36 avec un plafond de 12 000 caractères, refusait **tout** le
  lot de 512 dès qu'un seul texte le passait.
- L'exclusion du manifeste de Lucie (`minified_line_max_bytes: 5000`) ne couvre pas ces fichiers
  `texte_brut`. Par exemple, `demo.test` fait 87 709 octets pour 107 lignes de littéraux de
  vecteurs.

Le ticket est `docs/tickets/closed/2026-11-16-un-texte-au-dessus-du-plafond-du-service-fait-tomber-tout-l-embarquement.md`,
fermé par embarquement avec la coupe au plafond (828ff28153). Sur le B′ d'après, les 31 morceaux
longs ont tous leur vecteur : coupés à 12 000 dans B′, entiers dans A. Les 96 884 morceaux sont
embarqués, sans aucune passe en échec.

## 6. Le coût de B′

787 s en tout, à luciepc sous `poste lourd`, par le 7878 seul : 721 s d'index et d'embarquement
ensemble, puis le relevé.

## 7. Les erreurs du banc, et ce qu'elles apprennent

- **Une source sans sous-module.** Le premier B′ complet venait d'un worktree détaché sans
  `submodule update` : tout `codeparsers` lui manquait (92 708 `ScopeContent_Chunk`, contre
  96 884 avec le sous-module), et le cas « parse a json manifest » rendait une autre liste. Le script refuse maintenant une source dont un sous-module est
  vide.
- **Un relevé avant la fin de l'embarquement.** Sur une base vide, l'index finit avec une dette
  d'embarquement, et un relevé pris à ce moment ne classe qu'en bm25. Le relevé exige ses signaux,
  ce qui l'a dit au lieu de rendre un faux vert. La sonde peut solder la dette avant de lire
  (`RAG3WEAVER_PARITE_EMBED_BACKLOG=1`).
- **Une course contre un correctif.** La mesure de codeparsers a lu le juge 23 s avant qu'un
  correctif E0382 n'arrive sur la branche, et une copie de sonde vide a été prise pour la base.
  Les deux ont été rejouées.
- **Un guetteur sur un hash.** Un correctif rebasé arrive sur master sous un autre hash : un
  guetteur qui attend un hash précis ne part jamais. On attend le commit annoncé « poussé ».

## 8. Ce que Lucie lira

- La copie migrée ne perd rien de propre à la migration : un index neuf rend les mêmes deux pertes,
  et pour la même raison.
- Elle diffère d'un index neuf par deux choses, que la bascule efface toutes les deux :
  - le plein texte compte encore l'histoire, ce qu'attend lucivy 4.3.9 ;
  - ses vecteurs, de l'ordre du bruit de f32, sont remplacés par la bascule.
- Le dense gagne avec le découpage d'aujourd'hui : 8 rangs 1 contre 3, et 33 bonnes réponses à 10
  contre 27.
- **Reste à jouer** : quand le pointeur lucivy 4.3.9 sera sur master, le bm25 seul et tous les
  signaux, sur les mêmes copies A et B.

## 9. Les sorties

À luciepc, `~/.cache/rag3weaver-build/banc-parite/sorties/` :
- `migration-parity/`, A et B : les relevés, `relevance-*.json` et `deux-scopes-A.json` ;
- `b-prime/`, A et B′ : les relevés par signal, `judge-*.txt`, `vec2-*.json` et `gros-3-*.json`.

Les scripts sont dans `~/.cache/rag3weaver-build/` : `parity-migration.sh`, `b-prime.sh`,
`b-prime-signals.sh` et `vec-cmp.py`. Le juge de pertinence est
`~/.cache/rag3db-banc-notes/compaction/j_relevance.py`.
