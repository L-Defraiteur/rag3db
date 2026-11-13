# Un fournisseur à la demande ouvert par l'hôte ne s'évince jamais

- **État** : ouvert (13 novembre 2026, session embarquement, à la demande de c7) ; le lot de
  l'embarqueur activé à la demande le ferme
- **Famille** : produit
- **Gravité** : coût — la VRAM d'un modèle « à la demande » reste prise tant que le backend qui l'a
  ouvert vit, et un backend ouvert charge un modèle dont aucun index n'aura besoin
- **Atteignable en service** : oui, dès qu'un manifeste déclare une activation
  `Residence::ADemande` (aucun manifeste de Lucie ne le fait aujourd'hui)
- **Touche rag3weaver** : oui (`src/backend.rs`, `src/model_source/activation.rs`)

## Ce que c'est

`Backend::embedder_or_absent` appelle `model_source::connect_embedder` à l'ouverture du backend.
Pour une source qui déclare une activation, `connect_embedder` lance le fournisseur (local ou
distant) et prend un bail d'usage (`DaemonEmbedder::prendre_bail`, master `7042128d0`), renouvelé
par un fil tant que le client vit. L'hôte garde ce client tant que le backend est ouvert. Donc :

1. **l'éviction ne joue pas côté hôte** : le bail ne tombe jamais, le fournisseur déclaré
   `a_demande` (master `0ee027cf0`) reste chargé aussi longtemps que le backend ;
2. **un backend ouvert lance le modèle sans index** : l'ouverture seule paie le chargement et la
   VRAM, même si aucune écriture ni recherche n'a besoin de vecteurs.

Lu dans le code le 13 novembre (master `f3897600f`) ; pas encore de témoin.

## Ce qu'il faut pour le fermer

Un embarqueur activé à la demande pour `Residence::ADemande` : rien à l'ouverture ; activation au
premier calcul, bail tenu tant que les appels se suivent et rendu après un délai nommé, plus court
que l'inactivité du fournisseur ; réactivation transparente (une fois, identité revérifiée) si le
fournisseur est parti. Témoins rouges d'abord sur la fixture CPU : ouvrir sans lancer ; un index
qui lance et a ses vecteurs en moins d'une minute ; l'éviction après l'index et le délai ; un second
index qui relance ; une lecture sans besoin de vecteurs qui n'active rien, et une qui en a besoin
qui active. `Residence::Maintenir` et les services posés à la main ne changent pas.
