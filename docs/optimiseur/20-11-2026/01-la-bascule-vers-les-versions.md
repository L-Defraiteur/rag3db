# La bascule des bases de code vers les versions, 20 novembre 2026

Cette page dit comment la bascule du 20 novembre s'est décidée puis jouée. Elle donne aussi la preuve qui tient le
seuil de recherche.
- **Ce qui a été répété** : trois répétitions sur copie.
- **Le choix de l'hôte** : d'abord 1f22743e17 (2.12), puis baccb2ea6e, après le défaut du plein texte de 2.12.
- **Le résultat** : code-rag3db-depot basculée et verte ; code-lourd et code-principal remises sur leurs instantanés,
  avec la cause de leur recul (88 %).
- **La lib du train C++** : 4ad394f4d2, posée ensuite.

## Les répétitions

Chaque répétition joue la fenêtre entière sur une copie reflink neuve de l'instantané de la vraie base
(`avant-versions-20261119/code-rag3db-depot`). Un hôte isolé (port 7992) fait 1b, les générations, la réouverture,
puis le premier passage de l'écrivain jusqu'à « indexation terminée », avec la mémoire relevée toutes les 10 s. Le juge
compare la copie repassée à **la base de production relevée au même moment, sur le même arbre** : jamais à un ancien
index, dont l'arbre et l'analyse ont changé depuis. Le critère, donné par c7, comporte trois conditions :
- recouvrement moyen du top 10 de search_code ≥ 90 % sur dix requêtes ;
- même premier résultat sur au moins 9 requêtes ;
- **aucun site perdu** dans `usages`, sur quatre noms. Les sites gagnés par une analyse plus neuve sont attendus.

