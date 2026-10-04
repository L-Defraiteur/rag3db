# Cœur C++ — rapport de session

Session « cœur C++ » : le moteur (fork de Kuzu), son journal, sa reprise après arrêt,
l'index vectoriel, les lecteurs et écrivains concurrents, les verrous à venir.
Mis à jour sur place. **Dernière mise à jour : 4 octobre 2026, 17 h.**

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
| Relire ses relations ; voisins d'un vecteur mis à jour | `c8fdaf196` | une transaction relit juste ses relations après en avoir supprimé (défaut d'origine, par Cypher) ; la mise à jour d'un vecteur garde ses anciens voisins joignables |

A5, A5 bis et la garde 1 corrigent des défauts **atteignables en service avec un seul
écrivain**, pas seulement sous le mode multi-écrivains (qui reste éteint hors du banc).

## Ce qui est en cours

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

**Le lot en cours** : le point de reprise que `COPY` force — 66 s sur 177 du premier index
de ce dépôt avec la transaction par paquet. La demi-page est `03-le-point-de-reprise-de-copy.md`.
Décision de l'orchestration : la voie (c) d'abord — mesurer ce que font les 5 s d'un point de
reprise de paquet, **rendre le découpage (somme égale au total) avant de modifier**, puis
alléger ; une demi-journée au plus. La voie (b) touche à la durabilité : elle passera par
Lucie, après une mesure côté rag3weaver. La voie (a) : au journal comme le vrai remède, pas
maintenant. Puis V1, sur message de l'orchestration.

**Ce qui attend derrière**, dans l'ordre de l'orchestration : la revue des amonts de la
session du banc (`…/banc-de-concurrence/03-revue-des-amonts.md`, tickets dans
`docs/tickets/`) — proposer un ordre et une estimation pour les plantages et blocages de
stockage restants, dont le `CHECKPOINT` qui tue le processus après `ALTER TABLE … DROP`
d'une colonne ; l'échec du point de reprise sous petit tampon (recette `62a91829f`) ; le mode
« chargement initial » du `COPY` si regrouper les `COPY` dans une transaction ne suffit pas ;
puis V1. La mise à jour massive de vecteurs (un `finalize` de la mise à jour), la dimension
768 et la ligne lointaine à la construction : après, sauf rouge en usage réel. La mesure
d'A5 bis au calme (A5 ter) : jamais faite, sous `poste mesure`.

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
