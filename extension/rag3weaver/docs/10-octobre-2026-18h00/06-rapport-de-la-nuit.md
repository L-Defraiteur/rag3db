# Mémoire longue — la nuit du 10 au 11 octobre

Suite du `03-rapport-de-session.md`, écrit à 23 h. Il est 02 h 20.

## Ce qui est entré dans master

| commit | |
|---|---|
| `69bb9f4a9` | le serveur MCP — protocole, sous-commande à deux modes, `--keys`, le témoin sur stdio, la recette, les deux pages de conception, les pièges 7 bis / 8 / 9 du journal |
| `2c2e512b3` | les deux aiguilles retirées avec le Cypher direct, remplacées par leurs successeurs |
| `79d56229e` | la recette « une base par session » **jouée**, les deux tickets, le témoin tolérant au compte, le `cfg` du banc |

Plus, dans la soirée : `bd25d7238` (filet des gabarits), `773c5528a` (section
`"reactions"`), `aeec6888f` (réparation des deux bancs + les deux derniers sites
du piège 6), `f7b1df13b` (contre-épreuve de la garde 2 + ticket), `cada06ac3` et
`d98758f19`.

**Hors de master, exprès** : le témoin des `initialize` concurrents, jamais
concluant, sur `memoire-temoin-demon-2` avec son état en tête.

## Ce qui est mesuré, et donc utilisable

**Le serveur MCP marche de bout en bout** sur un vrai manifeste : `initialize`
négocie, `tools/list` rend les outils du **manifeste** (douze puis treize avec
`callees`), un appel traverse `run_tool` et le bac à sable, un refus arrive au
modèle dans `content` avec `isError`, une notification ne reçoit pas de réponse.

**Le premier index d'un dépôt C++**, sur `rag3db-lourd` entier :

| | |
|---|---|
| retenus | 7 172 fichiers, 60,7 Mo (4 135 code, 3 037 texte) sur ~450 Mo |
| plein texte | prêt en ~2 min |
| vecteurs | 133 984 morceaux, ~18 min (`granite-278m` par le service) |
| total | **~20 min**, base **1,18 Go** |

**Et la réponse réelle à une question réelle** : `usages` de `update` sous
`src/storage/table/node_table` rend sa définition (`node_table.cpp:619`) et son
appelant, `replayNodeUpdateRecord` (`wal_replayer.cpp:722`).

**La liaison partagée vaut 76 Go sur une session** : 49 Go de target au lieu de
125 pour le même travail. C'était la cause du disque plein.

## Ce qui reste ouvert, et où

- **`--demon`** : ni validé ni réfuté — ticket `2026-10-11-mode-demon-ni-valide-ni-refute.md`,
  trois énigmes dont une qui pourrait être un défaut de `rag3daemon` (une lecture
  qui pend après le départ des écrivains) ;
- **l'indexation enfant du serveur** : ticket `2026-10-11-indexation-enfant-du-serveur-mcp.md`,
  avec la forme générique attendue et un remède immédiat séparable (que la
  réponse de `index` dise de quoi dépend le travail en fond) ;
- **le montage de `Reactor::watch_bound`** : `watch_bound` est sur master
  (rag3db-97), la signature est convenue, la montre du rechargement est à moi
  dans le binaire, et l'ordre est tranché (vérifier, arrêter, mettre en service,
  remonter). Avec le témoin « une réaction hors liste est refusée **à
  l'exécution** », puis `review_anchored` déclaré — ce qui donne enfin le premier
  témoin de la proposition du 5 octobre ;
- **la page « mémoire par MCP »** est écrite (`05`), trois ordres avec leur coût,
  recommandation A puis B.

## Ce que la nuit a appris

### Sept interfaces supposées, et la procédure qui l'évite

Sept fois j'ai écrit du code contre une interface que je **croyais** connaître :
`path` au lieu de `path_prefix` ; `list_files` pris pour un outil de disque
alors qu'il liste l'**indexé** ; `workspace.index` retiré en croyant ne retirer
que l'indexation (il retire **le schéma de code** entier) ; un `schema` d'entité
omis ; une déclaration d'embarquement omise (`load` l'exige **quel que soit le
signal**) ; le port `events` ; le séparateur ` -- ` d'un paramètre de fiche.

**Le remède n'est pas « lire avant d'écrire » — c'est une procédure : copier un
gabarit livré et en retirer.** Un gabarit est la liste exhaustive de ce qui est
exigé, et `src/gabarits.rs` la vérifie à chaque passe de la lib. Je l'avais sous
la main depuis le matin.

### Un contrôle qui masque un vert coûte plus qu'un contrôle absent

