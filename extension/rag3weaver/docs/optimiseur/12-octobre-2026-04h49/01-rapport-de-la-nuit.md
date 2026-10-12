# La nuit du 11 au 12 octobre 2026 : rapport de l'optimiseur (G)

Pour Lucie et les sessions qui reprennent. Ce qui est sur master, ce qui attend, et ce qui a
mal tourné.

## Sur master

| lot | commit | ce qu'il fait |
|---|---|---|
| A, l'embarqueur absent | 068356677 | Sans service d'embarquement, un gabarit à vecteurs s'ouvre dans les quatre portes (lignes JSON, `mcp`, `serve`, l'hôte), en plein texte seul, vecteurs en dette. Le reçu dit pourquoi : rien de déclaré, ou un service pas joint. Relu par l'arbre principal. |
| B4, les identités relatives | 6e3561268 | `File.path`, `Scope.file_path` et les clés sont relatifs à la racine, en « / ». Une racine locale par base. Une base d'avant la règle refuse et se dit. Témoin : une base copiée par reflink sur un autre arbre garde ses identités. |
| B1, la portabilité | 617839ede | Chemins cités pour Cypher, fsync sous Windows, mémoire vive lue partout, lectures du manifeste qui nomment le chemin. Relu par l'arbre principal. |
| B2, les COPY | 5e93b4bef | Les deux COPY citent leur chemin. Relu par F, avec son test. |
| B3, l'identifiant sous Windows | 71cdf4185 | build.rs donne le vrai dépôt à cmake quand la source est une copie. Relu par le cœur. |
| `poste` | e70a78a72, ced1f2722, a94409139 | La priorité de la mesure, un poste sous un poste qui passe, le verrou de push tenu par le script, l'attente annulable. Témoin `scripts/test_poste.sh`, 21 cas. Déployé sur les deux postes. |

Plusieurs tickets ont été ouverts : la reprise d'un service injoignable (à l'arbre principal) et
le test instable de la montre. Les tickets de chaque lot sont fermés dans leur commit.

## En cours

- **Lot C, le mode démon portable** (branche `portage-c`). Vert sous Linux. Les runners Windows
  et macOS jouent le bâti sur la branche d'essai `ci-portage-c` (run 38197194079), qui sera
  effacée après. Relectures : la mémoire et l'arbre principal.
- **Les archives alpha.3** (branche `paquet-npm-2`, le paquet seul, sur master). Elles partent
  après le lot C. Linux se bâtit par Docker sur luciepc, qui est bloqué : voir ci-dessous.
  Rien n'est publié.

## Pour Lucie, sur luciepc

L'hôte est figé, l'`index.sh` de codeparsers attend derrière lui, et la place de poste est tenue
depuis 22 h. Mon bâti de la lib (avec le correctif e6f626df4), du magasin et du binaire attend
cette place. Quand il passera, il écrira la note `~/.local/share/rag3weaver/POUR-LUCIE-hote.md`,
avec l'ordre de remplacement et les md5. La section « Pour Lucie » du rapport de codeparsers
donne le premier pas : libérer la place.

## Ce qui a mal tourné

**À 3 h, mon témoin du poste a tué cinq pushes réels** : trois de codeparsers, un de F, un de
la recherche. Ses cas terminaient par motif sur tout le poste. Rien n'a été perdu : tous sont
morts avant `git push`, et chaque session a vérifié. Le témoin ne vise plus que ses propres
pids, sur ses propres verrous, et refuse les verrous réels. La règle est aussi dans ma mémoire.

**La régression « poste sous poste »** venait de mon lot de la priorité de la mesure. Deux
mesures du cœur y ont été perdues. Elle a été corrigée dans l'heure (ced1f2722).
