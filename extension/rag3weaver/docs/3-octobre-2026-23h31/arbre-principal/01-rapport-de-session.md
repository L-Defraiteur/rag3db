# Arbre principal — rapport de session

Session de l'arbre principal (`/home/lucied/git_workspaces/rag3db`, crate
`extension/rag3weaver`). Elle tient l'ingestion du code, la synchronisation,
le chargement en masse et les requêtes par lot du dialecte. Mis à jour le
4 octobre 2026, vers 16 h.

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

## Fusionné le 4 octobre (après-midi)

| Commit | Lot | Pourquoi |
|---|---|---|
| `1a57eb77b`, `561002bfc`, `d54985804` | **run_e2e.sh prend le verrou du poste** | Partagé par défaut, exclusif pour une mesure ou un rebâti ; la porte donne la priorité à la mesure ; priorité basse (`nice`, `ionice`) hors mesure. Une batterie écarte de jour les quatre familles de la carte locale (`RAG3WEAVER_SANS_CARTE_LOCALE=0` les fait entrer) et se dit « complète hors carte locale ». Le démon d'embarquement né pendant la passe s'arrête avec elle (marque `RAG3WEAVER_PASSE_E2E`). `561002bfc` refusait le démon local : erreur, chaque suite chargeait alors le modèle sur la carte ; retiré dans `d54985804`. |
| `5f1fefa39` | **Un NULL en tête d'un lot ne fait plus tomber le lot** | Un NULL de paramètre est typé STRING par le moteur ; un rendez-vous sans ligne en tête d'un lot de MENTIONS faisait refuser tout le lot, compté dans `failed` sans un mot. Des MENTIONS ont pu manquer dans tout index bâti avant. Ticket moteur ouvert. |
| `9b89ec823` | **Déclarations** (codeparsers `a3905ec`) | Une définition hors de sa classe rejoint sa classe ; `Scope.declarations` ; HAS_PARENT vers un conteneur seulement ; clés stables calculées par fichier (le ticket des 474 relations attend la sonde 64/512 de rag3db-c0). |
| `395d35523`, `f53d41488` | **Le tampon du moteur** | Choisi à l'ouverture : variable, clé `buffer_pool` du manifeste, puis la règle de l'orchestration, min(RAM/2, 8 Gio). Dit au rapport ; `DbConnection::buffer_pool()` le rend typé pour `estimate` (rag3db-eb). |
| `18b1a6889` | **Pointeur codeparsers `766e4bd`** | Locales Rust et conteneurs : moins de fausses références. 69 tests verts, aucun seuil touché. |

Sur une branche, pas sur master : `tx-par-paquet` (`6359f7100`), la
transaction par paquet derrière `RAG3WEAVER_TX_PAR_PAQUET=1`, pour la mesure
de rag3db-eb. Elle ne répond pas à l'échec du tampon à 4 Gio (elle regroupe
les points de reprise, chacun porte plus). Le premier paquet reste hors
transaction : il crée le schéma à la volée, qu'une annulation emportait.

`build/lecteurs-csv` rebâti sous verrou exclusif à 14 h 10 (garde 2 de
rag3db-e3) et à 15 h 16 (`34bde7eea`, CSV vide entre guillemets).

## En cours

1. **Le premier index vers 90 s sur disque.** La cause de l'échec à 4 Gio
   n'est pas connue (rag3db-eb : la sonde en Cypher brut dit qu'il casse sous
   des BLOB, 544 Mo suffisent avec 2 Gio). Leviers : seuil du point de
   reprise, transaction par paquet (à mesurer), paquets bornés en octets.
2. **Les 24 avertissements sans lecteur** restants (hors title_boost,
   content_boost, boost, à rag3db-88).
3. **Le ticket des journaux d'annulation dans /tmp** (ne rien effacer : Lucie
   décide).

## Ce qu'on a appris aujourd'hui

- Un compteur `failed` qu'aucun test ne lit cache une perte entière : le
  test des déclarations exige maintenant `failed == 0`.
- Une variable qui « refuse » une chose peut en déplacer le coût :
  `RAG3WEAVER_SANS_DEMON` ne renvoie pas au service, il charge sur place.
  Lire le chemin de repli avant de poser un refus.
- `poste lourd` / `poste mesure` (`~/.cache/rag3weaver-build/poste`) pour tout
  le lourd hors run_e2e ; plus d'annonces.

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
- Pièges : le verrou du poste remplace les annonces (run_e2e.sh le prend,
  `poste lourd` pour le reste) ; `/tmp` est en RAM ; e2e_code indexe
  `src/dataflow` (corpus vivant : un fichier neuf y change les comptes) ;
  après un rebase qui apporte des sources du moteur, rebâtir
  `build/lecteurs-csv` en exclusif, run_e2e.sh le refuse sinon.
