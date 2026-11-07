# Un index par dépôt, des vues par worktree

Lucie, 6 novembre 2026 : « il y a tellement de branches et de PR que faire des
clones à chaque fois de la mémoire me paraît abusé ; il faudrait, dans
l'immédiat, un système plus intelligent qui stocke une version alternative
juste pour les fichiers concernés, et qu'il y ait un filtre worktree dessus ».

## Ce qui existe, et pourquoi ça ne tient pas

Une tâche (`daemon_host/taches.rs`) ouvre un worktree git complet et **une copie
de la base** du backend (`CopieDeBase`, surcharge `database`), puis sa montre
réindexe le code dans cette copie. C'est le modèle de Git appliqué à l'index :
un arbre entier par branche. Il coûte, par tâche, une base ouverte (son cache,
ses verrous, son journal), une réindexation des fichiers touchés dans un index
qui ne partage rien avec le principal, et un worktree plein sur le disque. À
six PR d'agents par jour, c'est six bases ; à l'échelle d'un service hébergé,
c'est impossible.

## La forme

L'index suit **le contenu**, pas le chemin, et un worktree est **une vue**.

- **Une version par contenu.** Un fichier indexé est une version, identifiée par
  l'empreinte de son contenu (le blob de git, ou l'empreinte du texte pour un
  fichier non suivi). Ses morceaux, ses scopes, ses symboles, ses vecteurs, ses
  entrées de plein texte pendent de la version. Deux branches qui ont le même
  fichier partagent sa version et ses vecteurs, calculés une fois.
- **Le grain de la version est le scope, quand c'est possible.** Lucie : « on
  fait des différences au niveau des scopes et de leurs chunks quand possible,
  quand ce n'est pas possible le fichier entier ». Une version de fichier est la
  liste ordonnée de ses versions de scope (fonction, classe, section d'un
  Markdown…), chacune identifiée par l'empreinte de son contenu, position
  comprise à part. Éditer une fonction crée une version de scope et une liste
  neuve ; les autres scopes, leurs morceaux et leurs vecteurs sont partagés, et
  l'embarqueur ne recalcule que le scope changé. Quand le parseur ne découpe pas
  (langage inconnu, fichier plat), le fichier entier est le seul scope, et la
  version est celle du fichier. C'est le même principe que la montre, qui
  réindexe par fichier aujourd'hui : demain elle réindexe par scope.
- **Une vue est une table chemin → version.** Le principal est une vue ; chaque
  worktree, chaque branche, chaque tâche est une vue : la liste des chemins
  dont la version diffère du principal, et rien d'autre. Une tâche qui touche
  trois fichiers pèse trois lignes.
- **Le filtre worktree est un paramètre de lecture.** Tout outil de lecture
  (search_code, read_file, outline, usages, impact, callees, grep…) prend une
  vue, par défaut la vue du backend appelant ; il résout chemin → version par
  la vue, puis lit la version. Sous L6, la vue fait partie de l'instantané de
  l'appel : une recherche dans une tâche voit la tâche, cohérente.
- **Le drain d'une tâche n'indexe que ce qui change.** La montre d'un worktree
  compare l'empreinte de chaque fichier modifié à celle de la vue principale ;
  une empreinte connue est une version déjà indexée (rien à faire), une
  empreinte neuve crée une version ; au grain du scope, seuls les scopes dont
  l'empreinte change sont recalculés. L'édition par le chat qui code (edit_file
  sur la branche de la tâche) produit une version, une seule.
- **Les relations suivent les versions.** Une arête de code (DEFINES, MENTIONS,
  CONSUMED_BY…) part d'une version ; la résolution d'un symbole dans une vue se
  fait sur les versions de la vue. Les deux sens d'une relation sont écrits dans
  la même transaction, et le relevé ancré des deux sens (hotfixs, 25 octobre)
  devient un témoin de l'index.
