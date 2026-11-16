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

## 10. Ajout, 6 h : l'histoire au plein texte, prouvée par l'élagage

Le banc avait élagué une copie du lot de l'écrivain (794b14232), sur la base de test
`code-rag3db-depot`. Il en avait retiré les 14 935 versions sans aucune occurrence ouverte, avec
leurs morceaux et leurs arêtes, puis rebâti leur plein texte depuis les lignes. Le juge rendait alors
un rappel de 0,668 contre l'index neuf, et un cas vide (`read_text · src/include/`). Ce juge
cherchait avec 794b14232, avant le correctif « tout visible » d'ED (209bc299b9). Sur une base
rouverte, ce code admettait les scopes fermées.

Les trois mêmes bases ont été relues par master 11878bf6d2, chacune sur une copie neuve, avec le
même manifeste, en bm25 seul :

| | rappel à 10 | même premier | même classement |
|---|---|---|---|
| neuf contre élagué | 289 / 289 = 1,000 | 30 / 30 | 30 / 30 |
| neuf contre lot | 260 / 289 = 0,900 | 25 / 30 | 8 / 30 |

- Le cas vide a disparu. Sur la paire neuf/élagué, 257 scores sur 289 sont égaux au bit près ; les
  autres diffèrent d'au plus 7,6e-6, sans changer un seul rang.
- **L'écart entre un lot et un index neuf vient tout entier des versions fermées, que le plein
  texte compte encore** : les retirer rend le classement du neuf, au rang près. C'est la cause (1)
  du § 3, celle que lucivy 4.3.9 doit effacer sans élaguer.
- Le 0,668 d'avant venait du défaut « tout visible », pas de l'élagage.

## 11. Ajout, 9 h 50 : sous lucivy 4.3.9

Rejouée par master fe5d81f811 (pointeur cb4a1757a4 et ordre stable des ex æquo 0ae49fa84c). lucivy-core
est en 4.3.9 dans le lock du worktree comme dans `cargo tree`. Chaque relevé lit une copie reflink fraîche.

| A contre B′ | rappel à 10 | même premier | même classement | même classement, scores à 1e-5 |
|---|---|---|---|---|
| bm25 seul, 4.3.8 (§ 3) | 270 / 279 | 27 / 30 | 14 / 30 | 11 / 30 |
| bm25 seul, 4.3.9 | 271 / 279 | 27 / 30 | 21 / 30 | 12 / 30 |
| vector seul, 4.3.9 | 292 / 300 | 28 / 30 | 23 / 30 | 2 / 30 |
| tous signaux, 4.3.9 | 287 / 300 = 0,957 | 28 / 30 | 22 / 30 | 18 / 30 |

- **Le bm25 seul s'améliore** (14 → 21 classements égaux) **mais n'est pas égal** : 9 cas diffèrent encore.
- Les requêtes d'un mot (`read_text`, `rag3db`, `value`) ont des scores autour de 0,021, avec un écart relatif
  de 3 à 4,5 % sur les résultats communs. Les requêtes de plusieurs mots sont sous 5e-4, sauf « translate a
  hop into a query » (8e-3), où deux résultats changent aux rangs 9 et 10.
- Selon le ticket `docs/lucivy/tickets/2026-11-16-le-bm25-seul-de-la-migration-reste-a-prouver-egal-sous-4-3-9.md`,
  c'est à lucivy de départager par `test_compare_two_indexes`, sur le dossier `Lucivy_ScopeContent` des
  copies A et B′. Les entrées lui sont données.
- A contre B en tous signaux : 233 / 293 = 0,795. B garde son ancien découpage, donc ce n'est pas une parité.

Sorties à luciepc : `banc-parite/sorties/lucivy-439/`. Script : `~/.cache/rag3weaver-build/parity-lucivy-439.sh`.