Ma garde lisait la **première colonne** de `SHOW_TABLES` comme un nom alors que
c'est un identifiant. Elle annonçait « l'entité Note doit exister une fois et une
seule », concluait qu'elle n'existait pas, **et les quatre ouvertures avaient
réussi**. Lue sans la ligne au-dessus, elle m'aurait fait condamner le mode
`--demon` et proposer un chantier de pont.

Même famille : `pgrep -c -x rag3weaver-backend` rend **0** sur un processus
vivant depuis 24 minutes qui tenait une base verrouillée — le noyau tronque le
nom de processus à 15 caractères. Un contrôle qui répond zéro parce qu'il **ne
peut pas voir** est indiscernable d'un contrôle qui répond zéro parce qu'il n'y a
rien. (`pidof`, filtré par `/proc/<pid>/exe`, est la forme juste — rag3db-10.)

### Jouer une recette la corrige ; l'écrire ne suffit pas

Cinq erreurs dans ma propre page MCP, trouvées en l'exécutant : les graphes rangés
hors du dossier du gabarit ; les chemins absolus qui meurent avec un worktree ;
l'indexation enfant du serveur ; les « 7 min » qui ne couvrent que les vecteurs ;
une méthode C++ qui se cherche par son nom nu avec un `path`. **Aucune n'était
visible à l'écriture**, et les deux premières auraient envoyé les sessions cœur
C++ dans un mur que je venais de documenter de travers.

### Sur un master qui bouge, une vérification exhaustive ne converge jamais

Master a pris 21, puis 14, puis 4, puis 52 commits pendant que je cherchais à
fusionner. Chaque rebase invalide la compilation ; recompiler laisse master
repartir. La méthode qui a marché : **vérifier large une fois, puis étroit et
vite à chaque rebase, en nommant ce qu'on ne revérifie pas.**

### Les annonces croisées valent mieux qu'aucune vérification locale

Trois fois cette nuit, une annonce d'une autre session m'a évité de publier un
résultat faux — l'âge d'une lib relevé par réflexe, la correction de rag3db-73
sur le « bâti à part », et son rebâti annoncé **avec les deux horodatages et les
deux empreintes**, qui m'a fait voir que ma passe était contaminée de 82
secondes.

**Et j'ai cassé ce protocole une fois** : j'avais dit « fini, la lib est libre »,
puis j'ai relancé deux passes sans le redire. rag3db-73 a rebâti de bonne foi sur
une information que j'avais périmée en silence. La règle qui en sort : **un
« libre » tient jusqu'à mon prochain lancement, pas jusqu'à la fin de la nuit.**

### Se tromper par excès de prudence a un coût

J'ai averti rag3db-10 d'un piège qui ne le concernait pas, en généralisant une
note du journal au-delà de sa portée (« le même arbre », pas « le même poste »).
Une attente inutile, et un avertissement qu'on finit par ne plus écouter.

### Et le motif de la journée, rencontré sept fois

**Une information produite que rien ne consulte.** `Reactor::watch` sans
appelant ; `getRecoveryLoadFailures()` public et jamais lu ; `DataflowRecorder`
appelé seulement par des tests ; un uuid d'exécution fabriqué puis jeté dans la
même fonction ; la prose d'un banc annonçant 66 aiguilles quand il en avait 63 ;
`recoveryLoadFailures` qui consigne un échec que personne ne relit ; et
l'indexation qui dit son avancement dans un journal qu'elle emporte en mourant.

Le septième cas est le plus instructif et c'est le mien : j'ai failli accuser
`SearchOptions.scopes` d'être une déclaration morte alors qu'il **est** branché,
parce que je venais d'en voir six vraies. **Un motif qu'on reconnaît bien devient
un motif qu'on voit partout.**

## Ménage

Effacé après chaque fusion, sur les deux postes, après vérification par
`/proc/<pid>/cwd` : `target-memoire` (125 puis 54 Go), `target-aiguilles`
(5,6 Go), `target-recette-ici` (7,7 Go), `target-recette` et `target-demon`
(11,7 Go sur luciepc), et cinq worktrees. Le disque principal est passé de 63 %
à **51 %**.

**Un coût mesuré à porter à Lucie** : un target neuf par lot coûte **8 minutes de
bâti à froid pour une vérification de 3 minutes**. La règle a rendu 101 Go et le
disque était le vrai problème ; garder le target **entre les lots d'une même
session** et l'effacer à la fin de la session donnerait les deux.

**Gardée** : la base indexée de `rag3db-lourd` (1,1 Go sur luciepc), avec
l'accord du cœur C++ — elle est le point de départ des sessions C++, et sa
recette est écrite.