- **Un changement est un apport de la mémoire.** Lucie : « lier des diffs sur
  les fiches mémoire : un agent qui veut savoir ce qui change dans telle PR va
  voir les diffs appliqués aux fiches ». Quand une vue reçoit une version de
  scope qui diffère du principal, l'hôte écrit un apport L1 de section `diff`
  sur la fiche du sujet touché (le fichier par `path:`, le symbole par
  `symbol:`, et la fiche de session de la tâche), avec la différence lisible
  (avant → après, au grain du scope), la Ref de la version et celle de la vue
  (`view:<tâche>`), et pour source le run qui l'a produite. « Ce qui change
  dans cette PR » est alors une lecture : les apports `diff` dont la source
  est la tâche, groupés par fiche ; un agent les lit comme toute fiche, sans
  ouvrir un diff, et le juge y voit ce que le run a touché. À la fusion, les
  apports restent, cités par la version qui est devenue celle du principal ;
  au rejet, ils restent aussi, avec la clôture, comme l'histoire d'une
  tentative.
- **Une vue se ferme.** Tâche fusionnée ou rejetée : la vue part ; une version
  que plus aucune vue ne cite part avec elle (compte de références), ses
  vecteurs aussi. Une version citée par le principal et la tâche reste.
- **Une seule base, un seul hôte par dépôt.** Plus de copie de base : un backend
  par dépôt, des vues dedans. La mémoire ouverte du poste ne grossit plus avec
  le nombre de branches.

## Quatre difficultés qu'on nomme avant d'écrire

Relecture extérieure du 6 novembre, reprise parce qu'elle est juste :

- **L'identité d'un scope n'est pas son empreinte.** Trois choses distinctes :
  l'identité persistante du symbole (nom qualifié, parent, ce que le graphe
  sait de ses usages), sa version (l'empreinte de son contenu), sa position
  (fichier, lignes). Deux fonctions identiques partagent une version et ses
  calculs, pas une identité.
- **Un scope inchangé peut changer de comportement** parce qu'un type, un
  import ou un appelé a changé : on réutilise son embarquement, jamais ses
  analyses d'impact. Le graphe de dépendances dit ce qu'une version neuve
  invalide chez ses voisins, et ce que le juge doit rejuger.
- **Pas de conflit de structure n'est pas compatibilité.** La fusion scope par
  scope dit seulement qu'aucun scope n'a été modifié des deux côtés ; la
  compatibilité se juge par les tests et le juge sur la vue fusionnée. Le
  moteur ne fusionne jamais sur la seule absence de conflit.
- **La cohérence des vues** : une vue se lit dans un instantané immuable ; une
  suppression dans une vue est une **tombstone**, pas une absence (sinon la vue
  retombe sur la version du principal) ; le compte de références compte les
  vues **et les fiches** qui citent une version (un apport `diff` qui la cite
  la garde) ; le ramasse-miettes ne retire jamais une version encore citée.

Et une nuance qui vaut pour toute la provenance : elle documente ce qui a été
fait et les raisons déclarées ; elle ne prouve pas que ces raisons sont vraies.
C'est le juge qui vérifie, et la fiche dit lequel des deux parle.

## La cinquième et la sixième difficulté (relecture extérieure du 7 novembre)

**La cohérence temporelle entre les couches.** Le MVCC protège la cohérence
du stockage, pas celle d'un assemblage produit par des pipelines asynchrones :
le scope en v2, une relation calculée sur v1, un vecteur de v1 encore en route,
une fiche de v2 — chaque donnée valide, l'assemblage faux, et le graphe répond
avec confiance. Nos morceaux existants : l'instantané L6 épingle un état de la
base par appel ; la fraîcheur de l'index dit `writing:true` ; un vecteur en
retard est une « dette », pas une absence. La règle à poser :
- tout **artefact dérivé** (relation, vecteur, apport, diagnostic) porte
  l'empreinte des entrées qui l'ont produit, le producteur et sa version, et un
  état : `current`, `stale`, `pending`, `failed` ; une empreinte de contenu
  n'est pas une preuve de validité des analyses dérivées (un scope identique
  dont un include a changé a une résolution différente) ;
