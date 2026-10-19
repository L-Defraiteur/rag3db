# Le worktree de tâche : une branche, une base à soi, la relecture, la fusion

19 octobre 2026, everything declarative, sur la demande de c7. C'est le lot 4 de la page 09 de
codeparsers (`../10-octobre-2026-18h20/codeparsers/09-l-agent-qui-edite-des-gabarits.md`, §3),
avec les décisions de c7 en tête de cette page :

- la base d'une tâche est copiée, par reflink quand le système de fichiers le permet, sinon par
  une copie pleine annoncée avec sa taille ;
- elle n'est jamais partagée en écriture ;
- la mémoire longue reste partagée en lecture.

## Ce qui existe

- **La copie d'une base** : `code_tree.rs`.
  - `database_files` liste le fichier principal, `.wal`, `.extensions`, `.fts/` et
    `.checkpoints/`. Le verrou `.readers` ne se copie pas.
  - `copy` tente `cp -a --reflink=always`, puis une copie pleine.
  - Et la règle : **une base ne se copie que fermée**. Un reflink pris pendant une écriture donne
    un journal qui ne se rejoue pas. `add_with_host` ferme la source (`POST /backend/close`),
    copie, puis la rouvre.
- **L'hôte** identifie un backend par son manifeste canonique (`hosted`) et l'ouvre en relisant ce
  manifeste (`open_slot`). Deux backends sur la même base se heurtent à l'ouverture (le verrou du
  moteur), ils ne la partagent pas en silence.
- **Git** : `declare.rs` commite dans le dépôt qui englobe le dossier du backend. Aucun
  `git worktree`, aucune branche, aucune fusion n'existe encore.
- **La mémoire longue** est un autre backend. Aucun backend n'en référence un autre.

## Décisions

1. **Une tâche, un worktree, une branche** : `agent/<nom>`, partie du `HEAD` du dépôt qui englobe
   le backend. Le worktree vit hors du dépôt, sous la racine des tâches de l'hôte
   (`<racine>/<id du backend>/<nom>/arbre`). La racine est `RAG3WEAVER_TACHES_DIR`, sinon
   `~/.local/share/rag3weaver/taches`. Le manifeste de la tâche est le même chemin relatif, dans
   le worktree.
2. **La base de la tâche est une copie, prise sous la porte des écrivains, sans fermer personne**
   (c7 : la règle de Lucie est que personne ne soit bloqué). Ce que le repérage a établi :
   - toutes les écritures passent par le mutex du catalogue. Les lectures passent par la vue
     depuis L4, et les nœuds qui utilisent la connexion brute (`ConnService`) ne font que lire ;
   - le plein texte (lucivy) et le sparse sont validés sur le disque après chaque lot, sous ce
     même mutex (`fts_tampon`, puis `commit` des poignées) ;
   - le point de reprise automatique du moteur ne part qu'à la validation d'une écriture.

   Tenir le mutex est donc la porte des écrivains. `Backend::copier_la_base_sous_la_porte` le
   prend (les écrivains attendent, les lectures continuent), exécute un `CHECKPOINT`, copie
   chaque fichier de la base (`code_tree::database_files`, par reflink, sinon en copie pleine),
   puis rend la porte. Après le `CHECKPOINT`, le WAL n'existe plus : la copie est celle d'un
   point de reprise propre, sans journal à rejouer. Le premier essai l'a montré, en échouant à
   copier un `.wal` absent. Seuls les fichiers présents se copient.

   La copie dit si le reflink a pris, sa taille et la durée de la porte. Elle est refusée
   pendant une indexation. La fermeture du principal n'est pas employée : le repli n'a pas été
   nécessaire.
3. **Le manifeste de la tâche n'est pas réécrit** : c'est une déclaration suivie, et la fusion
   l'emporterait. L'hôte lui impose deux **surcharges** :
   - `database`, la copie ;
   - `declarations: auto`, puisque la tâche est l'espace de l'agent, sur sa branche.

   Elles vivent dans le `PreparedBackend` (`surcharges`). Le rechargement les réapplique
   (`check_reload`, `valider_sur`) : sans cela, une déclaration dans la tâche verrait la base
   changer et serait refusée comme une migration.
4. **La mémoire longue n'est ni copiée ni ouverte par la tâche** : l'agent la lit par l'hôte, comme
   avant. Une tâche n'a que son backend.
5. **La tâche est un backend de l'hôte comme un autre** : son propre id, ouvert par l'hôte. L'état
   (`/etat`) la dit, avec `tache: {nom, parent}`, et l'IDE la montre comme telle. L'agent y déclare
   directement (mode `auto`) ; les commits vont sur sa branche.
