# Le champ demande du journal des runs reste la première demande

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : trace fausse : le juge et l'IDE lisent une demande qui n'est plus celle du run.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui
- **Signalé par** : c7, campagne D6 du 16 novembre 2026 (J1)

## Ce que c'est

Le journal d'un run d'équipier (`src/daemon_host/journal_des_runs.rs`) porte la demande de la tâche
au moment de son ouverture, pas celle du pas.

## Ce qu'il faut faire

Écrire au journal la demande **en cours** du pas (celle de la frontière, voir le ticket S3) et le nom
de l'agent dans l'en-tête d'agent (le point 3 de `docs/rag3weaver-archi/15-11-2026/11-conception-le-nom-de-l-agent.md`,
non fait).

## Comment le prouver

Rouge : une tâche, deux requests, deux runs ; le journal du second porte la seconde demande et le
nom. Passe comme le plafond, plus `e2e_lectures`.

## Taille et blocage

2 h. Après S3.

