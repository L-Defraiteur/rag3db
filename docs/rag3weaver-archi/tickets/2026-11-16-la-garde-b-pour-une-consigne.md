# La garde B (muet relancé) ne joue pas pour une consigne

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : réponse manquante : l'équipier répond « fait » à une consigne sans rien rendre, et tty dit « il a fini sans répondre ».
- **Atteignable en service** : oui, toute `instruction` après la fin d'un pas.
- **Touche rag3weaver** : oui
- **Signalé par** : tty par c7, le 16 novembre 2026 (« Redis ton nom… » : un step_done « fait : Nom rappelé en une phrase. », rien d'autre)

## Ce que c'est

`request_without_a_line` (`teammate.rs`) ne regarde que la dernière ligne de `kind: "request"`.
Une `instruction` (ou un `message`) postée après la fin d'un pas, à laquelle l'équipier répond par un
seul `step_done`, n'est ni relancée ni attendue.

## Ce qu'il faut faire

Étendre la garde à la dernière **demande au sens large** reçue du superviseur : `request`,
`instruction`, `message` adressés à l'équipier (`addressed_to_teammate`). Même forme qu'aujourd'hui :
relance une fois en citant le `step_done`, puis attente bornée par `teammate.reply_wait_s`, et échec
nommé. `expects: "none"` vaut aussi sur une `instruction`. La trace (`request.trace`) suit cette
demande-là.

## Comment le prouver

Rouge : faux modèle, une request satisfaite (un report), puis une `instruction` « redis ton nom » à
laquelle le modèle ne répond que par `step_done` : la boîte porte la relance. Les témoins de la garde
B (`a_request_without_expects_waits_for_a_line…`, etc.) restent verts.

## Taille et blocage

3 h. Aucun.

