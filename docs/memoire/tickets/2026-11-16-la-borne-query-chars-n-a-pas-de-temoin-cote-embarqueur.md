# La borne query_chars n'a pas de témoin côté embarqueur

- **État** : ouvert, le 16 novembre 2026.
- **Famille** : produit
- **Gravité** : une garde sans preuve directe. Le témoin d'aujourd'hui lit la longueur dans la
  trace de l'archiviste, pas ce que l'embarqueur reçoit vraiment.
- **Atteignable en service** : non. C'est une question de preuve, la borne est en place.
- **Touche rag3weaver** : oui, `tests/e2e_archiviste.rs`, et l'aide `embarqueur_qui_note` de
  `tests/common/mod.rs`, posée par embarquement (73aa05511).
- **Pour** : Codex, ou la session mémoire (96).
- **Signalé par** : la session mémoire (96), à la passation du 16 novembre 2026.

## Ce qu'on voit

`query_chars` (4 000 par défaut, `graphs/archive.mmd`, `scripts/archive_query.rhai`) borne la
requête de la recherche des candidates. Le témoin
`la_requete_des_candidates_est_bornee_et_la_trace_dit_sa_longueur` lit `query_chars` dans la
trace. Son rouge, joué sans le correctif le 15 novembre, a envoyé une requête de 40 000
caractères au vrai bge-m3 partagé. Le service a tenu, mais c'est ce qu'il ne fallait pas faire.

## Ce qui serait juste

Un second témoin qui ouvre la mémoire sur l'embarqueur qui note les tailles, au lieu du service.
Il archive une section de 40 000 caractères, et vérifie que **aucun** texte reçu par l'embarqueur
ne dépasse 4 000 caractères, requêtes et morceaux écrits compris.

Le rouge se joue en posant `query_chars` à 100 000 dans l'appel, jamais sur le vrai service :
la taille reçue dépasse alors 4 000.

## Le témoin

`e2e_archiviste::no_text_over_query_chars_reaches_the_embedder`. Voir comment
`ouvrir(dir, retoucher)` connecte l'embarqueur (`prepared.connect_embedder()`) : le témoin doit
ouvrir le backend avec l'embarqueur qui note à la place, par `prepared.open(..., Some(embarqueur))`.

## Taille

2 heures.
