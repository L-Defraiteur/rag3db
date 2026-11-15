# 2a′-3 : les couches par branche et par worktree, et les vues des worktrees des sessions

16 novembre 2026, arbre principal, à la demande de c7. C'est la conception d'avant le code. Elle prolonge la page
03 du 14 novembre (`docs/arbre-principal/14-11-2026/03-2a-prime-3-ce-que-les-suivies-apportent.md`), dont elle
garde les décisions. Elle répond au point 5 de la vision d'Algareth
(`docs/orchestration/visions/16-11-2026-algareth-pair-des-sessions-claude.md`) : avoir les worktrees des sessions
sous la main, en index. Les identifiants neufs sont en anglais (règle de Lucie, 16 novembre, 1 h) ; la prose reste
en français.

## Ce qui existe quand 2a′-3 commence

- **2b′ lit une vue sans rien savoir de sa nature** : `vues::charger_la_pile` lit la chaîne d'une couche dans
  `_generation`, et `Visibilite::avec_la_pile` la compose en prédicat. Une pile travail → branche → … → racine se
  lit comme la pile d'un étage de `backend`.
- **2b-C (recherche)** prend `view = {layer, rank?}` dans tous les outils de lecture. Il résout `layer` par l'id,
  ou par le nom (`couche_nom:<dépôt>:<nom>`), dans l'instantané de l'appel.
- **Les suivies** donnent déjà la pile git :
  - les couches d'histoire (une par ref suivie) et leurs parentes ;
  - les couches de travail (une par worktree déclaré, `worktree:<chemin>`) ;
  - un id stable par couche, avec sa fin quand la couche disparaît.
- **Seule `backend` a des générations.** Toute autre vue se refuse : « couche sans génération ». C'est ce que
  2a′-3 lève.
- **Le doublon au bord haut d'un trou (2)** est l'identité retenue par c7 (15 novembre), d'après la mesure d'ED :
  0 réapparition sur 50 commits. La variante de l'uuid par le contenu est un ticket à part.

## Ce que 2a′-3 écrit

### 1. Une couche d'histoire par branche suivie, par les objets git

- **Sans checkout.** Pour un commit c d'une ref, l'arbre se lit par `git ls-tree -r c` et les fichiers changés
  depuis la génération d'avant par `git diff-tree -r <c_avant> c`. Les blobs se lisent par `git cat-file`. Rien
  n'est écrit dans aucun worktree.
