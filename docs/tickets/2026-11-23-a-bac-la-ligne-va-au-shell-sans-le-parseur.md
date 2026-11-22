# À `bac`, la ligne va au shell du bac, sans le parseur des classes

- **État** : ouvert (23 novembre 2026, 01 h), rag3weaver archi ; reporté au matin par c7 (le MVP de la nuit).
- **Famille** : moteur
- **Gravité** : le journal du run d'une ligne enveloppée montre la forme encodée en base64, illisible.
- **Atteignable en service** : oui, à `bac`, dès qu'une ligne a une redirection, une substitution, une expansion ou un joker.
- **Touche rag3weaver** : oui (`daemon_host/teammate.rs`, `StepTools::after` ; `dataflow/run_nodes.rs`).
- **Pour** : archi.
- **Signalé par** : archi, la nuit du terminal.

## Ce que c'est

**En tête des améliorations du 23** (c7 : « un journal doit se lire »). `bac-shell-lines` fait passer à `bac` une ligne refusée par `codeparsers::shell` par `sh -c 'eval "$(printf %s <base64> | base64 -d)"'`, parce que le parseur refuse un `*` ou un `?` même entre guillemets. Le fil, la boîte et le compte rendu montrent la ligne en clair ; le journal du run (les paramètres de `RunCommandNode`) montre l'encodée. La forme juste : à `bac`, `run_command` remet la ligne telle qu'écrite à `sh -c` dans le bac, sans la réduire ; la réduction en invocations ne sert qu'aux niveaux `judge` et `cards`, qui classent.
