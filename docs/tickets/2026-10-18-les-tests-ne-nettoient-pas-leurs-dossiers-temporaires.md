# Les tests lancés hors de run_e2e.sh laissent leurs bases dans /tmp, qui est de la mémoire vive

- **État** : en cours (8 novembre 2026) — la garde `BaseTemporaire` et dix témoins passés par elle (lot de l'équipier Gemini, fini par la session hotfixs) ; restent le témoin de suite, le cache par défaut de `Catalog` (en service aussi), sept témoins d'`e2e_prise_atomique` et deux journaux à nom fixe (ci-dessous). Ouvert le 18 octobre 2026, seconde session cœur C++, à la demande de l'orchestration.
- **Gravité** : confort, mais `/tmp` est un tmpfs sur le poste principal : ce qui y reste occupe
  la RAM, puis l'échange.
- **Atteignable en service** : non (tests seulement), à une exception près : `Catalog` écrit son
  cache par défaut dans `/tmp/rag3weaver_cache`, en service aussi.
- **Touche rag3weaver** : oui (ses e2e, ses tests et `Catalog`).
- **Famille** : produit

## Ce que c'est

Le 11 octobre, la garde de `run_e2e.sh` a donné à chaque suite e2e son propre `TMPDIR`, effacé
derrière elle (`extension/rag3weaver/docs/11-octobre-2026-recherche/03-les-e2e-en-un-binaire.md`,
§4). Le même jour, les tests gtest du moteur ont reçu leur parent par passe (ticket fermé
`closed/2026-10-11-les-tests-gtest-du-moteur-laissent-leurs-bases-dans-tmp.md`).

Ces gardes ne couvrent qu'un test lancé par `run_e2e.sh`. Une suite jouée à la main
(`cargo test --test e2e`, une boucle de rejeu, un script de session) écrit à la racine de `/tmp`,
et ses `remove_dir_all` échouent comme avant le 11. Le 15 octobre, l'orchestration a effacé
**755 dossiers morts** de `/tmp` (familles `rag3weaver-bulk-*`, `rag3weaver_cache`,
`/tmp/rag3db`). L'échange du poste est passé de 22 à 10 Go, avec l'arrêt de l'hôte : voir
[le ticket de la mémoire de l'hôte](2026-10-18-l-hote-ne-rend-pas-la-memoire-d-un-gros-index.md).

## Ce qui reste le 18 octobre (poste principal, relevé en lecture seule)

4 124 entrées à la racine de `/tmp`. Les familles de rag3weaver :

| famille | entrées | dates | poids | qui l'écrit |
|---|---|---|---|---|
| `rag3weaver-acces-<pid>-<ns>` (+ `.wal`, `.readers`) | 2 332, soit 804 bases | 11 oct. 13 h 42 → 12 oct. 14 h 45 | 6,3 Mo | `e2e_rag3daemon::un_lecteur_choisit_son_chemin_et_le_dit` |
| `rag3weaver-rag3daemon-<pid>-<ns>` (+ `.wal`) | 1 610 | même plage | 13 Mo | `e2e_rag3daemon` |
| `rag3weaver-bulk-<pid>` | 91 | 11 oct. 10 h 54 → 12 oct. 16 h 28 | 122 Mo | `e2e_code` (`tests/e2e_code.rs:738`) |
| `rag3weaver-lignes-*` | 14 | 11 → 12 oct. | 0,2 Mo | (non cherché) |
| `rag3weaver_cache` | 1 dossier | — | 16 Mo | `Catalog` (`src/catalog.rs:643`, `cache_base` par défaut) |

`/tmp/rag3db` est vide aujourd'hui : le parent par passe du moteur tient.

Les restes de `e2e_rag3daemon` viennent d'environ 800 processus distincts (deux bases par pid),
tous **après** la garde de `run_e2e.sh` (11 octobre, 12 h 27) : ils ont donc été lancés sans elle.
On ne sait pas encore qui les a lancés (une boucle de rejeu, sans doute). Ces restes pèsent peu.
Le volume vient des bases d'index des suites `e2e_code`, et de 34 fichiers de tests ou de `src/`
qui écrivent sous `std::env::temp_dir()`.

**Effacés le 18 octobre** (seconde session cœur C++, à la demande de l'orchestration) :

- **Ce qui est parti** : les trois familles des 11 et 12 octobre, soit 4 033 entrées (2 332
  `rag3weaver-acces-*`, 1 610 `rag3weaver-rag3daemon-*`, 91 `rag3weaver-bulk-*`). Toutes
  appartenaient à l'uid 1000.
- **Comment** : d'abord la preuve que rien ne les tenait (rien dans `/proc/*/fd`, `cwd` et `maps`,
  ni par `fuser`, ni par `lsof`). Puis l'effacement depuis une liste écrite dans un fichier,
  jamais par un motif.
- **Leur poids** : 141 Mio au `du` de la liste, 144 Mio rendus au `df` de `/tmp`. C'est faible,
  ce qui **écarte cette piste pour l'échange** : les 10 Go relevés le 15 ne venaient pas de ces
  restes (voir le ticket de la mémoire de l'hôte).
- **Ce qui reste** : `rag3weaver-lignes-*` et `rag3weaver_cache`, hors du périmètre fixé, ainsi
  que les dossiers des sessions. `/tmp` compte maintenant 92 entrées.

## La cause

Elle est la même qu'au 11 octobre :

- `let _ = std::fs::remove_dir_all(&dossier)` est appelé sur une **base**, qui est un fichier
  (`e2e_rag3daemon.rs:181`, `:294`, `e2e_code.rs:739`). L'appel échoue, `let _` avale l'erreur,
  et la base reste, avec son `.wal` et son `.readers` à côté.
- Rien n'efface `/tmp/rag3weaver_cache` (le `cache_base` par défaut de `Catalog`).

La garde de `run_e2e.sh` masque ces défauts sans les corriger.

## Témoin

Aucun pour l'instant. Il faudrait :

1. jouer `e2e_rag3daemon` et `e2e_code` **sans** `run_e2e.sh`, avec un `TMPDIR` neuf ;
2. vérifier qu'il n'y reste rien.

## Pour le fermer

1. Remplacer le `remove_dir_all` sur une base par l'effacement de la base et de ses voisins
   (`.wal`, `.readers`), ou créer chaque base dans un dossier à elle et effacer ce dossier. Le
   faire partout où un test crée une base sous `temp_dir()`.
2. Faire que l'effacement ait lieu même quand le test échoue, par une garde à `Drop`, sauf avec
   `RAG3WEAVER_GARDER=1`.
3. Donner à `Catalog` un `cache_base` de test dérivé de `TMPDIR`, ou l'effacer à la fermeture
   quand il est temporaire.
4. Une fois rien de vivant dessus (vérifié par `/proc/*/cwd`, `fd` et `maps`, jamais par nom),
   effacer les restes listés plus haut. L'effacement revient à qui Lucie ou l'orchestration
   désigne. Ceux de `e2e_rag3daemon` et `e2e_code` ont été effacés le 18 octobre.

## Le 8 novembre 2026 — la garde, et ce qui reste

**Ce que fait le lot** (`agent/tests-bases-dans-tmp`, essai 5 de l'équipier Gemini sous
supervision de c7, fini par la session hotfixs après l'arrêt de l'équipier sur son quota) :

- `tests/common/mod.rs` : `BaseTemporaire::dans(famille)`, une base `base.rag3db` dans un dossier à
  elle sous `temp_dir()`, que son `Drop` efface entier, succès ou panique, sauf avec
  `RAG3WEAVER_GARDER=1` ; et `compter_residus_temp(préfixe)`.
- Dix témoins passent par elle : `e2e_rag3daemon` (`deux_processus_partagent_la_base_par_le_demon`,
  `un_lecteur_choisit_son_chemin_et_le_dit`), `e2e_code`
  (`a_bulk_load_interrupted_by_a_caught_panic_is_repaired_on_reopen`), `e2e_prise_atomique`
  (`deux_processus_ne_peuvent_pas_ouvrir_la_meme_base`,
  `plusieurs_lecteurs_partagent_une_base_qu_aucun_ecrivain_ne_tient`,
  `un_lecteur_qui_insiste_pendant_qu_on_ecrit`, `la_marque_dingestion_se_voit_depuis_un_autre_processus`,
  `un_catalogue_en_lecture_lit_ce_qu_un_ecrivain_a_pose`) et `e2e_rouvrir`
  (`apres_un_point_de_reprise_echoue_le_catalogue_refuse_tout_et_se_rouvre`).
- La base tombe par l'ordre de portée, après ce qui la tient (démon, catalogue, connexion) : un
  `drop(base)` explicite avant eux l'effaçait sous un démon vivant (relu dans les dix).
- Le témoin de la garde, à la fin de `deux_processus_partagent_la_base_par_le_demon` : rien ne
  reste sous le préfixe de ce témoin. Rouge avant (c7, à luciepc) : « 3 fichier(s) » — la base,
  son `.wal`, son `.ecrivain`. Il prouve le `Drop`, pas une suite entière.

**Ce qui reste :**

1. **Le témoin de suite** : jouer `e2e_rag3daemon`, `e2e_code` et `e2e_prise_atomique` hors de
   `run_e2e.sh`, avec un `TMPDIR` neuf, et exiger qu'il soit vide à la fin.
2. **Le cache par défaut de `Catalog`**, `/tmp/rag3weaver_cache` (`src/catalog.rs`, `cache_base`) :
   en service aussi, et c'est le point qui compte pour la mémoire vive du poste (`/tmp` est un
   tmpfs).
3. **Sept témoins d'`e2e_prise_atomique`**, même défaut (`remove_dir_all` sur la base-fichier, qui
   échoue). Six passent par `dossier_temporaire(nom)` sous `temp_dir()` :
   `deux_modeles_se_partagent_un_index`, `une_base_v5_ouverte_par_la_v6_ne_bouge_pas`,
   `le_meme_nom_avec_une_autre_dimension_est_refuse`,
   `un_modele_absent_refuse_en_nommant_les_disponibles`,
   `le_choix_du_premier_index_se_note_une_fois_et_se_relit`,
   `le_lot_ne_tombe_que_l_index_du_modele_courant_et_le_retrouve`. Le septième,
   `aucune_ouverture_n_est_croisee_sous_l_ecrivain_qui_affamait`, écrit sous `$HOME` (pas dans le
   tmpfs, mais il reste aussi).
   La forme attendue pour les six (c7, 8 novembre) : que `dossier_temporaire(nom)` rende une
   `BaseTemporaire`, gardée en vie par le test après ce qui tient la base ; un seul endroit à
   changer.
4. **Deux journaux à nom fixe** de 130 octets, `/tmp/rag3weaver-e2e-acces` et
   `/tmp/rag3weaver-e2e-rag3daemon` (les journaux du démon des deux témoins d'`e2e_rag3daemon`).
