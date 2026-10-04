# Cœur C++ — rapport de session

Session « cœur C++ » : le moteur (fork de Kuzu), son journal, sa reprise après arrêt,
l'index vectoriel, les lecteurs et écrivains concurrents, les verrous à venir.
Mis à jour sur place. **Dernière mise à jour : 4 octobre 2026, 23 h 50, après la panne de mémoire de 22 h 51.**

Le registre commun est `docs/journal-des-chantiers.md` (§1 pour l'ordre et les
livraisons, §4 pour les décisions, §6 pour les défauts). Ce fichier dit ce que le journal
ne dit pas : comment reprendre, et pourquoi les choses sont dans cet ordre.

## Ce qui est livré (tout est sur `master`)

| Quoi | Commits | Ce que ça ferme |
|---|---|---|
| Marche T0 | `88f57e2ba`, `4c13619d1`, `a34538c9a` | après une erreur dans `BEGIN … COMMIT`, tout est refusé jusqu'au `ROLLBACK` |
| Marche A2 | `e81423a28`, `875da1772` | les offsets provisoires d'une transaction sont remappés au commit |
| Champ nul d'une liste de structures | `17c1ae41d`, `909c2573c` | `UNWIND $rows … CREATE` avec un champ `NULL` |
| Index vectoriel, deux défauts | `e64839489`, `13284a0fe` | un `DELETE` annulé ne rend plus l'index muet ; un survivant reste atteignable |
| Marche A5 | `1175cc0a2`, `e415e0277` | lecteurs contre suppression : informations de version et index des relations en mémoire sous verrou |
| Marche A5 bis | `7487fae08` | l'ajout en mémoire ne réalloue plus un bloc sous une recherche vectorielle |
| `DROP` d'index au rejeu | `f5acca417` | le rejeu retire aussi l'index de la table |
| Garde 1 de la reprise | `fcd9a7882` | une base ne plante plus à l'ouverture ; l'index se dit « en retard » |
| Garde 2 de la reprise ; borne du contrôle des voisins | `15fcc3474`, `110a65f15` | après un arrêt brutal l'index vectoriel reste juste sans être rebâti (`<base>.extensions`) ; la mise à jour d'un vecteur ne lance plus une recherche par ancien voisin (dix mille lignes par lots : 42 s au lieu d'environ 800) |
| Perte de relations au point de reprise | `80e3f2c32` | un point de reprise ne libère plus les relations des régions qu'il n'a pas réécrites (perte silencieuse, défaut d'origine) ; le plantage à la lecture après une relation créée puis supprimée |
| Gardes de mémoire ; taille exacte pour l'index vectoriel | `1ea49837f` | trois refus nommés à la place de trois écritures hors bloc (tableau des visités, graphe en mémoire, décalages du dictionnaire) ; l'index se dimensionne par le nombre de lignes, plus par la cardinalité estimée. Sans test neuf : voir le ticket |
| Après un `COPY` refusé | `05788a868` | le point de reprise n'écrit plus hors de son bloc (corruption de tas, défaut d'origine) ; la ligne d'origine d'une clé en double reste dans l'index de clé primaire (résultat faux silencieux, défaut d'origine) |
| Chargement journalisé, étape 2 (les nœuds) ; refus du `COPY` après des insertions | `0f4a54b2c` | derrière `force_checkpoint_on_copy=false`, un `COPY` de nœuds écrit ses lignes au journal et ne force plus de point de reprise ; dans une transaction, un `COPY` dans une table où elle a déjà inséré est refusé par son nom (il rendait des résultats faux, défaut d'origine) |
| Gestionnaire de verrous, marche V1 | `022c78402` | le gestionnaire seul, sans câblage : ressources ligne et index, partagé et exclusif, attente, interblocage à la prise, délai, interruption, prises groupées, `CALL lock_timeout` ; aucune écriture ne prend encore de verrou |
| Double ouverture en écriture dans un processus | `57c8389b4` | la cause de la corruption d'`e2e_code` : refus nommé `Database::ALREADY_OPEN_FOR_WRITING` avant de toucher à un fichier ; un lecteur du même processus reste permis |
| Clés fantômes après deux `COPY` annulés | `5c8507577` | régression de `05788a868` trouvée par l'arbre principal : l'annulation retire maintenant de l'index les clés de toutes les lignes non validées, et seulement elles |
| Chronométrage du point de reprise | `5771f0afb` | `RAG3DB_PROFILE_CHECKPOINT=1` : le découpage d'un commit et de son point de reprise sur la sortie d'erreur ; muet sinon |
| Relire ses relations ; voisins d'un vecteur mis à jour | `c8fdaf196` | une transaction relit juste ses relations après en avoir supprimé (défaut d'origine, par Cypher) ; la mise à jour d'un vecteur garde ses anciens voisins joignables |
| Chargement journalisé, étape 3 (les relations) | `1cfba2d6a` | derrière le même réglage, un `COPY` de relations écrit ses relations au journal (par le partitionneur, sous le verrou qui réserve leurs identités) ; un chargement entier est durable par le journal seul. Le témoin ne prouve pas l'ordre entre plusieurs fils |
| Sonde des deux sens | `168a63901` | une relation introuvable dans l'un de ses deux rangements fait refuser sa mise à jour ou sa suppression (`RelTable::REL_NOT_FOUND_IN_ONE_DIRECTION`) ; sans test, la condition ne se fabrique pas par l'interface |
| Chaînes permutées au point de reprise | `25b3b45dc` | défaut d'origine, résultat faux écrit sur disque : la relecture partielle d'un segment de chaînes échangeait les chaînes des lignes d'une région (propriétés `STRING`, `BLOB`, `STRUCT` à chaîne des relations ; listes de chaînes des relations **et des nœuds**). Garde `DICTIONARY_INDEX_OUT_OF_RANGE`. Ne répare pas : une base écrite avant se réindexe ; `tools/check_rel_directions` dit si des relations sont atteintes |

