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
2. **l'arbre principal, portable (pas luciepc), 21 h 26 → 21 h 33** : la passe du lot juge + graphe
   cité, tête d5f617bbf sur base dcaea137f, lib **17a094d12** avec `MOTEUR_ANCIEN=1` (sources
   7cd11e5fa), jeu large `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm`,
   target rebâti à neuf juste avant, seule sous `poste lourd` (une place) ; à côté, une copie `scp`
   de lib depuis luciepc (21 h 33). Rouges : `backend::tests::a_failed_batch_is_undone_without_poisoning_the_catalog`
   et `backend::tests::stopping_cuts_a_long_reaction_and_the_restart_resumes_it` ; « running for over
   60 seconds » : `daemon_host::taches::tests::la_base_principale_n_est_jamais_ecrite_par_la_tache`,
   `daemon_host::taches::tests::une_tache_ouverte_survit_a_l_arret_de_l_hote_et_se_fusionne`,
   `daemon_host::tests::deux_fenetres_l_une_enregistre_l_autre_voit_le_changement`,
   `dataflow::reactor::tests::a_frozen_reaction_does_not_hold_a_bounded_stop`,
   `dataflow::runtime::tests::un_niveau_mixte_sequentialise_et_ca_se_voit`,
   `ide::tests::the_relay_does_not_wait_for_the_graph_runtime`. Puis le journal s'immobilise à 21 h 33 m 53 s,
   **sans ligne `test result` ni panique** : le binaire n'était pas mort, il était figé, et la borne de
   55 min de l'étape l'a coupé (le navigateur n'est parti qu'à 22 h 24). Le journal du
   noyau de 21 h 30 à 21 h 36 : ni manque de mémoire, ni signal du noyau, ni vidage du binaire
   (`journalctl -k`, `coredumpctl` ; seul vidage, `plasma-discover`, sans lien). Charge non relevée.
   Rejoués seuls, les deux rouges sont verts sous 17a094d12 comme sous 7cd11e5fa ; la lib entière
   rejouée seule au même code et sous la même lib : 1 988 verts, 26 s. Pas de pile.
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

4. **l'arbre principal, portable, 8 nov. 23 h 18 → 23 h 59** (le rejeu court du même lot, tête
   74c23b87d rebasée sur d845744a2, lib 17a094d12, `MOTEUR_ANCIEN=1`, `RAG3WEAVER_EMBED_SERVICE` vers
   les services réels) : la même figure — les deux mêmes rouges, les mêmes tests à plus de 60 s, le
   journal immobile 40 min. **Le binaire vivant** : 1 257 fils, dont 1 202 en `futex_wait`
   (35 `inet_csk_accept`, 11 `hrtimer_nanosleep`, 7 `do_epoll_wait`), charge 1,8, 5,8 Gio de RSS :
   une attente, pas un calcul. Pas de pile (`ptrace_scope` à 1, le binaire n'était pas lancé sous
   le préchargement qui l'autorise).

## La piste : `RAG3WEAVER_EMBED_SERVICE`

Au même code (74c23b87d sur d845744a2) et sous la même lib, sur ce poste, seule sous `poste lourd` :

| la lib entière | résultat |
|---|---|
| **sans** `RAG3WEAVER_EMBED_SERVICE` (7 nov. 22 h 50) | verte, 1 988 tests, 26 s |
| **avec** `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,7980,7981` (8 nov. 23 h 59) | figée à l'identique (les deux rouges, les mêmes tests à plus de 60 s), arrêtée à 10 min |

Le cas 1 (archi) avait les services réels ; la passe de l'arbre principal du 7 nov. à 20 h 10, sur
207d334f9, était verte avec eux : le figement est apparu depuis. **Règle de c7 jusqu'au correctif :
la lib entière se joue sans `RAG3WEAVER_EMBED_SERVICE`** ; les e2e gardent les services.

**La bissection la plus courte.** Les tests de la lib ajoutés de 207d334f9 à d845744a2
(19 fonctions `#[test]` neuves) ; ceux qui ouvrent le gabarit memory par un tour de chat, et
donc prennent l'adresse d'embarquement dans la variable, sont les trois en gras :

- `a_teammate_resumes_after_the_host_restarts`
- **`la_barre_du_chat_parle_en_depots_et_chacun_a_sa_jauge`**
- `la_branche_d_un_depot_et_d_un_worktree_se_lit_dans_head`
- `la_nature_d_un_backend_vient_de_son_gabarit_sinon_de_son_espace_de_travail`
- `la_page_du_chat_tient_l_ordre_le_defilement_et_la_place`
- `l_appelant_vit_le_temps_de_l_appel_et_se_remet`
- `le_juge_se_journalise_se_plafonne_et_se_coupe`
- `le_premier_jeton_d_un_tour_arrive_dans_la_page`
- `le_role_d_un_outil_se_lit_dans_describe_et_dans_son_run`
- **`l_etat_du_chat_rend_les_fiches_ecrites_pendant_la_conversation`**
- `l_heure_iso_porte_les_millisecondes_et_le_decalage`
- `relays_publishing_together_lose_nothing_and_keep_the_order`
- **`sans_outil_de_lecture_la_memoire_dit_pourquoi_la_liste_est_vide`**
- `une_ligne_sans_fin_est_coupee_a_sa_borne`
- `une_reaction_a_role_l_annonce_dans_son_run`
- `une_sortie_pleine_ne_fige_pas_l_ecrivain`
- `une_tache_hors_depot_garde_les_chemins_relatifs_de_ses_gabarits`
- `une_tache_s_ouvre_quand_seul_le_code_est_dans_un_depot`
- `un_tour_a_deux_outils_garde_l_ordre_et_l_heure_de_ses_appels`

Les tests qui figent sont, eux, des tests plus anciens (`backend::tests`, `daemon_host::taches`,
`reactor`, `ide`) : l'hypothèse, non prouvée, est qu'un test neuf parle au service réel sans délai et
retient une ressource commune (un fil du planificateur, un verrou de l'hôte d'embarquement) que les
autres attendent. Le pas suivant : la lib entière **avec** la variable en sautant ces trois tests
(`-- --skip …`) ; verte, le coupable est parmi eux ; figée, il est ailleurs, et l'on bissecte le
reste de la liste. Correctif attendu (ED) : un test déterministe ou un délai borné, et « service
absent » dit en clair.

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
