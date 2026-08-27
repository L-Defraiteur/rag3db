# 06 — Le Tamagotchi et le compilateur : ce qu'il faut prendre de la conversation longue

Source : `Résumé de projet Tony Stark.md`, apporté par Lucie le 27 août.
Le [doc 05](05-la-reputation-des-abstractions.md) en analysait un extrait ; ce
document couvre le reste.

Deux mille lignes, beaucoup de plaisanteries, et **cinq idées qui valent la
peine**. Elles sont rangées ci-dessous par **distance à ce qui existe** — pas
par élégance — parce que c'est la seule mesure qui aide à décider.

Puis cinq naïvetés, nommées franchement : elles sont plus utiles nommées que
polies.

---

## 1. Les cinq à prendre

### 1.1 L'agent est une identité, pas une session — *la plus proche, et la meilleure*

L'inversion tient en une ligne :

> Le workspace devient le lieu où l'agent **travaille**, pas l'endroit où son
> existence est **stockée**.

Aujourd'hui, presque tous les outils rangent la mémoire sous le workspace. La
proposition la retourne : `Ada` a une identité, une mémoire, un historique, des
relations — et elle est *invoquée* dans un projet, un dépôt, `/tmp/truc-maudit`.

**C'est à trois fils de ce qui existe.** `Run`, `Message`, `SENT_BY`,
`SENT_TO`, `CHILD_OF`, `Conversation`, `Participant` : tout ça est livré. Il
manque **une identité au-dessus du run** — un run devient alors une incarnation
temporaire — et la relation qui les lie.

Et ça referme une asymétrie qu'on n'avait pas vue : `Participant` existe
*dans* une conversation, mais rien ne dit qu'un participant est **le même**
d'une conversation à l'autre. Le fil est cherchable, celui qui parle ne l'est
pas.

> **Ce que ça débloque, et qui n'est pas anecdotique** : la question « qu'est-ce
> qu'on avait décidé sur X, et pourquoi » cesse d'être une recherche dans du
> texte pour devenir une recherche **adressée** — qui a travaillé là-dessus,
> puis on lui demande.

### 1.2 Trois espaces de mémoire, et la provenance

```
mémoire d'agent      « ce que MOI j'ai appris »
savoir de workspace  « ce qui est vrai ici »
savoir partagé       « ce que notre groupe sait »
```

Le défaut que ça corrige est réel et bien nommé : les mémoires agentiques
naïves confondent *« dans le projet X, cette fonction faisait ça »* avec *« les
fonctions de ce nom font ça »*. La première est locale et périssable, la
seconde est générale. Les mélanger produit des affirmations fausses avec
l'assurance des vraies.

**On a déjà quatre des cinq axes** — org, cellule, origine, domaine, lentille
([doc 02](02-architecture.md)). Il manque exactement celui-là : **l'agent**. Et
il passe le test qu'on s'est donné : *deux choses qui changent à des rythmes
différents ne sont pas la même chose*. La mémoire d'Ada et l'état du dépôt ne
changent pas au même rythme, donc ce ne sont pas la même chose.

Une mémoire porte alors sa provenance — appris par, pendant quel run, depuis
quelle source, à propos de quoi, quand — et c'est de la structure ordinaire,
pas un mécanisme.

**Et le point le plus important de toute la section est déjà réglé** : la
mémoire n'a pas à être du texte injecté dans l'invite. Ada n'emporte pas
quatre cent mille jetons de souvenirs à son réveil ; elle **cherche dedans**,
comme elle cherche du code. C'est précisément ce qu'on a livré cet
après-midi — `absorb` réduit, `recall` va rechercher
([doc 04](04-la-session-tient-l-invite.md)). La plomberie du Tamagotchi existe
avant le Tamagotchi.

### 1.3 Le commit comme souvenir épisodique vérifiable

La meilleure des idées « rigolotes », et pour une raison sèche : **un commit
porte déjà de la structure gratuitement** — état avant, état après, diff,
auteur, date, message. Stocker « j'ai modifié le résolveur de `Symbol` » est une
phrase ; stocker **le commit qui l'a fait** est une preuve.

```
(Agent:Ada) ─CREATED→ (Commit:8ebf4ab35)
                          ├─FROM_RUN→   (Run)
                          ├─TOUCHES→    (File)
                          └─JUSTIFIED_BY→ (Decision)
```

Six mois plus tard, « pourquoi `split_unchanged` est ici ? » retrouve le
commit, le run qui l'a produit, **les deux tentatives ratées avant**, et la
raison exacte de la garde. On a vécu cet épisode il y a deux jours ; on ne
saurait pas le rejouer aujourd'hui.

On ingère déjà les fichiers avec leurs coordonnées git (`GitCoordinates`,
`head_revision`) : l'entité `Commit` est le petit morceau qui manque.