- **Une génération par commit publié**, au rang suivant de sa couche. Sa parente est épinglée selon les règles
  de la page 03 : la couche parente (la branche d'où elle part, ou la racine) à son rang au moment de la
  publication.
- **Ce qu'elle porte** : les lignes des fichiers changés (naissances), les retraits (`_jusqu_a`) des lignes qu'ils
  remplacent, et les tombstones des fichiers effacés.
- **Qui écrit** : `history` décide, comme la page 03 le dit. `tip` écrit la pointe, amortie (5 s, plafond 60 s).
  `all` et `{last: N}` rattrapent paresseusement, le plus récent d'abord, par pas de 5 générations ou 30 s.

### 2. Une couche de travail par worktree

- **Ses fichiers non commités** : `git status --porcelain=v2 -z` dans le worktree, puis le contenu de chaque
  fichier modifié, ajouté ou effacé. Une génération de travail porte ces fichiers seuls.
- **Sa parente épinglée** est la couche d'histoire de sa branche, au rang dont le commit est le HEAD du
  worktree. Si ce rang n'est pas encore écrit, la couche de travail attend `tip`, sous le même amortisseur.
  Elle ne s'écrit jamais sur une parente qui n'est pas son HEAD.
- **Un worktree détaché** (un HEAD sans branche) prend pour parente la couche d'histoire qui contient son
  commit, quand une seule le contient. Sinon, sa vue se refuse en le disant, comme `Ambigue` dans les suivies.
- **Le nom de la vue** est `worktree:<chemin>`, l'id stable étant celui des suivies.

### 3. La montre : un seul fil, et un plafond

- **Un seul fil de montre** pour toutes les couches, comme la page 03 l'a décidé. Il écoute les worktrees
  déclarés et les refs suivies.
- **Le plafond** est un réglage du manifeste, `tracked.writes.max_work_layers`, par défaut 32. Au-delà, une
  couche de travail neuve se refuse en se nommant (« plafond des couches de travail atteint : 32, tracked.writes
  .max_work_layers »), et l'état la montre « non écrite ». La place inotify se mesure au premier essai, sur les
  worktrees des sessions de ce poste, avant de fixer ce défaut.

### 4. Le sélecteur de worktrees par motif de chemin

- `{"select": "worktree:*"}` prend tous les worktrees git du dépôt.
- **Ajouté par 2a′-3** : `{"select": "worktree:<motif>"}`, un motif de chemin (`~` étendu), par exemple
  `worktree:~/git_workspaces/cx-*` ou `worktree:~/.cache/rag3weaver-build/wt-*`. Un motif qui ne prend aucun
  worktree se dit dans l'état, sans refus. Un motif mal formé est un refus qui le nomme.

### 5. La fin d'une couche

- Un worktree retiré (par la garde des worktrees, ou à la main) donne une couche finie : `fin` dans sa fiche,
  déjà écrite par les suivies. Sa vue se refuse ensuite : « couche finie le … ». Ses générations partent au
  ménage, au même pas que le retrait de l'ancien index d'une bascule.

## La fraîcheur d'une vue

Elle relève de la page 12 d'archi (`docs/rag3weaver-archi/15-11-2026/12-conception-l-attente-d-index-du-pas.md`)
et de ma page à part sur l'estimation du reste (à venir, point 7 de la vision). Une vue dont la couche de travail
n'est pas à jour répond « pas prêt, ~N s », ou attend sous le seuil réglable. Le reste d'une vue se compte en
fichiers : l'écart entre sa pointe (HEAD et fichiers non commités) et sa dernière génération publiée.

## Le nom court d'une vue (après 2a′-3, avec recherche)

`view: {"worktree": "<chemin>"}`, ou le nom de la tâche, se résout en `worktree:<chemin réel>` dans `call_tool_on`,
au même endroit que `layer`. C'est un sucre : la vue lue reste dite par son id, sa couche et son rang.

## Les témoins, rouges d'abord

1. **Une vue sur un autre worktree** (le témoin de c7) : une recherche venue d'une autre tâche, vue sur un worktree
   de test, voit le fichier non commité de ce worktree, et pas celui du worktree du backend.
2. **Une couche d'histoire sans checkout** : l'écriture d'une génération ne change ni l'index git ni aucun fichier
   d'aucun worktree (empreinte de `git status` et des mtimes avant et après).
3. **La parente épinglée au HEAD** : un worktree dont le HEAD avance attend la génération de sa branche à ce HEAD,
   et sa pile se lit travail → branche au bon rang.
4. **Le plafond** : la 33ᵉ couche de travail se refuse en nommant le réglage.
5. **Le sélecteur par motif** : un motif prend les worktrees attendus, un motif vide se dit, un motif mal formé se
   refuse en se nommant.
6. **La fin** : une vue sur un worktree retiré se refuse en disant sa fin.
7. **Le doublon au bord haut d'un trou** : compté dans l'état, comme la page 03 le dit.

## Ce que 2a′-3 ne promet pas

- pas de publication des générations de rattrapage au-delà de ce que `history` demande ;
- pas d'écriture dans un worktree, ni de checkout ;
- pas de vue sur un worktree d'un autre dépôt (une vue est une couche d'un index de dépôt) ;
- pas l'uuid par le contenu (le ticket d'ED, `docs/everything-declarative/tickets/2026-11-15-l-uuid-par-le-contenu-sans-annexe.md`).

## L'ordre

Après l'atterrissage de 2b′ et de 2b-C, et après la bascule des bases de Lucie :
1. les témoins rouges 1 à 7, poussés sur la branche du lot ;
2. les couches d'histoire, puis les couches de travail, puis la montre et son plafond, puis le sélecteur ;
3. le sucre de nom, avec recherche, dans un lot à part.
