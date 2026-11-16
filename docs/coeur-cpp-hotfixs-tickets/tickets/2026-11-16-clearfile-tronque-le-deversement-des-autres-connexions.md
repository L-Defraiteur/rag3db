# `clearFile` tronque le fichier de déversement des autres connexions

- **État** : ouvert (16 novembre 2026, hotfixs), trouvé en lisant, pas encore reproduit
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
