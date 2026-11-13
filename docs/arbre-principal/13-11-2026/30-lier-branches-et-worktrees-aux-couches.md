# Lier les branches et les worktrees aux couches (le pas des vues après 1a et 1b)

13 novembre 2026, arbre principal, à la demande de c7. Page de conception : aucun code avant le mot de
Lucie. Relue par recherche et par ED le 13 novembre ; leurs points sont reportés et attribués. Elle s'appuie sur la conception du 8 novembre
(`extension/rag3weaver/docs/08-novembre-2026-arbre/01-conception-des-vues-marche-1.md`, §§ 2 à 5),
sur le résolveur de couches 2a (`src/dataflow/migrations/publication/layers.rs`) et sur la décision
de Lucie du 13 novembre : les vues se bâtissent sur l'index de code réel, sans second index.

## Ce qu'il faut relier

Le résolveur 2a sait composer une pile de couches épinglées. Pour chaque occurrence, la couche la
plus proche gagne, et un tombstone masque le dessous. Mais une couche n'y est qu'un nom libre, plus
une publication tirée au hasard : rien ne la relie à Git. Cette page dit comment une branche, un
worktree et un commit deviennent des couches et des générations.

## 1. Une couche par branche, une ancre par dépôt

- **L'ancre** est la branche principale du dépôt (`master` ou `main`), indexée en entier une fois.
  C'est la couche du bas.
- **Une couche = une branche du dépôt**, nommée `<dépôt>:<branche>`. Elle ne porte que les
  occurrences qu'elle change par rapport à sa parente, plus ses tombstones (scopes et fichiers
  retirés).
- **Sa parente** est la couche d'où elle est née, l'ancre ou une autre branche (la pyramide de
  Lucie). Git ne garde pas cette information : il faut la noter quelque part.
  - Une tâche ouverte par l'atelier la note à sa création : `ouvrir_une_tache` connaît déjà sa
    branche de départ et son commit (`depart`).
  - Pour une branche venue d'ailleurs, je propose une inférence prudente : parmi les couches
    connues, celle dont la pointe est l'ancêtre le plus proche du point de fourche. À défaut,
    l'ancre. **Question 1.**
  - L'admission (ED) : une parente notée par l'atelier est une déclaration, admise. Une parente
    inférée n'est admise que si la réponse est **unique** : deux candidates à égale distance font
    une vue ambiguë, refusée en le disant (le contrat du 11), jamais un choix. Une fois admise,
    l'inférence s'**estampille** avec son genre (« inférée ») et n'est plus recalculée à chaque
    ouverture : l'admission lit une déclaration ou une estampille, jamais une inférence fraîche. La
    provenance d'une occurrence venue d'une parente inférée le dit.
- **Sa base** est le commit de la fourche. La couche se définit contre sa base, pas contre la pointe
  du moment de sa parente (la mesure du 7 novembre : 0 à 8 chemins contre la base, 5 à 53 contre le
  master du moment).

## 2. Une génération par commit indexé

