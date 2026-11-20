# Ouvrir et fermer une vue : les lots

20 novembre 2026, au mot de c7. Le troisième point de la page 02 (`02-les-outils-des-vues.md`), découpé avant tout code.
`views` et `use_view` (lot `arbre-views-use-view`) en sont la base : l'état d'une session gardé par son `sessionId`.

## 1. À qui est un dossier

Les dossiers des sessions existent déjà sous `~/git_workspaces/`, créés à la main (`arbre`, `archi`, `G`, `ed`,
`hotfixs`…). Leur nom ne se déduit pas du nom de la session (« arbre principal » a choisi `arbre`, « optimiseur » `G`).

**Je propose un fichier des dossiers**, `~/.config/rag3weaver/dossiers-des-sessions.json` :
`{"arbre": "arbre principal", "G": "optimiseur", …}`, le dossier vers le `name` de la session dans le registre de Claude
Code. c7 l'écrit une fois pour les sessions d'aujourd'hui ; l'hôte y ajoute un dossier neuf.

- un dossier inscrit au nom de la session qui appelle : accepté ;
- un dossier inscrit au nom d'une autre : **refusé**, en nommant à qui il est ;
- un dossier absent du fichier **et** du disque : inscrit au nom de la session qui le demande ;
- un dossier absent du fichier mais présent sur le disque : **refusé** (« dossier existant, sans propriétaire inscrit :
  le fichier des dossiers le dit à qui il est »). L'hôte ne devine pas.

## 2. La garde du chemin, avant tout `git worktree add`

Le chemin est `<espace>/<dossier>/<jj-mm-aaaa>-<lot>`, l'espace étant `~/git_workspaces`. Dans cet ordre, chaque refus
nomme le chemin et la règle :

1. **l'arbre de Lucie** : `<espace>/rag3db`, et tout chemin dessous ;
2. **sairen** : tout dossier de l'espace dont le nom commence par `sairen` ;
3. **le dossier d'une autre session** (§ 1) ;
4. **hors de `<espace>/<dossier de la session>/`** : un `lot` qui contient `/`, `..`, ou qui sort du dossier une fois
   le chemin résolu ;
5. **un chemin déjà pris** (un dossier qui existe).

Le nom du lot n'admet que lettres, chiffres, `-` et `_`.

## 2 bis. Un poste ou quinze (Lucie, 20 novembre, 18 h 1x)

« Imaginons le mec, il a quinze ordis, il doit pouvoir s'en sortir quand même, ou un seul. » Donc :

- **Le poste, c'est son hostname.** Aucun nom de poste n'est écrit dans le code.
- **Un hôte par poste** gère ses worktrees et ses vues.
- **La table des dossiers vit par poste**, dans la configuration de l'hôte. Une session vit sur un poste à la fois,
  et un même nom de session peut avoir un dossier sur chaque poste.
- **La fermeture tirée (O4) lit les atterrissages de son poste par une voie que sa configuration déclare** : où est
  le journal du pousseur, et comment le lire (un chemin local, ou une commande). Sur un seul poste, tout est local,
  sans ssh. Sur N postes, chaque hôte a sa voie. Jamais de `ssh` vers `luciepc` écrit en dur.
- **Les témoins jouent les deux cas** :
  - un seul poste : le journal en local ;
  - N postes : un faux hostname, deux tables, et deux journaux, dont un lu par la voie déclarée. Chaque hôte ne ferme
    que les vues de son poste.

## 3. Les lots

**O1, `open_view` sans index** (½ jour d'agent). La garde du § 2, le fichier des dossiers, puis
`git worktree add -b <dossier>/<lot> <chemin> origin/master` dans le dépôt du backend, et
`git submodule update --init`. L'état de la session note la vue ouverte : chemin, branche, target déclaré
(`~/.cache/rag3weaver-build/target-<dossier>/<jj-mm-aaaa>-<lot>`). La réponse est `{name, path, branch, target}`.
Elle n'est pas encore active : sa couche n'existe qu'après O2.

Témoins rouges, un par garde :

- l'arbre de Lucie refusé ;
- sairen refusé ;
- le dossier d'une autre session refusé ;
- un `lot` qui sort du dossier refusé ;
- puis le cas juste : l'arbre créé sous le dossier de la session, sur une branche neuve depuis `origin/master`, avec le
  sous-module initialisé.

**O2, la couche de travail au point de fourche** (1 jour). Une couche de travail dont la parente est la couche
d'histoire de master au rang du `merge-base`, et qui porte l'écart entier : commits du lot, non commités, non suivis.
C'est une variante de `write_work_generation`, qui n'exige plus la couche de la branche au HEAD. `open_view` indexe la
vue neuve et la rend active ; la montre la suit comme les autres couches de travail.

Témoin rouge : un worktree avec un commit et une modification non commitée. Sa vue lit les deux, et la vue du backend
n'en lit aucun.

**O3, `close_view`** (½ jour). Les refus, dans cet ordre :

- une vue que l'hôte n'a pas ouverte pour cette session ;
- un arbre sale, sous-module compris ;
- une tête qu'aucune branche distante ne porte ;
- un processus qui travaille dedans.

Sinon : `git worktree remove --force`, `prune`, le target effacé, la couche finie, la vue oubliée. L'active d'une session
qui la lisait retombe sur le backend, et la réponse suivante le dit une fois.

Témoins rouges : un arbre sale refusé, l'arbre de Lucie refusé (non ouvert par l'hôte), un processus dedans refusé, puis
un arbre propre retiré avec son target.

**O4, la fermeture tirée des atterrissages** (¼ jour). Toutes les quelques minutes, l'hôte lit les atterrissages
de son poste (la colonne `vue` de `landed.tsv`), par la voie que déclare sa configuration (§ 2 bis). Il ferme les vues
dont le poste est son hostname, par `close_view`, et un refus va au journal du poste.

Témoins rouges :

- un seul poste, journal local : une vue du poste se ferme par son nom, même quand la branche atterrie n'est pas
  celle de l'auteur ;
- deux postes : deux hôtes de hostnames faux, deux tables, un journal lu par une commande déclarée. Chacun ne ferme
  que sa vue.

**Total** : 2 ¼ jours d'agent. O1, O3 et O4 tiennent sans O2 : une vue ouverte sans couche se ferme quand même.

## 4. À trancher par c7 ou Lucie

- **Le fichier des dossiers** (§ 1) : qui l'écrit, et ce qu'il contient aujourd'hui.
- **La branche d'une vue ouverte** : `<dossier>/<lot>` (`arbre/views-use-view`), ou la forme des branches d'aujourd'hui
  (`arbre-principal/<lot>`). La première suit le dossier, la seconde garde l'habitude.
