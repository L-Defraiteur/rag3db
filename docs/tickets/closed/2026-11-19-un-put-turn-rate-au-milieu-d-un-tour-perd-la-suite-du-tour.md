# Un put_turn raté au milieu d'un tour perd la suite de ce tour

- **État** : fermé le 19 novembre 2026 par `1de6ae04c` (rangement-lot-2, PR #3, par le pousseur).
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

## Fermeture

Le fichier de rangement tient `written_through` (turn, seq). À chaque fin de tour, et avant une
compaction, tout ce qui suit cette position repart par `put_turn` : un message refusé repart au
tour suivant, avant ceux du nouveau tour. Le texte de `last_error` lit les erreurs du rapport plat
d'`archive_slice` (et `report.*`). Témoin `a_refused_put_turn_is_sent_again_with_the_next_turn`,
vu rouge sur e8d664ec6. Passe du pousseur verte sur la lib 7c2b506ec, e2e_equipier sous Vertex
compris.