- **Une génération est l'état publié d'une couche pour un commit.** Son identité (ED, relecture du
  13 novembre) :
  - le commit ;
  - la couche et le commit de sa base, puisqu'une même pointe déclarée sur une autre parente ne
    porte pas les mêmes occurrences ;
  - l'empreinte de la recette (format de la version de scope, découpage, versions de l'analyseur ;
    le modèle reste par colonne), et non un compteur.

  Deux processus qui indexent le même commit avec la même recette parlent de la même génération :
  réindexer est idempotent, une autre recette donne une autre génération, sans tirage ni compteur à
  synchroniser. Le nom de branche n'est qu'une étiquette : une branche renommée, ou effacée puis
  recréée, n'hérite pas d'une génération par son nom. Le registre de Codex (chaîne de publications, refus d'une tête qui a bougé)
  s'adopte pour la publication. Ses identifiants tirés au hasard ne s'adoptent pas.
- **Quand une génération se publie.** Je propose : quand l'index a fini un commit, c'est-à-dire
  quand la montre voit `HEAD` bouger et que les versions neuves sont écrites. La génération
  précédente reste lisible jusqu'à la fin des lectures en cours (épinglage L6). **Question 2** :
  à chaque commit, ou seulement à la demande, au push, ou à la fin d'une tâche ?
- **La fusion** d'une branche dans sa parente publie une génération de la parente au commit de
  fusion. Les versions de la couche y descendent sans réembarquement, ses tombstones s'appliquent,
  et la couche se ferme (§ 5 de la conception du 8 novembre).
- **Un rebase** déplace la base : c'est une génération neuve de la couche sur une autre génération
  de sa parente, publiée explicitement. Jamais un glissement silencieux. **Question 4.**

## 3. La couche de travail : le non-commité d'un worktree

- **Attacher un worktree, c'est monter la couche de sa branche**, plus une **couche de travail** :
  ce qui est modifié et pas encore commité.
- **La couche de travail n'est pas une génération.** Elle est vivante : la montre la réécrit à chaque
  édition, elle n'a pas d'identité stable, elle ne se publie pas. Elle appartient au worktree.
  - Elle reste hors du chargeur durable (2b-B), qui ne charge que des générations publiées. Elle
    vit dans l'hôte, par worktree, et se reconstruit depuis le disque après un redémarrage (ED).
  - Une lecture la fige : à son début, elle prend la génération épinglée **et** un instantané de la
    couche de travail. C'est une table immuable, échangée en entier à chaque réécriture de la
    montre. Une réécriture pendant la lecture ne la touche pas (ED).
  - Au commit, ses occurrences entrent dans la génération neuve de la branche, et elle se vide.
  - À l'effacement du worktree, elle part.
- **Question 3** : la couche de travail d'un worktree est-elle visible des autres sessions (un agent
  qui lit le worktree d'un autre), ou seulement de celle qui le tient ? Proposition d'ED : lisible
  par un `view` explicite (`<worktree>`), jamais par défaut ; la vue courante d'une session ne prend
  que la couche de travail de son propre worktree.

## 4. Ce qu'on lit, et ce que l'IDE montre

- **La vue courante se choisit par le contexte** (Lucie, 11 novembre) :
  - une session attachée à un worktree lit sa branche plus sa couche de travail ;
  - sinon, l'ancre.
  - Les outils prennent un paramètre `view` pour changer de vue ; une vue inconnue est refusée avant
    toute recherche.
- **Une version ancienne se demande explicitement** : `view: <branche>@<commit>`. Si ce commit a une
  génération, elle se lit sans rien indexer. Sinon, on matérialise ce seul état, sans indexer les
  commits intermédiaires.
- **Le coût d'une version ancienne** (recherche, relecture du 13 novembre) :
  - Avec 1a, ce n'est pas un index entier. Un vieux commit retombe surtout sur des versions déjà
    embarquées ; seules les versions jamais vues coûtent du dense. L'analyse se fait en delta contre
    l'ancêtre le plus proche qui a une génération, par `git show`, sans checkout.
  - **Le texte d'abord** : la réponse sert le plein texte tout de suite, la dette dense se paie hors
    du verrou, et la réponse le dit (« dense partiel : N versions en cours »). Une lecture n'attend
    jamais l'embarqueur.
  - **Une seule matérialisation par (couche, commit)**, sous un verrou par génération : deux lectures
    simultanées se partagent le travail.
  - **Ces générations de lecture sont jetables**, distinctes de celles qu'un commit publie, et
    évincées en premier (un cas de plus pour la question 5).
- **Lire une génération jusqu'au bout pendant qu'un commit publie la suivante** (recherche) :
  l'épinglage L6 ne suffit pas, car lucivy, et peut-être l'index dense, vivent hors de la MVCC de
  rag3db. L'instantané composé reste petit :
  1. un seul searcher lucivy pour toute la lecture, pris au début, jamais rechargé au milieu ;
  2. chaque résultat, plein texte ou dense, se résout par l'instantané épinglé : de la version à
     l'occurrence visible dans la génération lue. Un résultat d'une version neuve n'y a pas
     d'occurrence et tombe ;
  3. aucun effacement physique d'une version qu'une lecture peut voir : le ramasse-miettes (M5)
     attend les lectures épinglées, sous un bail ;
  4. le masque avant le top-k (2b-B), ou un sur-tirage, faute de quoi le rappel baisse en silence.
  - **L'index dense suit l'instantané de son lecteur** (le cœur, prouvé par un témoin vert :
    `VectorSearchUnderSnapshot`, `test/transaction/vector_search_under_snapshot_test.cpp`, branche
    `lecture-et-instantane`, d5aa49c49). Chaque nœud se lit sous la transaction du lecteur. Un nœud
    invisible à son instantané rend un vecteur nul : il n'est ni rendu ni compté. Un vecteur validé
    après le début de la lecture n'est jamais rendu. **La réserve** : le graphe lui-même n'est pas
    versionné. Les résultats sont exacts pour l'instantané, mais le rappel n'est pas celui d'un index
    figé à cet instantané. Si une vue exige un top-k exact, il faut un repli exhaustif sous le masque
    quand le rappel compte.
  - **Ce que cela change** : le masque des vues dans le dense (2b-B) n'attend pas de lot moteur ni
    de lib neuve. La lecture sous instantané est déjà là ; il reste à appliquer le masque avant le
    top-k, avec un repli exhaustif sous le masque quand un top-k exact compte.
- **Le plein texte suit le même modèle que le graphe et les vecteurs** (demande de Lucie) : un
  document lucivy par version de scope, une génération comme ensemble d'identifiants tenu par le
  graphe, une vue comme requête plus masque **avant** le top-k. Ce que lucivy en dit (lu dans le
  code, non mesuré) :
  - le masque existe déjà : `ShardedHandle` prend un `allowed_ids` de `_node_id`, changé en bitset
    par segment, qui sert de documents vivants au collecteur et de filtre aux pré-recherches. La vue
    est classée et coupée en elle-même, pas filtrée après coup ;
  - le classement : N vaut la taille de la vue, et le df des recherches `contains` et regex vient de
    la pré-recherche, qui ne voit que la vue. La longueur moyenne reste celle du corpus, exprès. Une
    réserve, non vérifiée : `doc_freq(term)` somme sur tout l'index pour une requête par terme qui
    ne passe pas par la pré-recherche, d'où une IDF possiblement incohérente. lucivy en fait un
    témoin rouge d'abord, dans son ticket ;
  - le coût : un passage sur `_node_id` par segment et par requête. Un bitset gardé par segment et
    par génération est à chiffrer, pas à faire. lucivy n'a pas besoin de connaître les versions.
- **Les trois lunettes de l'IDE** (« voir en tant que… ») :
  - **worktree** : la branche plus la couche de travail, ce que l'agent voit ;
  - **branche** : la dernière génération publiée de la branche, sans le non-commité ;
  - **pyramide** : la même vue, chaque scope marqué de sa couche (de la vue, de la parente, de
    l'ancre). On remonte la pile scope par scope, comme Lucie l'a décrite le 8 novembre.
- **Deux ajouts de recherche** :
  - la lunette worktree marque le non-commité à part, puisqu'il n'a pas de génération, et dit ce
    qu'il n'a pas encore : une édition pas encore embarquée, trouvable par mots seulement (la dette,
    montrée scope par scope) ;
  - la pyramide montre, à la demande, ce qu'un tombstone masque (le scope de la parente ou de
    l'ancre retiré par la branche). Sinon, « pourquoi je ne vois plus `f` » n'a pas de réponse.
- **La provenance** reste à la demande pour un agent (`provenance: true`), sans coût sinon. L'IDE la
  montre toujours.

## 5. Ce qui s'appuie sur 1a et 1b

- **Sur 1a** (la version de scope, en cours) : une couche ne porte que des références d'occurrences
  vers des versions partagées. Ouvrir une couche ne coûte que les versions neuves, embarquées une
  fois (12 à 151 par branche au lieu de 88 à 1 278 scopes).
- **Sur 1b** (la migration, à son mot) : une base de la forme d'avant n'a pas de versions, donc pas
  de couches. Tant qu'elle n'est pas migrée, ses vues sont indisponibles, et elle le dit (« à
  migrer ») au lieu de servir une vue fausse.
- **Le pas suivant** dépend de celui-ci : le chargeur durable des couches (2b-B), le masque avant le
  top-k dans les signaux réels, puis le paramètre `view` des outils (2b-C). Le témoin cruel (aucune
  réponse ne mélange deux générations) se joue dès qu'une vue est interrogeable.

## Les décisions de Lucie (13 novembre 2026, 21 h 30, relayées par c7)

1. **La parente** est inférée par le point de fourche et marquée « inférée ». Elle n'est admise que
   si elle est unique ; sinon, elle est demandée à l'ouverture.
2. **La publication** : une génération à chaque commit indexé, « pour l'instant ».
3. **La couche de travail** est lisible des autres sessions, sous le nom `travail`. Son mot : c'est
   un argument pour plusieurs agents qui codent en même temps et se parlent.
4. **Le rebase** : la couche suit d'elle-même, « pour l'instant ».
5. **L'histoire.** Lucie déplace la question et n'est « pas encore très sûre ». Plutôt que de
   décider combien de générations garder, on **choisit explicitement ce qu'on indexe** :
   - on déclare, par dépôt, les **branches suivies** (l'ancre toujours, puis telle et telle branche
     partie d'elle) ;
   - les branches et les commits entre deux ne sont **pas indexés systématiquement** : on sait
     seulement qu'ils existent (le graphe git les connaît, l'index non) ;
   - `view: <branche>@<commit>` d'un commit jamais indexé le matérialise à la demande, jetable
     (§ 4) ;
   - la rétention des générations indexées reste ouverte : on garde tout pour l'instant (le texte
     seul pour les anciennes), et on ne décide la coupe que sur une mesure.

**La précision de Lucie sur la réponse 5**, dans ses mots : « si un entre-deux est l'état d'un scope,
malgré qu'on n'ait pas indexé l'entre-deux, on peut le savoir en regardant le nœud, mais pour
chercher dans cet entre-deux, il faut d'abord indexer explicitement ». Il y a donc deux niveaux :
- **connu** : le nœud (le commit dans le graphe, et les scopes qu'il change) existe avec son
  identité, lisible depuis le nœud, sans embarquement ni génération complète. Son coût est celui
  d'une lecture git, pas d'un parse complet ;
- **cherchable** : seulement après un index explicite de ce commit (`view: <branche>@<commit>`, ou
  un geste « indexer ce commit » dans l'IDE), qui publie sa génération.

**Le choix à trancher pour « connu »** : comment savoir quels scopes un commit change, sans l'indexer ?
- **(a) Par la seule lecture git**, que je propose : `git diff -U0 <ancêtre indexé>..<commit>` donne
  les fichiers et les plages de lignes touchées. Croisées avec les positions des occurrences de
  l'ancêtre indexé le plus proche, déjà dans l'index, elles nomment les scopes modifiés ou retirés.
  Ce qui ne se nomme pas sans analyse, ce sont les scopes **neufs** : ils se disent « lignes
  ajoutées dans tel fichier, à telle place ».
- **(b) Par un parse des seuls fichiers changés**, sans embarquement : il donne aussi le nom et
  l'adresse des scopes neufs, mais coûte l'analyse de ces fichiers à chaque regard sur le nœud.

Le témoin de « connu, non cherchable » : un commit non suivi apparaît au nœud avec les scopes qu'il
touche ; une recherche sous `view` de ce commit répond « non indexé, indexer ? » au lieu d'un
résultat ; après l'index explicite, elle répond.

**Le modèle qui en sort** : par dépôt, un ensemble déclaré de branches suivies (l'ancre toujours),
une génération par commit indexé d'une branche suivie, et « connu, non indexé » pour le reste.
**Le témoin qui le prouve** : une branche non suivie n'a pas de couche ; l'IDE la montre comme
connue ; `view` la matérialise sans l'ajouter aux suivies.

## La rétention : une règle proposée, à trancher par Lucie (lot à part, après la liaison)

Lucie, sur la réponse 5 : « on essaie de garder une culture de tout ce que touche l'agent, ça reste
indexé, puis des branches intermédiaires sont supprimées quand trop lourd ? problème : faut savoir
c'est quoi la bonne racine à garder ». La règle que propose c7, à trancher par elle à la relecture :

- **Tout ce que l'agent touche reste indexé tant que la branche vit** : la couche de travail et
  chaque commit. Le « connu, non indexé » ne vaut que pour ce qu'aucun agent n'a ouvert.
- **À la fusion, la couche se ferme.** Ses générations intermédiaires tombent. **La racine qui reste
  est le commit d'atterrissage sur la branche suivie.** Les versions que l'ancre cite désormais
  restent ; celles des essais abandonnés deviennent collectables (une version citée par aucune
  génération).
- **Survivent** :
  - toute génération qu'une fiche de la mémoire cite (un apport y pointe) ;
  - un point de reprise complet par branche suivie, tous les N commits, avec des deltas entre (sa
    demande du 11 novembre).
- **« Trop lourd » se mesure** en versions citées par aucune racine et en octets par dépôt, jamais en
  nombre de générations. Aucune coupe sans mesure, et jamais automatique sur une base de Lucie sans
  son mot.

**Le budget, tranché par Lucie** : **30 Go par défaut, par dépôt**, réglable (relevé le 13 novembre
au soir depuis 15 Go). Son mot : c'est normal, un index vectoriel qui suit les worktrees pèse. Une
génération complète du dépôt pèse aujourd'hui 7 Go (2,9 Go de graphe, et 4,1 Go de plein texte, à
relire), pour 242 Go libres sur le portable et 1,3 Tio à luciepc. Au-delà, la collecte
**propose**, dans cet ordre :
1. les versions citées par aucune racine ;
2. puis les générations intermédiaires des branches fermées.

Elle ne propose jamais une racine ni une génération qu'une fiche cite, et elle le dit avant d'agir sur
une base de Lucie.

Les témoins de ce lot :
- une branche fusionnée ne laisse que la génération du commit de fusion et celles qu'une fiche cite ;
- une version orpheline est listée par la sonde avant d'être collectée ;
- une génération citée par une fiche survit à la fermeture.

**À la fusion d'une PR** (Lucie : « faut être intelligent au merge d'une PR : hop on supprime
worktrees et on supprime index intermédiaires »). Les règles que propose c7, pour ce même lot :
- **La couche se ferme automatiquement.** Quand l'hôte voit au fetch qu'une branche suivie est
  entièrement contenue dans sa parente (fusion ou squash, jugé par le contenu de l'arbre et pas
  seulement par l'ancêtre), il ferme la couche :
  - les générations intermédiaires sont supprimées ;
  - celle du commit d'atterrissage est gardée, ainsi que celles qu'une fiche cite ;
  - les versions orphelines deviennent collectables sous le budget.

  C'est réversible par un index explicite du commit, et c'est journalisé.
- **Le worktree n'est supprimé que sous condition.** L'hôte le supprime seulement si la branche est
  fusionnée **et** l'arbre est propre **et** aucun processus n'a de fichier ouvert dessous **et** le
  sous-module est sur son commit. Le target cargo lié, par la garde des targets, part avec lui.
  Sinon, rien n'est touché, et l'IDE montre « fusionné, à ranger » avec la raison (fichiers modifiés,
  base ouverte, processus). L'arbre principal de Lucie n'est jamais candidat. « Rien sur ses bases
  sans son mot » reste vrai pour tout ce qui n'est pas une couche fermée.
- **Les témoins** :
  - une branche fusionnée et propre : la couche est fermée, le worktree et le target sont partis,
    la génération d'atterrissage répond ;
  - une branche fusionnée avec un fichier modifié : la couche est fermée, le worktree est intact et
    signalé ;
  - une génération citée par une fiche survit.

Ce lot vient **après** celui de la liaison, et ne se code pas avec lui.

## Les questions qui étaient posées à Lucie

1. **La parente d'une branche venue d'ailleurs** : l'inférer par le point de fourche (notée comme
   inférée), ou exiger qu'elle soit déclarée ?
2. **Quand publier une génération** : à chaque commit indexé, ou seulement à la demande, au push,
   ou à la fin d'une tâche ? (ED : publier à chaque commit est sûr, puisque l'identité est
   déterministe ; une génération en double n'existe pas.)
3. **La couche de travail** : privée à la session qui tient le worktree, ou lisible des autres ?
4. **Un rebase** : la couche suit d'elle-même (génération neuve publiée par l'hôte), ou on attend un
   geste ?
5. **L'histoire** : combien de générations passées garder par couche (toutes, les N dernières, ou
   celles qu'une fiche cite) ? Et des points de reprise complets avec des deltas, comme tu l'as
   demandé le 11 novembre ?

## Les témoins, quand le code viendra (rouges d'abord)

- Une tâche ouverte depuis une branche note sa parente et sa base. Sa vue rend `f` de la parente, `g`
  d'elle, `h` de l'ancre.
- Un commit dans un worktree publie une génération. Une lecture commencée avant voit l'ancienne
  jusqu'au bout.
- Une édition non commitée se voit dans la vue du worktree, pas dans celle de la branche.
- La fusion fait descendre les versions sans réembarquement et ferme la couche.
- **Le témoin cruel** (recherche) : une génération se publie pendant une lecture tenue, dans les trois
  signaux à la fois, et aucune réponse ne mélange deux générations.
- `view: <branche>@<commit>` d'un commit jamais indexé matérialise ce seul état.
