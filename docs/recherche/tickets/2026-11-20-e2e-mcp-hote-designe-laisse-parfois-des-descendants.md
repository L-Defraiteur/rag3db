# e2e_mcp_hote_designe laisse parfois des descendants après la commande

- **État** : ouvert (20 novembre 2026, session recherche).
- **Gravité** : confort de la file : un rouge du confineur sans test rouge, qui peut renvoyer un lot.
- **Atteignable en service** : non (une suite e2e).
- **Touche rag3weaver** : oui (`tests/e2e_mcp_hote_designe.rs`, ou l'hôte qu'elle désigne).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Ce que c'est

Le 19 novembre vers minuit, sur la branche `index-ready-notify` (4f695ff7ad), `run_e2e.sh --test e2e_mcp_hote_designe`
a rendu ses 3 tests verts, mais la validation a jugé ROUGE : « descendants laissés après la commande ». Les preuves
sont dans `luciepc:~/.cache/rag3weaver-build/validations/validation-g0frmb_z`, mais `enfants.json` ne nomme pas les
commandes.

Rejoué aussitôt, dans le même worktree et sous la même lib (6e2476d41) : ma tête deux fois, puis sa base
`index-wait` (e0c33cc29f), sans le lot. Les trois jeux sont VERTS. Le rouge est intermittent : une fois sur
quatre jeux, et il ne vient pas du lot.

## Pour le fermer

Nommer les descendants que le confineur trouve : la commande de chaque pid dans `enfants.json`. Viennent ensuite
le processus que la suite lance sans l'attendre (un hôte désigné, un client mince), et l'arrêt qui manque. Le témoin
est la suite jouée dix fois de suite sous le confineur, toutes vertes.
