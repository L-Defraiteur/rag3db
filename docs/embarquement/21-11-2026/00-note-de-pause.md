# Embarquement : note de pause (21 novembre 2026, 15:26:20, redémarrage de la tablette)

## Où j'en suis

- **En file au pousseur** : `emb-reuse-by-hash`, branche `reuse-by-hash` @ 0e5910ec3c, attestation VERT 8/8. C'est la
  reprise d'un vecteur par l'empreinte de son texte. Page : `docs/embarquement/21-11-2026/01-une-vue-ne-reprend-aucun-vecteur.md`
  (dans le lot). Le guet de son atterrissage tournait à la tablette : il meurt avec elle.
- **Atterris depuis hier** : `values-guard`, `resolved-adapter`, `self-check`, `bge-reference`, `local-suites`,
  `memory-bound`, et les docs de la preuve de la mémoire (55ae3231d3). Les services 7878 et 7879 tournent avec eux depuis
  06 h 19, vérifiés au démarrage.

## Ce qui tourne

- **À luciepc** : rien de mon fait.
- **À la tablette** : rien, sauf le guet local cité, sans effet.

## Mes worktrees

- tablette : `~/git_workspaces/embarquement/21-11-2026-reuse-by-hash` (lot en file, aucun target), et
  `~/git_workspaces/embarquement/21-11-2026-rapport` (cette note).

## Les tunnels d'embarquement de la tablette (7979 à 7981)

Unité utilisateur `rag3weaver-tunnels.service`, activée (`WantedBy=default.target`, `Restart=always`). **`Linger=no`** :
elle revient à l'ouverture de la session de Lucie, pas au seul démarrage. Pareil pour
`rag3weaver-tunnel-hote-luciepc.service` (7971 vers 7970).

## Ma prochaine action, à la reprise

1. Lire `pusher/log.tsv` à luciepc pour `emb-reuse-by-hash` ; s'il est poussé, retirer le worktree `reuse-by-hash`. Sinon,
   reposer le guet de son atterrissage.
2. Ne relancer ni validation ni e2e avant l'annonce de c7 (l'atterrissage de `confiner-poste-classes`).
3. Le partage des morceaux entre vue et base (le lot (b)) est à ED ; je reste disponible pour une part.
