# Session recherche, 8 novembre 2026 : rapport

Ce que la session recherche a mis sur master le 8 novembre, ce qu'elle laisse en file, et ce qui
attend. Les décisions viennent de c7 (rag3db-74), qui tient la file, et de Lucie par lui.

## Sur master

| lot | hash | ce que ça change |
|---|---|---|
| dense filtré sous L6 | 0e528854a (témoin 62cc737a6) | une recherche filtrée (scope, langage, filtre) lit sous l'instantané de son appel : `SearchSourceNodeFactory::snapshot_safe` rend `true`, la lib dadbf0041 permettant la projection en lecture seule |
| best-signal | d540813d5 (témoin 001f3ec10) | `chunk.signal` dit l'échelle de `chunk.score` ; le morceau d'une ligne fusionnée se choisit par la contribution au rang, poids / (k + rang), plus par le score brut ; `bestSignal {label, score, rank}` par ligne ; `Provenance` le garde à travers ResolveParent et le reranker |
| mes recherches | 932197e79 (témoins 1f2600741, aad54dc9e) | `GET /ide/recherches`, `POST /ide/recherches/clic`, la vue `Recherches.tsx` ; le journal des runs écrit sous un verrou (`ecritures`), ce qui corrige une course existante, et une réécriture garde l'âge du fichier |
| mes recherches, suite | 022ade8e8 (témoin 705b2c8f9) | relecture d'everything declarative : 2 000 runs lus au plus par page, `relancable` dit par l'hôte, outils de recherche lus sans ouvrir la base |
| nouveautés | ad07c8b16 | deux lignes : mes recherches (et la course du journal), le score qui dit d'où il vient |

## En file

- **residus-de-suite** (08286f1e7 + a24541f3b, docs) :
  - le cache d'index de `Catalog` quitte /tmp : `index_cache_dir`, `RAG3WEAVER_CACHE_DIR`, sinon le cache XDG ; les dossiers `<pid>` des morts sont balayés à l'ouverture ;
  - run_e2e.sh rend chaque suite responsable de son TMPDIR, avec la liste `tests/e2e/suites_qui_laissent_des_residus.txt` : cinq suites « toujours », trois intermittentes en rouge connu du témoin, liées à leur ticket.
  - Mesure de départ : 70 suites laissaient des résidus, dont 64 le seul cache.
  - La passe ordinaire est verte sous cbc06c3e1. L'e2e entier a été arrêté à 94 suites, à la demande de c7 : bge-m3 saturé, carte de luciepc. Il reste à rejouer `e2e_sparse_dump` et le reste sous 9bae60786, puis à pousser au « à toi ».
- **arret-du-backend-3** (57f9ed822, sur 8be79cbbf) :
  - un seul registre d'arrêt pour les fils d'un catalogue (`catalog_threads`, qui tient un `Weak`), où s'inscrivent les indexations et le rebâti FTS ; ce dernier s'arrête au lot suivant ;
  - `impl Drop for Backend` égale `shutdown()`, une seule fois ;
  - `Drop for Catalog` vide la vue de lecture avant de fermer les FTS.
  - Deux témoins, rouges sur master pour la bonne raison (un détenteur tient le catalogue après le Drop), et verts : un backend de code, et une mémoire avec ses réactions.
  - Ensuite : rebase sur residus-de-suite, retrait de e2e_cited_by, e2e_juge et e2e_lectures de la liste, deux e2e entiers propres.

## Tickets ouverts par la session

- `2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md` : un backend tombé laisse
  des fils écrire dans son dossier (TempDir à moitié effacés), et une fermeture tardive du plein
  texte fait planter la sortie (SIGSEGV d'e2e_cited_by chez 96). Correctif : arret-du-backend.
- `2026-11-08-le-temoin-de-l-embarquement-hors-verrou-rougit-sous-charge.md` : rouge 5/10 sur
  master seul, sous attente disque à luciepc ; à rejouer sur disque calme, puis à départager. Il
  vient après arret-du-backend.
- Le ticket du score non comparable (archi) reste ouvert pour archi, qui a retiré
  `recall_min_score` dans son lot (f).

## Ce qui attend

- **Demain, la priorité de Lucie : les vues par worktree.** La page de conception de la marche 1
  vient de l'arbre principal. Ma part : le filtre de vue dans le plein texte (versions de
  documents, ou champ indexé, à trancher par mesure), le dense filtré par vue, le drain par
  empreinte côté FTS. Rien à coder avant la page relue par Lucie.
- Côté lucivy (c7 prévient la session) : `ShardedHandle::close` sort sur erreur avant d'arrêter ses
  pools.

## Incidents et leçons

- **Un timeout de lourd tue le haut de l'arbre, pas le reste.** `poste lourd timeout 14000` a
  expiré, et `run_e2e.sh` et son binaire, devenus orphelins, ont tenu bge-m3. Ils ont été
  identifiés par l'exécutable du target et le dossier de travail, puis arrêtés par l'arbre de leurs
  pids (fiche mémoire `un-timeout-tue-le-haut-pas-l-arbre`).
- **Un départage qui ne bascule pas joue la branche des deux côtés.** La bascule vers master
  échouait sur un fichier modifié, et le côté « master » jouait la branche. Désormais, le script
  vérifie `HEAD` avant de jouer.
- **Un embarqueur factice est refusé à l'ouverture** : l'identité du modèle ne correspond pas. Sans
  service, un témoin ouvre avec l'embarqueur absent (`None`).
- **À luciepc, une commande ssh tourne sous fish** : passer par `bash -s` ou `bash -c`.
- **Le 8 au soir, la charge de luciepc était une attente disque** (NVMe en TRIM asynchrone, PSI des
  E/S à 22-30), pas des cœurs pleins. Les chiffres pris sous cette charge sont à refaire.

## Branches distantes laissées à Lucie

dense-filtre-sous-l6, best-signal, best-signal-2, mes-recherches, mes-recherches-2,
mes-recherches-suite, mes-recherches-suite-2, nouveautes-recherches, et les branches en file
(residus-de-suite, arret-du-backend, -2, -3).
