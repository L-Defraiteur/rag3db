# Le bac ne borne pas les signaux : `kill -9` d'un processus hors de la tâche passe

- **État** : ouvert (23 novembre 2026, 06 h), rag3weaver archi ; pour le matin (c7).
- **Famille** : moteur
- **Gravité** : à `bac`, une commande peut tuer tout processus de l'utilisatrice (l'hôte, ses services, le navigateur).
- **Atteignable en service** : oui, au niveau `bac`.
- **Touche rag3weaver** : oui (`commande.rs`, le ruleset Landlock en `ABI::V4`).
- **Pour** : archi, banc.
- **Signalé par** : c7, banc et archi, la nuit du terminal.

## Ce que c'est

Le ruleset du bac s'arrête à l'ABI 4 (fichiers, TCP) : ni `Scope::Signal` ni `Scope::AbstractUnixSocket` (ABI 6). Le noyau 7.3 des deux postes les tient ; à vérifier : la crate `landlock 0.4` les porte-t-elle. À poser avec `effect-classes` (0e936bbd03), qui n'atterrit pas avant. La grille de banc rejouée sur `rule_class` (`~/.cache/rag3weaver-build/gate-replay.log` à luciepc) : 31 cas changent, 13 passent sans juge en `write_local`, dont 7 non autorisés (`sqlite3 … DELETE`, `rag3db-shell … DETACH DELETE`, `kill -9`, `gh auth switch`, `git worktree remove`). Les règles se disent **par l'effet** dans `rules.json` (détruire ou modifier des données hors de l'arbre de la tâche, signaler un processus qui n'est pas de la tâche, changer une identité ou un compte), jamais par nom de programme.
