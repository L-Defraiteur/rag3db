# Un put_turn raté au milieu d'un tour perd la suite de ce tour

- **État** : ouvert, le 19 novembre 2026 ; à reprendre par archi au lot 2 du rangement au fil.
- **Famille** : produit
- **Pour** : archi
- **Gravité** : une tranche rangée peut manquer des messages d'un tour, sans le dire, hors du fichier de rangement.
- **Atteignable en service** : oui, dès qu'un put_turn échoue (memory fermé, writer occupé, refus).
- **Touche rag3weaver** : oui, `src/daemon_host/rangement.rs`
- **Signalé par** : archi, en accordant les formes du lot A′ de mémoire (19 novembre 2026)

## Ce que c'est

À la fin d'un tour, `ranger_apres_le_tour` envoie chaque message neuf par `put_turn`. Au premier
échec, il s'arrête et écrit `last_error` dans `chats/<backend>.rangement.json`. Au tour suivant,
il ne renvoie que les messages de ce nouveau tour : ceux du tour raté ne partent jamais.
`archive_slice` rangera alors une tranche à trous (render_turns ne voit que les Turn écrits).

Un défaut voisin, mineur : le texte de `last_error` cherche les erreurs sous `report.errors`.
`archive_slice` de mémoire les rend peut-être au premier niveau. Le verdict, lui, ne change pas : il se
juge par `archived_through_turn`.

## Ce qu'il faut faire

- Tenir dans le fichier de rangement le dernier message écrit, `(turn, seq)`.
- À chaque fin de tour, renvoyer tout ce qui suit cette position. `put_turn` est idempotent sur une
  ligne identique (`written: false`) : un renvoi ne double rien.
- Lire les erreurs du rapport aux deux endroits, pour le texte de `last_error`.

## Comment le prouver

Rouge sur le faux memory : un `put_turn` refusé une fois au milieu du tour 1. Au tour 2, les
messages manquants du tour 1 partent avant ceux du tour 2. Puis la passe du lot.

## Taille et blocage

2 h. Après l'atterrissage de pile-archi-19.
