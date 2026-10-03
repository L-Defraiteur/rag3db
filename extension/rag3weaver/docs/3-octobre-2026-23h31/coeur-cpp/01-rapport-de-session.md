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

A5, A5 bis et la garde 1 corrigent des défauts **atteignables en service avec un seul
écrivain**, pas seulement sous le mode multi-écrivains (qui reste éteint hors du banc).

## Ce qui est en cours

**Le `SET` d'un vecteur vers un autre perd des lignes dans l'index** (en service, sans
arrêt). Branche locale `set-de-vecteur-et-index`, **rien de commité** ; l'état de travail
est dans `annexes/set-de-vecteur/` (`etat-de-travail-complet.patch` s'applique sur master
`fcd9a7882` et suivants).

Ce qui est établi (C++ seul, 1000 lignes de dimension 4, recherche exhaustive depuis une
sonde, lignes joignables sur 1000) :

| Variante, sur master | ligne à ligne | lots de 512 |
|---|---|---|
| `SET` depuis NULL, les mille lignes | 1000 | 1000 |
| `SET` vers un autre vecteur, les mille lignes | 969 | 532 |
| vingt `SET` vers le même vecteur | 1000 | 998 |
| supprimer puis réinsérer la ligne | 1000 | 1000 |
| repasser par NULL (deux `SET`) | 1000 | 599 |
| retirer l'index, `SET`, recréer l'index | — | 1000 |

- **Le compte n'est pas stable d'une passe à l'autre** (le tirage des niveaux de l'index) :
  la session du banc a mesuré 779, 593, 772 pour la même variante. Tout témoin doit être
  joué plusieurs fois, avec le seuil « toutes joignables ».
- Le chemin principal de rag3weaver (morceaux insérés sans vecteur, puis vecteurs posés)
  n'est pas touché. Le réembarquement d'une ligne gardée l'est ; la session de l'arbre
  principal le dit rare et n'ajoute pas de contournement. **Contournement sûr s'il en faut
  un** : supprimer puis réinsérer la ligne ; en masse, retirer et recréer l'index.
  Repasser par NULL n'est pas sûr par lots.
- Ladybug n'a ni mise à jour ni suppression dans son HNSW : rien à reprendre, tout ce
  chemin est notre greffe `98e35566a`.

Trois causes, où j'en suis de chacune :
1. **`OnDiskHNSWIndex::update` ne gardait pas joignables les anciens voisins** de la ligne
   (la suppression le fait depuis `13284a0fe`). Corrigé dans l'état de travail
   (`correctif-1-voisins-joignables.patch`). Effet mesuré seul : ligne à ligne 969 → 1000,
   vingt vers le même vecteur en une instruction 998 → 1000.
2. **Dans une instruction à plusieurs lignes, l'index relit de travers ses arêtes non
   validées** : le nœud qu'il vient de réinsérer rend 59 fois le même voisin au lieu de 59
   voisins. Les arêtes écrites sont justes (relues après le commit) ; c'est la lecture dans
   la transaction qui est fausse, et tout ce qui relit ses voisins (élagage, réparation)
   travaille alors sur du faux. `correctif-2-selection-des-relations-locales.patch`
   (`LocalRelTable::scan` pose `setToUnfiltered` au lieu de changer la seule taille du
   vecteur de sélection) rend la lecture juste. **Mais je n'ai pas compris par où le
   vecteur de sélection arrive périmé** : mon témoin au niveau du moteur
   (`uncommitted_rels_scan_test.cpp`, par `MATCH` et par `graph::OnDiskGraph`) est vert
   avant comme après. À faire : trouver ce qui distingue le chemin de l'index (une
   instruction à plusieurs lignes, `detachDelete` de toutes les arêtes sortantes puis
   réinsertion) de ce témoin.
3. **Une grosse transaction de mises à jour échouait** sur « bitset::set: __position (which
   is 2049) >= _Nb (which is 2048) » (mille `SET` en une instruction, ou vers la 465e ligne
   d'un `BEGIN … COMMIT`). Disparu avec le correctif 2 : c'était sans doute la lecture
   fausse qui faisait demander plus de 2048 vecteurs d'un coup. Non vérifié autrement.

Avec les correctifs 1 et 2, plus aucune instruction n'échoue, mais **il reste des pertes**
(par exemple 865 ligne à ligne, 491 par lots de 512 dans une passe ; 1000 et 964 dans une
autre) : quand toutes les lignes sont mises à jour, l'élagage des voisins retire des arêtes
entrantes à des nœuds que personne ne recontrôle. Piste : un passage de contrôle en fin
d'instruction sur tout ce qu'elle a touché — il faut pour cela un `finalize` de la mise à
jour, comme celui de la suppression ; l'état de mise à jour de l'index est aujourd'hui
recréé à chaque ligne. À regarder aussi : `shrinkForNode` reprend ses voisins à partir du
second (`for (auto i = 1u; …)`), donc laisse tomber le plus proche — code de l'amont, non
examiné.

Pas encore fait : la dimension 768, les dix mille lignes, la ligne lointaine injoignable à
la construction.

Le repérage de la garde 2 est rendu (voir le relevé de connaissances, §1).

## L'ordre, et pourquoi

1. **Le `SET` vecteur → vecteur** (orchestration, 3 octobre au soir) : un index faux en
   service passe devant un confort.
2. **La garde 2 de la reprise** : que le rejeu ait l'extension avant de rejouer. Sans elle,
   chaque mort pendant une écriture dans une table indexée coûte un index entier à rebâtir.
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
