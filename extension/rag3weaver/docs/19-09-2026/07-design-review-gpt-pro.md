**Mon reproche principal : tu as un framework qui compose des recherches en DAG, mais le contrat d’optimisation reste encore assez centré sur « une entité, des champs, des index ».** Pour le design idéal, je déplacerais le centre de gravité : **une recherche définie sémantiquement, plusieurs réalisations physiques possibles.**

Je garderais ta séparation données / usages / physique. En revanche, je renforcerais les objets qui relient ces trois couches. Voilà mes vraies remontées d’architecture.

## 1. `access_patterns` ne devrait pas devenir un deuxième langage de requêtes

Ton exemple décrit des égalités, un intervalle, un ordre et une pagination. Tu prévois aussi que les patterns proviennent de l’observation des DAG. Il y a donc deux façons de décrire le besoin : la déclaration dédiée et le programme de recherche lui-même.

**Je ferais converger les deux vers la même représentation logique.**

Le risque conceptuel, ce n’est pas qu’il manque quelques propriétés dans le JSON. C’est que, progressivement, `access_patterns` réinvente un langage parallèle : après `equals` et `range`, il faudra décrire les relations, les intersections, les contraintes sur un même élément imbriqué, les recherches classées, leurs combinaisons…

Alors que ton DAG porte déjà une composition de recherches.

Dans mon design cible :

**Le DAG décrit la recherche. Le pattern identifie une recherche ou un fragment logique, décrit ses usages et lui attache des objectifs.**

La syntaxe simple `equals/range/order_by` peut parfaitement rester. Mais comme raccourci compilé vers cette représentation commune, pas comme une seconde vérité indépendante.

Point important : **je parle d’un DAG logique, pas de la topologie physique qui a effectivement été exécutée.** « Ces événements appartenant à ce deck, sur cette période, dans cet ordre » doit rester le même besoin, qu’on le réalise par parcours relationnel, scan filtré ou accès composite.

Je distinguerais aussi l’identité sémantique du besoin et les versions utilisées pour l’exécuter. Ton empreinte inclut déjà les versions du schéma et du DAG ; je garderais ces versions pour l’invalidation et la traçabilité, sans nécessairement traiter toute modification physique comme un nouvel usage sans historique.

**La propriété que je chercherais : changer la stratégie d’exécution ne change pas l’identité de la question posée.**

## 2. Les garanties de résultat devraient être des propriétés du modèle, pas seulement des règles à respecter

Ta section sur l’exactitude est déjà riche : portée, tableaux imbriqués, ordre, déduplication, scores, preuves, approximations. Je ne te remonte donc pas « attention au classement », puisque tu l’as écrit.

Ma remontée serait plutôt : **ces distinctions méritent d’être représentées explicitement dans les entrées et sorties des opérateurs.**

Par exemple, ces deux expressions ne décrivent pas le même résultat :

```text
TopK(20, Rank(Filter(tenant = T, documents)))

Filter(tenant = T, TopK(20, Rank(documents)))
```

La première demande les vingt meilleurs documents du tenant. La seconde demande, parmi les vingt meilleurs documents globaux, ceux appartenant au tenant. Elle peut n’en rendre que deux alors que le tenant possède des milliers de documents.

Ce n’est pas une subtilité d’indexation : **ce sont deux questions différentes.**

Je voudrais donc que le contrat d’un résultat dise notamment quel ensemble il représente, s’il est exhaustif ou tronqué, sur quel domaine son classement est défini, à quel état des données il correspond, et quelles preuves il fournit.

Dans ton cas, les preuves peuvent être une vraie partie du résultat : **renvoyer les mêmes documents sans les mêmes spans demandés n’est pas nécessairement une réalisation équivalente.**

Ça permettrait de distinguer proprement :

* une transformation équivalente, applicable lorsque ses préconditions sont satisfaites ;
* une réalisation approximative, admissible seulement dans le contrat demandé ;
* une transformation qui change la recherche et n’appartient donc pas à l’optimisation physique.

**L’optimiseur ne devrait pas seulement savoir ce qu’un nœud fait ; il devrait savoir quelles propriétés de sa sortie peuvent être utilisées par le nœud suivant.**

Et tes nœuds personnalisés restent libres : un nœud dont la sémantique est opaque constitue une frontière pour certaines transformations, pas une raison d’interdire l’optimisation de tout le reste. Pas besoin de créer un ministère de la Pureté du DAG.

## 3. Une capacité backend devrait décrire une réalisation composable

Ton adaptateur annonce déjà bien davantage que « supporte les index » : opérations, ordre, listes, nulls, collation, cohérence, maintenance. C’est la bonne matière première.

**Je donnerais à ces capacités une forme directement exploitable par le planificateur :**

> Je peux réaliser ce fragment logique, sous ces préconditions, avec ces propriétés de sortie, en utilisant ces ressources.

Par exemple, une réalisation pourrait annoncer qu’elle applique les égalités sur `source_id` et `deck_id`, restreint `created_at`, produit déjà l’ordre `(created_at, _uuid)`, et fournit soit les objets demandés, soit seulement leurs identifiants.

Ce dernier point compte : un accès qui trouve les bons identifiants mais impose ensuite de récupérer tous les objets n’a pas les mêmes propriétés qu’un accès qui fournit directement la projection demandée.

Le planificateur peut alors composer les réalisations : ajouter une récupération, conserver un ordre, éviter un tri, appliquer une restriction résiduelle. **Il raisonne sur ce qu’elles fournissent, pas sur leur nom de famille algorithmique.**

Ça préserverait ton interchangeabilité : chaque backend choisi expose ses propres réalisations, sans être forcé de prétendre posséder les mêmes structures.

