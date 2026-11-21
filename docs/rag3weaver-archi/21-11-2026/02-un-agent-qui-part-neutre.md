# Un agent qui part neutre

*rag3weaver archi, 21 novembre 2026, à la demande de Lucie relayée par c7 (8 h 1x). Une page de conception commune, avant
tout code. Ce qui est dit « aujourd'hui » a été relevé dans le code de master ce matin, sans être joué, sauf mention
contraire.*

## Le parcours voulu

Lucie : « tu lances un agent puis tu lui dis ce que tu veux faire, va dans tel dossier… démarre un travail sur tel repo
GitHub… crée-moi un repo GitHub ; à la base tu pars neutre, ou à la limite tu ouvres un dossier d'avance ; l'agent doit
très bien pouvoir te guider : est-ce que tu veux que je crée un index pour ce dossier ? non merci, et hop il travaille
avec les outils standards (sauf mémoire) ».

Trois règles en découlent :
1. on peut démarrer sans dossier ;
2. l'index est une proposition, et le refus est un état normal, pas une panne ;
3. rien ne sort de la machine (un push, un dépôt GitHub créé) sans l'accord explicite de l'utilisateur.

## 1. Un agent démarré sans dossier

**Ce qu'il a aujourd'hui.** Le point de départ neutre existe déjà : c'est **l'atelier** (backend de type `project`, celui
de Lucie est `bb347ff93e83`). Il a les outils standards de fichiers sur ce qu'il a attaché (`read_file`, `list_files`,
`glob`, `grep_files`, `write`, `replace_text`, `replace_lines`, `insert_lines`, `run_command`), les outils de worktree
(`list_worktrees`, `attach_worktree`, `open_branch`), et `ask_permission`. L'hôte sait ouvrir un dossier :
`ouvrir_un_dossier`, route `POST /dossiers/ouvrir`. Un dossier absent ou vide y est créé avec `git init`, jamais poussé ;
l'arbre de Lucie, sairen et le dossier d'une autre session sont refusés.

**Ce qui lui manque.**
- **La mémoire longue.** Le gabarit de l'atelier ne déclare ni `memory` ni outils de mémoire. Lucie la veut partout (« sauf
  mémoire » se lit « les outils standards, plus la mémoire »).
- **Le geste « ouvrir un dossier » comme outil de l'agent.** `ouvrir_un_dossier` est une route de l'hôte, pas un outil du
  chat : l'agent ne peut pas le faire quand on le lui demande.
- **Un attachement sans index.** Aujourd'hui, attacher un worktree lance son premier index (`index-suivi:<id>`). C'est à
  vérifier par arbre : peut-on attacher en servant read, grep, glob, edit et run sans index ?
- **Une invite qui guide.** La consigne de l'atelier dit l'ordre de ses outils de worktree ; elle ne dit rien d'un départ
  sans dossier, ni de proposer l'index.

## 2. Ses gestes

