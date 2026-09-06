# Knowledge dump — lancer, chercher, et où regarder quand

**6 septembre 2026, 11 h 43.** Ce qu'il faut savoir pour reprendre sans
retrouver les mêmes murs.

## Les documents du même genre, avant celui-ci

| | quoi |
|---|---|
| [`5-septembre-2026-16h13/04`](../5-septembre-2026-16h13/04-knowledge-dump.md) | la version du 5 — **corrigée depuis** sur le régime, et complétée d'une mesure de référence |
| [`3-septembre-2026-17h50/04`](../3-septembre-2026-17h50/04-knowledge-dump.md) | version du 3 septembre |
| [`30-aout-2026-07h00/02`](../30-aout-2026-07h00/02-knowledge-dump-lancer-et-chercher.md) | lancer et chercher, fin août |
| [`29-aout-2026-12h24/03`](../29-aout-2026-12h24/03-knowledge-dump.md) | la version qui décrit le mieux le démon |
| [`27-aout-2026-13h01/13`](../27-aout-2026-13h01/13-knowledge-dump.md) | août, avec les pièges GPU |
| [`25-aout-2026-18h58/09`](../25-aout-2026-18h58/09-knowledge-dump.md) | la première de la série |

## 1. Lancer les tests

```sh
cargo test --lib                       # 935 tests, ~0,5 s, aucune base, aucun modèle
cd extension/rag3weaver && ./run_e2e.sh   # la passe complète
./run_e2e.sh --test e2e_search         # une suite
./run_e2e.sh --summary --test e2e_code # avec le tableau par suite
```

`run_e2e.sh` ne joue que les tests marqués `#[ignore]` (`-- --ignored`). Un test
sans `#[ignore]` **ne tournera jamais** dans la passe — c'est le défaut qui a
caché vingt-deux tests jusqu'au 5 septembre.

**Le script ne garde que le dernier `--test`.** Pour plusieurs suites, boucler :

```sh
for t in e2e_search e2e_postgres; do ./run_e2e.sh --test $t; done
```

### La passe complète ne tient pas en une seule tâche de fond

Deux tentatives tuées par le veilleur mémoire dans la nuit du 6, **en `plein`
comme en `confort`** — donc ce n'est pas le régime. Le script confine
délibérément la passe dans un cgroup `MemoryHigh=16G`, pour que ce soit *son*
cache qui soit récupéré plutôt que les pages des applications ouvertes ; la
passe entière dépasse ce plafond, part en récupération (9,6 Go de swap mesurés)
et ralentit jusqu'à se faire tuer.

**Suite par suite, tout passe et va vite.** `RAG3WEAVER_BUILD_MEMORY_HIGH`
change le plafond si on préfère l'autre compromis.

### Le régime — et la bévue à ne pas refaire

```sh
RAG3WEAVER_REGIME=confort   # le DÉFAUT du script
RAG3WEAVER_REGIME=plein     # ⚠ pas pour la passe complète
```

`confort` met les **trois** rôles burn sur la carte la moins chargée — et
depuis le 6 septembre au soir, **c'est tout** quand le poste a deux cartes.
Le rapport cyclique (60 %) et la rafale courte (2 048 caractères contre
8 192) ne s'appliquent plus que si l'embarqueur **partage** la carte du
compositeur, c'est-à-dire sur un poste à une seule carte
(`Regime::carte_partagee`). Lucie demandait « la deuxième carte », pas une
carte bridée ; le bridage sur une carte libre a coûté quatorze minutes pour
trente fichiers dans la suite cloud.

Ce que ce paragraphe disait avant — « la rafale décide du pic mémoire, le
régime est un budget mémoire » — **n'avait pas été mesuré**. Les deux cartes
ont 32 Go chacune, et le cgroup de 16 Go du script borne la mémoire hôte du
build, pas la VRAM. La règle « la passe suite par suite, pas `plein` sur la
passe entière » tient toujours, mais pour une autre raison : plusieurs
binaires de test qui chargent chacun leurs modèles en même temps.

`confort` envoie aussi l'agentique vers un fournisseur distant
(`Origine::Distante`). `RAG3WEAVER_LLM=local` dit une intention **pour cette
passe** et gagne ; `RAG3WEAVER_LOCAL_LLM`, qui traîne dans un profil, ne reprend
pas la carte sous `confort`.

### Cinq suites ne tournent jamais par défaut

`e2e_avis_du_modele`, `e2e_cloud_code_agent`, `e2e_cloud_schema_probe`,
`e2e_conversation_a_plusieurs`, `e2e_lecture_mermaid` sont derrière
`#![cfg(feature = "openai-llm")]` : elles appellent un **vrai modèle** et
dépensent le quota Vertex. Exclusion délibérée. Depuis le 6 septembre, le script
l'**annonce au lancement** et le résumé écrit `AUCUN TEST` au lieu de les
aligner comme des réussites — « ok. 0 passed » n'est pas un succès.

