# Session codeparsers — rapport

Tenu à jour sur place. Dernière mise à jour : 11 octobre 2026, 22 h 40.

## Pour Lucie (11 octobre, 22 h 40) — l'hôte de luciepc tourne en boucle, gardé tel quel pour toi

L'index de `code-lourd` sur luciepc s'est figé une quatrième fois, **sans la montre** (retirée
des deux manifestes pour l'essai). L'orchestration a décidé de ne pas arrêter l'hôte cette nuit,
et ma permission refuse l'arrêt. Personne n'y touche avant toi.

**La cause du figement est connue** : une boucle de réclamation. L'index réclame des morceaux
dont `_text` est vide ou nul, l'embarquement les saute sans les marquer, la passe les compte
faits, et il les reprend au tour suivant, sans fin. Les relevés qui l'ont tranché (lecture seule,
bruts dans `~/.cache/codeparsers-notes/b2d/releves-132823*.txt` sur ce poste-ci) :

- le `.wal` de `code-lourd` croît de 255 à 376 Ko toutes les 10 s, alors que la base ne bouge plus ;
- les 25 fils actifs (`demon-3`) sont nés à 22 h 00 min 55 s, de mon appel `index` ; aucune
  recherche n'a eu lieu depuis le démarrage ;
- le service d'embarquement 7878 est au repos, et aucune panique n'apparaît.

L'arbre principal a écrit deux défenses, qui vont sur master ce soir : la réclamation filtre
`_text` vide ou nul, et `run_index` s'arrête sur une passe sans progrès.

- **Hôte** : pid `132823`, démarré à 21 h 53, binaire `~/.local/bin/rag3weaver-backend@1791744137`.
- **Index** : lancé à 22 h 02 min 46 s, journal
  `~/.cache/rag3weaver/commandes/index/1791748859452-132823.out` ; 28 369 vecteurs sur
  138 579 (20 %), tous hérités des essais précédents ; ce run n'en a écrit aucun.

À faire, dans l'ordre :

1. *(Facultatif.)* La pile, si tu veux la voir de tes yeux :
   `sudo gdb -p 132823 -batch -ex 'thread apply all bt' > ~/pile-hote-code-lourd.txt 2>&1`
2. L'arrêt propre : `systemctl --user stop rag3weaver-daemon` (mon `index.sh`, pid `132918`, sort
   de lui-même ou se tue par `kill -TERM 132918`). Dis-le à codeparsers.
3. **Ce qui compte ensuite, base fermée** : l'origine des morceaux vides, encore ouverte. Avant de
   redémarrer, une copie reflink de la base fermée (une seconde, sans place prise) :
   `cd ~/.local/share/rag3weaver && mkdir -p code-lourd.boucle && cp -a --reflink=always code-lourd.rag3db code-lourd.rag3db.wal code-lourd.rag3db.extensions code-lourd.rag3db.fts code-lourd.boucle/`.
   Sur cette copie, je compte les `Scope_Chunk` à `_text` nul ou vide, avec
   les 20 premiers fichiers d'où ils viennent, leur nombre, et `_parent_uuid`, `_parent_field`,
   `_text_hash`. Sans parent ni hash, c'est une ligne fantôme ; un parent texte brut d'extension
   exotique, c'est un binaire lu avec un octet nul. Rien à faire de ta part sur ce point.
4. La montre remise et le redémarrage sur le binaire prêt de G, **une fois le correctif de l'arbre
   principal dedans** (G le rebâtit si le push arrive avant son réveil) :
   `cp ~/.config/rag3weaver/code-lourd/templates/backends/code/backend.json.avec-montre ~/.config/rag3weaver/code-lourd/templates/backends/code/backend.json`,
   `cp ~/.config/rag3weaver/code-principal/templates/backends/code/backend.json.avec-montre ~/.config/rag3weaver/code-principal/templates/backends/code/backend.json`,
   puis `mv ~/.local/bin/rag3weaver-backend.pret ~/.local/bin/rag3weaver-backend && systemctl --user start rag3weaver-daemon`.

