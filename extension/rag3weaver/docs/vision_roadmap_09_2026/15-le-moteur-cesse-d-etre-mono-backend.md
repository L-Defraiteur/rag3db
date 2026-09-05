# 15 — Le moteur cesse d'être mono-backend

**3 septembre 2026.** Ce document n'était pas dans la feuille de route. Il n'y
était pas parce que personne n'avait dit à voix haute ce que Lucie a fini par
dire :

> Une boucle étrange qui ne sert que kuzu ne sert pas les projets réels.

Le [13](13-avaler-une-base-existante.md) l'avait déjà écrit pour l'ingestion
d'une base étrangère. Le même argument vaut un cran plus tôt : **une boucle qui
ne tourne que sur notre propre base ne sert qu'à nos propres projets.**

## 1. Ce qu'un backend doit fournir

Le catalogue n'est plus une couche au-dessus d'**une** base. Il est une couche
au-dessus de **plusieurs**, et ce qu'il réclame tient en quatre organes :

| organe | ce qu'il fait |
|---|---|
| `DbConnection` | exécuter, avec ou sans paramètres — **synchrone** |
| `SchemaDialect` | l'**intention** rendue dans la langue du backend : DDL, DML, filtres, jointures |
| `SearchBackend` | vecteur, résolution de décalages, enrichissement — et **facultativement** le plein texte |
| `BlobStore` | où vivent les index lucivy et sparse |

Un cinquième est facultatif et ne l'est plus vraiment : le **magasin de
checkpoints**. Sans lui, une ingestion morte en route ne reprend pas — c'est la
différence entre un backend qui marche et un backend qui sert.

Les deux existent aujourd'hui pour rag3db et pour PostgreSQL/pgvector, sur les
cinq organes.

## 2. Ce que « prouver » a coûté, et ce que ça dit

`PostgresDialect` faisait 944 lignes, compilait, et **n'avait jamais parlé à une
base**. Écrire huit tests de bout en bout a sorti neuf défauts, tous de la même
espèce : du code qui compilait et n'avait jamais tourné.

| défaut | conséquence |
|---|---|
| l'hôte formaté avec le `Debug` d'une énumération | aucune connexion n'était possible ; le dialecte n'avait donc jamais pu être faux |
| treize requêtes en lot dans une forme qu'`unnest` n'a pas | **tout le chemin d'écriture était mort-né** |
| cinq organes parlant Cypher en dur | dont neuf appels se rabattant en silence sur le dialecte de rag3db |
| aucun index hors clés primaires | le chemin le plus chaud balayait la table entière |
| le plein texte natif sans cloisonnement | fuite entre locataires, deux fois — sur les deux chemins de recherche |
| l'identifiant d'insertion lu comme une chaîne | il est entier sur SQL : la table uuid→identifiant restait **vide**, et lucivy n'indexait rien |

**Le fait qui compte n'est pas la liste, c'est ce qu'elle a en commun.** Ce qui
rendait PostgreSQL difficile n'était pas SQL contre Cypher : c'était chaque
endroit où une représentation propre à **kuzu** avait fui hors du dialecte —
`ID(n)` rendu en chaîne, `OFFSET(id(n))`, la résolution des chunks, les deux
magasins.

D'où une conséquence sur l'ordre : **Neo4j est le dernier backend à faire, pas
le deuxième.** Il parle Cypher, donc il masquerait ces fuites au lieu de les
révéler. On aurait un troisième backend qui marche vite, et l'abstraction
resterait cassée pour le prochain backend SQL. Il restera bon marché — c'est
précisément pour ça qu'il peut attendre.

## 3. Deux invariants que cette semaine a ajoutés

Le [01](01-la-vision.md) §4 énumère ce qui rend le chaos *contrôlé*. Deux
entrées y manquaient, et elles ne sont pas venues d'une décision mais d'une
répétition : on les a trouvées violées dix fois en une journée.

### « C'est le silence qui est le défaut, pas l'erreur »

Un repli, un saut, une garantie qui se dégrade, une option jamais empruntée :
tout cela **compile**, ne lève rien, et ment. Quelques-uns de la même journée —

