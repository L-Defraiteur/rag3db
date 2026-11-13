# L'empreinte de lecture de l'atelier ignore les changements faits ailleurs, et note des lignes jamais montrées

- **État** : ouvert (13 novembre 2026), pour archi.
- **Famille** : produit
- **Gravité** : perte — `replace_lines` ou `insert_lines` remplace des lignes qui ne sont plus celles que le modèle a lues
- **Atteignable en service** : oui, dès qu'un fichier lu par une conversation change par un autre chemin (une autre conversation, une commande, un `git checkout`, l'utilisatrice) avant son écriture ; ou par `read_scope` d'un scope de plus de 200 lignes
- **Touche rag3weaver** : oui (`src/daemon_host/empreintes.rs` 11-14, 49-53 ; `src/daemon_host/chat.rs` 825-859 ; `extension/rag3weaver/src/daemon_host/atelier_fichiers.rs` `atelier_lire_scope` 447-449)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur)
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1, à la demande de l'orchestration (13 novembre 2026) ; lignes de origin/master à cette date

## Ce que c'est

L'empreinte tient, par conversation, un chemin lexical et des plages lues ; elle ne retient ni le
contenu, ni la date, ni l'empreinte du fichier, et n'est oubliée que par une écriture de la même
conversation (`chat.rs:857`). Deux fuites de plus : `read_scope` note la plage D-F de l'en-tête
alors qu'il n'affiche que 200 lignes (`chat.rs:845-853`, `atelier_fichiers.rs:447-449`) ; `read_file`
note la plage demandée, pas la plage rendue (`chat.rs:840-843`). La clé est lexicale quand
l'écriture canonicalise : une écriture par un lien symbolique n'oublie pas la lecture.

## Le cas

A lit les lignes 1-50 de `x.rs` ; un `sed` accordé insère dix lignes en tête ; au tour suivant,
`replace_lines 20-25` d'A est accepté et remplace d'autres lignes. Ou : `read_scope` d'un scope de
500 lignes, puis `replace_lines 300-400`, accepté sans que ces lignes aient été montrées.

## Le témoin à écrire

Lecture, changement du fichier hors de la conversation, `replace_lines` : refusé (« relis d'abord »).
`read_scope` d'un long scope, puis une écriture au-delà de ce qui a été affiché : refusée.

## Ce qui le fermerait

L'empreinte porte l'empreinte du fichier lu (ou son contenu sur la plage) et les seules lignes
rendues ; l'écriture compare ; le témoin vert.
