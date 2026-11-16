# Le fichier de déversement garde son compte de pages après chaque requête

- **État** : fermé (16 novembre 2026, hotfixs) : `clearFile` remet le fichier de déversement à la page 0 (bc894ccfc, pile `spill-file-restarts-at-zero` puis `clearfile-keeps-live-spills`).
- **Famille** : moteur
- **Gravité** : à établir par un témoin. Un hôte qui vit longtemps, et dont les COPY déversent, voit grandir sans fin
  le compte de pages du fichier temporaire. Il finit peut-être sur « No more frame groups can be added to the
  allocator », et chaque déversement écrit au-delà de ce compte, d'où un `<base>.tmp` creux.
- **Atteignable en service** : seulement sous déversement (un COPY qui manque de mémoire). Aucun `<base>.tmp` n'existe
  aujourd'hui sur les deux postes : les hôtes actuels ne déversent pas.
- **Touche rag3weaver** : par le moteur
- **Pour** : hotfixs

## Ce qu'on lit

`ClientContext` appelle `Spiller::clearFile()` après chaque requête, réussie (`client_context.cpp:594`) ou en échec
(`:607`). `clearFile` fait `getFileInfo()->truncate(0)` sur le fichier temporaire, mais ne touche pas au `FileHandle` :
son `numPages`, ses `pageStates` et ses `frameGroupIdxes` restent. Le déversement suivant
(`Spiller::spillToDisk`) fait `addNewPages(n)` à partir de l'ancien compte, puis écrit à `startPage * pageSize`. Le
fichier redevient donc creux, et chaque groupe de 1 024 pages ajouté réserve un groupe de cadres de plus dans la
région du gestionnaire de tampons. `FileHandle::resetToZeroPagesAndPageCapacity` existe, mais `clearFile` ne
l'appelle pas.

## Le témoin et le correctif (16 nov., hotfixs)

`test/storage/buffer_manager_test.cpp`, `EmptyBufferManagerTest.ClearingTheSpillFileStartsTheNextSpillAtPageZero` : un
déversement, `clearFile`, puis le même déversement. Rouge sur master (bâti Release à luciepc) : `file_size` vaut
262144 au lieu de 131072. Le second déversement s'est écrit au-delà du compte gardé, et le fichier est creux.

Le correctif (bc894ccfc) : `FileHandle::restartAtPageZero()`, sous `fhSharedMutex`, remet `numPages` à 0 et tronque
le fichier. Il garde la capacité et les groupes de cadres, qui resservent aux pages suivantes. `Spiller::clearFile`
l'appelle à la place du seul `truncate(0)`. Je n'ai pas repris `resetToZeroPagesAndPageCapacity`, parce qu'il
raccourcit `frameGroupIdxes` sans rendre les groupes à la `VMRegion`, qui en réserverait donc de nouveaux à chaque
déversement.

Le lien avec le premier événement du ticket de confort
(`docs/coeur-cpp/tickets/2026-11-08-un-sigsegv-dans-optimisticread-…`) reste à établir : `<base>.tmp` n'est pas le
fichier de données. Le témoin prouve la forme du défaut (des écritures posées au-delà d'un compte gardé, un fichier
creux), pas le premier événement.

## Pour fermer

1. Un témoin rouge : avec un petit pool, deux COPY qui déversent, dans deux requêtes de suite. Après la seconde,
   `numPages` du fichier de déversement repart de zéro, et la taille apparente de `<base>.tmp` ne dépasse pas ce
   qu'une seule requête a déversé.
2. Le correctif : `clearFile` remet le `FileHandle` à zéro sous son verrou (pages, états, groupes de cadres rendus),
   en plus de tronquer le fichier.

## Fermeture (16 nov.)

Le témoin `ClearingTheSpillFileStartsTheNextSpillAtPageZero` est rouge sur master (262144 octets au lieu de 131072)
et vert avec le correctif bc894ccfc.

La passe de la pile, à luciepc, sur sa tête 3311437745 (bâti Release à moi, `build/hotfixs-moteur`) :

| Suite | Résultat |
| --- | --- |
| Témoins de `EmptyBufferManagerTest` | les 3 verts |
| Les 11 binaires de `test/storage` | 102 tests verts |
| `transaction_test` entier, extension vector bâtie | 390 verts, 2 sautés (des outils qui attendent une variable : `VectorIndexCrashReopenProbe.OpenInAFreshProcess`, `WalReader.PrintAFile`), code 0 |

Un premier `transaction_test` avait rendu 10 rouges, tous de mon montage : l'extension vector n'était pas bâtie, et
chacun s'arrêtait sur son `LOAD EXTENSION`.