Je modifierais aussi la présentation de `unsupported`. Dans ton exemple, le pattern a ce statut alors qu’un `available_plan` existe.

Je séparerais les axes :

**La recherche est-elle réalisable correctement ? Une accélération particulière est-elle disponible ? L’objectif de service est-il atteint ou simplement inconnu ?**

Un scan correct, un index composite absent et un p95 non évalué peuvent être vrais simultanément. Un unique statut ne devrait pas les écraser.

## 4. Le contrôleur devrait choisir une configuration, pas seulement approuver des candidats

Ta boucle est présentée autour de candidats : estimation, construction, validation, activation. Le score expose déjà lecture, écriture, construction, espace et amortissement.

**J’ajouterais explicitement un objet au-dessus : la configuration physique cible.**

Parce que l’intérêt d’un accès dépend des autres accès disponibles.

Deux structures peuvent se compléter : leur intersection rend possible un plan intéressant alors qu’aucune n’est suffisamment utile seule. Deux autres peuvent se remplacer : chacune paraît rentable face à la configuration initiale, mais construire les deux paie deux fois pour satisfaire les mêmes usages.

La bonne question devient donc :

> Avec cette charge et ces objectifs, quelle configuration de ressources permet les meilleurs plans admissibles ?

Pas simplement :

> Cet index améliore-t-il cette requête ?

Dans ce modèle, un index est une ressource parmi d’autres. Une matérialisation, un résultat réutilisable ou une organisation physique pourraient entrer dans le même raisonnement, avec leurs propres contrats de validité. Ton document les distingue déjà et en exclut certains du premier contrôleur ; **je garderais cette distinction entre actions, mais pas une abstraction centrale qui ne saurait représenter que des index.**

Et je rendrais la frontière entre les deux décisions très nette :

**Le planificateur d’exécution choisit comment réaliser une recherche avec les ressources actuellement disponibles. Le contrôleur adaptatif choisit quelles ressources devraient exister pour l’ensemble des usages.**

Le moteur conserve son optimisation interne pour les fragments qu’on lui délègue. Ça prolonge ta frontière actuelle, sans transformer rag3weaver en deuxième optimiseur Cypher.

Autrement dit : **le contrôleur change les possibilités offertes au planificateur ; il ne réécrit pas la signification du programme métier.**

## 5. Les objectifs devraient appartenir au parcours de recherche, pas seulement à ses accès

Dans l’exemple, le `target_p95_ms` est attaché au pattern d’accès. Plus loin, tu mesures déjà le chemin critique du DAG et tu distingues les coûts d’accès, d’embedding, de reranking et d’attente.

**Je remonterais aussi l’objectif au niveau où le résultat est effectivement consommé.**

Sinon, le contrôleur peut optimiser parfaitement une branche qui ne limite pas la réponse finale. Faire passer une branche parallèle de 80 à 20 ms ne réduit pas nécessairement la latence si une autre branche impose toujours 500 ms. Cela peut économiser des ressources, mais ce n’est pas le même gain.

Je distinguerais donc trois choses : les garanties sémantiques non négociables, les objectifs de service du parcours complet, et les priorités d’arbitrage entre usages.

Les objectifs locaux peuvent exister, mais comme objectifs explicitement choisis ou budgets dérivés, pas comme substituts automatiques à l’objectif global.

Même principe pour les tenants : **la fréquence observée décrit la charge ; elle ne décide pas, à elle seule, de l’importance du besoin.** Une recherche rare mais critique doit pouvoir conserver sa priorité, même face à un tenant qui produit beaucoup de requêtes banales. Ta question sur la monopolisation du budget trouverait sa place dans ce modèle d’objectifs, plutôt que dans une correction ajoutée au score après coup.

## 6. L’optimisation devrait utiliser les abstractions de ton framework, pas former un système parallèle

Tu prévois déjà un journal persistant, des générations, des états et des décisions déterministes.

**J’en ferais un véritable artefact de décision versionné**, avec la configuration de départ, les usages et objectifs concernés, les références aux mesures utilisées, les alternatives évaluées, la configuration désirée et les raisons de son choix.

Ensuite, l’application de cette décision serait un **DAG de transition** : obtenir les ressources nécessaires, construire, valider, publier, retirer ce qui doit l’être.

Pas un second petit moteur de workflows caché dans le contrôleur, avec ses propres conventions et une machine à états qui réinvente la moitié de ton runtime.

Le modèle serait déclaratif : **voici l’état désiré, voici l’état observé, voici la transition nécessaire**. Le journal raconte ce qui s’est passé ; la configuration désirée dit ce qui doit être vrai. Ce sont deux responsabilités différentes.

Ça donnerait aussi une unité beaucoup plus propre pour expliquer une décision : pas seulement « pourquoi cet index existe », mais **« pourquoi cette configuration a été choisie pour ces usages, face à ces alternatives »**.

---

**Donc, ma remise en cause de fond n’est pas “réduis l’ambition”. C’est plutôt : ton contrat peut monter d’un étage pour rejoindre l’abstraction de ton DAG.**

Je viserais cette articulation :

**Programme logique commun → contrats de résultat composables → réalisations offertes par le backend → plans d’exécution → choix adaptatif d’une configuration de ressources.**

Les trois modifications les plus structurantes seraient de ne pas maintenir un langage de patterns indépendant du DAG, de donner une représentation explicite aux garanties de résultat, et de faire porter la décision adaptative sur une configuration entière.

**La boucle étrange devient alors très précise : le système peut changer la manière dont il réalise un calcul, sans changer le calcul demandé.** L’index est un organe. Il n’a pas à rédiger la constitution. 😈
