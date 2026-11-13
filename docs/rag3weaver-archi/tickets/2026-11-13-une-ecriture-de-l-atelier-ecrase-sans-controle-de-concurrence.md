# Une écriture de l'atelier écrase sans contrôle de concurrence

- **État** : ouvert (13 novembre 2026), pour archi.
- **Famille** : produit
- **Gravité** : perte — une modification arrivée entre la lecture et l'écriture est perdue sans un mot, et le diff rendu est faux
- **Atteignable en service** : oui, dès que deux écritures (deux conversations, une commande, l'utilisatrice) visent le même fichier en même temps
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs` : `atelier_ecrire` 469-504, `ecrire_atomique` 555)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur)
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1, à la demande de l'orchestration (13 novembre 2026) ; lignes de origin/master à cette date

## Ce que c'est

`atelier_ecrire` lit `avant`, calcule `apres`, puis écrit tout le contenu (488 par
`edit_file {content}` côté worktree, ou 504 hors worktree) sans attendre une empreinte : le
backend a pourtant `code_tools::edit_file_with_hash`. Le fichier temporaire a un nom fixe,
`<fichier>.atelier-tmp` (555) : deux écritures simultanées du même fichier se marchent dessus.

## Le témoin à écrire

Une écriture dont le fichier change entre la lecture d'`avant` et l'écriture (un crochet de test) :
refusée en le disant, le changement concurrent gardé.

## Ce qui le fermerait

L'écriture passe l'empreinte d'`avant` (`expected_hash`) et refuse sur un écart ; un temporaire au
nom unique ; le témoin vert.
