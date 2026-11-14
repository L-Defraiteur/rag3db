# Un creux à la demande n'a pas de dual

- **État** : ouvert (14 novembre 2026, embarquement, laissé par le lot des activations nommées)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : coût. Un backend dont le dense et le creux citent le même groupe bge-m3 embarque une
  requête en deux appels (dense puis creux) au lieu d'un passage dual.
- **Atteignable en service** : oui, dès qu'un manifeste déclare `models.sparse` avec une activation
  (aucun manifeste de Lucie ne le fait aujourd'hui).
- **Touche rag3weaver** : oui (`src/model_source/activation.rs`, `connecter_creux`)

## Ce que c'est

`connect_sparse` décide le dual sur l'identité du fournisseur (`/sante`, `dual`), avant de le rendre.
Un creux à la demande ne connaît pas cette identité avant son premier appel. `connecter_creux` rend
donc `SparseClients { dual: None }`, et le catalogue embarque le dense et le creux séparément.

## Ce qu'il faut pour le fermer

Un dual à la demande : décidé par la déclaration (le `modele` de l'activation est celui du dense), et
tenu par la même tenue que le creux. Un témoin rouge d'abord : la requête d'un backend dense et creux
sur le même groupe fait un seul appel `/embed_dual`.