A5, A5 bis et la garde 1 corrigent des défauts **atteignables en service avec un seul
écrivain**, pas seulement sous le mode multi-écrivains (qui reste éteint hors du banc).

## Ce qui est en cours

**Après le correctif des chaînes (4 octobre, 23 h 50).** Tout est poussé, l'arbre
`rag3db-moteur` est propre, sur la branche locale `chaines-relecture-partielle` (égale à
`master`). La suite, dans l'ordre de l'orchestration : le vrai correctif du `COPY` après des
insertions (verser les lignes locales au début du `COPY`, ce qui retire le refus et rend leur
forme aux quatre tests Cypher) ; l'étape 4 du chargement journalisé avec le banc ; la forme
compacte des vecteurs au journal ; l'étape 5 ; puis le câblage des verrous.

Ce qui reste ouvert de ce lot :
- les listes de chaînes du carnet (`Note.labels`, `Snapshot.labels`) sont sur des nœuds et
  n'ont pas d'autre source que la base : le contrôle ne les voit pas, rien ne les rebâtit.
  À dire à Lucie ; l'étendue côté produit est au ticket ;
- Vela n'a pas été regardé pour ce défaut ; Kuzu le porte tel quel, Ladybug l'a corrigé ;
- `expectSameRows` (la comparaison bornée du test) n'a jamais été vue en rouge ;
- la passe ASan date d'avant la garde d'indice ;
- un rouge isolé, vu une fois dans une liste et jamais revu :
  `transaction~ddl~ddl_tinysnb.AddInt64PropertyWithoutDefaultRollbackRecovery`, « Cannot open
  file …db.kz.shadow » au premier `COPY` du `SetUp`. Pas de recette, pas de ticket.

**La panne de 22 h 51.** systemd-oomd a tué toutes les sessions. Cause établie : un
`EXPECT_EQ` rouge de gtest entre deux textes de 400 000 lignes (un test neuf du banc, joué
sur le moteur non corrigé, hors de `poste`) — gtest calcule la différence dans une table
quadratique. Mesuré sous plafond : 4 000 lignes, 186 Mio ; 20 000 lignes, plus de 4 Gio.
Le moteur n'y est pour rien (le même scénario sans gtest : 438 Mio). Depuis, `poste` met
chaque commande dans sa portée avec un plafond (`POSTE_MEM_MAX`, 40 Gio par défaut).

