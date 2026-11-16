# Arbre principal, du 15 novembre au matin du 16 : le rapport de session

## Ce qui a atterri sur master

| Lot | Master | Ce qu'il fait |
|---|---|---|
| `migration generations --base` | 1bf1483d6 | une base passée par 1b monte aux générations par son chemin : refus nommé d'une base tenue ou qui n'est pas à versions, bilan (forme avant et après, génération publiée, rang lu) |
| 2b′, le chargeur et la composition | 988651d39 | une vue `(couche, rang)` se lit dans `_generation` en pile de lecture ; la composition se fait en prédicat (masque par uuid et par fichier), une pile d'un étage au bit près ; le chargeur sur la connexion d'un instantané ; le nom et le dépôt d'une couche |
| Le renommage en anglais | a529ea144 | les identifiants neufs des deux lots en anglais (règle de Lucie, 16 novembre), les appels de 2b-C compris |
| Le refus typé du chargeur | b9951b7d2 | `StackError`, une variante par refus ; le repli par le nom de 2b-C teste `UnknownLayer` et non plus le texte |

Recherche a posé 2b-C (`view` dans tous les outils de lecture) et « index pas prêt » sur ces lots.

## Les pages et les tickets

- `docs/arbre-principal/15-11-2026/01-2b-prime-le-chargeur-qui-compose-la-pile.md` : la conception de 2b′.
- `docs/arbre-principal/15-11-2026/02-la-fenetre-de-la-bascule.md` : la fenêtre, la politique de c7, les trois
  répétitions et leurs chiffres.
- `docs/arbre-principal/16-11-2026/01-2a-prime-3-couches-par-branche-et-par-worktree.md` : la conception de 2a′-3
  (les couches par branche et par worktree, la montre et son plafond, le sélecteur par motif, sept témoins).
- Tickets ouverts : une vue sans rang lit l'état de la bascule tant que rien ne publie (2a′-3) ; la garde des
  règles de l'analyse lit la mention avec sa casse (pour ED).

## La bascule des bases de Lucie

- **Répétée verte** sur copies de code-lourd et code-principal, sur l'écrivain corrigé et la montée : miroir
  CONSUMES/CONSUMED_BY 0/0, banc de recherche d'avant retrouvé. Fenêtre fermée ~15 min par base (1b de 10 à 12
  min, montée en 2 s), repassage base ouverte de 45 à 60 min.
- **Ce qu'elle a trouvé** : le miroir CONSUMED_BY recopié sans son CONSUMES (corrigé par ED), le pilote qui
  lisait `busy` comme une fin (corrigé), les instantanés de la nuit d'avant la réparation des deux index
  (réparés en répétition ; les bases vivantes le sont).
- **La parité du banc** (A migrée contre B l'instantané) : même modèle ; à texte égal, même vecteur ; ~9 % des
  morceaux ont un découpage neuf. Le dense s'améliore (rang 1 : 8 contre 3 sur 30). Tous signaux, A perd 2
  bonnes réponses sur 38 à 10 (`src/include/`). B′, un index neuf du même arbre, départage.
- **Ce qu'elle attend** : le verdict de B′, la pose qui porte l'écrivain et la montée, lucivy, et le mot de c7.
  La bascule change le découpage d'environ 9 % des morceaux : c'est à dire à Lucie avant.

## Reprendre

**Aucun lot de code n'est en vol** : les quatre ont atterri.

- `~/.cache/rag3weaver-build/wt-docs-73`, au portable, détaché, pour les docs de la session.
- **À luciepc, à retirer au ménage** (après la bascule, par `garde_worktree.py`) :
  - les worktrees `~/git_workspaces/rag3db-arbre-vues`, `rag3db-arbre-migration`, `rag3db-sync-2bp`,
    `rag3db-sync-2bc` ;
  - les copies de répétition sous `~/.cache/rag3weaver-build/bascule/` : `rep-*-copie-*`, `essai-requete-*`.
    `rep-code-principal-copie-20261115-2046` est la copie A du banc : la garder jusqu'à son verdict.
  - les binaires figés `bascule/rag3weaver-backend-*` et le script `bascule/repetition-bascule.sh`.
- **Au portable** : les worktrees `wt-vues`, `wt-migr-gen`, `wt-anglais`, `wt-stack`, et les copies de 1b sous
  `~/.cache/rag3weaver-build/1b-copie/` (à effacer après la bascule réelle).
- **La suite** : 2a′-3 selon sa page, rouge d'abord, après la bascule ; la vue courante par le contexte
  (`VUE_DE_L_APPEL` depuis l'invocation) ; la page de l'estimation du reste d'une vue (journal horodaté, débit
  par phase), contre le contrat de la page 12 d'archi.
