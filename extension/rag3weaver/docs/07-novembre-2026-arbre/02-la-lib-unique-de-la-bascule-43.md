# La lib unique de la bascule 43

7 novembre 2026, session « arbre principal » (73), sur la décision de c7 de bâtir la lib du
format 43 sans attendre le retrait des générations ni la projection en lecture seule, qui
viendront ensuite comme des libs ordinaires au même format. Les étapes 1 à 5 de
`docs/procedures/bascule-de-lib.md` sont les miennes ; G tient les étapes 6 à 11 (hôtes,
instantanés `avant-43`, contrôle des deux sens, `courant`, réveil).

## Ce que la lib porte

Bâtie sur master 17a094d12, son dernier commit moteur : V2-1c, V2-2a et 2b, (A) au format 43,
le bloc annulé (b7447937d), l'ordre des verrous du groupe CSR (e526cc4b6) et l'instruction en
échec qui ne condamne plus une transaction en lecture seule (17a094d12).

## Les étapes, avec leurs heures

| étape | quand | ce qui a été lu |
|---|---|---|
| 1. bâti | luciepc, 19 h 03 m 17 s → 19 h 05 m 43 s | `batir58.sh 17a094d12` sous `poste lourd`, `-j 8` : 889 cibles, extension vector comprise ; publiée `--sans-courant` |
| 2. le nom | 19 h 05 | `lib58/id` : `rag3db-build-17a094d12d8c29418fbf3b3ed941ad64e73b4c29-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF` ; `BUILD_ID` et MANIFESTE concordent |
| 3. l'ABI | 19 h 05 | contre 04063ee06 (la lib en service) : **elle change** (ci-dessous) |
| 4. le verdict | 19 h 07 → 19 h 11 | **vert**, sources de master cf14a7771 (ci-dessous) |
| 5. la copie | 19 h 08 | ici, `~/.cache/rag3db-libs/<id>`, les quatre md5 égales au MANIFESTE ; `courant` reste 04063ee06 aux deux postes |
| « fini » à G | 19 h 12 | id, md5, ABI, verdict, copie |

md5 : `librag3db.so` 16d684928a2afb8d39a25438a392ed44, `libvector.rag3db_extension`
a45319542a8bd958d87217e63f2988ee, `rag3db.hpp` bd0e1dc67ffd5c30c177b1ebc0a3d556.

## L'ABI

Contre 04063ee06, ce que les binaires liés voient :

- **trois symboles retirés** : `NodeTable::commitIndexChange(unsigned long, bool, unsigned long)`,
  `NodeTable::rollbackIndexChange(unsigned long, bool)`,
  `Transaction::pushIndexChange(NodeTable&, unsigned long, bool) const` ;
- dix-sept ajoutés ;
- **trois vtables qui changent de taille** : `ListChunkData`, `ColumnChunkData`,
  `InternalIDChunkData`.

Ce n'est pas le dernier lot du cœur (il ne change aucune signature exportée, et le lot des verrous
n'a touché que deux méthodes privées de `CSRNodeGroup`) : c'est l'accumulation depuis 04063ee06.
Tout binaire lié se rebâtit donc contre l'id, hôtes compris. `ABI_CHANGEE` n'est pas posé : le
cœur ne déclare aucun changement invisible aux noms et aux tailles.

## Le verdict

Sous `RAG3DB_LIB=<id>`, **sans** `RAG3WEAVER_MOTEUR_ANCIEN`, avec `VERDICT_FORMAT=43`, à luciepc :

- `e2e_code` : 27 passés, 0 échoué ;
- `e2e_lectures` : 22 passés, 0 échoué, dont les trois témoins nommés par la procédure :
  `la_branche_dense_filtree_par_langage_repond_dans_l_instantane_de_l_appel`,
  `une_recherche_dense_bornee_a_un_chemin_tient_sous_l_instantane`, et
  `la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel` — **jugé** pour la première
  fois (aucun « non jugé » : la lib a la colonne `state` du format 43) ;
- la ligne « moteur lié » de chaque passe nomme la lib 17a094d12, bâtie le 07/11 à 19 h 05.

## Ce qui a servi, et ce qui a été corrigé en route

- Le script de verdict de luciepc reprend celui de la procédure (`e2e_code` puis
  `e2e_lectures`, les trois témoins par leur nom, le « non jugé » compté rouge sous
  `VERDICT_FORMAT=43`).
- Le contrôle des deux sens (étape 8, à G) prenait l'outil dans l'arbre de Lucie, qui avait du
  retard (la version à comptes) : il le prend désormais dans `origin/master` et dit son commit
  (cf14a7771), et il est rouge quand il ne contrôle aucune base (b99b67de0, la remarque
  d'hotfixs). Sa référence, relue avec l'outil ancré 339cbb5dc sur l'instantané du 25 octobre de
  `code-rag3db-depot` : LOCKS 206 relations d'un seul côté, 179 vues deux fois, 247 aux bouts
  différents ; QUERIES 286, 286 et 306 ; 14 minutes.
- La surveillance des lots du cœur cherchait leur **sujet**, pas leur hash : un rebase au push
  change le hash (la leçon du 25 octobre, quand la lib attendue sous un nom avait été publiée sous
  un autre).
