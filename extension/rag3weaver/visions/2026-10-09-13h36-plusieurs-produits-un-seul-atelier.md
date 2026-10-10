# Plusieurs produits, un seul atelier

9 octobre 2026. Vision de Lucie, notée par l'orchestration. Rien n'est codé.

Lucie : « Pour le côté Westworld, faire plusieurs produits comme ça : Magic
the Gathering, le code, le DXF, et pourquoi pas étendre après à Blender, et
d'autres trucs sympas — rendre vraiment possible le futur comme dans les
films. »

## 1. L'idée

Le « côté Westworld » de la vision générale (§0 : *tu parles à ton code ou à
ton site, et il change sous tes yeux ; l'écran montre ce qui se passe, les
graphes que l'agent écrit et exécute, et tout se clique*) n'est pas propre au
code. C'est un **atelier** : un agent, des graphes qu'on voit se former et
tourner, des fiches, une page. Ce qui change d'un produit à l'autre, c'est le
**domaine** derrière les outils — et un domaine, chez nous, est une
déclaration.

Donc : plusieurs produits qui sont le même atelier avec un domaine différent
chacun, et chaque domaine ajouté prouve un peu plus que l'atelier est
général.

## 2. Les domaines

| Domaine | Ce qu'on parle | Les outils (nœuds) derrière | Où ça en est |
|---|---|---|---|
| **Magic : les decks** | « monte-moi un deck mono-rouge agressif sous 60 € » | les cartes et leurs textes dans la base, la recherche par sens et par mots, les règles de construction | le seul produit que Lucie a utilisé pour de vrai (`experiments/mtga`, septembre) ; la base sera refaite |
| **Le code** | « où ça casse si je change cette fonction ? » | le graphe de code (`codeparsers`), `usages`, `impact`, lecture, édition, commandes | livré, en usage quotidien sur ce dépôt |
| **Le dessin (DXF / CAO)** | « ajoute une cote sur ce mur, passe les portes sur un calque à part » | un service qui lit et écrit le DXF (`ezdxf` ou un crate Rust), ou qui pilote DraftSight sur le poste ; un rendu du dessin dans la page | envisagé (9 octobre) : DXF pour vendre, DraftSight pour le père de Lucie |
| **Blender** | « fais tourner la caméra autour de l'objet et rends dix vues » | l'API Python de Blender (`bpy`) servie comme un service local, ses opérations déclarées comme nœuds ; la scène comme graphe | idée, « pourquoi pas après » |
| **D'autres** | | tout logiciel qui a une API scriptable ou un format ouvert devient un domaine : une déclaration et un service | |

## 3. Ce qui est commun, et ce qui ne l'est pas

Commun — et c'est tout ce que l'atelier doit porter une fois :

- l'agent et sa boucle, les outils tirés des schémas de nœuds, les rapports
  d'exécution nœud par nœud, le réacteur, les sessions et les mémoires ;
- la page qui montre le graphe en train de se former et de tourner, la fiche,
  l'exécution cliquable (marches 2 à 4 de la vision générale) ;
- la base : le domaine y vit comme des entités déclarées (`EntityConfig`),
  indexées par mots, par sens et par graphe.

Propre à chaque domaine :

- **un service** qui sait faire les gestes du domaine (lire un DXF, lancer
  une opération Blender, lire la collection de cartes) ; il tourne à côté,
  dans le langage qui convient au domaine, et parle au backend par HTTP —
  comme les embarquements aujourd'hui ;
- **la déclaration** : les entités du domaine, les nœuds-outils (leur schéma
  suffit pour en faire des outils de l'agent), les graphes de traitement,
  les vues ;
- **un rendu** dans la page : un dessin, une scène, un deck, un diff.

Ce que ça exige de l'atelier, et qui n'est pas encore là : la marche 2 (un
backend déclaré, des services en sous-graphes, le rechargement à chaud) et la
marche 3 (des vues déclarées, des graphes autour de composants). Chaque
domaine ajouté est une épreuve de ces deux marches : s'il demande du code
dans l'atelier plutôt qu'une déclaration, c'est l'atelier qui a un trou.

## 4. Ce que « comme dans les films » veut dire ici

Pas l'effet. Ce qui fait l'ordinateur de Star Trek ou les tablettes de
Westworld, c'est que **la personne parle, la chose change sous ses yeux, et
elle voit par où c'est passé**. Trois conditions, toutes mesurables :

1. le geste est fait pour de vrai (le deck existe, le mur a sa cote, la scène
   est rendue) — un service réel, pas une maquette ;
2. ce que l'agent a fait se voit sans qu'il le raconte — les rapports
   d'exécution, principe 4 des visions ;
3. l'erreur est visible et réparable — un refus nommé, un pas qu'on rejoue,
   jamais un résultat faux en silence.

Un produit de cette liste qui tient les trois est une vitrine (marche 5). Un
produit qui n'en tient qu'une « fait beau » — règle de la vision générale :
rien qui ne sert qu'à faire beau.

## 5. L'ordre, à décider par Lucie

Proposition de l'orchestration, pas une décision : le code d'abord (il
existe), les decks ensuite (la base à refaire sur la marche 2, ce qui en fait
la première épreuve du backend déclaré), le DXF quand le paquet npm et le
bâti Windows existent (chantier G), Blender quand un service `bpy` vaut la
peine d'être écrit. Un domaine à la fois ; chacun doit tenir les trois
conditions du §4 avant le suivant.

## 6. Le magicien du chaos (Lucie, 10 octobre au soir)

Entre « rien » et « un produit », il y a un agent dont le domaine est le
système lui-même. Une fois les services et le modèle déclarés (par un
formulaire ou la CLI : rien d'autre à ce stade, pas de code à indexer), le
**magicien du chaos** sait ce qui est disponible et te monte un produit à la
demande — le code, les decks, le dessin, un autre — comme un outil de
*boilerplate*, sauf que le boilerplate est une déclaration (un backend, son
gabarit, ses graphes) écrite dans le même système : la boucle étrange en
boilerplate. Puis il te donne le lien de l'**agent dédié** de ce produit, qui
a ses propres outils et sa propre page. Le magicien n'est pas une pièce à
part : c'est un backend comme les autres, dont les outils sont « créer un
backend depuis un gabarit » — il vient avec la marche 2.

Deux ajouts de Lucie, le même soir :

- **Le premier cas, très simple** : « donne-moi des outils MCP sur ce projet
  de code pour chercher dedans via rag3weaver » — et, dans la foulée, « l'agent
  demande aussi si tu veux une mémoire long terme accessible à Claude ». Les
  deux sont le même mécanisme : un backend déclaré (code, mémoire) exposé en
  serveur MCP, ses outils tirés de ses schémas. C'est le chantier H du plan de
  reprise ; le magicien le fera d'une phrase.
- **Les noms sont des thèmes.** « Le magicien du chaos », « le classeur de
  Dawson du chaos » : c'est fun, et on pourra le vendre avec un thème plus
  sérieux sans rien changer dessous — les noms sont des déclarations comme le
  reste, pas du code.

## 7. Déclaré n'est pas exposé (Lucie, 10 octobre au soir)

Lucie : « un backend boucle étrange développé par quelqu'un peut avoir des
agents, mais il ne les veut pas forcément tous disponibles après publication ;
il y en a peut-être de mode debug ; et la mémoire pareil : pas la peine, sur
un site où il y a du RAG, d'avoir dedans la mémoire de comment le projet s'est
construit. » Puis : « rendons ça générique : `exposure: clé… | clé… & clé…` ».

- **Chaque déclaration porte une exposition** — outil, agent, réaction,
  graphe, cellule de données — sous la forme d'une **expression sur des
  clés**, pas d'une paire fixe dev/publié : `exposure: published`,
  `exposure: dev | admin`, `exposure: published & premium`, `exposure: dev &
  (alice | bob)`. Les clés sont des mots libres, déclarés comme le reste.
- **Un contexte présente ses clés** : un déploiement (« published »), une
  session (« dev », « admin »), une personne, un abonnement, un serveur MCP
  lancé avec `--keys dev,admin`. Une déclaration est **chargée** si son
  expression est vraie pour les clés présentées ; sinon elle n'existe pas pour
  ce contexte.
- **Les cellules de données suivent la même règle** : la mémoire de la
  construction vit dans une cellule dont l'exposition est `dev` ; le RAG du
  site publié dans une cellule `published`. Déployer, c'est présenter des
  clés, et ce qui ne les satisfait pas ne part pas. Même isolation
  structurelle que pour les locataires : pas un `WHERE` à se rappeler.

Le point dur : la frontière se tient **au chargement** (ce qui n'est pas
exposé n'est pas chargé), pas par la politesse des outils — sinon un outil
`dev` lit une cellule publiée et la fuite par une réponse d'agent. Un serveur
MCP n'expose que ce que ses clés rendent vrai, rien par défaut.

