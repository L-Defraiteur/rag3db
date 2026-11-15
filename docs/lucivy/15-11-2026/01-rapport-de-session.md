# lucivy — rapport de session au 15 novembre 2026

Session « lucivy » : maintenir lucivy (moteur de plein texte publié sur crates.io, npm et PyPI), publier ses
versions au mot de Lucie, et faire les lots lucivy de rag3db à la demande de c7. Ce rapport couvre le 14 et la
nuit du 15 novembre. Le précédent est `docs/lucivy/11-11-2026/01-rapport-de-session.md`.

## 1. Les versions publiées

- **4.3.7**, le 14 novembre vers 14 h (tag `v4.3.7`, main `9a09170`) : une recherche filtrée par `allowed_ids` classe
  comme son sous-ensemble. rag3weaver est passé en 4.3.7 sur master (c99b2f05e).
- **4.3.8**, le 15 novembre vers 1 h (tag `v4.3.8`, main `dd04204`, PR #35 à #38, publication du premier coup, sur
  l'accord explicite de Lucie, donné d'avance). Elle porte :
  - les fusions qui lâchent les ids des mots dont tous les documents sont supprimés ;
  - la compaction qui réécrit un segment seul porteur de supprimés ;
  - la compaction du dictionnaire (`IndexWriter::compact_dictionary`) ;
  - `ShardedHandle::maintain` (purge, compaction, `index_report` avant et après, garde `max_purge_ratio` 0,5) et
    `live_node_ids` ;
  - `index_report` ;
  - le ménage du dossier de cache de lucivy-cpp ;
  - les outils de mesure, en tests ignorés.

  Registres vérifiés : ld-lucivy, lucivy-core, sparse-vector, lucistore et luciole 4.3.8 sur crates.io ; npm
  lucivy et lucivy-wasm 4.3.8 ; PyPI lucivy 4.3.8. CLAUDE.md de lucivy à jour (main c11b973).
- rag3weaver est en 4.3.8 sur master (e95a1b8c6), avec la maintenance par l'hôte (section 2).

## 2. Sur master de rag3db

- **e95a1b8c6** : la maintenance du plein texte par l'hôte.
  - `POST /backend/fts-live-ids` rend les vivants et un jeton, pris sur le même lecteur et relus après.
  - `POST /backend/maintain` purge et compacte. Il tient le slot en écriture puis le catalogue, et refuse si la
    table a changé depuis le relevé, au-delà de `max_purge_ratio`, ou dans une transaction.
  - Le module est `src/fts_maintenance.rs`.
  - L'ordre que l'appelant tient : relevé, puis lecture du graphe, puis maintenance (avec hotfixs).
  - Rouge d'abord : trois témoins e2e sur la base, et un bouchon unitaire. Puis tout vert : e2e_demon_hote 19/19,
    e2e_code 38/38, e2e_lectures 29/29, check_features 8/8, sous la lib 373b5477b avec `RAG3WEAVER_MOTEUR_ANCIEN=1`
    dit. Poussé par la garde en mode pile (`PILE_AVANT=8ef45a3af`, codeparsers), trois `=`.
  - L'hôte en service n'a les routes qu'après un rebâti sur ce master.
- Docs : `docs/lucivy/14-11-2026/02-les-fils-de-lucivy-et-le-profil-leger.md` (fils, profil léger, filtre,
  dictionnaire) et les tickets de la section 4.

## 3. Ce que les mesures ont dit

Le détail et les tableaux sont dans la page 02. L'essentiel :

- **Les fils** : lucivy borne ses pools sur `available_parallelism`, qui suit le quota du cgroup (vérifié : 32 sans
  quota, 5 à 500 %). `RAYON_NUM_THREADS` ne le borne pas. L'hôte `rag3weaver-daemon.service` n'avait pas de quota.
  La variante async est sans objet. Les fils `docstore-compressor`, un par segment en écriture, sont bornés en CPU
  par le quota.
