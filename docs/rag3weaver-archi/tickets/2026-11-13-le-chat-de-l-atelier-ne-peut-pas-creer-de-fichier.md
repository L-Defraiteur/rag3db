# Le chat de l'atelier ne peut pas créer de fichier

- **État** : ouvert (13 novembre 2026), pour archi.
- **Famille** : produit
- **Gravité** : capacité manquante — le modèle n'a aucun chemin pour créer un fichier neuf
- **Atteignable en service** : oui, à toute demande de création d'un fichier depuis le chat
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/chat.rs`, l'empreinte de lecture exigée par `insert_lines` ; `atelier_fichiers.rs`, `read_file` d'un fichier absent)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur), lot d'après avec les six autres tickets de la relecture
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1 (13 novembre 2026), mis en ticket à la demande d'archi ; lignes de origin/master à cette date

## Ce que c'est

`insert_lines at=1` exige que la ligne 1 ait été lue ; `read_file` d'un fichier absent échoue,
donc l'empreinte ne peut jamais être prise. Hors du chat (sans empreinte), la création passe ;
depuis le chat, jamais.

## Le témoin à écrire

Depuis le chat, la création d'un fichier absent sous la racine passe et rend son diff de
création ; sur un fichier existant non lu, `insert_lines` reste refusé.

## Ce qui le fermerait

Un outil de création explicite, ou `insert_lines at=1` permis sans empreinte quand le fichier
n'existe pas (refusé s'il existe) ; le témoin vert.
