# Arbre principal — rapport de session

Session de l'arbre principal (`/home/lucied/git_workspaces/rag3db`, crate
`extension/rag3weaver`). Elle tient l'ingestion du code, la synchronisation,
le chargement en masse et les requêtes par lot du dialecte. Mis à jour le
3 octobre 2026, vers minuit.

## Fait aujourd'hui, sur master

Dans l'ordre de fusion.

| Commit | Lot | Pourquoi |
|---|---|---|
| `f4cceb80b` | **Édition pendant l'indexation** | Règle de Lucie : une édition ne ralentit jamais l'indexation. Un fichier pas encore passé est lu à son paquet ; un fichier déjà passé est écrit, puis repris à la fin. |
| `7e5f6fde2`, `cfb9da12b` | **Mention typée** (codeparsers) + mon correctif des parents génériques | ~220 fausses arêtes retirées (`.len()`, `.clone()`… vers des homonymes du projet). La seule juste perdue (`Compiler<'_, '_>.chain`) est rendue par le correctif. |
| `439d8f29f` | **COPY des liens qui lit ses guillemets** | Deux vidages à 56 s et 145 s (200 s sur 643) : le renifleur du moteur décidait seul des guillemets, le COPY était refusé et les liens retombaient sur le chemin lent. |
| `359d61daf` | **Pile codeparsers 4 → 7** + poids `test_role` à 0,5 | Les fonctions de `mod tests` passaient devant leur définition (e2e_code:331). Poids choisi au banc par la session recherche. |
| `31e82a69e`, puis dans `218faf854` | **run_e2e.sh** : tous les tests d'une suite, et zéro test fait échouer la passe | `usages_rendu` rendait « 0 passed » comme un succès. |
| `b3db4244c` | **Requêtes par lot par jointure** (`dialect::unwind_par_cle`) | `MATCH (n {_uuid: item.x})` balayait la table entière. Dépôt entier : 499 → 352 s (mesure de rag3db-eb). |
| `218faf854` | **Plus de repli muet** + refus « ANY type » | Les COPY refusés se comptent (`bulk_load_refused`). Le refus des morceaux en plein texte venait d'un défaut moteur (`ALTER … DEFAULT NULL`), contourné. |
| `bb98827af` | **Accès de champ sans rendez-vous** (codeparsers) + pointeur 8 | `self.config`, `r.chunk` ne se relient plus à une fonction homonyme. Signatures Rust avec `-> T`. |

## Décidé, et pourquoi

- **Édition pendant l'indexation.** Le registre « à reprendre » vit hors du
  catalogue (`code_sync::note_change`), parce qu'une édition ne doit jamais
  attendre le verrou. La justesse tient à un ordre : la synchronisation
  inscrit un chemin comme lu *avant* de le lire, l'édition écrit *avant* de
  consulter. `reingest_file` retire d'abord les arêtes sortantes des scopes
  du fichier ; sans ça, un scope gardé qui n'appelle plus `f` gardait son
  arête.
- **La recette des jointures diffère de celle du cœur C++.** `item` voyage, et
  chaque clé est extraite dans le `WITH` qui précède immédiatement son
  `MATCH`/`MERGE`. Le refus « Cannot evaluate expression with type VARIABLE »
  venait d'une clé extraite trop tôt, pas de MERGE : les liens gardent MERGE.
- **Le défaut NULL ne s'écrit pas** dans le dialecte rag3db. C'est déjà le
  défaut, et l'écrire fait refuser les COPY.
- **Un repli reste permis, mais il se dit.** Rapport de synchronisation, et
  test qui échoue.
- **Les poids de pertinence se déclarent** (`FieldWeight`), aucune règle de
  classement neuve. « La correspondance exacte du nom en tête » attend Lucie.

## En cours (mis à jour le 4 octobre, vers minuit et demi)

Fusionné depuis le premier jet :
- `a5cef54cc` (journal) : décision du **résolveur unique** entre fichiers.
- `2914d470e` : `run_e2e.sh` dit contre quel moteur il tourne, et refuse un
  moteur plus vieux que ses sources.
- `build/lecteurs-csv` rebâti sur `a5cef54cc`, garde 1 et DROP au rejeu
  compris.