Pour les jouer : `./run_e2e.sh --features openai-llm --test e2e_...`

### Tuer proprement

`pkill` sur `cargo test` **ne tue pas les binaires de test** qu'il a lancés. Il
faut viser `target/debug/deps/e2e_*` aussi. Toujours la lettre entre crochets —
`pkill -f 'nom[x]'` — sinon on tue son propre shell.

### Vérifier sans perturber une passe en cours

```sh
CARGO_TARGET_DIR=/tmp/.../target_verif cargo test --lib -j16
```

Un répertoire cible séparé compile et teste sans écraser les binaires que la
passe exécute. Utilisé toute la nuit du 6.

## 2. Le PostgreSQL de test

```sh
docker start rag3weaver-pg     # il s'arrête au redémarrage de la machine
```

`run_e2e.sh` sonde le port 5433 avec le `/dev/tcp` de bash — **`nc` n'est pas
installé sur ce poste**. Si la base répond, la feature `postgres` entre ; sinon
l'écart est annoncé au lancement et dans le résumé.

Deux pièges de schéma : les tables non qualifiées vont dans le premier schéma du
`search_path`, qui commence par `"$user"` — avec un rôle nommé `rag3weaver`,
tout atterrit dans `rag3weaver` et **rien dans `public`**. Même chose pour les
extensions sans `SCHEMA` explicite.

## 3. La machine

93 Gio de RAM, 24 cœurs, deux cartes. **Jamais `-j$(nproc)`** : c'est la
compilation qui fige le poste. `-j16` à `-j20` laisse de quoi travailler.

Le modèle local est `Huihui-Qwen3-Coder-30B-A3B-Instruct-abliterated.i1-Q6_K.gguf`
(24 Go) dans `~/ML/models/Qwen3-Coder-30B-abliterated-Q6_K/`, lancé par
`~/ML/start-qwen-coder-server.sh`. **`--jinja` est obligatoire** : sans lui, ce
modèle rend ses appels d'outils en XML brut, ce qui se manifeste par des appels
qui n'arrivent jamais plutôt que par une erreur.

Les identifiants Vertex et Hugging Face sont dans `.vault`
(`vertex-sa.json`, projet `lr-hub-472010`). Une suite cloud « 0 passed » est un
saut à corriger, jamais une ligne verte.

## 4. La mesure de référence d'une ingestion

Prise le 6 septembre à 03 h 22, régime `confort`, sur notre propre source :

```
95 fichiers · 4 438 scopes · 46 698 relations · 9 226 symboles · 0 perdu
ingestion : 1 911 366 ms  (31 min 51 s)
  entities_ms  1 895 158   (99,1 %)
  symbols_ms      12 287   ( 0,6 %)
  relations_ms      3 920   ( 0,2 %)
GPU card0 : moyenne 31 %, p90 100 %, 3 024 rafales
pic RAM 25 % · pic swap 9,6 Go
```

**Le partage compte plus que le total** : tout le temps est dans le découpage et
l'embarquement. Optimiser ailleurs vise 0,8 % du problème. À rejouer par
`./run_e2e.sh --test e2e_charge_ingestion` (~32 min).

## 5. Où regarder, et à quel moment

| la question | l'endroit |
|---|---|
| « qu'est-ce qui doit être prêt » | `src/disponibilite.rs` — lire la **table d'approximation** en tête avant de raisonner |
| « qui décide d'attendre quoi » | `SearchOptions::ce_qui_doit_etre_pret`, puis `Catalog::appliquer_la_consigne` — **un seul arbitre chacun** |
| « pourquoi ma recherche rend zéro » | `Catalog::expliquer_le_silence_d_un_signal`, et les avertissements de `SearchMeta` |
| « qu'est-ce qui reste à embarquer » | `count_marqueur_manquant` / `select_chunks_sans_marqueur`, et `Catalog::embarquer_le_retard` |
| « pourquoi ce chunk n'est pas réembarqué » | les **deux** marqueurs, `_embed_hash` et `_sparse_hash` — et le filtre par signal dans `KBEmbedNode`/`EmbedNode` |
| « pourquoi rien n'est indexé » | `open_fts_handles_for` et ses **quatre** appelants ; `tables_sans_index_plein_texte` crie quand il en manque un |
| « ce que le drain a vraiment fait » | `FlushResult` — `processed`, `unchanged`, `warnings`, `rendu_pret` |
| « comment un backend s'ajoute » | `SchemaDialect` : ce qu'il sait offrir, jamais son nom |
| « pourquoi cet avertissement n'arrive pas » | les trois canaux, dans [`02-l-architecture-actuelle`](02-l-architecture-actuelle.md) §7 |
| « le graphe de recherche des agents » | `templates/tools/search.mmd` → `search_base.mmd` → `SearchSourceNode` |
| « le graphe d'ingestion » | `Catalog::build_ingestion_graph`, son commentaire ASCII en tête |

