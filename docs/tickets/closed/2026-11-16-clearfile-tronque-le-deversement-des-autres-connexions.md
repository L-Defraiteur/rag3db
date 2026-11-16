# `clearFile` tronque le fichier de déversement des autres connexions

- **État** : fermé (16 novembre 2026, hotfixs) : `clearFile` ne vide plus le fichier sous un groupe encore déversé (3311437745, pile `clearfile-keeps-live-spills`).
- **Famille** : moteur
- **Gravité** : perte possible sous plusieurs connexions, au-dessus du confort (c7). Une requête qui relit un groupe
  qu'elle a déversé lirait un fichier vidé par la fin d'une autre requête.
- **Atteignable en service** : seulement si deux connexions d'une même base tournent en même temps, et que l'une
  déverse (un COPY qui manque de mémoire) pendant que l'autre finit sa requête. L'hôte rag3weaver ouvre plusieurs
  connexions par base.
- **Touche rag3weaver** : par le moteur
- **Pour** : hotfixs (défaut suivant, si le cœur n'y voit pas d'inconvénient)

## Ce qu'on lit

Le fichier de déversement `<base>.tmp` et son `Spiller` sont uniques par base, partagés par toutes les connexions
(le `BufferManager`). `ClientContext` appelle `Spiller::clearFile()` à la fin de chaque requête, réussie
(`src/main/client_context.cpp:594`) ou en échec (`:607`), sans regarder si une autre requête y a encore des groupes
déversés. `clearFile` tronque le fichier. Un groupe déversé garde sa position (`setSpilledToDisk(startPage *
pageSize)`) et se relit par `Spiller::loadFromDisk` → `readFromFile` à cette position, dans un fichier qui a pu être
vidé entre-temps. Ce qui se passe alors (une lecture courte, une erreur ou des zéros) reste à établir par le témoin.

Le lot `spill-file-restarts-at-zero` ne change rien à ce risque : il tronque comme avant, et remet en plus le compte
à zéro.

## Pour fermer

1. Un témoin rouge : deux connexions sur une même base. L'une déverse un groupe et ne le relit pas encore, pendant
   que l'autre finit une requête (qui appelle `clearFile`). Le groupe relu doit être celui qui a été déversé.
2. Le correctif, à choisir après le témoin : `clearFile` ne vide le fichier que si aucun groupe déversé n'est encore
   attendu (un compte des déversements vivants), ou bien un fichier de déversement par requête.

## Fermeture (16 nov.)

Le témoin `AnotherQueryEndingDoesNotEraseAGroupStillSpilled` (un groupe de 16 384 INT64 connus, déversé, puis
`clearFile`, puis relu) est rouge sur master, et silencieusement : aucune exception, mais 4 valeurs fausses. Il est
vert avec le correctif 3311437745.

Le correctif : le spiller compte les tampons encore sur disque. Le compte monte au déversement, et descend à la
relecture, qui remet aussi `filePosition` à `UINT64_MAX`, ou au destructeur d'un tampon déversé jamais relu. Le
déversement et le vidage passent sous un même verrou, et `clearFile` ne vide le fichier (`restartAtPageZero`) que si
le compte est nul. Un tampon protégé par `preventDestruction` n'est pas compté.

La passe de la pile, à luciepc, sur sa tête 3311437745 (bâti Release à moi, `build/hotfixs-moteur`) :

| Suite | Résultat |
| --- | --- |
| Témoins de `EmptyBufferManagerTest` | les 3 verts |
| Les 11 binaires de `test/storage` | 102 tests verts |
| `transaction_test` entier, extension vector bâtie | 390 verts, 2 sautés (des outils qui attendent une variable : `VectorIndexCrashReopenProbe.OpenInAFreshProcess`, `WalReader.PrintAFile`), code 0 |

Un premier `transaction_test` avait rendu 10 rouges, tous de mon montage : l'extension vector n'était pas bâtie, et
chacun s'arrêtait sur son `LOAD EXTENSION`.