**La corruption de mémoire qui tue `e2e_code`** — tout est dans le ticket
`docs/tickets/2026-10-04-memoire-corrompue-dans-e2e-code.md` : les mesures (une mort sur 60
avec la bibliothèque de master, donc antérieure à la garde 2), la pile d'AddressSanitizer
(une écriture de 4 096 octets après un bloc de 32 768, dans `DictionaryColumn::scanValue`,
la lecture d'une colonne de chaînes longues par un balayage), ce qui a été essayé sans effet.

Où j'en suis exactement (16 h 40, seconde limite de l'orchestration atteinte : huit passes) :
- **Fait nouveau** : avec l'extension vector bâtie elle aussi sous ASan, un second débordement,
  dans l'extension — `VisitedState::contains` lit l'indice 255 d'un tableau de 211 octets
  dimensionné par la cardinalité estimée de la table (`query_hnsw_index.cpp:293`) ; son
  pendant `add` écrit un octet hors tableau. Même test, même base sur disque. Un rapport sur
  six passes réelles ; le rapport de `scanValue` n'est pas revenu.
- **La condition n'est pas établie.** Un contrôle « cardinalité < nombre de lignes » n'a
  jamais parlé en quatre passes : soit le retard est rare, soit c'est le voisin rendu par le
  graphe qui est faux. Détail, lectures et suite au ticket.
- L'arbre est propre (`110a65f15`), l'extension ordinaire est rebâtie. **`build/asan` porte
  encore la bibliothèque instrumentée** : la rebâtir avant de s'en servir. L'instrumentation
  est en patch dans `annexes/instrumentation-traque-4-octobre.patch`, avec
  `build-traque.sh` et `boucle-traque.sh`.
- Leçon : un contrôle provisoire se joue une fois seul avant d'entrer dans une boucle
  comptée — deux passes sur huit sont mortes de mes propres contrôles.
- Piège : l'extension vector sort dans `extension/vector/build/`, commun à tous les dossiers
  de build de l'arbre ; ninja ne la refait pas si le fichier d'un autre dossier est plus
  récent : supprimer le fichier puis rebâtir la cible.

**La stèle du moteur** (`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`, décision de
Lucie) : le point d'arrêt de version du moteur. J'y ai écrit le tri des tickets (§3.1) et
corrigé le §2. Leçon du jour, à garder : un contrôle plus strict que nécessaire (« la clé
mène dans la plage annulée ») a fait planter `copy_tests` — la liste complète l'a attrapé
avant le push ; ne jamais pousser sur les seuls tests du changement.

