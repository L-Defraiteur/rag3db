# Un creux à la demande n'a pas de dual

- **État** : fermé (14 novembre 2026, embarquement) : un creux activé qui déclare `dimensions` rend un dual sur la même tenue ; ouvert le 14 novembre, laissé par le lot des activations nommées
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

## Le lot (14 novembre 2026, embarquement, branche `dual-du-creux`)

- **Le dual se déclare** : un creux activé qui dit `dimensions` rend un `DualTenu` sur **la même**
  `Tenue<C>` que le creux. Un bail, une activation, une éviction pour les deux.
- **À l'activation**, le fournisseur doit dire `dual: true` et cette dimension. Sinon, refus nommé
  (« sans le dual », ou « en 384 au lieu de 768 »).
- **Au chargement du manifeste**, un creux qui déclare une autre dimension que le dense est refusé
  (garde de c7) : le dual rendrait des vecteurs d'une autre taille que ceux du catalogue.
- **Les témoins**, rouges d'abord (dual absent, manifeste accepté), puis verts :
  - `dual_*` (trois) ;
  - le chargement ;
  - l'e2e du groupe : une recherche dense et creuse fait un seul `/embed_dual`, aucun
    `/embed_sparse` séparé.
- **Corrigé en route** : l'attente du témoin du chargement suivait un dense en 768, alors que le
  gabarit de la fixture déclare 1024. Le refus, lui, était juste.