Et la distinction que la conversation pose est **juste et importante** :

> Un snapshot restaure **le monde**, pas **l'individu**.

`git checkout HEAD~20` ne doit pas lobotomiser Ada de trois semaines. C'est
encore notre test des rythmes, appliqué correctement.

### 1.4 Le schéma comme programme — *la plus sous-estimée du document*

Ce n'est pas la drôle, et c'est peut-être la plus importante.

Au lieu de demander à un modèle huit mille lignes de backend correctes, on lui
demande une **représentation intermédiaire très contrainte** — entités, champs,
signaux, relations, politiques, transitions, événements — puis **le compilateur
fait le travail chiant** : le stockage, la recherche dérivée des signaux
déclarés, les opérations en graphes, les événements sur le bus, la surface
HTTP/RPC/MCP par-dessus.

> **Le LLM choisit dans un langage que le moteur sait vérifier.**

C'est exactement la forme qu'on a déjà validée : `register_entity` déclare
identité, champs, signaux, chunking ; `drain()` compile les mutations en
graphe ; `choices`, les paramètres typés et les erreurs *avant* instanciation
bornent ce que le modèle peut inventer ; `NodeTypePolicy` retire ce qu'il ne
doit pas voir. Un graphe-outil **est déjà** une RI compilée.

L'idée est donc : pousser davantage de sémantique dans la déclaration, et
laisser le compilateur descendre. C'est la continuation directe de
« la complexité monte dans les données plutôt que dans le moteur ».

### 1.5 Le registre de concepts

Chercher un concept **avant de savoir où il vit** — sans connaître le dépôt, le
workspace, ni le nom exact du fichier — et récupérer les graphes récents qui
s'y rattachent, avec leurs versions, leurs runs, leurs projets.

Les quatre morceaux existent et se complètent proprement :

| Rôle | Qui le tient |
|---|---|
| retrouver malgré un nom approximatif, une variante, un bout | lucivy (fuzzy, substring, regex) |
| la proximité de sens | dense + sparse |
| ce qui est **réellement** relié | le graphe |
| ce qui est actif, et depuis quand | les dates livrées ce matin |

Manque `Concept` comme entité. Et son identité est **exactement** la question
du [doc 05 §3.4](05-la-reputation-des-abstractions.md) : proposer, ne pas
fusionner — sinon `auth`, `authentication`, `Auth`, `user-auth`, `login`, et
« le royaume des graphes devient une république italienne du XVe siècle ».

### 1.6 (bonus) Tout est un agent — mais surtout : les builtins sont le produit

« Agent » cesse d'être une catégorie pour devenir une **interface** : identité,
mémoire, capacités, protocole de conversation. Un projet, un dépôt, une
organisation peuvent l'implémenter, et la plupart ne sont **que des vues
actives sur un graphe**, avec un modèle invoqué seulement quand on leur parle.

C'est presque gratuit **si 1.1 est fait**, et sinon ça ne veut rien dire.

Le vrai point produit est ailleurs, et il est bien vu :

> Le moteur reste universel, mais **les builtins donnent une opinion forte**.

Sans quoi le super-pouvoir devient le risque : puisque tout est composable,
quelqu'un fabriquera une cathédrale à quatorze étages pour faire un `grep`.
Et l'élégance, c'est que les gabarits ne sont pas du code spécial —
`builtin/project-agent@v3` est un graphe versionné et cherchable, qu'on peut
*forker*.

---

## 2. Les cinq naïvetés, nommées

### 2.1 « Un backend en quelques minutes »

La naïveté est dans **les minutes**, pas dans la destination — et il faut le
dire dans ce sens-là, parce que Lucie a repris ce paragraphe : *« qu'un backend
soit facile à poser quelque part et à écrire par les agents m'importe
beaucoup »*. C'est une ambition à part entière, elle est au
[doc 01 §2](01-progression-et-ambitions.md), et elle est atteignable par
étapes — c'est l'objet du [doc 07](07-le-langage-de-declaration.md).

**Le gain n'est pas la vitesse.** Un backend généré vite est un passif si
personne ne peut le relire. Ce qui vaut, c'est qu'il soit **inspectable et
versionné**.

> **Suite au [doc 07](07-le-langage-de-declaration.md)** : où passe vraiment la
> frontière (les invariants exprimables, pas les applications simples), pourquoi
> le mur est la v2 et pas la v1, et surtout **ce que le langage encourage** —
> la contrainte que Lucie a posée le 27 après-midi et qui décide de tout.

### 2.2 La spécialisation qui « émerge » de l'historique

Séduisant, à moitié vrai. Ce qui émerge de trois cents runs sur Rust, c'est une
**recherche qui trouve des choses Rust** — pas une compétence. Ada n'est pas
meilleure en Rust ; sa mémoire en contient davantage.