- une recherche qui se repliait de PostgreSQL sur lucivy parce qu'un service
  manquait, et rendait des résultats plausibles ;
- un cloisonnement entre locataires qui disparaissait sans un mot ;
- un filtre utilisateur qui ne descendait pas, donc rendait des lignes que
  l'appelant croyait exclues ;
- `Consistency::Strict` qui rendait `Immediate` dès qu'on franchissait la
  frontière du processus ;
- deux compteurs de chunks présentés comme des mesures et écrits en dur à zéro ;
- une falaise de rappel qui se présentait comme « ça n'existe pas ».

Le remède n'est pas la vigilance, c'est la forme : **une absence se nomme**. Un
service manquant produit une erreur nommée en aval ; un backend déclare ce
qu'il ne garantit pas ; un repli porte sa raison ; une méthode qui ne peut pas
tenir sa promesse le dit à l'appelant plutôt qu'à personne.

Et le corollaire qui a servi quatre fois : **une pièce écrite mais jamais
appelée se dégrade sans bruit.** Un magasin de blobs inatteignable, un
`set_moteur_texte` sans appelant, un magasin de checkpoints qu'on ne montait
pas — chacun tenait sur le papier et aucun ne tenait dans les faits.

### Une recherche doit pouvoir dire qu'elle ne trouve rien de probant

Notre recherche rendait **toujours** quelque chose, même sur une requête
absurde, et rien ne distinguait une réponse d'une coïncidence lexicale. C'est le
manque le plus visible pour qui s'en sert, et il a fait accuser le moteur
pendant six mois sur un défaut qui n'était pas le sien.

Le seuil se **mesure**, il ne se recopie pas : celui de ragkit vaut 0,7 sur des
noms de médicaments, des noms propres courts où deux chaînes proches désignent
la même molécule. Sur des descriptions, la frontière est ailleurs.

Deux règles en sont sorties, et elles valent au-delà de la recherche :

- **marquer, pas filtrer** — le recouvrement est réel, et décider à la place de
  l'appelant lui retire un résultat qu'il aurait su reconnaître ;
- **une phrase nomme ce qu'elle a mesuré** — la marque porte sur le plein texte
  et le dit, sinon elle parle au nom du vecteur, qui est précisément fait pour
  rapprocher ce qui ne partage aucun mot.

## 4. La concurrence, et ce qui reste

Trois configurations, et l'ordre dans lequel elles sont tombées :

| | avant | aujourd'hui |
|---|---|---|
| plusieurs fils écrivains, un processus | non | oui, par le report d'un fork (non adopté) |
| plusieurs processus **lecteurs** | non | **oui, mesuré** — 80 ouvertures pendant qu'on écrit, zéro refus |
| plusieurs processus **écrivains** | non | **non** |

Ce que la lecture concurrente a demandé, et qui n'était pas prévu :

- une **marque d'eau d'ingestion**, parce que `Consistency` vivait en mémoire.
  Un lecteur ne peut pas vider la file d'un autre : il peut seulement attendre
  qu'elle soit vide, **et encore faut-il que l'écrivain le publie**. C'est la
  limite honnête de ce qui est faisable, et elle est écrite dans le code.
- une **reprise sur refus transitoire**, parce qu'un refus de quelques
  millisecondes se présentait comme « la base est inaccessible ».
- un **choix de chemin explicite** pour le lecteur — direct ou par le relais —
  avec la seule règle qui empêche cette souplesse de devenir un piège : jamais
  de repli silencieux, dans aucun des deux sens.

La troisième configuration reste ouverte, et aucun des deux forks examinés ne la
résout. Elle demandera un gestionnaire de verrous **inter-processus**, qu'on ne
peut pas poser sur une couche de stockage supposant un écrivain unique.

## 5. Ce que ça change pour la vision

Rien de l'axe : tout est toujours un graphe, et un graphe est toujours une
donnée. Mais **la matière sur laquelle la boucle se referme n'est plus la
nôtre**. Un agent qui construit un backend peut désormais le construire sur la
base que son utilisateur possède déjà — et c'est la condition que le
[13](13-avaler-une-base-existante.md) attendait pour cesser d'être une idée.
