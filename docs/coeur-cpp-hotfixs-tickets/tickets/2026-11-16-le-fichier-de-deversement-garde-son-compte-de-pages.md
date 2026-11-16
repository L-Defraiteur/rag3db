# Le fichier de déversement garde son compte de pages après chaque requête

- **État** : ouvert (16 novembre 2026, hotfixs), trouvé en lisant, pas encore reproduit
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

## Pour fermer

1. Un témoin rouge : avec un petit pool, deux COPY qui déversent, dans deux requêtes de suite. Après la seconde,
   `numPages` du fichier de déversement repart de zéro, et la taille apparente de `<base>.tmp` ne dépasse pas ce
   qu'une seule requête a déversé.
2. Le correctif : `clearFile` remet le `FileHandle` à zéro sous son verrou (pages, états, groupes de cadres rendus),
   en plus de tronquer le fichier.
