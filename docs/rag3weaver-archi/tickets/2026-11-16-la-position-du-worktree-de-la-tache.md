# Une tâche ne sait pas où est son arbre : 350 commits derrière master

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : réponse fausse : Algareth a dit « absente » d'une fonction qui n'existait pas encore sur sa branche, sans dire où il était.
- **Atteignable en service** : oui, toute tâche ouverte depuis longtemps.
- **Touche rag3weaver** : oui
- **Signalé par** : c7, le 16 novembre 2026 (tâche `atelier`, branche à 88cbad8611, 350 commits derrière)

## Ce que c'est

Une tâche garde la branche `agent/<nom>` de sa création (`taches.rs`, `ouvrir_une_tache_nommee`).
Rien ne dit à l'agent sa position, ni n'avance une branche qui n'a aucun commit propre.

## Ce qu'il faut faire

1. À l'ouverture d'une tâche (`open_slot` d'un backend de tâche) : si la branche n'a **aucun commit
   propre** (`git rev-list --count <master>..<branche>` vaut 0), avancer l'arbre en avance rapide
   sur la tête du dépôt principal, et le dire dans la boîte (une ligne de l'hôte, en anglais).
2. Sinon, calculer la position (`agent/atelier, 12 own commits, 350 behind master`) et la mettre
   dans l'invite d'un agent de code (`system_prompt_du_backend`, `chat.rs`), pour qu'il la dise
   quand une demande cite du code absent.
Ne jamais avancer une branche qui a des commits propres ; ne jamais toucher à l'arbre de Lucie.

## Comment le prouver

Rouge : un dépôt de témoin (`depot_inscrit`), une tâche ouverte, puis un commit sur master : à la
réouverture, la tâche sans commit propre est avancée, et la boîte le dit ; avec un commit propre, la
position est dans l'invite. Passe comme le plafond, plus `e2e_demon_hote`.

## Taille et blocage

5 h. Aucun.