- une requête **épingle une génération** et ne mélange pas les suivantes ; la
  génération se publie par ensemble de dépendances, jamais par une barrière
  globale (un scope prêt n'attend pas un vecteur lent sans rapport) ;
- un résultat partiel est **explicite** : `pending` ne vaut jamais « pas de
  similarité » ;
- la **publication est conditionnelle** à la génération attendue et
  idempotente : un worker en retard ne ressuscite jamais un artefact effacé ;
  le ramasse-miettes compte aussi les générations encore épinglées et les
  travaux en vol.
- le **témoin cruel**, dans le prototype : pendant qu'un agent modifie un scope
  et ses dépendances, les délais de l'indexeur, de l'embarqueur et de
  l'archiviste varient artificiellement sous requêtes concurrentes ; aucune
  réponse ne mélange deux générations incompatibles.

**L'identité comme hypothèse.** Un déplacement, un renommage, une réécriture
peuvent préserver une identité, mais c'est une correspondance à établir : une
relation `successor_of`, éventuellement ambiguë et datée, plutôt qu'une
identité artificiellement certaine.

**La configuration de bâti.** Deux analyses fraîches du même commit peuvent se
contredire sans qu'aucune soit fausse : flags, macros, features Cargo, cibles.
La résolution sémantique est `ScopeRevision × BuildConfiguration` ; INCLUDES
(la visibilité par la fermeture des #include) en est déjà un cas. Ne pas
multiplier les scopes par configuration : garder seulement les divergences de
résolution qui ont un effet.

## Des diffs au grain du scope

Lucie : « le grain du scope a d'autres avantages que l'IDE pour agents : c'est
aussi un très bon moyen d'avoir des diffs git granulaires ». Deux versions
d'un fichier se comparent par leurs listes de versions de scope, pas par leurs
lignes :

- un scope dont l'empreinte est la même à une autre position ou dans un autre
  fichier est **un déplacement**, pas une suppression et un ajout ;
- un scope dont le corps a la même empreinte sous un autre nom est **un
  renommage** ; les usages à mettre à jour se lisent dans le graphe ;
- un scope dont l'empreinte change est **une modification**, et c'est lui seul
  qu'on relit ; le diff textuel ne se montre qu'à l'intérieur du scope ;
- deux tâches qui touchent deux scopes du même fichier **ne sont pas en
  conflit** : la fusion se fait scope par scope, et le conflit n'existe que sur
  un même scope ;
- l'histoire d'un scope (qui l'a touché, quand, dans quelle tâche, pourquoi) se
  lit par ses versions et leurs apports `diff`, sans blame ligne à ligne.

C'est le diff que le juge, la relecture de l'IDE et `find_tasks` montrent ; git
garde son diff à lui pour ce qui n'est pas du code découpé.

## Ce que ça change pour Git

Le worktree git reste ce qu'il est, l'espace où l'agent écrit et où la personne
relit : il est bon marché tant qu'on n'en fait pas un index. Un jour, la vue
pourra se lire directement dans l'arbre git (chemin → blob) sans checkout,
pour les tâches qui ne font que lire ; ce n'est pas la première marche.

## Graine : des résumés de scopes, et plus loin un transpileur

Lucie, 7 novembre : « un mécanisme de résumés de scopes, avec un titre
indépendant de celui du scope ; et un jour un transpileur universel qui se
baserait sur des résumés imbriqués de scopes, de paramètres et de retours ;
pas tout de suite, très lourd en jetons ». Un résumé est un artefact dérivé
comme les autres : calculé une fois par révision de scope, partagé par toutes
les vues qui ont la même révision, recalculé seulement pour les scopes changés
— c'est le grain du scope qui le rend abordable. Le titre indépendant est ce
que l'archiviste fait déjà pour une fiche. Des résumés imbriqués (paramètres,
retours, effets) sont une signature sémantique par scope : la matière d'un
transpileur, et celle d'un juge qui compare ce qu'un changement dit et ce qu'il
fait. Après les cinq marches.

## Très long terme : un moteur de versionnage par scope

Lucie : « en vision très long terme, ça nous donne un moteur de versionnage
par scope, ce que git ne saurait pas faire ». Git versionne des blobs et des
arbres de chemins ; il n'a pas d'identité d'un scope à travers un déplacement
ou un renommage, et sa fusion est textuelle. Les références à connaître avant
d'écrire une ligne : **Unison** (chaque définition est versionnée par
l'empreinte de son contenu, les noms sont des métadonnées, un renommage ne
change rien au code), **Pijul** et **Darcs** (on versionne des changements qui
commutent, pas des instantanés), **GumTree**, **difftastic**, **Plastic SCM
semantic merge** (diff et fusion d'arbres syntaxiques).

Ce que nous aurions que ces outils n'ont pas : le versionnage s'appuie sur le
même graphe que la recherche, le juge et la mémoire ; il vaut pour tout
langage que tree-sitter découpe ; et il reste compatible avec git en se
matérialisant en blobs à la demande (un commit git est une vue figée). La
difficulté réelle, et la seule : **l'identité d'un scope dont le corps
change**, puisque son empreinte change avec lui. Il faut une identité au-dessus
du contenu : le nom qualifié et le parent, et ce que le graphe sait de ses
usages ; Unison la résout en séparant nom et empreinte, GumTree par la
similarité des arbres. C'est une recherche, pas une marche : elle vient après
les cinq marches, quand l'index en versions aura vécu sur nos dépôts.

## Un worktree est jetable, donc distribuable

Lucie, 7 novembre : « vision très loin : des mini-VM par worktree qui
répartissent toutes seules la charge de travail quand beaucoup d'agents
travaillent en même temps ; une façon de toujours supprimer les worktrees quand
une PR est mergée ; un harnais pour forcer les agents à supprimer l'ancien et
refaire neuf si nécessaire. »

Tout de suite, **le harnais** (ce matin : 46 worktrees à luciepc dont 32 déjà
fusionnés, 338 Gio de targets) :
- une tâche fusionnée ou rejetée efface son worktree, sa vue et son target dans
  le même geste, par l'hôte, pas par la bonne volonté de l'agent ;
- une session qui pousse un lot efface son worktree et son target dans le
  script de push, après les trois contrôles ;
- un contrôle en début de passe refuse un worktree dont la base a plus de N
  commits de retard sur master, ou dont la branche est déjà atterrie : on
  efface et on refait neuf, jamais de rebase héroïque ;
- un relevé périodique de l'hôte liste les worktrees et targets orphelins des
  deux postes et les propose à l'effacement.

À l'horizon, **la mini-machine par vue** : une vue devient un conteneur ou une
micro-VM éphémère qui porte le worktree, son target et son équipier ; un
ordonnanceur la place là où il y a de la place (le portable, luciepc, un pod)
selon la charge, et la détruit à la fusion. L'hôte du dépôt et son index
restent uniques et partagés ; seul le travail se distribue. `poste lourd` et
`poste mesure` sont la version à la main de cet ordonnanceur.

## L'ordre

1. La version comme entité : empreinte, morceaux et vecteurs rattachés ; le
   principal indexé en versions ; témoin : deux fichiers identiques dans deux
   chemins partagent leurs vecteurs (une seule exécution de l'embarqueur).
2. La vue : la table chemin → version, le paramètre de lecture sur les outils,
   la vue par défaut d'un backend ; témoin : une recherche filtrée par vue rend
   la version de la vue et pas celle du principal.
3. La tâche sans copie de base : le worktree + une vue, le drain par empreinte ;
   témoin : trois fichiers édités dans une tâche → trois versions, zéro copie,
   search_code dans la tâche voit l'édition, dans le principal non.
4. La fermeture : compte de références, la vue qui part emporte ses versions
   orphelines ; témoin : la fusion d'une tâche ne laisse rien.
5. Le relevé ancré des deux sens comme témoin permanent de l'index.

Chaque marche est un lot ; la première change le schéma du backend de code et
demande une migration de l'index (une version du format du gabarit, le
manifeste le dit). Sessions : everything declarative (tâches, vues), recherche
(outils de lecture, index), l'arbre principal (L6, résolution).