C'est utile quand même, mais il faut l'appeler par son nom : router vers l'agent
dont la mémoire score le mieux est **une décision de recherche déguisée en
décision sociale**. Ce qui est une bonne nouvelle — c'est moins cher et bien
plus débogable qu'une « architecture multi-agent » — à condition de ne pas
promettre autre chose.

### 2.3 La promotion automatique des concepts par fréquence

« Si plusieurs graphes, commits et traces parlent souvent du même terme, le
système crée ou renforce un `Concept`. » C'est de la **fréquence**, donc de la
popularité, et le [doc 05](05-la-reputation-des-abstractions.md) vient
justement de passer une section à la séparer de la confiance.

Un terme qui apparaît partout est souvent un **mot vide du domaine** —
`handler`, `service`, `data`, `manager`. La promotion d'un concept demande la
même discipline de preuve que la promotion d'un outil.

### 2.4 Le snapshot du graphe attaché à chaque commit

L'idée est belle — `git checkout` restaurerait aussi l'état de connaissance — et
**son coût n'est pas mentionné une seule fois**. Un instantané cohérent et bon
marché d'une base graphe par commit est un vrai problème de stockage et de
cohérence.

Or on a une dette ouverte exactement là : **`Consistency` est déclaré et jamais
honoré, et `flush_insertions` écrit sans indexer** — c'est la plus vieille dette
de la maison ([doc 01 §3](01-progression-et-ambitions.md)). Bâtir des
instantanés sur une incohérence connue, c'est bâtir sur du sable. L'ordre est
donc contraint, et c'est utile de le savoir : **honorer `Consistency` d'abord**.

### 2.5 Sept sortes d'agents, juste après avoir plaisanté sur `AgentCoordinatorFactoryEnterprise`

La conversation se moque de la sur-abstraction, puis propose agent perso,
projet, dépôt, organisation, dataset, service, workflow. La plaisanterie et la
proposition sont en tension.

Le remède est celui qu'on utilise déjà : *deux choses qui changent à des rythmes
différents ne sont pas la même chose*. Appliqué ici, la plupart de ces sept
s'effondrent en **une identité plus un domaine**, ce qui est précisément 1.1 et
1.2 — et rien de plus.

---

## 3. Trois choses que le document ne dit pas

1. **Le Tamagotchi est bloqué sur un point déjà inscrit.** Un agent qui voyage
   de projet en projet a besoin qu'`Origin` soit une **entité du graphe avec un
   `local_path` par poste** — c'est l'ambition n°2 du [doc 01 §3](01-progression-et-ambitions.md).
   Sans elle, « la même origine vue depuis deux machines » n'existe pas, et
   l'agent itinérant non plus. Cette conversation **monte la priorité** de cet
   item : il servait au cloud, il sert maintenant à deux choses.

2. **Le signal négatif existe ici aussi.** Un agent qui consulte un pair et
   **n'utilise pas** la réponse dit quelque chose que personne n'écrira — même
   famille que le [doc 05 §2.2](05-la-reputation-des-abstractions.md). C'est la
   mesure honnête de « les agents se parlent-ils utilement ».

3. **`recall` se généralise, et ce n'est pas un hasard.** Il résout aujourd'hui
   un renvoi vers un résultat d'outil gardé en session. La même forme résout un
   renvoi vers un souvenir, un commit, un concept. C'est le principe
   « il cherche ses souvenirs comme il cherche ses capacités », et la plomberie
   a été posée cet après-midi sans qu'on vise ça.

---

## 4. Ce que j'en retiens pour l'ordre des choses

Rien de ce document ne demande de casser quoi que ce soit, et c'est le signe
qu'il tape juste. Dans l'ordre du moins cher au plus cher :

| # | Ce que c'est | Bloqué par |
|---|---|---|
| 1 | Identité d'agent au-dessus du run (`AgentIdentity`, `HAS_RUN`) | rien |
| 2 | L'axe **agent** de la mémoire, avec provenance | 1 |
| 3 | `Commit` comme entité, relié au run qui l'a produit | rien |
| 4 | `Concept` + promotion **sur preuve**, pas sur fréquence | doc 05 |
| 5 | Agent itinérant entre projets | `Origin` comme entité |
| 6 | Instantanés attachés aux commits | **`Consistency`** d'abord |
| 7 | Schéma-programme étendu (politiques, transitions) | rien, mais gros |

Et une remarque de méthode, parce qu'elle vaut plus que la liste : cette
conversation est **une source d'idées, pas une source de vérité**. Les cinq
naïvetés du §2 y sont écrites avec exactement la même assurance que les cinq
bonnes idées du §1 — ce qui est le rappel de la phrase de lucivy qu'on garde
en tête depuis trois jours :

> Une explication convaincante d'un artefact est plus dangereuse que
> l'artefact.