Après l'arrêt, le cœur et hotfixs rebâtissent la lib de `rag3db-lourd` (ils attendent ce
signal). `code-principal` reste tel quel : le cœur s'en sert. Le ticket :
`docs/tickets/2026-10-11-un-index-se-fige-sur-un-appel-d-embarquement-sans-delai.md`.

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
| B2c — les chaînes de champs | codeparsers `55c27d1` ; rag3db master (`d6a2827ce`, fusionné par l'arbre principal) | Le premier champ d'une chaîne dit le propriétaire, les suivants deviennent des pas de champ (`.b`) ; le lecteur du fichier et la résolution au rendez-vous (par tours, quatre au plus) les suivent. Forme réelle : `tableInfo.table->update()` de set_executor.cpp. |
| B3 — les verrous sur le chemin, dans `impact` | rag3db master (`d6a2827ce`) | Un relevé générique du nœud de voisinage, déclaré au gabarit (`collect='LOCKS>'`) : pour la méthode modifiée et chaque scope qui en dépend, les mutex qu'il verrouille, groupés par `Classe::champ`. L'IR n'a rien à changer (Hop suit LOCKS par son nom). Témoin `e2e_verrous`. |

| Impact : les sûrs d'abord | branche `impact-surs` (`99e449806`, `6451dad2a`), proposée à l'arbre principal ; oui de l'orchestration pour Lucie, renversable | Les usages trouvés par le nom seul (nom ambigu, attribués à aucune définition) ne sont plus listés ni comptés parmi les touchés : une ligne « N par le nom seul, non montrés », dans le rendu complet et dans le résumé. `include_by_name` (bool, défaut false) au schéma d'`impact` les remontre, marqués. Sur `NodeTable::update` : 2 appelants au lieu de 30 lignes. Témoins `impact_rendu` et `e2e_impact` (gabarit réel, avec et sans le paramètre). |
| L'outil « ce que ça appelle » | branche `callees` (`026e9ca53`, empilée), proposée | `templates/tools/callees.mmd` : le même nœud de voisinage, CONSUMES sortants, arêtes « nom » non suivies, la même section des verrous ; déclaré dans les deux manifestes (treize outils). Le nœud gagne `heading` (titre du rendu), la ligne de définition de l'appelé en sortant (c'était celle du site d'appel, chez l'appelant), et se tait sur le groupe quand aucun n'est déclaré. Témoin `e2e_verrous` : depuis `top`, `bulk` puis `insert`, `Index::autre` puis `Index::mtx`. |

| Les verrous applicatifs déclarés (`locks_via`) | branche `locks-via` (`c3e03e736`, `001711935`, `148e8da47`, empilée), oui de l'orchestration pour Lucie, renversable | `workspace.locks_via` dans le manifeste (refusé sans `index: "code"`), posé sur le catalogue à l'ouverture ; un scope qui appelle une fonction déclarée reçoit LOCKS vers le symbole de la fonction, dans le fichier comme au rendez-vous ; marque `via` (« champ » / « appel ») ; titre « Verrous pris sur le chemin » ; page du gabarit code. Sonde rag3db/src avec `acquireLock`, `acquireLocks`, `lockRowForWrite` : LOCKS 169 → 178, `callees` de `NodeTable::update` rend `lockRowForWrite` → `acquireLock` → `acquireLocks`. **Une erreur à moi corrigée** : j'avais d'abord déclaré `lockKeyOf` et `lockKeyOfRow`, qui calculent la clé sans verrouiller (« update prend lockKeyOfRow », faux) ; le mécanisme fait ce qu'on déclare, la page dit lesquelles mettre. |

| B2d — la ventilation des receveurs | codeparsers `26b9a56` | `banc_couverture --receveurs` : chaque méthode non résolue rangée par l'origine de son receveur. rag3db/src entre fichiers : 16 041, dont 6 638 types différés (le rendez-vous les résout), 3 290 receveurs de type externe (aucune arête « nom » : `choose_target` ne relie un receveur typé que par son type), 2 410 de type du projet, 1 690 retours d'appel. Les 8 720 arêtes « nom » viennent du repli « un seul définisseur » sur un receveur sans type. |
| B2d — les boucles Rust (T3 de la session recherche) | codeparsers `1fc380a`, pointeur donné à l'arbre principal | Un appel sur une locale non typée est écarté (règle d'origine du crate) : `for o in &v { o.jouer() }` ne laissait aucune référence. `o` a maintenant le type de l'élément ; `vec![Marteau, …]` et `let m = Marteau;` nomment leur type. Témoin `tests/boucles_rust.rs`, rouge 4/5 puis vert. Banc : recul apparent par visibilité, prouvé (+33 références et +33 non-résolues sur rag3weaver, −28 non-résolues en fichier seul). **Point ouvert pour l'orchestration** : rendre visibles (non attribués) les appels sur une locale qui reste non typée — change ce qu'affiche `usages`. |

| B2d — les appels par chemin sur un type | codeparsers `f91fc58`, vérifié côté rag3weaver (e2e usages, types différés, verrous, banc des relations, liens, impact, impact par fichier, code), pointeur donné | `Catalog::Get(…)`, `Outil::fabrique()`, `IntegerBitpacking<T>::f()` : le chemin nomme le type du receveur. rag3db/src : appels reliés entre fichiers 40,3 → 41,4 %, en fichier seul 11,7 → 12,3 %. rag3weaver : 330 faux liens par le seul nom écartés (types externes, variantes d'enum), relevés un par un sur les 40 premiers. Sonde rag3db/src : CONSUMES « nom » 8 720 → 7 215, « type » 3 377 → 5 517. Les sorties réelles d'`impact` et de `callees` : page 03, section du 11 au matin. |

| T1 — un bloc impl ne redéfinit pas son type (B9 Rust, cas de rag3db-97) | codeparsers `02a526a` (modificateur `impl`, banc inchangé) ; branche rag3db `impl-pas-definisseur` (`b310e847e`), proposée | Le bloc `impl` était un second scope `Class Marteau` : `Marteau` ambigu partout où un impl existe. Il porte désormais le modificateur `impl`, et le rendez-vous ne lui fait plus définir le nom. Témoin `e2e_usages` par le gabarit réel : une définition de `Marteau` ; `frappe` (`let m = Marteau`) et `tous` (`for o in &outils`) attribués à `jouer` de l'impl — T2 et T3 fermés avec. |

| T4 — un macro_rules! est un scope (B9 Rust) | codeparsers `93e82b8` ; pointeur dans la branche rag3db `impl-pas-definisseur-2` (rebasée, proposée) | La macro devient un scope `Function` (modificateur `macro`), ses références lues dans le corps de ses règles : `par_macro → disons → aide`, le corps ne pollue plus le scope du fichier. Banc Rust en gain net (non-résolues en baisse ; appels reliés de codeparsers entre fichiers 20,9 → 21,8 %). Tickets dans la même branche : « appel par chemin vers un type externe » fermé (témoin de sa recette, rouge avant f91fc58) ; « receveurs sans type » en cours ; T5 (fermeture locale, `map(aide)`) ouvert. |

| B2d — les boucles C++ sur intervalle | codeparsers `06d6e92` | `for (Noeud* n : v)` se lit tel quel ; `for (auto& n : v)` prend l'élément du conteneur déclaré dans la fonction (`vector<unique_ptr<Noeud>>` → `Noeud`). Témoin `tests/boucles_cpp.rs`, rouge 2/3 puis vert. Banc sans recul (rag3db/src : non-résolues entre fichiers 69 756 → 69 745) ; le gain porte au rendez-vous de rag3weaver. Vérification côté rag3weaver en cours, pour un ajout au bout de la branche `impl-pas-definisseur-2`. |

| La borne à la source des types différés | rag3db master (`d6a2827ce`) | Remarque de relecture : chaque mention se résout avec les types écrits de la source et du dépôt de son scope (couple source, repo : deux dépôts locaux partagent la source `file`). Témoin : une classe homonyme dans un instantané. |

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

Tout est sur master (`d6a2827ce`, sonde `36cd2604c`). La page pour Lucie :
`03-impact-node-table-update.md` — la sortie réelle d'`impact` sur
`NodeTable::update` (les deux appelants réels, le verrou pris par le
gestionnaire de verrous de la transaction sous `update`, pas chez les
appelants), les chiffres avant / après la nuit, ce qui reste.

Ordre donné par l'orchestration (11 oct., nuit) : (1) impact, les sûrs
d'abord — fait, branche `impact-surs` ; (3) « ce que ça appelle » — fait,
branche `callees` ; (2) les fonctions de verrou applicatives déclarées par
dépôt dans le manifeste (`locks_via`), qui posent LOCKS par l'appel — fait,
branche `locks-via`. Les trois reprises par l'arbre principal. Ensuite :
B2d, les 8 720 arêtes par le nom (receveurs non typés) ; B9 côté Rust
partagé avec rag3db-97 (à rag3db-97 l’indexation de rag3weaver et les cas
minimaux par catégorie, à moi la lecture et les correctifs codeparsers).
## État en fin de nuit (11 octobre, vers 3 h 30)

**Sur master rag3db** : impact les sûrs d'abord, `callees`, `locks_via`
(fusionnés par l'arbre principal, `6044723a5`) ; codeparsers `f91fc58`
(`80054b530`).

**En attente chez l'arbre principal** : branche `impl-pas-definisseur-2`
(tête `5626429a7`) — T1 (`code.rs` : un bloc impl ne définit pas son nom),
pointeur codeparsers `06d6e92` (T4 macro_rules!, boucles C++), témoin et
fermeture du ticket « appel par chemin vers un type externe », tickets.
Vérifiée sur la lib de 02:18. La branche `impl-pas-definisseur` (sans -2)
est à ignorer : rebasée, poussée sous un nom neuf.

**Ouvert** :
- T5 (ticket `2026-10-11-appel-a-une-fermeture-locale-ecarte`).
- Une décision : un appel sur une locale qui reste non typée est écarté en
  silence (règle d'origine du crate, `base_scope_extraction_parser.rs`,
  `exclude.contains(q) && !typed_binding(q)`). Le rendre visible, non
  attribué, changerait ce qu'affiche `usages` et le banc (recul de
  visibilité) : à proposer, pas à trancher seule.
- B2d suite, par `banc_couverture --receveurs` : retours d'appel (le premier
  poste en Rust, 12 066), paramètres de fermeture, indexation, C++ `auto x =
  f()` sur une fonction du projet.
- La mesure des marques CONSUMES sur rag3weaver (Rust) avant/après : deux
  passes de la sonde avec `SONDE_SOURCES`.

**Ce qu'il faut savoir pour reprendre** :
- Un recul du banc peut être de la **visibilité** (des appels écartés
  reviennent, typés mais non résolus) ou de la **précision** (de faux liens
  par le seul nom écartés par le type) : le prouver chaque fois — références
  comptées avant/après, et un échantillon relevé un par un (exemple jetable
  sur `unresolved_references`, raison « un candidat, écarté »).
- `typed_bindings` (C++, autres langages) lie un nom à un type pour tout le
  scope, sans région ; `receveur::Bindings` (Rust) a des régions. Une
  variable de boucle C++ homonyme d'une autre variable de type différent
  perd son type (conflit → aucun).
- `choose_target` ne relie un receveur typé que par son type : un type
  externe ne crée jamais d'arête « nom ». Les arêtes « nom » viennent du
  repli « un seul définisseur » sur un receveur sans type.
- Worktree `~/git_workspaces/rag3db-cp` (branche `impl-pas-definisseur-2`)
  et `target-impact` (17 Go) : à effacer à la fusion de la branche. Clone
  `~/.cache/rag3weaver-build/cp-b2d` et `target-cp` : le chantier codeparsers,
  à garder tant que B2d continue.

**Mise à jour (vers 4 h)** : `impl-pas-definisseur-2` fusionnée par l'arbre
principal (`17cca6e17` … `a514e66ed`, pointeur codeparsers `06d6e92`) ;
worktree `rag3db-cp` et `target-impact` effacés. Reste sur le poste : le
clone `cp-b2d` et `target-cp` (chantier codeparsers, B2d), `wt-docs`.

## 11 octobre, matin — les appels sur une locale non typée, visibles (décision de Lucie)

**Codeparsers `d0c0e46`** (master). Un appel sur une locale qui reste non
typée était écarté en silence. La référence est désormais gardée, marquée
`untyped_receiver` ; le résolveur ne la relie jamais par le nom et la range
sous la raison « receveur non typé ». Une locale liée à une expression
différée (`let t = f()`, `auto t = f()`) porte la même marque : le type
différé reste lu s'il se trouve.

**Un écart corrigé avant le commit** : ma première version marquait aussi
les appels déjà visibles que le résolveur reliait par le nom dans le
fichier — 1 492 sur rag3weaver, à moitié justes sur un échantillon de 40
(`agent.run()`, `conn.execute()` justes ; `uuid.as_str()` vers
`Resolution::as_str`, `r.get(0)` vers `Catalog::get` faux). Ce n'était pas
la décision (« écarté en silence ») : la marque est resserrée aux appels
qui étaient écartés et aux locales à type différé. **Question ouverte pour
Lucie** : retirer aussi ces liens par le nom dans le fichier (marqués
« fichier », donc suivis par `impact` comme sûrs) ?

**Banc, corpus égal** : références d'appel rag3weaver 53 199 → 56 811
(+3 612 visibles, 7 044 « receveur non typé » entre fichiers) ; liens
d'appel comparés arête par arête avant/après : rag3weaver et codeparsers
identiques, IR identique, rag3db/src 0 perdue et 1 paire gagnée (juste :
`~MiniZStreamWrapper() → MiniZStreamWrapper::close()`). Un lien faux
trouvé en chemin et fermé : la classe agrégeait l'appel marqué d'une de ses
méthodes comme référence de type (`GraphTool → spec`, vers une fermeture
homonyme).

**Côté rag3weaver** (branche `receveurs-non-types`, vérifiée, proposée) : la
marque voyage jusqu'au rendez-vous (`MENTIONS.untyped_receiver`), et
`choose_target` ne fait plus de repli par le seul nom sans type lu ;
`usages` dit « (par le nom) » des usages venus du seul rendez-vous (défaut
antérieur : un rendez-vous non relié s'affichait comme un usage sûr).
Témoin `e2e_usages::un_appel_sur_une_locale_non_typee_est_visible_et_jamais_relie`,
rouge avant (une arête `run → update` par le nom), vert après.

**Suite (décision de l'orchestration pour Lucie, renversable)** : les
liens posés par le nom seul sur un receveur non typé ne sont ni retirés ni
laissés « fichier » ; ils prennent la marque « nom ». codeparsers `a83ba74`
(`metadata.by_name`, et au banc le compte neutre
`relations.consumes.par_le_nom`) ; second commit de la branche
`receveurs-non-types` (`718cfbe1d`, `CodeRelation.by_name` → résolution
« nom »). Témoin `e2e_usages::un_lien_par_le_nom_dans_le_fichier_se_marque_nom`,
rouge (« fichier ») puis vert.

**Correction d'un chiffre que j'avais donné** : les « 1 492 liens marqués
fichier » venaient du mode entre fichiers de codeparsers. En fichier seul
— le mode de rag3weaver — un receveur non typé passe au rendez-vous, déjà
marqué « nom » par le repli d'un seul définisseur. Les liens « fichier » par
le nom y sont **45 sur rag3weaver, 0 sur le C++ de rag3db** ; ce sont eux
que la marque corrige. Entre fichiers : 1 997 sur rag3weaver, 583 sur
rag3db/src.

## 11 octobre, après-midi — la stèle du produit, ma part

La stèle du produit (page de l'arbre principal, `docs/11-octobre-2026-arbre-principal/02-la-stele-du-produit.md`) :
j'y ai écrit l'ordre de mes lots, corrigé les colonnes de mes tickets et retrouvé le 21ᵉ ticket
(fermé à 11 h 28, avant la page).
- **T5** (fermeture locale, `map(aide)`) : codeparsers `d90a1f6`, sur master (`48ae8e4b5`).
- **Lot 2, deux recettes rejouées** (`e7950860a`, sur master) : le type d'un champ déclaré dans
  un autre fichier est fermé (B2b) ; la réexportation est rouge — avec un homonyme, aucune arête.
- **Lot 2 bis, la réexportation** : codeparsers `77c8ea6` (`ImportReference.reexport`) ; branche
  `reexportations-2` (`code.rs` : `MENTIONS.reexport_from`, `par_reexportation`), vérifiée,
  proposée. Témoin rouge (aucune arête) puis vert (« import »).
- **Les targets** : un seul pour la session, `target-codeparsers` (règle de midi) ; le banc sous
  lui.
- **Les e2e** sont un binaire unique depuis midi : `cargo test --test e2e -- --ignored
  e2e_usages::…` ; un rejeu après rebase doit le savoir (les cibles `--test e2e_usages`
  n'existent plus).
Suite : `analyze` en racine relative (P3), puis les prototypes C, les C++ anonymes, le nom seul.

**Suite de l'après-midi (stèle du produit)** :
- **Lot 3, `analyze` en racine relative** (branche `analyze-racine`) : la racine se résout et se
  dit ; le défaut avait changé de forme (chemins « absolus » relatifs et doublés).
- **Lot 4, les prototypes C** (codeparsers `9531860`, branche `prototypes-c`) : membres du scope
  de fichier ; un `struct` sans corps n'est plus une seconde définition. Arêtes de rag3db/src
  identiques.
- **Lot 5, les C++ anonymes** (codeparsers `35faa0f`, branche `cpp-anonymes`, en attente de la
  lib) : 287 → 0 sur rag3db. Les classes exportées (`class RAG3DB_API Foo`) se lisaient comme des
  fonctions ; masquées avant l'analyse, elles redeviennent des classes avec leurs méthodes, et les
  méthodes gabarits ont leur scope. **Appels reliés entre fichiers sur rag3db/src : 41,5 →
  50,7 %**, +1 362 méthodes. Le message du commit dit « 2 restants » : compté trop tôt, c'est 0.
- **Le target** : un sous-dossier par worktree (règle de l'orchestration, à partir du piège des
  artefacts partagés) ; `check_features.sh` avant tout hash.
Reste de ma part de la stèle : le nom seul et les segments de module (P1).
- **Lot 6, le nom seul et les segments de module** (codeparsers `a6111cc`, branche `nom-seul`,
  vérifiée, proposée) : un paramètre n'est plus une référence du bloc `impl` (96 faux liens
  retirés sur rag3weaver, arête par arête) ; un préfixe de chemin en minuscules n'est plus une
  référence (~7 000 non-résolues de moins sur rag3weaver). **Toute ma part de la stèle (P1, P4)
  est faite** ; restent les lots 5 et 6 à reprendre par l'arbre principal, puis la passe de
  contrôle (`e2e_banc_relations`, marques par résolution).
- **Pousser** : désormais par `poste push <session> -- <commande>` (verrou de master, par poste).

**Le premier index réel de rag3db par l'hôte** (demande de l'orchestration, 11 oct. après-midi) :
- **Le montage** : le gabarit `code` copié sous `~/.config/rag3weaver/code-rag3db/templates/`,
  manifeste actif `backends/code/rag3db.json` (nom `code-rag3db`) : racine l'arbre principal en
  lecture seule, commandes coupées, `edit_file` et `run_command` retirés, granite-278m sur
  127.0.0.1:7979, `locks_via` = `acquireLock`, `acquireLocks`, `lockRowForWrite` (à compléter de
  `lockIndexForWrite`). Base `~/.local/share/rag3weaver/code-rag3db-depot.rag3db`.
- **La mesure** : `estimate` : 7 310 fichiers (62,3 Mo), ~11 min de vecteurs, tampon ~8 Gio sur
  8. `index` par l'hôte : **12 min 27 s** (15 h 22 → 15 h 34 ; plein texte en ~30 s, 135 592
  morceaux), base **1,2 Go + 728 Mo** de plein texte, l'hôte à **9,8 Go résidents au pic**.
  Aucun mur. Instantané reflink de la mémoire longue avant l'index
  (`sauvegardes/2026-10-11-15h26/`).
- **La question du cœur** posée par le client, réponse recopiée et confrontée au source dans
  `04-ce-qui-manque-a-un-agent-dans-cet-ide.md` : un appelant manqué sur trois, le verrou de
  l'index absent d'`impact` (présent dans `callees`), 101 usages par le nom hors du C++.
- **Quatre tickets produit** (`c41e53986`) : on ne peut pas arrêter un index ; l'hôte ne relit
  pas un manifeste modifié ; pas de gabarit `code` en lecture seule ; on ne peut pas dire ce
  qu'on indexe.
- **La recette pour les sessions du cœur et d'hotfixs** (donnée à l'orchestration, non
  appliquée : la configuration de Lucie n'est pas à moi), sur le modèle de
  `templates/mcp/README.md` §6 :

  ```
  claude mcp add code-rag3db \
    --env LD_LIBRARY_PATH=/home/lucied/git_workspaces/rag3db/build/lecteurs-csv/src \
    --env RAG3DB_ROOT=/home/lucied/git_workspaces/rag3db \
    --env RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979 \
    -- /home/lucied/.local/bin/rag3weaver-backend mcp \
       --manifest /home/lucied/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json \
       --keys dev
  ```

  Le client est mince : il parle à l'hôte déjà lancé (127.0.0.1:7970). L'index est fait. La base suit l'arbre principal tel qu'il était
  à 15 h 22 : à réindexer après le rebâti de l'hôte avec les lots 5 et 6 (qualité C++).

**Fin d'après-midi (11 oct.)** :
- **La mesure des marques sur rag3weaver** (page `05-les-marques-sur-rag3weaver.md`) : quatre
  passes de la sonde sur un corpus figé, le lot 6 prouvé arête par arête (« import » −469 : 468
  visaient une déclaration de module, l'objet du lot).
- **Trois défauts trouvés par la mesure et par le cœur, corrigés** :
  - un mot de chaîne ou de commentaire n'est plus une référence (codeparsers `1f5506f`, branche
    `chaines-hors-references`) : 6 728 références retirées sur rag3weaver, toutes dans une
    chaîne ou un commentaire ;
  - une réexportation part du scope de son `pub use` (sur master : `5d0223d6e`, `39734d772`,
    relu par l'arbre principal) : les 300 arêtes « nom » sans ligne disparaissent ;
  - le nom du scope n'est plus un mot interdit dans son corps (codeparsers `af7bc52`, branche
    `nom-propre-par-position`) : l'appel récursif et `sink->acquireLocksBeforeExecution(…)`
    reviennent ; rag3db/src +810 relations.
  Les deux branches codeparsers reposent sur `nom-seul` et suivront sa reprise.
- **Tickets** : quatre manques du MCP (`c41e53986`), l'index ne suit pas l'arbre édité
  (`e7fcbe938`), `new` mot vide en Rust.
- **Suite** : l'envoi virtuel (le type de retour d'un gabarit à argument explicite, le champ,
  OVERRIDES au rendez-vous), la récursion au rendez-vous (`code.rs`, après nom-seul), puis
  l'index des arbres du cœur sur luciepc dès que l'hôte y est.

**Soir (11 oct.) — l'index des arbres du cœur sur luciepc** (hôte de luciepc, binaire master
a3c1ca98f, granite-278m sur 7878, lecture seule, sous le `poste lourd` de là-bas, après la
mesure `bascule2` du cœur qui tenait le verrou en exclusif) :

