# Session codeparsers — rapport

Tenu à jour sur place. Dernière mise à jour : 11 octobre 2026, nuit.

Chantier décidé par Lucie : **indexer nos propres dépôts**, la couverture la
plus grande possible de codeparsers, C++ et Rust d'abord, mesurée sur nos
dépôts (rag3db `src/` et `test/`, rag3weaver, codeparsers, l'IR). Points
bloquants et ordre : `../orchestration/04-indexer-nos-propres-depots.md`
(B1 à B9). Mien : B4, B5, B2 (et B1), B3.

## L'évaluation de départ : le C++ de rag3db (codeparsers 4c7897c)

Sonde `~/.cache/rag3weaver-build/sonde-cpp` (projet cargo hors de l'arbre,
dépend de codeparsers par chemin), sur tout `src/` : 1 543 fichiers `.h` /
`.cpp`, 6,3 Mo, 1,2 s.

- **Scopes** (`.h` / `.cpp`) : Namespace 1 825 / 1 441, Class 2 378 / 243,
  Method 3 518 / 377 (corps en ligne), Function 1 351 / 6 243, Lambda
  52 / 322, gabarits 371. Une définition hors classe `A::f` sort avec sa
  classe pour parent (6 449 / 6 620) ; la déclaration du `.h` est un membre
  de la classe (6 691), rag3weaver les réunit (HAS_PARENT, `declarations`).
- **Relations** : en fichier seul (le mode de rag3weaver), CONSUMES 6 143,
  aucune entre fichiers — les liens entre fichiers ne passent que par le
  rendez-vous par le nom ; en mode entre fichiers, CONSUMES 22 822 dont
  16 315 entre fichiers, héritage 620 (459). `#include` : 6 581 références,
  aucune relation fichier → fichier.
- **Pas su comprendre** : nœuds ERROR de tree-sitter dans 215 fichiers,
  1,37 % des octets (72 Ko dans le seul `c_api/rag3db.h`). Des 26 028 appels
  relevés (scope, nom), **11 % reçoivent un CONSUMES en fichier seul**, 36 %
  entre fichiers. Le relevé des non-résolues de codeparsers rendait 0 : B4.
- **« Qui appelle NodeTable::update, qui tient le verrou de l'index avant ? »**
  : 0 appelant relié, même entre fichiers (receveurs C++ non typés :
  `table->update(…)`, `cast<NodeTable>()`, `auto`) ; 188 gardes de verrou
  RAII dans le texte, aucune vue comme référence, pas d'identité pour un
  mutex membre.

## Lots

| Lot | Où | Ce qu'il change |
|---|---|---|
| B4 — le relevé des non-résolues | codeparsers `eb41801` | Chaque nom d'un scope qui ne produit aucune relation (locaux et builtins exceptés) est relevé avec sa raison : aucun scope de ce nom ; défini dans un autre fichier (fichier seul) ; plusieurs candidats ; un candidat écarté. Témoin `tests/non_resolues.rs`, rouge puis vert ; 24 suites vertes. |
| B5 — le banc de couverture | codeparsers `573c1e3` | `examples/banc_couverture` (`--ecrire`, `--verifier` qui échoue sur un recul), quatre références dans `banc/` (rag3db src et test, rag3weaver, codeparsers), notice `banc/README.md`. Appels reliés, fichier seul / entre fichiers : rag3db src 11,3 / 35,0 %, test 13,7 / 23,4 %, rag3weaver 14,5 / 27,0 %, codeparsers 15,6 / 21,3 %. Une erreur de mon banc corrigée avant le commit : les noms résolus dans leur fichier (`LocalScope`) étaient retirés du dénominateur. L'IR : chemin demandé. |
| Le rapport des trous | codeparsers `9b1460d`, page `02-les-trous-classes-par-cout.md` | `--trous` : chaque non-résolue rangée par forme, pondérée par son coût pour les outils (storage/transaction pèse double). Premier coût partout : la méthode sur un receveur non typé. L'IR en cinquième référence. |
| Le préprocesseur | codeparsers `75edad9` | Une ligne de directive (`#pragma`, `#include`, `#define`, `#if`, suite `\`) ne nomme rien en C/C++ ; le `#include` reste un import. rag3db src : non-résolues 100 813 → 86 796, imports reliés 10,1 → 34,4 % (entre fichiers 24,9 → 99,3 %). `banc/comparer.sh` : la vérification se fait avant / après sur le même corpus (une ligne fixe faisait « reculer » un dépôt qui grossit). |
| B2a — les receveurs C++ dans le fichier | codeparsers `876f606` | Pointeurs intelligents traversés ; fabriques et conversions (`make_unique<T>`, `cast<T>()`, `static_cast<T*>`…) rendent `T`. Témoin `tests/receveurs_cpp.rs` (8 formes). rag3db src : appels reliés entre fichiers 35,0 → 39,2 %. Appelants de `NodeTable::update` (`--appelants`) : 0 → 1 entre fichiers. |
| Les champs C++ | codeparsers `e8e2893` | Un champ de classe en pointeur ou référence (`NodeTable* table;`) n'était pas relevé ; il l'est avec son type (champ `type` du nœud, déclarateurs déballés, `int a, b` en donne deux). rag3db src : appels reliés entre fichiers 39,2 → 40,3 %. |
| B2b — les types différés au rendez-vous (B1) | rag3db master `6002c3ccd` (fusionné par l'arbre principal, avec le pointeur e8e2893) | `field_types` et `return_type` sur Scope, `deferred` sur MENTIONS, résolus à la matérialisation en `qualifier_types`. Témoin `e2e_types_differes` : les trois formes réelles des appelants de `NodeTable::update`, rouge (deux sur trois) puis vert. Pas de mode entre fichiers dans codeparsers : le graphe ne dépend pas du paquet. |
| B3 — les verrous (codeparsers) | codeparsers `65c1c7a`, `8881d52` | Deux genres d'usage, `Lock` et `SharedLock` : une référence au champ mutex, le propriétaire en qualificatif. Gardes RAII C++ certaines (`unique_lock`, `lock_guard`, `scoped_lock`, `shared_lock`, accolades ou parenthèses — `std::lock_guard lck(mtx);` se lit comme une fonction) ; `.lock()` / `.read()` / `.write()` seulement sur un champ déclaré mutex ou RwLock dans le fichier. Un verrou est compris : hors du relevé des non-résolues. rag3db src : 140 exclusifs, 32 partagés (188 gardes dans le texte) ; rag3weaver : 75 et 1. |
| B3 — la relation LOCKS (rag3weaver) | rag3db master `2a29c10c6` (fusionné, pointeur 8881d52) | Un symbole `Classe::champ` par champ typé, défini par sa classe ; un verrou est la relation LOCKS du scope vers ce symbole (genre, ligne), posée à l'ingestion. « Qui verrouille NodeTable::mtx » = les LOCKS entrants du symbole. Témoin `e2e_verrous`. |
| B2c — les chaînes de champs | codeparsers `55c27d1` ; rag3db, branche `chaines-de-champs` (`e94c29959` pointeur, `5999cb640` code.rs), proposée | Le premier champ d'une chaîne dit le propriétaire, les suivants deviennent des pas de champ (`.b`) ; le lecteur du fichier et la résolution au rendez-vous (par tours, quatre au plus) les suivent. Forme réelle : `tableInfo.table->update()` de set_executor.cpp. |
| B3 — les verrous sur le chemin, dans `impact` | rag3db, branche `verrous-dans-impact` (`627b3e6a9`, empilée sur B2c), proposée | Un relevé générique du nœud de voisinage, déclaré au gabarit (`collect='LOCKS>'`) : pour la méthode modifiée et chaque scope qui en dépend, les mutex qu'il verrouille, groupés par `Classe::champ`. L'IR n'a rien à changer (Hop suit LOCKS par son nom). Témoin `e2e_verrous`. |

## La mesure réelle : le C++ de rag3db indexé par rag3weaver

Sonde `tests/sonde_cpp_rag3db.rs` (tout `rag3db/src`, 1 543 fichiers,
19 044 scopes, en mémoire, embarqueur de hachage), avec B2b, B3 et B2c :

- **appelants de `NodeTable::update`** : 0 au départ, 1 avec B2a/B2b
  (`replayNodeUpdateRecord`), **2 avec B2c** (`set` de set_executor.cpp) —
  tous marqués « type ». (`localTable->update()` vise `LocalNodeTable::update`.)
- CONSUMES par marque : fichier 5 086, type 3 375, import 4, nom 8 719.
- **LOCKS : 169**, sur des mutex réels (`ChunkedNodeGroup::versionInfoMtx`
  18, `HashIndexLocalStorage::mtx` 13, `UpdateInfo::mtx` 9, `WAL::mtx` 5…).
- La résolution des types différés : 97 ms pour 1 543 noms en 3 tours, sur
  une ingestion de 167 s (remarque de relecture de l'arbre principal).

## Suite

B2c et `verrous-dans-impact` attendent leur fusion par l'arbre principal.
Ensuite : la borne à la source de la requête des types différés (remarque
de relecture), puis la sonde relancée pour l'exemple réel d'`impact` sur
`NodeTable::update` (appelants et verrous pris), à montrer à Lucie.
