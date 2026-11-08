# La lib entière se fige ou rougit par intermittence (réacteur, runtime, relais, tâches de l'hôte)

- **État** : ouvert, **cause trouvée** (8 novembre 2026, 2 h 45, l'arrière-plan : c7, la bissection
  de l'arbre principal) — ce n'est pas un commit, c'est **un service d'embarquement qui tient une
  requête et un délai trop long, ou absent, côté client** ; correctif en cours (le lot du délai, arbre
  principal). Jusqu'à lui, la règle de c7 : la lib entière se joue **sans** `RAG3WEAVER_EMBED_SERVICE`.
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

## La cause (8 novembre, la bissection)

Tout au portable, la lib entière sous le jeu large, lib 17a094d12 + `MOTEUR_ANCIEN=1`, un figement =
aucune ligne de sortie pendant 10 min, binaire vivant (pile prise par `eu-stack` sous
`LD_PRELOAD=ptracer-any.so`, `ptrace_scope` valant 1) :

| commit | `RAG3WEAVER_EMBED_SERVICE` | heure | résultat |
|---|---|---|---|
| c1bdbb183 | les tunnels 7979-7981 | 0 h 41 → 0 h 44 | 3/3 verts, 30 s |
| d845744a2 | les tunnels | 0 h 45 → 1 h 28 | **3/3 figés** |
| 6f48da01f | les tunnels | 1 h 30 → 1 h 46 | **figé** |
| 76f19526b, 682655352, e851d8e9b (+ l'accolade), 4976de23d (+ l'accolade) | les tunnels | 1 h 46 → 2 h 09 | aucun figement (3/3 chacun) |
| 4976de23d + l'accolade, **sans son seul test neuf** (= le code et les tests de 6f48da01f) | les tunnels | 2 h 22 | 3/3 verts |
| c1bdbb183 | **un faux service muet** (127.0.0.1:7999 : accepte, ne répond jamais) | 2 h 25 | **figé** |
| c1bdbb183 | le faux service muet, **`RAG3WEAVER_EMBED_TIMEOUT_SECS=2`** | 2 h 41 | vert, 2 002 tests, 30 s |

- **Pas un commit.** Le même code et les mêmes tests figent à 1 h 30 et passent à 2 h 22 : le figement
  suit l'heure. La fenêtre de 0 h 45 à 1 h 46 est celle où luciepc portait la passe complète d'ED
  (0 h 33 → 1 h 26), les lots de recherche et la passe de hotfixs : le bge-m3 de 7878-7880, derrière
  les tunnels, répondait à `/sante` et laissait `/embed` en file. La même heure a vu les 429 Vertex de
  l'équipier (lecture de c7).
- **Pas le tunnel.** Une sonde des trois tunnels toutes les 60 s (`curl -s -m 3 …/sante`) n'a jamais
  été muette : 15 à 170 ms.
- **Le mécanisme, à la demande, sans heure ni charge** : un service qui accepte et ne rend pas fait
  figer master ; le même avec un délai client de 2 s ne fige plus. Le « figement » est une attente
  **bornée mais longue** : chaque test qui s'attache au service attend le délai entier de son client
  (60 s par défaut, `default_embed_timeout`), et ces attentes s'enchaînent derrière l'arrêt des
  réactions au-delà de toute borne de test.
- **La pile** (les figements de d845744a2 et du service muet : jusqu'à 1 226 fils sur 1 283 en `futex_wait`, charge 0,25 à 1,8) : les tests bloqués
  attendent dans `Backend::stop_reactions` → `ReactorHandle::stop_within` → `dataflow::rt::bloquer`
  (un `block_on` tokio) : **l'arrêt « borné » ne revient pas** pendant qu'un client attend le
  service. Le seul fil dans un client HTTP est celui du test
  `daemon_host::taches::tests::une_tache_ouverte_survit_a_l_arret_de_l_hote_et_se_fusionne`, en `ureq`
  sans délai (`RequestBuilder::send` → `TcpStream::read` → `recv`) vers l'hôte de son test, qui ne
  répond plus.
- **Les clients sans délai** : `DaemonReranker` (`/rerank`) et `DaemonOcr` (`/ocr`), servis par
  bge-m3 (7879), prennent `daemon::agent()` sans délai, à l'attachement (`identite_de`) comme à
  l'appel (`src/daemon/embeddings.rs`, `joindre`) ; l'embarqueur a le sien (60 s, chiffre rond,
  « mesure par service à venir »).

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

- **Un commit** (la bissection ci-dessus), **le tunnel** (la sonde), **un test neuf** (4976de23d sans
  son test).
- **luciepc nu** (8 nov. 0 h 29 → 0 h 39, services locaux 7878-7880) : 4/4 verts, sans les trois
  tests de la mémoire 4/4 verts — les services n'y étaient pas en file à cette heure.

- **La lib 43 seule** : le cas 1 est sous 04063ee06.
- **La charge seule** : le cas 1 est à une charge de 1,24.
- **Une reproduction à la demande** : neuf passes de la lib entière sur master nu à luciepc (22 h 55
  à 23 h 13), en 28 à 66 s, aucun figement ; puis la passe du lot rebasé (f3a115887), verte en 48 s.

## Pour le fermer (c7, 8 novembre)

- **Un délai partout** (le lot de l'arbre principal) : le relecteur et l'OCR prennent le même délai
  que l'embarqueur, par la même variable (`RAG3WEAVER_EMBED_TIMEOUT_SECS`, défaut **60 s** inchangé :
  un seuil ne se touche pas sans mesure) ; l'erreur se nomme (« service d'embarquement <adresse> muet
  après N s ») ; **l'arrêt borné revient dans sa borne** (`Backend::stop_reactions`, `stop_within`)
  même pendant qu'un client attend. Témoins par le faux service muet : rouge aujourd'hui (attente sans
  fin sur `/rerank` ou `/ocr`), vert avec le délai (l'erreur nommée en N s) ; et `stop_within` qui
  revient dans sa borne pendant qu'un client attend.
- **La valeur** : le p99 d'un `/embed` sous charge à luciepc, mesuré dans la fenêtre de G (8 nov.),
  remplace ensuite le défaut rond.
- **Un défaut à part** : un client `ureq` sans délai dans un test qui attend un hôte
  (`une_tache_ouverte_survit_a_l_arret_de_l_hote_et_se_fusionne`) change un rouge en figement ; un
  délai de lecture sur le client des e2e de l'hôte, plus tard.
- **La règle** « lib entière sans `RAG3WEAVER_EMBED_SERVICE` » tombe quand le lot du délai est sur
  master et qu'une passe sous un service muet est rouge en N s, pas figée.

## Pour la suite

- **La pile d'abord.** Au prochain figement, avant de tuer : `eu-stack -p <pid>` (ou
  `gdb -p <pid> -batch -ex "thread apply all bt"`), et le `wchan` de chaque fil. Avec
  `ptrace_scope=1`, lancer le binaire sous `LD_PRELOAD=~/.cache/rag3weaver-build/ptracer-any.so`.
  Le guetteur de codeparsers le fait (luciepc, `~/.cache/codeparsers-notes/lib-figement.sh` : journal
  immobile 240 s avec des « has been running », puis la liste, les piles, le wchan, l'arrêt).
- **Le réacteur** (everything declarative) : un témoin sous charge de
  `a_batch_collapses_a_burst_into_one_run`, puis la cause.