- **Le profil léger** : sans positions, −67 % d'octets, mais `search_code` serait ×5 à ×150 plus lent, et la requête
  de l'archiviste prendrait 63 s. **On garde les positions.** Le dictionnaire partagé ne rapporte que 2 %, mais
  reste le bon défaut.
- **Le filtre `allowed_ids`** coûte 6 à 15 ms par requête dans lucivy. Le ×8 de la base à versions venait de rag3db
  (5 s de `resolve_filter_to_ids`, sonde d'ED). La vue `define_view` est convenue avec ED, mais n'est pas écrite.
- **Le dictionnaire** pèse par le vocabulaire : 5,63 M entrées distinctes, chaque octet d'entrée un suffixe. Il ne
  pèse ni par les segments, ni par les mots longs. Plusieurs générations coûtent environ 3 %.
- **La purge et la compaction** :
  - sur une copie de `Lucivy_Scope` (instantané `avant-835d5298f`), 4,27 → 1,16 Go, dont un dictionnaire de 513,8 à
    400,7 Mo ;
  - **sur la vraie base code-rag3db-depot, par hotfixs, 4,93 → 1,35 Go** (relayé par c7, le 15 novembre).

## 4. Tickets de la session

Tous sous `docs/lucivy/tickets/`, famille produit.

| ticket | état | pour |
|---|---|---|
| `2026-11-13-le-plein-texte-du-depot-pese-cent-fois-ses-sources.md` | ouvert ; les exclusions sont chez recherche, le dictionnaire mort est corrigé en 4.3.8 | recherche |
| `closed/2026-11-13-une-recherche-filtree-ne-classe-pas-comme-sa-vue.md` | corrigé en 4.3.7, fermé le 15 novembre | lucivy |
| `2026-11-14-les-traces-de-run-e2e-font-rougir-la-garde-de-retrait.md` | ouvert ; rencontré encore cette nuit à luciepc et contourné (préserver, retirer, rejouer) | hotfixs |
| `2026-11-14-le-chemin-sans-positions-relit-tout-pour-une-requete-longue.md` | ouvert, pour un jour | lucivy |
| `2026-11-14-un-ulp-d-ecart-de-score-entre-profils-en-parse.md` | ouvert | lucivy |
| `2026-11-14-le-dictionnaire-garde-les-textes-des-documents-effaces.md` | corrigé en 4.3.8 ; se ferme quand `maintain` a tourné sur les vraies bases | lucivy |
| `2026-11-14-le-pre-passage-des-ids-vivants-relit-les-postings.md` | souhait (la réduction (a)), décision de c7 : garder tel quel | lucivy |

## 5. Ce qu'une 4.3.9 attendrait

Rien d'urgent. La règle de Lucie : une version qui n'apporte pas trop peu, sauf un correctif obligatoire.

- L'ulp en `parse` : sommer les clauses du OU dans un ordre fixe, ou en f64, puis rejouer le banc du profil léger.
- La réduction (a) du pré-passage, si les 12 % d'une compaction de maintenance se mettent à compter.
- Peut-être exposer `maintain` et `index_report` dans les bindings (Node.js, Python), si un usage le demande.
- Les pistes notées plus tôt et non commencées :
  - un avertissement pour `indexed: false` sur un champ texte ;
  - un test qui lit `/tmp/s3fs_head.txt` et passe en silence quand le fichier manque ;
  - un cache des bitsets de la recherche filtrée (la vue `define_view`).

## 6. Décisions de la session

- Versions en 4.3.x pour les correctifs, 4.4 seulement pour un changement d'index ou d'API (Lucie).
- Fusion dans main de lucivy dès la CI verte ; un tag seulement quand une version apporte assez, ou pour un
  correctif obligatoire (Lucie).
- Le profil léger des index de code : positions gardées (c7, sur la mesure).
- master de rag3db ne porte que des versions publiées de lucivy : un pointeur par révision reste en branche (c7).
- Le résumé de passe que juge la garde de push s'écrit dans la passe elle-même, jamais à la main.

## 7. Reprendre

**Aucun lot n'est inachevé.** Les worktrees de la session sont retirés par la garde, aux deux postes. Ce qui reste
sur disque, et **rien ne s'efface avant le mot de Lucie** :

Les copies de mesure, à garder tant que recherche et hotfixs s'en servent :
- portable : `~/.cache/rag3weaver-build/mesure-fts/scope-20261114-130846` (4,3 Go, une copie de `Lucivy_Scope` du
  14 novembre, avant la purge) ;
- luciepc : la même copie, avec `documents.tsv` (`~/.cache/rag3weaver-build/mesure-fts/`, 4,3 Go) ;
- luciepc : `mesure-profil/`, `mesure-filtre/`, `anatomie/`, `dico-mort/` et `lucivy-passe-hote/`. Ce ne sont que
  les scripts et les journaux des passes de cette nuit, moins de 5 Mo en tout.

Les worktrees lucivy d'avant, à luciepc, des lots du 13 et du 14 novembre, tous détachés sur des commits fusionnés
dans main, à trier au mot de Lucie :
- `lucivy-check` (7005bdb), `lucivy-close` (c040e0f), `lucivy-cpp-residus` (ab16a82), `lucivy-figement-G`
  (a925d94) ;
- `lucivy-graisse` (30ba023), `lucivy-poids` (e0a318c), `lucivy-score` (736e988), `lucivy-tmp` (aa23bbc).

Ceux qui ne sont pas à moi, laissés tels :
- les `cx-lucivy-*`, à Codex : au portable `cx-lucivy-shutdown-destruction`, et dans rag3db
  `cx-lucivy-plan-shutdown` ; à luciepc `cx-lucivy-mesure-ancien`, `cx-lucivy-mesure-nouveau` et
  `cx-lucivy-shutdown-cpu` ;
- `lucivy-hotfixs-sonde` (branche `purge-orphelins-depot`, db2154d), à hotfixs ;
- `lucivy-figement-G` au portable (`correctifs-rag3db-19-octobre`), à G.

Le worktree `~/git_workspaces/lucivy` des deux postes est sur `main` à ae71a0c (4.3.0), avec des modifications
locales anciennes : c'est l'arbre de travail de Lucie, on n'y touche pas. Le travail se fait dans un worktree neuf :
`git -C ~/git_workspaces/lucivy worktree add -b <branche> ../lucivy-<lot> origin/main`.

Les branches locales anciennes du dépôt lucivy du portable (`4.3.0.1-commit-path` à `4.3.7`,
`backup/v3-recovery-avant-rewrite`…) et de rag3db (`lucivy-4-3-5`, `lucivy-4-3-5-2`, `fts-lucivy-v3`…) sont à trier
au mot de Lucie. Aucune branche distante n'est effacée.

**Les commandes pour rejouer les mesures** (tests ignorés de main lucivy, toujours sur une copie) :
- le rapport d'un index : `LUCIVY_REPORT_INDEX=<copie> cargo test --release -p lucivy-core --test test_index_report report_of_an_index -- --ignored --nocapture` ;
- la purge et la compaction :
  `LUCIVY_MAINTAIN_INDEX=<copie> LUCIVY_MAINTAIN_IDS=<ids> LUCIVY_MAINTAIN_COMPACT=1 cargo test --release -p lucivy-core --test test_maintain maintain_an_index -- --ignored --nocapture` ;
- l'anatomie d'un dictionnaire : `LUCIVY_ANATOMY_INDEX=<index> … --test test_dictionary_anatomy -- --ignored --nocapture` ;
- le profil léger : `LUCIVY_PROFILE_SOURCE`, `_OUT`, `_QUERIES`, `_VARIANTS`, `--test test_light_profile` ;
- le coût du filtre : `LUCIVY_FILTER_INDEX`, `_QUERIES`, `_IDS`, `--test test_filter_cost`.
- Toute passe lourde passe par `~/.cache/rag3weaver-build/poste lourd`, avec `POSTE_MEM_MAX=32G` pour run_e2e.sh.
