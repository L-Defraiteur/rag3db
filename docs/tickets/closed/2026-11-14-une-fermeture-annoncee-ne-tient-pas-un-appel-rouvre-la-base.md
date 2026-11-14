# Une fermeture annoncée ne tient pas : un appel rouvre la base

- **État** : corrigé (14 novembre 2026), lot `fermeture-qui-tient` d'archi
- **Famille** : produit
- **Gravité** : écriture sous une passe — une base fermée pour une passe (la copie de
  l'archiviste) se rouvre au premier appel qui la touche, et la passe écrase ou perd ces écritures
- **Atteignable en service** : oui, vu le 14 novembre 2026 à 18 h 35 : la mémoire longue fermée par
  G (`/backend/close`, fenêtre annoncée à toutes les sessions) s'est rouverte parce que le juge d'un
  run d'agent a écrit par l'hôte (une Source, une fiche, deux réécritures)
- **Touche rag3weaver** : oui (`src/daemon_host.rs`)
- **Pour** : archi (relevé par 96, mis en ticket par c7)

## Ce que c'était

`POST /backend/close` lâchait la base, et « le prochain appel le rouvrirait » : un appel d'outil, un
hook, un juge, l'ancienne route `/open` d'un client qui s'attache. L'annonce de la fenêtre aux
sessions n'était pas un verrou.

## Ce qui l'a fermé

`/backend/close` prend une raison (`reason`) et une heure annoncée (`until`, facultative). Avec une
raison, la fermeture tient : toute ouverture est refusée en disant « fermé : <raison>, jusqu'à
<quand>, par <client> (session <id>) — rien ne le rouvre avant un POST /backend/open explicite ».
L'heure annoncée n'ouvre rien d'elle-même ; passée, le refus dit « au-delà de l'heure annoncée, la
fermeture tient encore ». `/open` ne la lève pas, `/backend/open` la lève. `/etat` et
`daemon_status` la disent (`held_closed`). Sans raison, l'ancien sens demeure (le prochain appel
rouvre), dont dépendent l'équipier, le `remove` de code-tree et `e2e_demon_hote`.

Témoin : `e2e_demon_hote::une_fermeture_avec_une_raison_tient_jusqu_a_l_ouverture_explicite`.
