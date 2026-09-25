# Recherches prévues

`search_cards.mmd` est le graphe commun : résultat structuré `render.results`,
avertissements des trois ports `source.meta`, `bm25.meta`, `vector.meta` à remonter
avec le résultat. Ne pas jeter les avertissements en choisissant le port résultat.

| Recherche | Restriction avant recherche | Enrichissement après pagination |
|---|---|---|
| Cartes | condition structurée de l'appelant | payload complet par external_id |
| Ma collection | `MtgHolding.quantity > 0`, éventuellement source_id | quantités de chaque impression |
| Dans un deck | `MtgDeckEntry.deck_id = UUID` et éventuellement pile | quantité et pile |
| Par mécanique | chercher `MtgMechanic`, suivre les liens exacts vers les cartes | capacité native et définition |

Les variantes collection/deck instancient le même graphe avec un `must` englobant
la restriction et le filtre utilisateur. Un filtre n'est jamais appliqué après
avoir tronqué une recherche globale. Un UUID de deck est l'identifiant externe,
pas le nom modifiable. Les relations de collection et d'entrées de deck sont
représentées par des entités pour conserver quantité, profil et emplacement.

Modèle : `MtgCard -> MtgAbility -> MtgMechanic`. Une relation directe
`MtgCard -> MtgMechanic` peut être matérialisée à partir des liens exacts pour
les restrictions inter-entités actuelles (un saut). Aucune mention lexicale
n'est transformée en possession d'une capacité. Les définitions sont indexées
sur les mécaniques ; le texte imprimé reste dans la carte.

Brique manquante pour un graphe de découverte mécanique entièrement composé :
`FetchRelatedNode` rend un port `children`, pas une nouvelle liste de résultats.
Il faut une conversion générique dédupliquant les voisins avant un deuxième
parcours et la pagination des cartes. Ne pas dessiner un port fictif comme si
ce parcours était déjà exécutable. Le filtre par identifiant mécanique exact
peut employer directement la relation matérialisée.