6. **La relecture** (`/ide/code/<id>/taches/<nom>`) montre :
   - la différence de la branche contre sa base : les fichiers changés, la différence textuelle,
     et les graphes touchés en diff nœud à nœud (le même diff que les propositions) ;
   - les témoins rejoués : le chargement entier de la branche, sur sa copie de base.
7. **La fusion** se fait en avance rapide, après rebase dans le worktree si la base a bougé (un
   conflit abandonne le rebase et refuse en le disant). Puis le backend principal recharge, et la
   tâche se ferme : son backend est fermé, le worktree retiré, la branche effacée, la copie de base
   retirée. **Le rejet** fait la même fermeture, sans fusion, et garde son pourquoi.
8. **La base principale n'est jamais écrite par la tâche** : la base de la tâche est un autre
   chemin, et un témoin le prouve. La tâche écrit dans sa base, et les fichiers de la base
   principale sont les mêmes, octet pour octet, avant et après.

## Les lots

- **T1** : créer une tâche (le worktree, la branche, la copie fermée, les surcharges, l'ouverture
  par l'hôte), la dire dans `/etat`, la rejeter et la fermer.
  - Témoin : la base principale n'est jamais écrite par la tâche.
  - Témoin : une déclaration dans la tâche commite sur sa branche et ne touche pas l'arbre
    principal.
- **T2** : la relecture et la fusion, les routes et la vue de l'IDE.
  - Le témoin de bout en bout dans le navigateur : une tâche ouverte, une déclaration dans la
    tâche, la relecture (texte et graphe), puis la fusion en avance rapide, avec le backend
    principal qui sert le neuf ; et un rejet.

## T2, tel qu'il est fait

- **La relecture** (`DaemonHost::relecture`) lit la branche par git, sans ouvrir de fichier du
  worktree à la main : `git diff --name-only <départ>...agent/<nom>` donne les chemins touchés,
  `git show` leur texte avant et après. Les graphes touchés et leur diff nœud à nœud viennent de
  la même fonction que les propositions (`propositions::graphes_touches`, extraite pour servir
  aux deux). Les témoins rejoués sont le rechargement entier du backend de la tâche
  (`recharger`, sur sa copie de base, avec ses surcharges) : `chargement` vrai, ou le `refus`.
  `base_a_bouge` dit si le `HEAD` du dépôt a avancé depuis le départ.
- **La fusion** (`fusionner_une_tache`), dans cet ordre :
  1. le worktree de la tâche n'a rien de non commité (les fichiers non suivis n'entrent pas
     dans une fusion et ne la bloquent pas) ;
  2. si la base a bougé, rebase dans le worktree ; un conflit abandonne le rebase et refuse ;
  3. **les fichiers de la branche sont validés sur le backend principal, par recouvrement**
     (`valider_sur`), avant que rien ne bouge : une branche qui se charge sur sa copie mais pas
     sur le principal est refusée, rien n'est fusionné ;
  4. l'arbre principal n'a rien de non commité ;
  5. `git merge --ff-only`, le principal recharge, la tâche se ferme.
- **Le rejet** ferme sans fusion et garde son pourquoi. Les tâches fermées (fusionnées avec
  leur commit, rejetées avec leur pourquoi) sont gardées en mémoire par l'hôte.
- **L'IDE** : `/ide/code/<id>/taches/` liste les tâches ouvertes (branche, copie et durée de la
  porte, liens vers les vues de leur backend) et fermées, avec le formulaire pour en ouvrir une ;
  `/ide/code/<id>/taches/<nom>` est la relecture, avec Fusionner et Rejeter. Le dessin d'un
  graphe en diff est le même que celui des propositions (`sectionDeGraphe`). L'accueil montre
  un lien « tâches » par backend, et une pastille sur un backend qui est une tâche.
- **Le témoin** : `une_tache_se_relit_puis_se_fusionne_dans_l_ide` (`apercu_a_chaud.rs`). Une
  tâche ouverte par le formulaire ; le graphe de clic du compteur changé dans la tâche (`n + 5`,
  une proposition appliquée sur son backend, donc un commit sur sa branche), l'arbre principal
  intact ; la relecture montre le texte neuf, le nœud `plus` changé et la branche qui se charge ;
  Fusionner amène le commit dans l'arbre principal, la branche est effacée et l'aperçu du
  principal compte par cinq ; une seconde tâche est rejetée avec son pourquoi, sans rien écrire.
  La console du navigateur est vide. Les tâches du témoin vivent dans son dossier
  (`hote_garde_avec`, racine des tâches réglée), jamais sous `~/.local`.
