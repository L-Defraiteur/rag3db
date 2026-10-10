# Une extension chargée au rejeu n'est pas vérifiée contre le bâti du moteur

- **État** : corrigé `458ff7157` (10 octobre 2026, seconde session cœur C++) — témoin `ExtensionBuildIdTest` (`test/transaction/extension_build_id_test.cpp`), rouge avant (`2b6515a87`), vert après
- **Gravité** : blocage à l'ouverture (boucle sans fin) ou plantage, au lieu d'un refus nommé. Pas de perte constatée
- **Atteignable en service** : seulement si le moteur et l'extension ne sont pas du même bâti — une bibliothèque rebâtie sans son extension (ou l'inverse), une base ouverte depuis un autre arbre de travail, une extension livrée à part
- **Touche rag3weaver** : à l'atelier, oui (plusieurs arbres, une bibliothèque partagée rebâtie souvent) ; en produit, tant que l'extension est livrée avec le moteur, non
- **Ouvert le** : 5 octobre 2026, session cœur C++
- **Pour** : cœur C++

## Ce que c'est

Une base fermée sans point de reprise garde un journal. À l'ouverture — en écriture comme en lecture seule — le rejeu charge d'abord les extensions notées à côté de la base (`<base>.extensions`, un nom et un chemin **absolu** par ligne ; `ExtensionManager::loadExtensionsNotedBesideTheDatabase`, `src/extension/extension_manager.cpp`), parce que le journal écrit dans des tables dont elles portent les index.

Rien ne vérifie que le fichier chargé a été bâti pour ce moteur. Une extension d'un autre commit est chargée telle quelle ; ses structures ne sont pas celles du moteur.

## Ce qui a été vu

Un programme d'essai lié à une bibliothèque du moteur à `5771f0afb`, ouvrant en lecture seule la copie d'une base de l'arbre principal (journal de 2 587 octets, extension vector notée au chemin de l'arbre principal, bâtie sur un commit plus récent) : l'ouverture ne rend jamais la main — dix minutes de processeur, interrompue à la main. La pile :

```
pthread_rwlock_rdlock
CatalogSet::containsEntry
Catalog::containsFunction
VectorExtension::load            [libvector.rag3db_extension de l'autre arbre]
ExtensionManager::loadExtension
ExtensionManager::loadExtensionForRecovery
ExtensionManager::loadExtensionsNotedBesideTheDatabase
WALReplayer::replay
StorageManager::recover
Database::Database
```

Avec une bibliothèque du même commit que l'extension, la même ouverture passe (100 000 clés lues, fichiers inchangés). Vu une fois, à la main ; pas de témoin au dépôt.

## Ce qui est déjà couvert

- **Le chemin noté n'existe plus** (base copiée sur un autre poste, arbre retiré, dépôt déplacé) : ce n'est pas ce défaut, et c'est un refus nommé. Le chargement échoue, l'échec est gardé (`recoveryLoadFailures`), le rejeu continue sans l'extension, et l'index dont la table a été écrite sans lui est détaché. La base s'ouvre ; les requêtes qui ne passent pas par l'index répondent ; le premier usage de l'index est refusé par son nom — « Index … is behind its table …: rows were recovered from the journal while its extension was not loaded. Drop it and build it again. At recovery, extension … could not be loaded from … » (`extension/vector/src/index/hnsw_index_utils.cpp`, `throwIfBehindItsTable`). Témoins verts : `VectorIndexCrashReopenTest.TheNotedExtensionFileIsGone`, `…TheExtensionFileNamedByTheJournalIsGone`, `…AWriteWithoutTheExtensionIsRefusedByName` (`test/transaction/vector_index_crash_reopen_test.cpp`). Pas de réponse fausse.
- **L'ouverture en lecture seule devant un journal non vide** rejoue le journal en mémoire sans rien écrire (`WalTest.ReadOnlyRecovery*`).

## Ce qui est attendu, un jour

Une empreinte de bâti (le commit, ou une somme des en-têtes dont l'extension dépend) inscrite dans le moteur et dans chaque extension, comparée au chargement — au rejeu comme à `LOAD` ; refus nommé si elles diffèrent, traité au rejeu comme un chargement manqué (donc l'index dit « en retard », pas une boucle).

## Pour le fermer

Le contrôle, et un témoin : une extension d'une autre empreinte notée à côté d'une base à journal non vide ; l'ouverture passe, l'index se dit en retard avec la raison.

## Corrigé (`458ff7157`)

- **L'identifiant de bâti** : « rag3db-build-<commit>[-dirty-<diff>]-<options> », généré à CHAQUE
  bâti par la cible `rag3db_build_id` (`cmake/build_id.cmake`) dans `build/…/src/include/common/build_id.h`.
  `<commit>` est le dernier commit qui touche le moteur ou ses extensions (`src`, `extension` hors
  rag3weaver et lucivy, `third_party`, `cmake`, `CMakeLists.txt`), `<diff>` l'empreinte du diff de ces
  chemins ; un commit de docs, de tests ou de rag3weaver ne le change pas. Les options sont celles
  qui changent l'interface binaire (type de bâti, tailles, contrôles d'exécution, mono-fil,
  sanitizers), pas `BUILD_EXTENSIONS`. Déterministe ; sans dépôt git, « nogit ».
- **Le contrôle** : chaque extension exporte `build_id()` (vector, geo) ; `ExtensionManager::loadExtension`
  le compare à celui du moteur entre `name()` et `init()` : absent ou différent, la bibliothèque est
  déchargée et le chargement refusé — « Extension vector was built from <X>, this engine from <Y>:
  rebuild the extension with the engine. » Au rejeu, le refus est un chargement manqué : l'index
  se dit « is behind its table », avec cette raison. Aucune variable de contournement.
- **Conséquence** : toute extension bâtie avant `458ff7157` est refusée par un moteur d'après ; la lib et
  ses extensions se rebâtissent ensemble.
- **Ce que l'identifiant ne prouve pas** (remarque du banc) : deux variantes non commitées d'un même
  commit ont des identifiants différents seulement si leurs diffs diffèrent dans les chemins du
  moteur ; pour prouver quelle extension a tourné, l'empreinte du fichier et sa date restent la
  preuve.