1. **Le résolveur unique** (décidé, au journal). codeparsers (rag3db-c0) livre
   deux options : `cross_file` (résolution dans le fichier seulement),
   `project_files` et `import_source` sur chaque référence. De mon côté :
   - départager les définisseurs par le chemin d'import (`crate::estimate` →
     `src/estimate.rs`) ;
   - remplir `project_files` depuis le WorkingTree ;
   - le test « paquets de 1, de 64, d'un seul tenant → même graphe » ;
   - la justesse avant/après, au banc des relations et sur `e2e_code`.

   Mesures qui l'ont décidé : 15 justes sur 50 dans l'écart, deux
   échantillons indépendants. Sur `src/` de rag3weaver : 59 152 relations par
   64, 72 721 d'un seul tenant.
2. **Le mode « premier index » par gros paquets bornés en octets**, ensuite.
   Le dépôt entier en un paquet fait 132 s pour 16 Go au pic (rag3db-eb).
   Mes deux mesures d'un seul tenant (analyse 44 s, plein texte 306 s) sont à
   refaire : elles ont tourné en même temps qu'une autre passe lourde.
3. ~~Renommer~~ fait : `a_bulk_load_interrupted_by_a_caught_panic_is_repaired_on_reopen`
   (une panique rattrapée, pas une mort).

Dégagé par l'orchestration :
- **« is behind its table » à l'ouverture** : rag3db-dc. Elle touche
  `restore_dropped_vector_indexes` et l'étape 10 ter d'`initialize` ; je n'y
  touche pas.
- **SET vecteur → vecteur et l'invariant « toute ligne à vecteur est
  joignable »** : rag3db-eb, dans `EmbedNode`.
- **Report des blobs** : gelé (2 s en un paquet), au journal.

Règle du poste : une mesure lourde à la fois (plusieurs Go ou plus de 8 fils),
annoncée à rag3db-eb et rag3db-c0, qui répondent « libre » puis « fini ».

## Ce qui attend quelqu'un

- **Lucie** : l'affichage « · test_role=case » au rendu (conséquence du champ
  dans `return_fields`) ; la règle « nom exact en tête ». Les deux sont posées
  par l'orchestration.
- **Cœur C++ (rag3db-e3)** : le SET d'un vecteur vers un autre perd des lignes
  dans HNSW (969/1000 ligne à ligne, 532/1000 par lots de 512). Le SET depuis
  NULL ne perd rien, ce qui met notre dette de vecteurs à l'abri. Témoins
  rag3weaver possibles dans `e2e_recherche_dense_apres_suppressions`.
  Intermittent du WAL vide à la réouverture
  (`e2e_idempotent_registration::register_entity_persists_and_reloads`, 1 sur 9).
- **Session mémoire (rag3db-dc)** : le cas « chargement en masse interrompu »
  et le cas « blobs différés tués » dans `e2e_arret_brutal`.

## Comment reprendre

- Branche de travail : partir de `origin/master` (`git checkout -b X origin/master`).
  L'arbre est partagé, donc commit par chemins
  (`git commit -F msg -- <chemins>`), `user.email` gmail, sans trailer d'IA,
  jamais de push en force. Fusion : `git push origin X:master` après
  `git fetch && git log --stat HEAD..origin/master` ; si du code est arrivé,
  rejouer les suites concernées avant de pousser.
- Tests : `./run_e2e.sh --test <suite>`, avec `LUCIVY_SCHEDULER_THREADS=8`
  et, s'il y a de l'embarquement, `RAG3WEAVER_EMBED_CHAR_BUDGET=4096
  RAG3WEAVER_GPU_DUTY=70`. La lib :
  `LD_LIBRARY_PATH=../../build/lecteurs-csv/src cargo test -j6 --lib
  --features rag3db-native,burn-embedder,burn-ocr,code,daemon`. La batterie
  complète tient dans un script du bloc-notes (lib, chaque e2e une à une,
  bins, scripts Python), et se joue une fois avant une fusion qui touche les
  chemins d'écriture.
- Mesure : `MESURE_RELATIONS=per_batch|bulk ./run_e2e.sh --test
  e2e_mesure_sync_source` (src/ du moteur, 1 643 fichiers). Aujourd'hui :
  47 s par paquet, 37 s en masse.
- Pièges : annoncer chaque passe cargo à l'orchestration, jamais deux e2e en
  même temps ; `/tmp` est en RAM ; e2e_code indexe `src/dataflow` (corpus
  vivant : un fichier neuf y change les comptes).
