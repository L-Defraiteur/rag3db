# L'atelier perd les fins de ligne CRLF

- **État** : ouvert (13 novembre 2026), pour archi.
- **Famille** : produit
- **Gravité** : réponse fausse — un remplacement sur plusieurs lignes ne trouve jamais son texte ; une écriture mélange les fins de ligne ; le diff peut dire « aucun changement »
- **Atteignable en service** : oui, sur tout fichier en CRLF (dépôts venus de Windows, fichiers `.bat`, certains CSV)
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs` : `.lines()` en 345 et 565)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur), lot d'après avec les six autres tickets de la relecture
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1 (13 novembre 2026), mis en ticket à la demande d'archi ; lignes de origin/master à cette date

## Ce que c'est

La lecture découpe par `.lines()`, qui retire les `\r`. Un `old` de `replace_text` sur plusieurs
lignes, recopié de ce que le modèle a lu, ne contient plus les `\r` du fichier : il n'est jamais
trouvé. `replace_lines` et `insert_lines` réécrivent en `\n` les lignes touchées et laissent
les autres en `\r\n`. Le diff, calculé sur les lignes sans `\r`, peut ne rien montrer.

## Le témoin à écrire

Un fichier en CRLF : `replace_text` sur deux lignes lues le trouve et le remplace, `replace_lines`
garde des fins CRLF partout, et le diff montre le changement.

## Ce qui le fermerait

Détecter la fin de ligne du fichier à la lecture, la garder à l'écriture des lignes neuves, et
comparer `old` à fins de ligne normalisées ; le témoin vert.
