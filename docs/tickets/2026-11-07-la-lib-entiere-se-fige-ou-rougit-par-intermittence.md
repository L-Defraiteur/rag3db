# La lib entière se fige ou rougit par intermittence (réacteur, runtime, relais, tâches de l'hôte)

- **État** : ouvert (7 novembre 2026) — trois figements dans la soirée, un rouge sur huit répétitions,
  aucune pile encore ; le réacteur part à everything declarative (c7, 8 novembre au matin).
- **Gravité** : passe bloquée (un push attend une heure un binaire figé) ; aucune réponse fausse connue.
- **Atteignable en service** : inconnu — les tests touchés sont ceux de l'hôte et du réacteur ; rien
  ne montre encore que le produit fige.
- **Touche rag3weaver** : oui (`src/dataflow/reactor.rs`, `src/dataflow/runtime.rs`,
  `src/daemon_host/`, `src/ide/`, `src/backend/`).
- **Famille** : produit

## Ce que c'est

`cargo test --lib` sous le jeu large (`rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,
openai-llm,anthropic-llm`) passe d'ordinaire en 28 à 66 s. Par intermittence, il ne finit plus :
une poignée de tests restent « has been running for over 60 seconds » et le binaire tient des
heures. Une fois sur huit, sans figer, un test du réacteur rougit.

## Les cas

1. **archi, portable, 19 h 58** : base master 75d8d6d1e, lib **04063ee06** (l'ancienne) avec
   `RAG3WEAVER_MOTEUR_ANCIEN=1`, `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,7980,7981` (les services
   réels, réveillés à 19 h 57), `LUCIVY_SCHEDULER_THREADS=8`, charge 1,24 à 20 h 41. Coupé à
   20 h 42 après environ 43 min ; 1 229 fils (beaucoup `backend::tests`), 1 967 tests passés.
   Arrêtés : `daemon_host::taches::tests::une_tache_ouverte_survit_a_l_arret_de_l_hote_et_se_fusionne`,
   `daemon_host::taches::tests::la_base_principale_n_est_jamais_ecrite_par_la_tache`,
   `daemon_host::tests::deux_fenetres_l_une_enregistre_l_autre_voit_le_changement`,
   `dataflow::reactor::tests::a_frozen_reaction_does_not_hold_a_bounded_stop`,
   `ide::tests::the_relay_does_not_wait_for_the_graph_runtime`, et rouge
   `backend::tests::stopping_cuts_a_long_reaction_and_the_restart_resumes_it`. Pile refusée
   (eu-stack sans ptrace). Rejoué à 20 h 42 sur 1b7fb111e sous 17a094d12 : vert en 38 s.
2. **l'arbre principal, même soirée** : relayé par c7, liste presque la même ; pas de pile.
3. **codeparsers, luciepc, vers 21 h 50** : la passe du lot garde-pointeur-3 (dae922e9f), lib
   **17a094d12** avec `MOTEUR_ANCIEN=1`, à côté d'autres lourds. Binaire tué après 62 min, 6,5 Gio de
   RSS, journal immobile depuis 21 h 51. Arrêtés, les huit : les cinq d'archi plus
   `backend::tests::stopping_cuts_a_long_reaction_and_the_restart_resumes_it`,
   `dataflow::runtime::tests::un_niveau_mixte_sequentialise_et_ca_se_voit`,
   `dataflow::runtime::tests::un_noeud_asynchrone_traverse_le_pont_synchrone`. Pas de pile.

Le **rouge** : à la 8e de huit répétitions sur master 76ecc990e nu (luciepc, 23 h 13, lib 17a094d12,
`MOTEUR_ANCIEN=1`, un second lourd à côté),
`dataflow::reactor::tests::a_batch_collapses_a_burst_into_one_run` panique à
`src/dataflow/reactor.rs:587` : « un lot, une exécution », left `[2]`, right `[5]`.

## Ce qui est écarté

- **La lib 43 seule** : le cas 1 est sous 04063ee06.
- **La charge seule** : le cas 1 est à une charge de 1,24.
- **Une reproduction à la demande** : neuf passes de la lib entière sur master nu à luciepc (22 h 55
  à 23 h 13), en 28 à 66 s, aucun figement ; puis la passe du lot rebasé (f3a115887), verte en 48 s.

## Les pistes, non prouvées

- Les tests communs aux listes sont ceux qui tiennent des fils, des canaux et des **arrêts bornés** :
  le réacteur, le runtime, le relais de l'IDE, les tâches de l'hôte. Un test qui dépend du temps sous
  charge, ou une vraie course dans l'arrêt.
- Un test qui attend un **vrai service** d'embarquement sans délai à lui (le cas 1 exportait les
  services réels) : le ticket d'hygiène d'archi,
  `2026-11-07-un-test-de-la-lib-qui-parle-au-service-d-embarquement-attend-sans-delai.md` (branche
  `hooks-pont-7`), auquel celui-ci se rattache.

## Pour la suite

- **La pile d'abord.** Au prochain figement, avant de tuer : `eu-stack -p <pid>` (ou
  `gdb -p <pid> -batch -ex "thread apply all bt"`), et le `wchan` de chaque fil. Avec
  `ptrace_scope=1`, lancer le binaire sous `LD_PRELOAD=~/.cache/rag3weaver-build/ptracer-any.so`.
  Le guetteur de codeparsers le fait (luciepc, `~/.cache/codeparsers-notes/lib-figement.sh` : journal
  immobile 240 s avec des « has been running », puis la liste, les piles, le wchan, l'arrêt).
- **Le réacteur** (everything declarative) : un témoin sous charge de
  `a_batch_collapses_a_burst_into_one_run`, puis la cause.
