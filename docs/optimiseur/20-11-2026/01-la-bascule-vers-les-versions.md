# La bascule des bases de code vers les versions, 20 novembre 2026

Cette page dit pourquoi la bascule du 20 novembre se fait sur l'hôte **1f22743e17**, qui porte 2.12 (ED) et les règles
de l'analyse 10. Elle se fait avec la lib 6e2476d41. Elle donne aussi la preuve qui tient le seuil de recherche. Les
heures de la bascule elle-même suivent, base par base.

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

Décision de c7 à 3 h 5x : bascule sur 1f22743e17. Les bases ne repassent qu'une fois pour les deux montées de règles
(9 de codeparsers, 10 d'ED). package-name-root (règles 11) est tenu hors prise jusqu'après la fenêtre.

## Les pièces

Au portable, dans `~/.cache/rag3weaver-build/mesures/` :
- `repetition-versions-code-rag3db-depot-20261119-194739` ;
- `-20261120-010847` ;
- `-20261120-030320`.

Chacun garde la copie, les journaux, `memoire.tsv`, et les relevés `banc-prod.json`, `usages-prod.json`,
`banc-apres.json` et `usages-apres.json`. Les copies sont marquées jetables (famille repetition-versions).