## 6. Les pièges qui ont coûté une demi-journée chacun

- **Une suite verte peut ne rien jouer.** `#[ignore]` manquant, ou
  `#![cfg(feature)]` non activée. Le résumé le dit maintenant.
- **Une suite verte peut couvrir tous les chemins sauf celui des gens.** Tous
  les e2e de recherche passaient par `ingest_entities` + `Immediate` ; la
  combinaison du produit — `create()` + `Eventual` — n'était testée nulle part,
  et c'est là qu'était le pire défaut de la semaine.
- **Un commentaire n'est pas un contrat.** `open_fts_handles_for` décrivait son
  propre défaut depuis des semaines ; il s'est produit exactement là.
- **Un `let _ =` dans un chemin normal cache le seul cas qui compte.** Trois
  fois dans `KBUpdateNode`, trois fois sur `create_vector_index` — dont le DDL
  est idempotent, donc l'erreur avalée était forcément la vraie.
- **Rendre de la donnée périmée est pire que ne rien rendre** : rien ne signale
  qu'il faut douter.
- **Le marqueur avant l'écriture est un trou permanent.** Rien ne le reprendra,
  puisqu'il dit que c'est fait.
- **Un embedder tronque en silence.** Le MiniLM multilingue à **128 jetons**,
  quand la taille de chunk vaut 1 500 caractères. Compté et dit depuis le
  6 septembre (`Embedder::troncatures`), pas évité.
- **Deux façons de dire la même chose divergent** dès que deux endroits les
  lisent. D'où : `Consistency` traduit vers `Disponibilites` et ne porte pas
  d'état.

## 7. Git

Messages de commit **en français**, et **jamais de trailer d'attribution IA** —
c'est une règle de ce dépôt, elle prime sur le défaut de l'outil.

Les docs de session vont dans `extension/rag3weaver/docs/<jour>-<mois>-<année>-<HhMM>/`
pour le crate Rust ; `docs/` à la racine pour le fork kuzu et ses extensions C++.

## 8. Les autres sessions

`rag3db-57` travaille sur le cœur C++ (Vela, MVCC). Un message qui parle de
**Sairen** ici est un collage égaré : le signaler, ne pas enquêter.

## 9. Après la passe fondations (soirée du 6) — où regarder de plus

Ajouté à la table de la §5, sans la réécrire :

| la question | l'endroit |
|---|---|
| « la recherche du produit » | `Catalog::rechercher(&Arc<Mutex<Catalog>>, …)` — le gabarit `search_base` lancé sur le catalogue ; `Catalog::search` est le monolithe, ses appelants sont des tests |
| « pourquoi ce drain n'a pas tout pris » | `Catalog::fermeture(graine, pour_ecrire)` — un drain n'emporte que ce qui est en lien ; `PendingWork::extraire_les_tables` |
| « qu'est-ce qu'un `create` promet » | `RegimeEcriture` (`au tick` par défaut), `create_jusqu_a(…, exige)`, `tenir_l_exigence_d_ecriture` |
| « qu'est-ce que l'autre processus doit encore » | la marque `_ingestion/pending/<écrivain>` : `horodatage\|Table:data,textsearch,…` ; `lire_une_marque`, `marque_nous_concerne` |
| « pourquoi `failed` n'est plus zéro » | le service `echecs` (`EchecDeGroupe`), `consigner_l_echec` dans les nœuds, `FlushResult::absorber_les_echecs` |
| « lire une base qu'un autre tient » | `Catalog::ouvrir_en_lecture` + `initialiser_en_lecture` ; `CatalogError::LectureSeule` |
| « deux rattrapages sur la même dette » | `_embed_claim`, `reclamer_le_retard`, `SchemaDialect::reclamer_chunks_sans_marqueur` |
| « une mise à jour sans redécoupage » | `_chunked_hash`, `MarquerDecoupeNode`, `rattraper_le_decoupage`, `build_ingestion_graph(…, avec_decoupage)` |
| « le schéma d'une base » | `scope::SCHEMA_VERSION` = 5 ; les migrations v3 → v5 dans `migrate_scope_columns` ; un lecteur refuse une base en retard |
| « les options traversent du JSON » | `FilterValue` est `untagged` avec `Ops` en premier — l'ordre des variantes est celui de la lecture |

Deux pièges neufs :

- **`MockEmbedder` rend des vecteurs nuls.** Deux appels ne se comparent pas
  sur le vecteur ; prendre `HashEmbedder` dès qu'un test compare des ordres.
- **Un port consommé n'est plus lisible** dans la sortie d'un graphe. Le
  lanceur retire `render` pour relire `resolve.results` et les métas.
