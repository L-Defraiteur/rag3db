# Cœur C++ — rapport de session

Session « cœur C++ » : le moteur (fork de Kuzu), son journal, sa reprise après arrêt,
l'index vectoriel, les lecteurs et écrivains concurrents, les verrous à venir.
Mis à jour sur place. **Dernière mise à jour : 4 octobre 2026, dans la nuit.**

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
| Relire ses relations ; voisins d'un vecteur mis à jour | `c8fdaf196` | une transaction relit juste ses relations après en avoir supprimé (défaut d'origine, par Cypher) ; la mise à jour d'un vecteur garde ses anciens voisins joignables |

A5, A5 bis et la garde 1 corrigent des défauts **atteignables en service avec un seul
écrivain**, pas seulement sous le mode multi-écrivains (qui reste éteint hors du banc).

## Ce qui est en cours

**La garde 2 de la reprise** (ordre de l'orchestration, 4 octobre) : rendre la demi-page —
version de stockage montée, marqueur dans l'en-tête, ou enregistrement réécrit en tête du
journal — avec une recommandation, puis la coder. Le repérage est dans le relevé de
connaissances, §1. Dans le même lot : le rejeu continue quand le fichier d'une extension
notée au journal a disparu (aujourd'hui la base ne s'ouvre pas).

**La mise à jour de vecteur, ce qui reste** (après la garde 2, avant V1 seulement si
l'invariant côté produit rougit) : quand presque toutes les lignes d'une table sont mises à
jour, il reste des lignes injoignables, en nombre variable d'une passe à l'autre. Piste :
un `finalize` de la mise à jour, comme pour la suppression — l'état de mise à jour de
l'index est recréé à chaque ligne, rien ne s'y accumule. À regarder aussi : `shrinkForNode`
reprend ses voisins à partir du second (`for (auto i = 1u; …)`), code de l'amont, non
examiné. Pas fait : la dimension 768, les dix mille lignes, la ligne lointaine à la
construction. Les brouillons et l'état de travail d'avant livraison sont dans
`annexes/set-de-vecteur/`.

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
