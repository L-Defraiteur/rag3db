# Archive du correctif — inventaire lu et build privé proposé

Porteur : cœur C++. Même [ticket d’annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md).
Aucune compilation, configuration CMake, reconstruction, mesure ou nouveau worktree
créé pour cet inventaire. Aucune demande de créneau avant partage de cette proposition.

## Résultat de l’inventaire en lecture seule, 12 novembre

Périmètre vérifié à luciepc : `~/.cache/rag3db-moteur-notes/annulation/` (sauvegardes
et journaux de ce lot), puis `~/git_workspaces/rag3db-colonnes/build/moteur/src/`
(archives et objets courants), et les dépendances du build sous `third_party/`.
Sur le portable : worktree `~/git_workspaces/cx-coeur-annulation` et emplacement
`~/.cache/rag3db-moteur-notes/annulation` (absent). Ce n’est pas une recherche
exhaustive dans les sauvegardes de Lucie ou celles d’autres sessions.

- Notes du lot distant : **aucune archive `.a` ni aucun objet `.o` sauvegardé**.
- `binaires-0511dece7/` : `concurrence_test` (31 333 672 octets),
  `local_hash_index_test` (27 032 056), `transaction_test` (31 617 280),
  `SHA256SUMS` (464). Ces exécutables ne sont pas des archives liables.
- Build courant `src/` : 741 objets (61 179 080 octets au total), une archive
  statique et une lib partagée. Ce sont les produits de la base.
- Dépendances courantes : 251 objets (10 313 264 octets), 18 archives
  (10 881 604 octets). Aucune ne remplace l’archive moteur du correctif.

| Produit courant à luciepc | Taille | SHA256 |
|---|---:|---|
| `rag3db-colonnes/build/moteur/src/librag3db.a` | 65 685 818 | `40dca10ed39bca8c45769f086e3bfd95fe0f6502876fc12482b33c44ddd662cc` |
| `rag3db-colonnes/build/moteur/src/librag3db.so` | 26 451 152 | `46994ae078dd13764f143c22cfc6af303130134ba255cdcccf2d1664d474c6ba` |

Les deux bibliothèques datent du 11 novembre à 22:04:37. Les objets pertinents
ont aussi été remplacés :

- `src/storage/table/CMakeFiles/rag3db_storage_store.dir/node_table.cpp.o` :
  553 064 octets, 22:04:31,
  SHA256 `4e0d0b0fa70f6851277cec56ce0c49b1fa55540de0d7861ab67a7cff68110c74` ;
- `src/storage/index/CMakeFiles/rag3db_storage_index.dir/in_mem_hash_index.cpp.o` :
  459 080 octets, 22:04:26,
  SHA256 `1fe8673051110099346bfe61c960a4ec467af88d82b6b89fd0e85dc6497ef1bd`.

Source courante contrôlée : HEAD `6e7f116806034c7ac6eb3bf2f6309d1c8aaf1b93`,
diff vide contre le parent `65486c02d` pour `src`, `test`, `extension/vector`,
`extension/geo`. L’archive du correctif **n’est pas disponible avec provenance
établie dans les emplacements vérifiés**. Ne pas réassembler des morceaux
d’exécutables ou des objets de provenance partielle pour prétendre la retrouver.

## Ce que prouvent les journaux conservés

- `annulation/vert.log` : lignes de build `[86/142] Linking CXX static library
  src/librag3db.a`, `[87/142] Linking CXX shared library src/librag3db.so`,
  puis `[142/142]` concurrence_test. Le correctif avait donc bien une archive
  statique pendant sa validation. Le journal finit le 11 novembre à 18:27:19.
- `annulation/base.log` : `[82/141]` relie la même archive statique, `[83/141]`
  la même lib partagée ; les tests de base finissent à 22:24:33.
- `vert.sh` ne sauvegardait pas l’archive ; `base.sh` remplaçait les sources du
  lot puis rebâtissait dans ce même dossier. La sauvegarde intermédiaire ne
  copiait que les trois exécutables. Aucune archive du correctif à revendiquer.