**La corruption d'`e2e_code` est close côté moteur** : deux exemplaires de la base ouverts
en écriture dans le même processus (ticket, avec la méthode qui l'a trouvée). Les outils de
la traque sont dans `annexes/` : `sonde-base-fautive/` (rouvrir une base abîmée, relire ses
tables, relever les plages de pages). Le journal des pages, lui, n'a pas été gardé en patch
(retiré de l'arbre sans copie, ma faute) ; il se refait en une demi-heure : une fonction qui
imprime l'opération, la plage et la pile (`backtrace`, adresses relatives par `dladdr`, à
résoudre par `addr2line`) quand le chemin du fichier porte un fragment donné par une variable
d'environnement, appelée dans `PageManager::allocatePageRange`, `freePageRange`,
`freeImmediatelyRewritablePageRange` et `finalizeCheckpoint`, dans
`FileHandle::writePagesToFile` et `removePageIdxAndTruncateIfNecessary`, et dans
`ShadowFile::applyShadowPages`.

**L'ordre, revu par Lucie le 4 octobre à 20 h 40 (stèle §3.2)** : le `COPY` hors journal
est la cause commune de ce qui a été payé ce jour-là ; **le chargement en masse journalisé
passe avant le câblage des verrous**. Donc : V1 (fait, `022c78402`) → le chargement
journalisé → A3′, A4′, V2, la maintenance de l'index au commit → les écritures parallèles.
Les autres tickets « bloque » sont au banc.

**Le lot en cours : le chargement en masse journalisé.** Tout est dans
`04-le-chargement-en-masse-journalise.md`, §7 bis pour l'avancement : étapes 1 et 2 faites,
**étape 3 (les relations) à faire**. Ordre décidé par l'orchestration pour la suite : étape 3,
puis le vrai correctif du `COPY` après des insertions (ticket « bloque la stèle » : verser les
lignes locales dans la table au début du `COPY`, ce qui retire le refus et rend quatre tests
Cypher d'origine à leur forme), puis l'étape 4 avec le banc (retrait du forcé et de son
remède 2a), la forme compacte des vecteurs avec une version du journal, l'étape 5.

Pour l'étape 3, ce qu'il faut savoir : `RelBatchInsert::initGlobalStateInternal` force
aujourd'hui le point de reprise ; la journalisation des relations doit suivre l'ordre du
stockage comme pour les nœuds ; le rejeu existant est `replayRelTableInsertRecord`. Le banc
prépare deux témoins (mort au milieu et juste après un `COPY` validé). Tenir le compte des
passes au fil de l'eau, erreurs comprises : l'orchestration le rend à Lucie.

**V1, ce qu'il faut savoir pour la suite** : `src/transaction/lock_manager.{h,cpp}`, un
`LockManager` par `TransactionManager`, `releaseAll` appelé dans `clearTransactionNoLock`.
Le genre de ressource « base » envisagé pour l'attente du départ des autres n'est pas
construit (voir l'ajout daté de la note de conception des verrous). Tests :
`test/transaction/lock_manager_test.cpp`, à rejouer sous ThreadSanitizer (`build/tsan`,
cible `transaction_test`) à chaque changement du gestionnaire.

**Le point de reprise de `COPY`, voie (c) : rendu.** Le découpage (mesure de la session des
embarquements avec `RAG3DB_PROFILE_CHECKPOINT=1`) : les 5 s par paquet sont celles d'une
seule table, `_index_blobs`, réécrite en entier à chaque point de reprise (48,9 s sur 75 à
une validation par paquet, 23,8 sur 28 à une validation tous les quatre). Le remède est
parti côté rag3weaver (pousser les blobs une fois, à la fin) ; côté moteur c'est un ticket
d'optimisation, hors stèle. La demi-page reste `03-le-point-de-reprise-de-copy.md` ; ses
voies (b) et (a) attendent.

**Leçons de la soirée, à garder.**
- Mon premier correctif de l'index (`05788a868`) reposait sur une lecture fausse de ce que
  l'annulation balaie. Avant de corriger un chemin d'annulation, imprimer ce qu'il relit
  réellement (plages, lignes rendues) : dix minutes d'instrument auraient évité la
  régression.
- Avant de poser un refus sur un chemin, chercher si la batterie d'origine le tient pour
  permis (`grep` dans `test/test_files`) : quatre tests Cypher inséraient puis copiaient dans
  une transaction, et une liste complète a été perdue à le découvrir.
- Un balayage de table (`NodeGroup::scan`) attend une sélection neuve à chaque bloc ; et le
  masque ne filtre pas les lignes d'un bloc qui porte des informations de version. Filtrer
  soi-même par décalage.
- Quand un défaut intermittent abîme un fichier, la première chose à obtenir est le fichier
  abîmé : des refus nommés à la place des plantages l'ont laissé sur disque, et vingt minutes
  de lecture de pages brutes ont dit plus que la journée d'hypothèses sur le code.
- Un témoin ne doit pas vérifier la seule ligne qui a révélé le défaut : celui de la clé en
  double ne regardait que la clé 5, et cachait que toutes les voisines étaient perdues.

## Les amonts et la licence (4 octobre 2026)

Décision de Lucie : on lit les amonts, on ne leur transmet rien ; tout reste sous LRSL ;
par défaut, **lire, ne pas copier**.
- Ladybug est sous MIT (« Copyright (c) 2022-2025 Kùzu Inc. — Copyright (c) 2025-2026
  Ladybug Memory Inc. », `LICENSE` au tag `ladybug-main-2026-08-31`). Notre `LICENSE` est la
  LRSL et renvoie à `NOTICE` pour la notice MIT de Kuzu ; `NOTICE` ne mentionne pas Ladybug.
- Ce qui a été repris, et comment : les gardes des index non chargés (`d2db8acb4`,
  `1ba0cc540`) — lues puis réécrites (`NodeTable::isWritableIndex`, l'index détaché, le refus
  nommé sont à nous) ; la ligne de `LocalRelTable::scan` (`ffe855873`) — écrite ici avant
  d'avoir lu la leur, retrouvée identique ensuite ; le `DROP` d'index au rejeu — correction
  différente de la leur. Aucun bloc copié ; non vérifié par un outil de comparaison.
- La règle : le message de commit dit ce qui vient d'où (« lu puis réécrit », commit de
  l'amont cité). Copier un bloc — le port de `e92346c97` serait le premier cas tentant — se
  demande avant ; c'est Lucie qui tranche l'ajout d'une ligne à `NOTICE`.

## L'ordre, et pourquoi

1. **La garde 2 de la reprise** : que le rejeu ait l'extension avant de rejouer. Sans elle,
   chaque mort pendant une écriture dans une table indexée coûte un index entier à rebâtir,
   sur le chemin de chaque première indexation.
2. **La mise à jour massive de vecteurs** : rare chez nous et contournable ; avant V1
   seulement si l'invariant côté produit rougit.
3. **Les verrous** (décision de Lucie : avant H4) : V1, A3′, A4′, V2, puis la maintenance de
   l'index vectoriel au commit — une marche du plan, pas un affinage : sans elle deux
   écrivains sur la même table indexée s'attendent du début à la fin.
4. **H4**, ce qu'il en reste (le doublon de clé qui empêche de rouvrir : A3′ et une reprise
   qui met de côté la transaction fautive).
5. Le port de la recherche par clé par ligne de Ladybug ; la marche A5 ter.

## Ce qui attend quelqu'un

- **Session de l'arbre principal** : reconnaître `is behind its table` à l'ouverture, faire
  `DROP_VECTOR_INDEX` puis `CREATE_VECTOR_INDEX`, et le dire dans le reçu d'ouverture.
  Regarder si `e2e_idempotent_registration` ferme vraiment avant de rouvrir.
- **Lucie** : supprimer sur `origin` les branches d'avant rebase
  (`a5-suppression-sure-entre-fils`, `-2`, `-3`, et celles listées au journal §1).
- **Une mesure au calme** du coût d'A5 bis sur `e2e_search` et l'ingestion (marche A5 ter) :
  jamais faite, le poste était trop chargé.

## Comment reprendre

**Les arbres.**
- `~/git_workspaces/rag3db-moteur` : mon arbre de travail. Builds : `build/moteur` (Release,
  tests, extensions `vector;geo`), `build/lecteurs-csv` (bibliothèque partagée pour Rust),
  `build/tsan` (ThreadSanitizer, cibles `concurrence_test` et `transaction_test`). Target
  cargo dans `extension/rag3weaver/target`.
- `~/git_workspaces/rag3db-docs-note` : arbre détaché, pour pousser les docs vers `master`.
- Ne rien écrire dans l'arbre principal `~/git_workspaces/rag3db`.

**Livrer une marche du moteur** (la liste, dans l'ordre) :
1. La liste C++ : `annexes/liste-cpp.sh` (build, `transaction_test`, `api_test`,
   `c_api_test`, `copy_tests`, les huit suites de stockage, le banc comparé à
   `known_red.txt`, les tests de l'extension vector sur disque et en mémoire, la batterie
   Cypher). Avec ThreadSanitizer quand la marche touche à la concurrence (la version d'A5
   bis du script le fait).
2. La passe Rust, une fois : `annexes/passe-rust-1.sh` puis `-2.sh` (lib, dix-huit suites
   e2e, binaires, neuf scripts Python). Annoncer cargo à l'orchestration avant.
3. Reposer sur `origin/master` sous un **nom de branche neuf** (jamais de push en force),
   vérifier par `git diff --stat` ce que master a apporté, pousser en avance rapide :
   `git push origin <branche>:master`.
4. Le journal des chantiers, depuis l'arbre des docs.

**Les pièges, tous rencontrés.**
- `ctest -R concurrence_test.known_red` depuis la racine du build ne trouve aucun test et
  sort 0. Lancer `compare_known_red.cmake` directement (voir `liste-cpp.sh`).
- Les binaires Rust demandent la feature `code` ; sans elle la construction échoue et les
  scripts Python passent quand même, sur d'anciens binaires. Lire le code de sortie.
- Un test de reprise qui ferme la base avec son point de reprise final ne rejoue rien :
  exiger un journal non vide juste avant de rouvrir. `CREATE_VECTOR_INDEX` et `COPY`
  écrivent eux-mêmes un point de reprise ; la reprise aussi, à la fin du rejeu.
- Un processus qui a déjà chargé l'extension vector ne voit pas les défauts du rejeu sans
  extension : rouvrir dans un processus neuf (`exec`).
- Un témoin vert avant comme après un correctif ne prouve rien : celui des relations relues
  de travers était vert parce que ses relations étaient encore en mémoire — il manquait un
  `CHECKPOINT`. Le jouer d'abord sans le correctif.
- Un compte de lignes joignables dans l'index varie d'une passe à l'autre : seuil « toutes
  joignables », plusieurs passes.
- La pile de `git stash` est commune à tous les arbres du dépôt : ne pas s'en servir.
- **Jamais d'`EXPECT_EQ` entre deux grands textes à plusieurs lignes** : comparer par `==` et
  écrire un résumé borné (`expectSameRows` dans `rel_string_property_checkpoint_test.cpp`).
  Rouge, gtest réserve (lignes + 1)² cases.
- Le pic de mémoire d'une passe : `/usr/bin/time` n'existe pas sur ce poste ; lire
  `memory.peak` du cgroup de la portée (`annexes/borne.sh`, à lancer sous `poste`).
- Laquelle des deux formes de requête (`->` ou `<-`) lit quel rangement d'une table de
  relations dépend du planificateur : aucune n'est une référence, on compare les deux.
- Une étendue annoncée d'après ses propres variantes n'est pas l'étendue du défaut : « nœuds
  et listes indemnes » était faux, la relecture du banc a montré l'appelant manquant
  (`ListColumn::scanSegment`). Chercher les appelants du code fautif avant d'annoncer.
- **Le verrou de poste** (depuis le 4 octobre) : tout ce qui est lourd — build, liste C++,
  passe Rust, boucle — se lance sous `flock -s ~/.cache/rag3weaver-build/poste.lock` ; une
  mesure de durée ou de mémoire sous `flock -x`. Deux mesures d'une autre session ont été
  faussées par une boucle et un build lancés sans prévenir.
- `compare_known_red.cmake` prend un argument de plus, `-DLONG_FILE=…/long.txt` ; sans lui
  il sort en erreur. Les cas longs du banc sont à part (`CONCURRENCE_LONG=1`).
- Reposer une branche pendant qu'un script de passe tourne : le script compile un arbre en
  conflit. Finir le rebase d'abord, lancer ensuite.
- Une ligne très loin des autres est injoignable dans un index bâti d'un coup : ne pas
  s'en servir comme sonde dans un test qui prouve autre chose.
- Le poste est partagé par plusieurs sessions : une mesure de temps faite sous charge ne
  départage rien. Noter la charge (`uptime`) à côté de chaque chiffre.
- Dans un nouvel arbre ou après un rebase : `git submodule update --init
  extension/rag3weaver/codeparsers`, et vérifier `git config user.email`.
- Le mode de permission peut refuser un push vers `master` : s'arrêter et le dire à Lucie,
  ne pas contourner. Un appel refusé a pu s'exécuter en partie : vérifier l'arbre.
- Mes messages aux autres sessions donnent des faits avec `fichier:ligne` ; ce qui n'est pas
  vérifié est dit non vérifié. Deux constats faux ont circulé ce soir (H4 « seulement sur
  une table indexée », le « second défaut » du `MERGE`) : tous deux corrigés au journal.

**Les annexes** (`annexes/`) : les scripts de livraison et de mesure, les tests brouillons
qui ont servi aux mesures (à remettre dans `test/transaction/` et dans son `CMakeLists.txt`
pour les rejouer), et les patchs essayés puis écartés. Ils vivaient dans
`~/.cache/rag3db-moteur-notes/`, qui n'est pas durable ; les journaux d'exécution y sont
restés.