| arbre | commit | temps | base + plein texte | pic de l'hôte (VmHWM) |
|---|---|---|---|---|
| `rag3db-bascule` (le cœur) | `4ee70e386` | 450 s | 879 Mo + 578 Mo | 7,9 Go |
| `~/git_workspaces/rag3db` (`mtg-experiments`) | `ab95c3a2d` | 490 s | 1 014 Mo + 621 Mo | 12,1 Go (deux bases ouvertes) |

Manifestes `~/.config/rag3weaver/code-bascule` et `code-principal` (gabarit de `rag3db-hote`,
la montre `reindex_on_change` comprise). Témoin : un client local à luciepc, `usages
acquireLocksBeforeExecution` sur `code-bascule`, voit l'arbre du cœur ; sans mes lots, un seul
appel (ni la récursion, ni l'envoi virtuel). Recette du client d'ici : par le tunnel de G
(`docs/optimiseur/11-octobre-2026-16h40/01-l-hote-sur-luciepc.md`), le jeton lu sur luciepc —
la part de la session mémoire.

**L'envoi virtuel** : codeparsers `b473c3a` (gabarits explicites, `this` implicite) puis
`353d2c4` (la récursion fait une arête vers soi) ; branche rag3db `envoi-virtuel` en cours
(rendez-vous, `also_path` A/B et +, section « Par envoi virtuel » d'`usages`, l'arbre et le
commit en tête de chaque réponse).