Les preuves de validation, y compris celles référencées par `b06fbbff3`, et tous
les journaux terminés restent conservés. Aucun effet sur leurs verdicts historiques.

## Proposition minimale, non exécutée

Il s’agit explicitement d’un **build privé du moteur statique**, distinct de la
compilation du pilote. Il doit être autorisé avec son propre créneau et coût.
Pas de magasin `rag3db-libs`, lien `courant`, release, service, hôte, extension
ou lib distribuée touché. Ne pas reconstruire le correctif dans le build courant
qui porte maintenant la base.

1. Après accord sur le créneau, ouvrir un worktree privé `cx-` du **même lot** sur
   `0511dece7`, sans rebase ni modification source, et un build neuf hors `/tmp`.
   Pas de retrait du parent ni de nettoyage d’un ancien worktree pour faire place.
2. Enregistrer le commit complet, le diff vide, le compilateur et sa version,
   les options effectives du build de référence. Le cache contient des valeurs
   BOOL `OFF` pour les tailles, mais CMake applique les défauts 12/11/17/18 :
   **ne pas recopier `OFF` comme taille**.
3. Configurer Release et cible statique seulement ; comparer les définitions
   effectives à celles du build validé avant compilation. Une divergence
   d’interface non expliquée arrête la préparation. Ne pas réutiliser des objets
   du build de base sous des noms qui suggèrent un build homogène du correctif.
4. Avant compilation, lire la liste prévue par `ninja -n` dans le build privé,
   compter objets et dépendances, vérifier disque disponible ; arrêter si les
   plafonds proposés ne suffisent pas. Pas de téléchargement implicite.
5. Construire sous `poste lourd`, `-j8`, **cible `rag3db` seulement** (statique,
   contrairement à `src/librag3db.so`). Pas de test ou de mesure en parallèle.
6. Conserver archive, en-têtes source/générés, dépendances tierces et manifeste
   avec leurs SHA256, options, commit, commande et résultat. Aucun install/copie
   au magasin. Le pilote ne sera compilé qu’après revue de ce manifeste.

### Commandes à relire, pas à exécuter à cette étape

Les chemins sont proposés et n’ont pas été créés. Réserver configuration,
compilation et groupe de processus comme un travail lourd unique. Le lanceur doit
appliquer le délai au groupe qu’il crée et ne toucher aucun autre processus.

```bash
# Seulement après autorisation distincte et contrôle des plafonds :
git -C "$HOME/git_workspaces/rag3db" worktree add --detach \
  "$HOME/git_workspaces/cx-coeur-archive-0511" 0511dece7
cmake -S "$HOME/git_workspaces/cx-coeur-archive-0511" \
  -B "$HOME/.cache/rag3db-moteur-notes/annulation/cx-build-archive-0511" -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_TESTS=OFF -DBUILD_EXTENSION_TESTS=OFF -DBUILD_SHELL=OFF \
  -DBUILD_SINGLE_FILE_HEADER=OFF -DBUILD_EXTENSIONS:STRING= \
  -DENABLE_RUNTIME_CHECKS=OFF -DSINGLE_THREADED=OFF \
  -DENABLE_ADDRESS_SANITIZER=OFF -DENABLE_THREAD_SANITIZER=OFF -DENABLE_UBSAN=OFF \
  -DENABLE_MALLOC_BUFFER_MANAGER=OFF -DENABLE_LTO=OFF \
  -DENABLE_BACKTRACES=OFF -DUSE_STD_FORMAT=OFF \
  -DRAG3DB_PAGE_SIZE_LOG2:STRING=12 -DRAG3DB_VECTOR_CAPACITY_LOG2:STRING=11 \
  -DRAG3DB_NODE_GROUP_SIZE_LOG2:STRING=17 -DRAG3DB_MAX_SEGMENT_SIZE_LOG2:STRING=18
ninja -C "$HOME/.cache/rag3db-moteur-notes/annulation/cx-build-archive-0511" -n rag3db
# Après revue des définitions et du plan sec seulement, toujours sous poste lourd :
cmake --build "$HOME/.cache/rag3db-moteur-notes/annulation/cx-build-archive-0511" \
  --target rag3db -j 8
```