| Geste | Aujourd'hui | Attendu |
|---|---|---|
| Aller dans un dossier | route `POST /dossiers/ouvrir` (lot ouvrir-un-dossier d'arbre) | un outil `open_folder(path)` de l'atelier, qui appelle la même fonction et attache **sans index** |
| Cloner un dépôt GitHub | `run_command git clone …` : réseau, donc une carte d'accord ; destination libre | un outil `clone_repository(url, dossier?)` : le clone va dans l'espace des worktrees (`~/git_workspaces/<session>/…`), puis il est attaché sans index ; la sortie réseau reste une approbation |
| Créer un dépôt local | `ouvrir_un_dossier` sur un chemin neuf (crée et `git init`) | le même `open_folder`, rien de plus |
| Créer sur GitHub | rien de dédié ; `gh repo create` passerait par `run_command`, réseau et écriture | un outil `create_github_repository(nom, privé)`, **toujours** sous une approbation qui montre le nom, le compte et la visibilité ; il ne pousse rien. Le premier push est un autre geste, approuvé à part |

Pour tous ces gestes, l'agent n'agit pas sur un dépôt distant sans un « oui » de l'utilisateur sur **cette** opération.
Le mode `auto` ne couvre pas ces gestes ; une carte de famille déjà accordée non plus. C'est la règle déjà tenue pour
`git config --global` et l'identité git (lot identite-git) : refusée quoi qu'il en soit, ou demandée à chaque fois.

## 3. L'index proposé, le refus comme état normal

- **Ne pas proposer sans raison** (Lucie, 8 h 1x) : « il doit pas forcément demander d'indexer non plus, si c'est même pas
  un projet de code ou des docs, il faut pas qu'il propose h24, genre « stp va voir mon dossier steam j'ai l'impression que
  j'ai trop de jeux lourds… » ».
- **Le critère.** L'agent propose l'index seulement quand **les deux** conditions tiennent :
  1. le dossier est un projet de code ou de docs, ce qui se lit sans modèle : un dépôt git, ou un manifeste connu
     (`Cargo.toml`, `package.json`, `pyproject.toml`, `CMakeLists.txt`…), ou une majorité de fichiers source et texte ;
  2. la demande est un travail dans la durée sur ce projet (le développer, le corriger, le documenter), et non une tâche
     ponctuelle (« regarde ce qui prend de la place », « renomme ces fichiers »).
  Une tâche ponctuelle sur un dossier quelconque se fait avec les outils standards, sans question.
- **Une seule fois.** La proposition est chiffrée : la taille du dossier et une estimation du temps (l'outil `estimate`
  existe). Un refus, ou un dossier jugé hors critère, est retenu pour ce dossier, et la question ne revient pas.
- **Où le critère vit.** En deux parts :
  - la condition 1 est une **règle déclarée**, lue par l'hôte. C'est une liste de marqueurs dans le gabarit de l'atelier
    (`index_offer.markers`), que l'hôte rend dans le résultat de `open_folder` sous la forme `"index_offer": true|false` ;
    le refus retenu est écrit au même endroit que l'état du dossier ;
  - la condition 2 est un jugement sur la demande, donc dans **l'invite** déclarée de l'atelier (`chat_instructions`) :
    « ne propose l'index que si open_folder dit index_offer et si la demande est un travail suivi sur ce projet ; sinon
    travaille avec les outils de fichiers sans rien demander ».
  Le modèle ne propose donc jamais un index que l'hôte n'a pas déclaré possible.
- Un dossier attaché **sans index** est servi par les outils de fichiers (`read_file`, `grep_files`, `glob`, `list_files`,
  `write`/`replace_*`, `run_command`). `search_code`, `usages`, `outline` et `impact` répondent « pas d'index sur ce
  dossier ; veux-tu que je le crée ? ». C'est une réponse, pas une erreur : ni ROUGE ni panne dans l'interface.
- La mémoire longue reste disponible avec ou sans index : elle ne dépend d'aucun backend de code.
- L'index peut être créé plus tard à la demande (« indexe ce dossier »), et l'état du dossier le dit (`/ide/etat` : sans
  index, en cours, prêt).

### Les témoins du critère

Rouges d'abord, chacun dans un fichier de tests à lui :
- **Un dossier hors code, aucune proposition.** Un dossier de gros fichiers binaires sans manifeste ni git (le « dossier
  steam »). `open_folder` rend `index_offer: false`. Sur faux modèle, avec la demande « regarde ce qui prend de la
  place », le tour se fait par `list_files` et `run_command du`, et ni la boîte ni le texte ne parlent d'index.
- **Un projet de code, une proposition, une seule.** Un dépôt git avec un `Cargo.toml`. `open_folder` rend
  `index_offer: true`. Un « non merci » est retenu, et le `open_folder` suivant sur le même dossier rend
  `index_offer: false`, avec la raison « refusé le … ».
- **Une tâche ponctuelle sur un projet, aucune proposition.** La condition 2 se joue par l'invite. Ce témoin est un rejeu
  par le lanceur sur le vrai modèle, jugé par le banc : une règle d'invite ne se prouve pas sur faux modèle.

## 4. Qui fait quoi

| Qui | Quoi |
|---|---|
| **archi** | Le comportement de l'agent : l'invite de l'atelier (départ neutre, proposer l'index une fois, retenir le refus), les outils `open_folder`, `clone_repository`, `create_github_repository` côté chat, et leurs approbations (aucun geste distant sans « oui » sur l'opération) |
| **arbre** | Attacher sans index ; cloner dans l'espace des worktrees ; indexer à la demande ; l'état « sans index » d'un dossier |
| **recherche** | « Nouvel agent » dans l'IDE, le dossier optionnel au départ ; l'affichage d'un dossier sans index comme état normal ; la carte d'approbation d'un geste distant |
| **tty** | Le terminal sans `--repo` : démarrer sur l'atelier |
| **mémoire** | La mémoire sans backend de code : les outils de mémoire déclarés dans le gabarit de l'atelier |

## L'ordre proposé

1. arbre : attacher sans index (le reste en dépend) ;
2. archi : l'invite et `open_folder` ; mémoire : la mémoire dans l'atelier ;
3. recherche et tty : le départ neutre dans l'IDE et au terminal ;
4. archi et arbre : `clone_repository`, puis `create_github_repository` sous approbation.

Chaque lot commence par son témoin rouge, dans un fichier de tests à lui.

## À trancher par Lucie

- La liste des marqueurs d'un projet de code ou de docs : celle ci-dessus suffit-elle (un dossier de notes en Markdown
  sans git compte-t-il) ?
- `create_github_repository` : privé par défaut ? Sur quel compte : celui de `gh auth status` au moment du geste, dit
  dans l'approbation, ou un compte déclaré par le manifeste ?