| Hôte | Règles | Repassage | RSS max | Usages perdus | Recouvrement | Même premier |
|---|---|---|---|---|---|---|
| a83e234fc4 (19 nov., 19 h 47) | 8 | 20 min | 9,1 Gio | **41** (Catalog, tests/*.rs) | 96,8 % | 10/10 |
| baccb2ea6e (20 nov., 01 h 08) | 9 | 19 min | 9,7 Gio | 0 (+341 Catalog) | 97,8 % | 10/10 |
| **1f22743e17** (20 nov., 03 h 03) | **10** | 27 min | 8,9 Gio | 0 (+341 Catalog) | **88,5 %** | 10/10 |

- **La première répétition**, sur a83e234fc4 (le correctif du tampon d'ED, 9,1 Gio au lieu d'un tampon plein), a
  perdu 41 usages de `Catalog` dans les tests. Codeparsers l'a corrigé avec stop-words-locals : un appel `Type::f()`
  compte comme usage du type. À la deuxième répétition, les 41 sites reviennent. Sur 12 sites gagnés tirés au hasard,
  12 sont de vrais `Catalog::new(`, `Catalog::rechercher(` ou `Catalog::Get(`.

## Pourquoi 88,5 % tient le seuil

Le seuil n'est pas relâché : il est tenu par la preuve. Sans 2.12, sur l'hôte baccb2ea6e, le recouvrement contre la
production est de **97,8 %**. Avec 2.12, il tombe à **88,5 %**. L'écart est le changement de classement que 2.12 apporte,
et non celui de la bascule. Requête par requête, la copie de 2.12 diffère de la production exactement comme elle
diffère de la copie baccb2ea6e, avec la même intersection. Ce sont ces mêmes écarts que la parité du banc d'ED a jugés
verts, sur la base existante comme sur un index neuf. Le principe de 2.12 est qu'un conteneur n'est plus candidat : des
fichiers-conteneurs sortent, des définitions et des pages proches de la question entrent.

| Requête | Top 10 commun | Sort | Entre |
|---|---|---|---|
| commit | 9/10 | `src/storage/undo_buffer.cpp:186` | `extension/rag3weaver/src/dataflow/record_nodes.rs:3352` |
| index du plein texte | 9/10 | `extension/rag3weaver/tests/e2e_scope_versions.rs:264` | `extension/rag3weaver/src/catalog.rs:1661` |
| point de reprise | 9/10 | `extension/rag3weaver/tests/e2e/main.rs:176` | `extension/rag3weaver/tests/e2e_tx_par_paquet_arret.rs:695` |
| mutex | 9/10 | `third_party/re2/include/mutex.h:69` | `extension/rag3weaver/src/read_view.rs:118` |
| fermer un backend | 8/10 | `tests/e2e_demon_hote.rs:495`, `src/dataflow/index_nodes.rs:270` | `docs/recherche/11-11-2026/02-knowledge-dump.md:1`, `docs/tickets/2026-10-11-on-ne-peut-pas-arreter-un-index.md:1` |
| les liens par COPY | 8/10 | `tests/e2e/main.rs:114`, `tests/e2e_copy_liens.rs:80` | `docs/tickets/closed/2026-11-18-la-pose-des-liens-de-la-resolution-repasse-par-merge.md:1`, `src/catalog.rs:10260` |
| test e2e | 6/8 | `tests/e2e/main.rs:1`, `docs/arbre-principal/13-11-2026/raccord-physique/globale.py:265` | quatre scripts de preuve et `scripts/test_run_e2e.sh:1` |
| FileSource | 9/10 | `src/dataflow/run_nodes.rs:471` | `src/dataflow/render_nodes.rs:967` |
| chunk_uuid, acquireLock | 10/10 | — | — |

Le premier résultat est le même sur les dix requêtes, et aucun usage n'est perdu.

Décision de c7 à 3 h 5x : bascule sur 1f22743e17. **Elle a été revue à 5 h** (section suivante).

## La décision revue : la bascule sur baccb2ea6e

Relue sans rien rejouer, la répétition sur 1f22743e17 montrait un défaut de 2.12. À la réouverture de la copie
migrée, la garde des comptes a trouvé « plein texte de Scope : 67610 documents pour 75246 lignes » et rebâti l'index
depuis les lignes, en 22 s. Après le repassage, l'écart demeurait : 94 469 documents pour 103 999 lignes. Chaque
réouverture des bases de Lucie aurait donc rebâti leur plein texte.

La répétition sur baccb2ea6e, qui porte déjà lucivy 4.3.12 mais pas 2.12, n'avait rien de tout cela. ED a confirmé dans
le code que la garde comptait toutes les lignes, alors que 2.12 laisse sans document les conteneurs écartés. C'est
aussi le rouge `e2e_tx_par_paquet_arret::en_fichiers_*` de master. Son correctif, ed-fts-count-guard (82837aec86), a
atterri à 6 h 14.

**Décision de c7 à 5 h** : la bascule se fait sur baccb2ea6e avec la lib 6e2476d41, le couple répété vert et sans écart.
Aucun hôte posé ne porte 2.12 avant son correctif.

## La bascule du matin

**L'hôte** : baccb2ea6e, posé à 6 h 42 aux deux postes. La coupure a duré 6 s au portable et 1 s à luciepc.

**code-rag3db-depot (portable) : basculée, critère tenu.**
- 6 h 43, l'instantané `avant-versions-20261120`, puis le contrôle des deux sens : aucun écart.
- 1b en 681 s, puis les générations en 3 s. Rouverte à 6 h 55 : 12 min 22 s de coupure.
- Le repassage, de 6 h 55 à 7 h 11, jusqu'aux vecteurs prêts.
- Contre la base vivante d'avant : recouvrement **98,8 %**, même premier **10/10**, **0 usage perdu** (Catalog
  789 → 1 133). Aucun « rebâti depuis les lignes » ni « écart » à la réouverture.

**code-lourd et code-principal (luciepc) : basculées, puis remises sur leurs instantanés.**
- 1b en 597 s et en 467 s, puis le repassage.
- Le critère est manqué : recouvrement **88,0 %**, même premier **8/10**, et une quarantaine d'usages « perdus ».
- Le retour : à 7 h 38, l'hôte de luciepc est arrêté à froid pendant 18 s, et les deux RETOUR.sh sont VERTS.
- Rouvertes à 7 h 39, les deux bases sont identiques à l'avant (100 %, premier 10/10). Une copie de code-principal
  migrée et repassée est gardée pour le diagnostic.

### Les usages « perdus » étaient un faux rouge

À luciepc, l'hôte refuse l'argument `format`. L'outil de relevé s'est donc replié sur le markdown, où chaque section de
`usages` est plafonnée à 200 lignes affichées, le titre portant le total. Pour code-principal :
- les sections attribuées sont complètes : type 123 → 136, import 21 → 21, other 4 → 4. **Aucun usage attribué d'avant
  ne manque après** ;
- la section « non attribués (nom ambigu) » passe de **466 à 739**, mais n'est affichée qu'à 200 des deux côtés
  (« … 266 de plus », « … 539 de plus »). Les 40 sites « perdus » sont tous dans cette section : ils sont sortis des
  200 affichés, pas de la base. Le total va de 610 à 896.

**Le juge compte donc par section** : les `(N)` des titres en markdown, la liste entière en JSON (le format JSON ne
coupe rien). Jamais les lignes rendues.

### La cause du recouvrement à 88 %

Elle a été départagée sur la copie migrée, rouverte par un hôte isolé, contre la base vivante, sur le même arbre et le
même scope.

- **La ligne** : `experiments/mtga/scripts/build_plant_variant.py:32`, une ligne Python à trois instructions,
  `index=ROOT/'README.md';s=index.read_text();line=f'…'`. Ses scopes `variable` (index, s, line) existent avant comme
  après.
- **Avant**, le morceau du scope `s` est **la ligne entière**. Pour « index du plein texte », il n'est pas dans le
  top 50.
- **Après**, il n'est que **`s=index.read_text()`**. Pour « index du plein texte », il est 22ᵉ, avec le signal vectoriel
  au rang 4 (0,725). Ses quatre jumeaux, dans quatre scripts de `experiments/mtga`, entrent ensemble dans le top 10.
- **Pourquoi** (codeparsers) : 57bfc142bf, du 14 novembre, borne le contenu d'un scope aux octets de son nœud. Ce lot
  est voulu, puisqu'il retirait 844 Mo de doublons. Il n'avait pas monté REGLES_DE_L_ANALYSE. La repasse forcée des
  règles 9 l'a donc appliqué d'un coup à l'arbre de luciepc, du 18 octobre, presque jamais réanalysé depuis. Le dépôt
  du portable était déjà à jour, d'où 98,8 % d'un côté et 88 % de l'autre avec le même binaire.
- **Ce qui n'est pas voulu** : qu'un scope `variable` de 18 caractères concoure à la recherche et l'emporte.
  **La leçon** (dite à codeparsers) : un lot qui change l'analyse monte REGLES_DE_L_ANALYSE dans le même lot.
  **Décision de c7** (9 h 02, définitive) : le remède, variable-not-candidate, revient à ED, et codeparsers relit le
  diff. Sa forme : un scope `variable` dont le contenu propre est sous 80 caractères perd ses morceaux, donc sa
  candidature vectorielle, mais garde son plein texte. Le seuil de 80 est prouvé sur la copie migrée, avec les
  premiers résultats exacts sur des constantes courtes. Le scope reste dans le graphe et dans `usages`.
  Ce lot est l'enfant de package-name-root (règles 12), et les morceaux d'ED suivent en règles 13. Le banc juge le
  remède sur la copie migrée et son hôte isolé, avant l'atterrissage. La re-bascule de code-lourd et code-principal se
  fera à la fenêtre suivante, sur une répétition qui porte le remède.

### Le critère de la re-bascule de code-lourd et code-principal

Le banc a jugé le remède d'ED sur la copie migrée (`docs/banc/20-11-2026/04`, 4ea69dc2d3). Contre la référence « base
vivante des règles 8 », master échoue autant que le lot : 71 % et 74 % de recouvrement, même premier 7/10, et 38 sites
de Catalog perdus des deux côtés. Cette référence ne départage donc plus rien, puisque l'écart vient de l'analyse
d'aujourd'hui, pas de la tête à poser.

**Le critère devient : la tête à poser contre master**, sur la même copie migrée et la même source, avec les mêmes 10
requêtes, le top 10, les usages comptés par section et les seuils du critère de c7. Le juge lit aussi le journal
d'index : ni « rebâti depuis les lignes », ni « garde des comptes », ni « écart entre documents et lignes ».

Le remède d'ED est rouge à ce juge : les noms courts ne sont plus trouvés, et le plein texte gardé ne sert pas
`search_code`. ED corrige, et la re-bascule attend sa tête verte.

## La re-bascule du soir (20 novembre, 20 h 50 - 21 h 33)

Ce soir, code-principal et code-lourd sont passées en versions à luciepc, et le dépôt de la tablette a été repassé.
Il n'y a pas eu de retour arrière. Les heures ci-dessous sont lues à l'horloge, dans les journaux.

### Les préalables

- **(a) Le remède d'ED** (variable-not-candidate, règles 13) a atterri à 19 h 10 (f8c74d0876).
- **(b) L'hôte** a été bâti depuis la tête de master, **83c667e420**, contre la lib 4ad394f4d2 (le moteur n'avait pas
  bougé), avec lucivy 4.3.12. L'essai et check-lib sont verts aux deux postes. Les trois manifestes « refusés » au
  portable étaient des manifestes d'essai effacés, donc des fichiers absents.
- **(c) La répétition** a tourné sur les instantanés du matin de code-principal et code-lourd :
  - 1b en 423 s et 545 s, repassages de 12 et 13 min, RSS au plus 5,7 Gio ;
  - les journaux sont propres.

  Deux faux rouges du banc d'essai :
  - la maintenance quotidienne du plein texte d'un hôte neuf tenait le backend pendant le relevé d'après (422) ;
  - la copie du matin notait VECTOR depuis la lib 6e2476d41, que l'ouverture ne chargeait pas. Une fois corrigée, le
    chiffre était identique.
- **Le critère de recouvrement est remplacé, pas relâché** (c7, 20 h 20). Contre la référence même copie et même
  source (l'instantané repassé à l'ancien format par l'hôte posé baccb2ea6e), le recouvrement est de 71 % et le même
  premier de 7/10, sans aucun usage perdu. C'est le chiffre que le banc a trouvé l'après-midi : entre les règles 9 et
  13, le classement change par construction. Le juge devient :
  - aucun usage perdu ;
  - un journal propre ;
  - des versions rendues ouvertes avec un rang publié ≥ 1, lus par Cypher (`cypher-sur-copie.sh` de codeparsers) :
    code-principal 99/99 ouvertes, code-lourd 98/98, rang publié 1 sur les deux ;
  - le juge du nom exact de banc.
- **Les gabarits de recherche étaient périmés** (banc, 20 h 30). code-principal et code-lourd servaient 9 gabarits
  d'outils sur 14 depuis des copies installées le 11 octobre, dont `search_workspace.mmd` sans `bm25_exact` ni
  `exact_name`. Le juge de banc donnait 5/10. Une fois les chemins remis au livré (`builtin:`) sur les copies, il donne
  **10/10**, avec un recouvrement de 100 % contre les relevés de feuille-5d. À la tablette, le dépôt et l'atelier
  étaient déjà en `builtin:` : leurs copies installées, vieilles elles aussi, ne sont pas lues.

### Les pas

| Heure | Pas | Coupure |
|---|---|---|
| 20 h 49 | 7878 et 7879 sans bail ; à luciepc, aucun backend ouvert | — |
| 20 h 50 min 07 s | luciepc : hôte 83c667e420 posé ; réveil VERT à 20 h 50 min 16 s (l'aide `notes-extensions.sh` manquait à la copie) | aucune vue |
| 20 h 50 min 25 s - 20 h 50 min 54 s | tablette : hôte posé, réveil VERT | 29 s |
| 20 h 51 min 15 s | code-principal et code-lourd fermées ; instantanés `avant-versions-20261120-soir-<base>` ; contrôle des deux sens sans écart | |
| 20 h 59 min 26 s | code-principal rouverte (1b en 464 s ; manifeste au livré, 13 chemins, `templates drift` à 0) | 8 min 11 s |
| 21 h 02 min 11 s | code-lourd rouverte (1b en 613 s ; manifeste au livré) | 10 min 56 s |
| 21 h 02 min 47 s - 21 h 02 min 53 s | dépôt : `Lucivy_Scope` (885 Mo) déplacé dans `instantanes/lucivy-scope-mort-20261120/`, jetable 72 h | 6 s |
| 21 h 03 | `reference.json` de granite-278m posé à luciepc (24 textes, tolérance 5e-4) | — |
| 21 h 11 min 14 s | code-principal repassée en 13 : journal propre ; Catalog 348 → 900 sites, 0 perdu | base ouverte |
| 21 h 13 min 29 s | dépôt repassé en 13 : le geste a joué, les comptes collent (Scope 97 539 = 97 539) | base ouverte |
| 21 h 15 min 29 s | code-lourd repassée en 13 : journal propre ; Catalog 348 → 859, 0 perdu | base ouverte |
| 21 h 2x | juge de banc sur les bases vivantes : code-principal VERT (10/10, 100 %, 13/13, 30 cas) ; code-lourd et dépôt VERTS à un côté | — |

Le recouvrement contre l'avant de la base vivante, relevé mais non jugé, vaut 31 % et 33 % : l'avant servait les
gabarits du 11 octobre. Les gains de Catalog viennent de l'outil `usages` livré.

### Ce qui reste ouvert

- **Le dépôt : 12 373 versions écartées, contre 9 449 nommées.** Le compte du journal est la somme des gestes des deux
  règles des règles 13. Il additionne les conteneurs repliés (environ 9 449) et les déclarations de valeur courtes,
  `variable|constant|type_alias` de moins de 80 caractères, privées de leurs morceaux (environ 2 900). Le libellé de
  la ligne (`index_nodes.rs:637`) ne nomme que les conteneurs : c'est un défaut de libellé, et ED en a ouvert le
  ticket. Une preuve par Cypher sur la copie répétée de code-principal : **0 morceau restant** à ces déclarations
  (1 156 versions visées), et les déclarations de valeur plus longues gardent les leurs (529 morceaux).
- **Le revers de la fusion (A)** (banc) : sur « parse a json manifest », la variable courte `manifest` passe au rang 1
  ou 2 (3 ou 4 sur baccb). C'est un défaut connu, dans le graphe de fusion seul. ED dépose (A′) ; à son atterrissage,
  un hôte neuf sera posé, sans repassage ni coupure longue.
- **Cinq déclarations courtes du dépôt absentes par leur nom** (banc, juge à un côté) : `__all__`, `lib`, `result`,
  `con`, `PORT`. Le plein texte devrait les garder. Banc et ED regardent si le geste l'a retiré aussi, ou si la
  recherche du nom passe par les morceaux.
- **bge-m3 n'a pas de `reference.json`**. Sa vérification au démarrage enverrait 32 textes allant jusqu'à 1 742
  jetons : c'est la forme qui a fait tomber le 7879 à 20 h 35 (ticket d'embarquement). Pas de référence tant que le lot
  de la mémoire libre d'embarquement n'a pas atterri. Les services gardent leur binaire ce soir, et leur `/sante` reste
  « non vérifié » jusqu'à leur repose avec un binaire ≥ e2725d9f6c.
- **Les règles 14** (a44474d874, les morceaux bornés d'ED) viendront au repassage suivant, base ouverte, sans coupure.

**Retour arrière, s'il le faut** : le RETOUR.sh de l'instantané `avant-versions-20261120-soir-<base>`, puis le
manifeste remis depuis `backend.json.avant-builtin-20261120`. Pour le dépôt, remettre `Lucivy_Scope` à sa place,
base fermée.

## La lib du train C++

- **Le bâti** : la lib 501d7ccc90 a été bâtie à 4 h 38 sur la tête du train de cœur, train-20-11. La garde de cœur a
  rebasé ce train au push, si bien que le commit moteur de master est **4ad394f4d2**.
- **La voie transférable** : `git diff 501d7ccc90 4ad394f4d2 -- src test extension/vector tools/rust_api cmake` est
  vide. La lib 4ad394f4d2 est rebâtie à 7 h 53. Les deux `librag3db.so`, de même taille (26 520 776 octets), ne
  diffèrent que de 125 octets : les trois chaînes BUILD_ID (3 × 40) et la note `GNU build-id` de l'éditeur de liens
  (20).
- **Les empreintes** : librag3db.so de 501d7ccc90, md5 e89cfa9d ; de 4ad394f4d2, md5 c918c6b3.
- **Le verdict** sous 501d7ccc90, joué sur les sources baccb2ea6e, se transfère donc à 4ad394f4d2 : e2e_code 36/36,
  e2e_lectures 28/28, e2e_demon_hote 26/26.
- **La pose**, à 8 h 49 et 8 h 50 :
  - l'épingle sur 4ad394f4d2 (pin-lib-4ad394f4d) ;
  - le contrôle des deux sens sous 4ad394f4d2 sur les trois bases : aucun écart ;
  - l'hôte baccb2ea6e lié à 4ad394f4d2 (md5 aec89843), posé et réveillé aux deux postes ;
  - `courant` = 4ad394f4d2.

## Les pièces

Au portable, dans `~/.cache/rag3weaver-build/mesures/` :
- `repetition-versions-code-rag3db-depot-20261119-194739` ;
- `-20261120-010847` ;
- `-20261120-030320`.

Chacun garde la copie, les journaux, `memoire.tsv`, et les relevés `banc-prod.json`, `usages-prod.json`,
`banc-apres.json` et `usages-apres.json`. Les copies sont marquées jetables (famille repetition-versions).