### Coût, plafonds et inconnues

Le build de référence comporte 741 objets moteur et 251 objets tiers : un build
propre peut refaire cet ordre de grandeur, **pas une seule unité de pilote**.
Les produits observés totalisent environ 148 Mo pour objets moteur/tiers et
archives ; ce n’est pas le coût disque complet du nouveau worktree et du build.
Leur temps de compilation propre n’est pas connu : les 142 étapes du rebâti
précédent ne permettent pas de promettre une durée de build propre.

Proposition à discuter pour le créneau : un seul lourd, `-j8`, 30 minutes globales
configuration+build, scratch de build plafonné à 2 Gio, mémoire maximale du groupe
8 Gio ; lecture du plan et estimation avant compilation. Ce sont des limites
proposées, **pas un engagement de fin**. Mesurer séparément l’espace du worktree
avec les tailles des fichiers suivis avant sa création, et tenir compte de la
place libre. Si le plan ne tient pas, revenir avec les chiffres, sans élargir
silencieusement. Le lanceur de limites n’est pas encore préparé/validé.

Aucune compilation autorisée à ce point. La mesure reste `ready=false` ; les
15 minutes de son propre plan sont un plafond GLOBAL distinct du futur budget
éventuel du build privé. Pas de benchmark sans lien assorti présenté comme
explication de la régression. Le différentiel DdlEveryMinute reste ouvert.

## Préflight des limites — préparation autorisée ensuite par orchestration

Sondes légères, sans moteur/pilote/configuration : namespaces user/mount/net
disponibles ; unité utilisateur systemd avec `MemoryMax=8589934592`,
`MemorySwapMax=0` vérifiés dans les fichiers cgroup du processus. Un tmpfs privé
bwrap de 8 Mio a refusé le neuvième Mio par ENOSPC. La présence de bwrap seule
n’était pas la preuve ; cette sonde vérifie l’application réelle de la limite.

Le build prévu utilise donc un **tmpfs privé borné à 2 Gio sous un chemin de
cache**, compté dans les 8 Gio du groupe cgroup, jamais `/tmp`. Racine, sources
et build de référence montés en lecture seule ; seul le dossier de résultats
privés est lié en écriture. Namespace réseau fermé et FetchContent disconnected.
Le tmpfs est libéré par la fin du namespace ; aucun nettoyage du worktree parent.
Les artefacts requis sont copiés et empreintés dans les résultats avant cette fin.

Lanceur préparé : [cx-build-archive-privee.py](outils/cx-build-archive-privee.py).
Il vérifie cgroup/scratch/source exacte propre, configure, compare les commandes
de tous les objets src/ et les versions compilateur, produit le plan sec, puis
**attend une gate de revue liée au SHA du plan et au commit**. Toute différence
ou absence de gate arrête avant build. Aucun test, pilote, mesure ou install.
Le wrapper systemd doit imposer `RuntimeMaxSec=1740`, `TimeoutStopSec=5`,
`KillMode=control-group` : marge conservatrice sous le plafond global de 30 min.
La gate n’autorise que `cmake --build … --target rag3db -j 8`.

Coordination : arbre confirme son ciblé terminé et ne réserve pas de créneau ;
il accepte deux places partagées sous poste lourd, sans mesure et sans source
commune. Recherche CI sollicitée, réponse attendue avant lancement moteur.
Poste lu à 0/4. Aucun nouveau build moteur commencé à ce point.

Source complète envisagée : 7 310 blobs suivis, 444 673 124 octets (lecture git).
Place libre distante lue : 1 386 872 340 480 octets. La création du worktree et
l’unité de configuration n’ont pas encore été lancées à ce point du suivi.
