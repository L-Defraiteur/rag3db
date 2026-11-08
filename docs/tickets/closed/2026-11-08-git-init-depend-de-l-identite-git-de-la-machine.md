# git_init dépend de l'identité git de la machine

- **État** : corrigé (8 novembre 2026, branche git-init-identite, hooks-essai), relevé par c7 sur la
  CI de master.
- **Famille** : produit
- **Gravité** : CI rouge. Le produit aussi est en défaut : sur une machine sans identité globale,
  `git_init` refuse ; sur une machine qui en a une, il l'écrit dans la racine.
- **Atteignable en service** : oui, toute tâche ouverte sur une racine sans git avec
  `workspace.git_init: true`.
- **Touche rag3weaver** : oui (`src/daemon_host/taches.rs`, `src/backend_code.rs`).

## Ce que c'était

Le premier commit de `git_init` (tache-sans-git, d987d2143) prenait l'identité de la config git de
la machine. Le runner de la CI n'en a pas : « Author identity unknown », cinq runs rouges de suite
depuis 14 h 14 UTC, sur `daemon_host::taches::tests::git_init_declare_fait_de_la_racine_un_depot`.
Nos postes en ont une, d'où le vert local. Celle du poste portable porte en plus une adresse qui
ne doit pas entrer dans les dépôts de Lucie.

## Le correctif

- Le premier commit porte `rag3weaver <rag3weaver@localhost>`, ou l'identité que le manifeste
  déclare (`workspace.git_identity {name, email}`). Elle passe par `-c user.name=… -c
  user.email=…`, jamais par la config de la machine ni par celle du dépôt.
- Le message du commit le dit (« premier commit par rag3weaver (git_init déclaré) »), et la boîte
  aussi à l'ouverture.
- La fonction `backend_code::git_identity_or_default`, avec `GitIdentity::git_args`, est commune :
  everything declarative l'appellera pour les déclarations d'`attach_worktree`.

Témoin : `git_init_declare_fait_de_la_racine_un_depot` lit l'auteur du premier commit. Rouge sur
43fdd69d8 de deux façons : l'auteur était l'identité globale du poste, et sous
`GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1`, le refus était celui de la CI. Vert sur
1e85ddce1 sous `GIT_CONFIG_GLOBAL=/dev/null`.
