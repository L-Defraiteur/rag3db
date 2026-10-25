# Vision générale — le gloubi-boulga

Écrite le 4 octobre 2026, à mettre à jour sur place. Elle réunit ce que les
visions de ce dossier disent chacune de leur côté ; elle ne décide rien que
Lucie n'ait décidé. Chaque document du dossier porte sa date dans son nom.

## 0. Le cap

Lucie, 5 octobre 2026 : « Bien qu'on préfère la tuyauterie avant l'interface,
il faut quand même savoir ce qu'on vise ; sinon on fait un gloubi-boulga pour
un agent de code sans trouver un truc qui se vend vraiment, ou sans exploiter
le potentiel de la plomberie. »

**Ce qu'on vise, en une image** : *tu parles à ton code ou à ton site, et il
change sous tes yeux.* Pas de compte rendu à lire : l'écran montre ce qui se
passe — la chose qui change, les graphes que l'agent écrit et exécute, les
fiches qu'il ouvre — et tout se clique. L'ordinateur de Star Trek, les
tablettes de Westworld. La voix accompagne, elle ne raconte pas.

**Ce qui se vend** (un seul abonnement, §3) :

1. **l'atelier local** — parler à un agent, explorer des fiches et un graphe
   de code, sans IDE ;
2. **le backend tout-en-un qui porte son propre atelier** — un site déclaré
   (contrôleurs, vues, données) que sa propriétaire modifie en lui parlant,
   en place, sans compilation ni redéploiement ;
3. **les embarquements servis**, pour qui n'a pas de GPU.

**Pourquoi nous, et pas un agent de code de plus** — ce que la plomberie
rend possible et qu'un outil posé sur des fichiers ne peut pas faire :

| Parce que la plomberie… | …le produit peut |
|---|---|
| range tout dans un seul système, avec une adresse pour chaque chose | rendre tout cliquable : une ligne, une requête, un graphe, un pas d'exécution, une fiche |
| enregistre chaque exécution nœud par nœud | montrer ce que l'agent fait, au lieu de le lui faire raconter |
| tient ensemble le graphe, les mots et le sens | donner des fiches vivantes : ce qui a changé, ce qui dépend, ce qui ressemble |
| ne connaît que des choses déclarées | recharger à chaud, sans compiler : l'agent modifie le site en service |
| valide et reprend proprement (la stèle) | laisser un agent écrire dans une base en service sans la perdre |
| saura faire écrire plusieurs à la fois | faire travailler une équipe, chacun avec son agent, sur le même site |

Chaque ligne de gauche est du travail de plomberie déjà fait ou en cours ;
chaque ligne de droite est ce qu'on montre. Une pièce de plomberie qui
n'allume aucune ligne de droite attend.

## 1. L'idée en une phrase

**Tout vit dans un seul système, et tout s'y déclare** : la base (graphe,
mots, vecteurs), les traitements (des graphes), les outils, les mémoires, les
vues, les intégrations. Une chose déclarée de plus, et le système sait la
faire — sans code à part, sans service à part.

C'est le « gloubi-boulga » de Lucie, et le classeur de Cartman : il avale ce
qu'on lui tend. Ce qui l'empêche de mal finir, c'est que tout ce qui entre
est déclaré, vérifié, et accepté par la personne.

## 2. Les étages

| Étage | Ce que c'est | État au 4 octobre 2026 |
|---|---|---|
| le moteur (rag3db) | la base de graphe, ses index, sa durabilité | marche pour un écrivain ; sa **stèle** dit où l'on s'arrête (`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`) |
| rag3weaver | ingestion, recherche, graphes de traitement, manifestes, gabarits | livré, mesuré sur ses bancs |
| l'analyse du code (codeparsers) | scopes, usages, relations entre fichiers | livré ; `usages`, `impact`, liens |
| les outils de l'agent | recherche, lecture, grep, édition, sections après outil | livré dans le backend de code |
| la mémoire longue | gabarit `memory`, références, états | en cours |
| les fiches de contexte | lectures réunies par fichier, vivantes ; commits ; états | **vision** — rien de codé |
| l'archiviste | résumés à niveaux, porte à deux temps | **vision** |
| l'interface | parler à un agent, explorer des fiches et un graphe | **vision** — n'existe pas |
| le backend déclaré | contrôleurs, vues, services, gardes, secrets, intégrations | **vision** |
| les services vendus | embarquements ; démo gratuite | le service existe en interne, sans authentification |

## 3. Ce qu'on vend (dit par Lucie, 4 octobre)

Une **interface de code locale, « sans IDE »** : on parle à un agent, on
explore des fiches et un graphe de code. Un **service d'embarquement**. Rien
d'autre pour l'instant. Et une **démo gratuite**, à quota bas, surtout pour
montrer la force des fiches de contexte.

**Le modèle de prix, dit par Lucie le même jour** (« je ne sais pas » : une
piste, pas une grille) :

| Ce qui se paie | Comment |
|---|---|
| le logiciel lui-même | **un seul abonnement, de l'ordre de 20 € par mois, quel que soit l'usage** — coder avec, ou s'en servir comme backend en ligne |
| les embarquements | servis par nous et payants, **ou** branchés par la personne sur les siens |
| le modèle de langage | **pas vendu** : nous ne le servons que pour la démo gratuite ; à l'installation, un assistant dans le CLI fait choisir le modèle et la façon de le brancher |

Lucie, un peu plus tard : « au pire un seul abonnement, 20 € par mois, peu
importe l'usage, tant pis ; sinon ça va être embêtant pour mettre en avant que
ça peut servir direct de backend tout-en-un. » Une licence « backend » plus
chère avait été envisagée, puis écartée pour cette raison.

Le modèle de langage et les embarquements se déclarent déjà en service ou en
local (`models.<capacité>`) : l'assistant d'installation écrit cette
déclaration.

## 4. La carte des visions

| Document | Sujet | Nature |
|---|---|---|
| `2026-10-03-20h16-explorer-les-relations.md` | à quoi servent les relations du code, comment les explorer | vision |
| `2026-10-03-20h37-le-produit-code.md` | les idées pour le produit code | visions |
| `2026-10-03-20h37-memoire-longue.md` | une mémoire qui dure, les sujets abstraits | vision |
| `2026-10-04-00h09-memoires-par-gabarit-et-fiches-qui-pointent-vers-tout.md` | autant de mémoires qu'on veut, l'adresse à genres, la boucle étrange, l'archiviste | vision |
| `2026-10-04-00h09-l-archiviste-branche-sur-claude-code.md` | ce que les extensions de Claude Code permettent | étude |
| `2026-10-04-14h58-fiches-de-contexte-et-contexte-comme-une-vue.md` | curseurs, lectures réunies par fichier, commits avec leur raison, états d'une fiche, le contexte comme une vue | brouillon, en débat |
| `2026-10-04-16h17-depot-de-reference.md` | indexer un dépôt figé à la demande ; le grep par l'index y est sûr | brouillon |
| `2026-10-04-16h24-montrer-nos-produits.md` | démos, positionnement, ce que la personne voit, backend en graphes, secrets et intégrations, contributions | brouillon |
| `2026-10-09-13h36-plusieurs-produits-un-seul-atelier.md` | le même atelier Westworld avec un domaine différent à chaque fois : decks, code, DXF, Blender… ; ce qui est commun, ce qui est propre à chaque domaine | vision de Lucie |

À côté, hors de ce dossier : la feuille de route de septembre
(`../docs/vision_roadmap_09_2026/`), les propositions de la session mémoire
(`../docs/3-octobre-2026-20h41/`, `../docs/4-octobre-2026-14h00/`), les
modèles de décision (`../docs/optimiseur/`).

## 5. Ce qui revient dans toutes les visions

1. **Déclaré, pas codé** : un outil, une mémoire, une vue, un contrôleur, une
   intégration sont des manifestes et des graphes ; rien n'est écrit en dur
   pour un cas.
2. **Une seule adresse pour tout** (`row:`, plage, `query:`, `path:`, `url:`,
   `schema:`, `graph:`…) : ce qu'une fiche cite, ce qu'un curseur tient, ce
   qu'un lien ouvre au clic.
3. **Une donnée, deux rendus** : ce que l'agent lit en texte et ce que la
   personne voit à l'écran sortent du même résultat.
4. **L'agent choisit quoi montrer ou citer, pas ce qui existe** : les arêtes,
   les références et les secrets viennent du système, jamais de sa plume.
5. **Jamais un refus, une confirmation** : écrire une mémoire, créer un
   genre, partager un module — le système montre ce qui ressemble et demande.
6. **Ne pas injecter sans cesse** : un contexte se paie ; budget fixe, porte à
   deux temps, indices plutôt que rappels.
7. **Lisible par une personne** : un retour d'outil sera lu par quelqu'un qui
   regarde par-dessus l'épaule de l'agent.
8. **Mesurer avant d'allumer** : une idée entre éteinte, se mesure sur un jeu
   qui n'est pas de notre main, et un résultat négatif se dit tel quel.
9. **La boucle étrange** : ce dont le système est fait est adressable comme
   le reste ; il se documente et s'étend en lui-même.

## 6. La largeur

L'avis de l'orchestration, tel que Lucie a demandé de le garder :

> « La largeur. Un moteur de base de données, un orchestrateur, un analyseur
> de code, une mémoire longue, et maintenant une interface, un cadre web, des
> intégrations et une place de marché. Chacun est le travail d'une équipe. Le
> risque n'est pas qu'une pièce échoue, c'est que dix pièces soient finies à
> 70 %. »

La réponse de Lucie :

> « Je pense que le moteur nous aveugle sur la complexité : c'est le moteur
> qui nous prend beaucoup de temps à régler ; quand on l'aura bien fait, ce
> sera plus simple de faire ces sujets qui pour toi ressemblent à ceux d'une
> équipe. »

Ce sur quoi les deux s'accordent :

- au-dessus du moteur, un sujet nouveau est surtout une déclaration et
  quelques nœuds (le gabarit de mémoire : zéro Rust ; liens et cohésion : une
  journée) — la comparaison avec une équipe surestime leur coût ;
- le moteur a besoin d'un point d'arrêt écrit : c'est **la stèle** (verrous,
  chargement en masse journalisé, écritures parallèles, plus de défaut qui
  corrompt ou qui perd) ;
- tout n'attend pas la stèle : un agent seul sur un dépôt marche déjà.

### La preuve de l'agent (8 octobre 2026)

L'agent dans les graphes est, à ce jour, la preuve la plus nette de « une
déclaration et quelques nœuds » : la boucle (`src/agent.rs`) assemble des
pièces qui existaient pour autre chose — les outils sont tirés des schémas de
nœuds, le retour d'un outil est le rapport d'exécution d'un graphe, le
déclenchement est le réacteur sur le bus, le modèle est un service comme
l'embarqueur. Aucune pièce n'a été écrite pour l'agent seul.

Lucie, 8 octobre : « ça fonctionne étonnamment bien, l'agentique dans les DAG ;
j'ai été étonnée comme on a progressé vite sur l'inclure. Ça a été même plus
vite que d'écrire un agent LangChain ou Google ADK, et ça me semble plus
pratique : si on travaille direct en DAG, c'est bien mieux pour décrire des
boucles et des harnais. »

Ce qu'on en garde, et la réserve :

- la vitesse vient de ce que le cadre ne connaît que des choses déclarées,
  pas d'une astuce propre à l'agent ; c'est l'argument pour les marches 2 et 3
  (le backend et les vues déclarés devraient coûter de même) ;
- « mieux pour les boucles » se lit ainsi : le graphe est sans cycle, la boucle
  est autour (la boucle d'agent, le réacteur), et le harnais (bornes, rapports
  de validation, `src/harness.rs`) est déclaré au même endroit que ce qu'il
  borne ; un cadre à graphes cycliques (LangGraph) sait aussi dessiner une
  boucle — la différence n'est pas « graphe contre pas graphe », c'est que
  chez nous le graphe, ses exécutions, les données et les outils vivent dans
  le même système, avec une adresse chacun ;
- c'est jugé sur ce dépôt, par nous : pas de mesure extérieure encore (un jeu
  de tâches qui n'est pas de notre main, un utilisateur qui n'est pas Lucie).
  Même règle que pour le reste : mesurer avant de l'affirmer dehors.

### Le moteur et les autres bases (décision de Lucie, 5 octobre 2026)

> « Je pense qu'on garde rag3db, mais on rend rag3weaver vraiment compatible,
> pour que quelqu'un ajoute des implémentations aux dialectes. »

- **rag3db reste le moteur** : le graphe, l'embarqué, tout dans un seul
  processus sont ce qui rend le tout-en-un possible. Sa dette héritée (les
  chemins d'écriture, d'annulation et de reprise, peu éprouvés à l'origine)
  se paie par la stèle.
- **rag3weaver ne doit pas en dépendre par construction** : le dialecte de
  schéma (`src/dialect.rs`, avec une implémentation PostgreSQL) est le point
  d'extension. Le rendre « vraiment compatible » veut dire : un contrat écrit
  de ce qu'un dialecte doit fournir, ce que chaque dialecte déclare savoir
  faire (transactions, chargement en masse, index de mots et de vecteurs),
  et une batterie de tests qu'une implémentation nouvelle fait passer.
- Les migrations en verbes
  (`2026-10-04-16h24-montrer-nos-produits.md`, §12) passent par cette même
  couche : le vocabulaire est portable, les garanties se déclarent par base.

## 7. Les marches

Proposées par l'orchestration, **reprises par Lucie le 5 octobre** : « pas si
vite — le backend déclaratif en 2, avant ton "voir" ; et en 3 je mettrais les
vues aussi, peut-être aller plus loin et déclarer des DAG autour de
composants React, je ne sais pas encore comment. J'aime viser quelque part,
mais pas plonger dans la facilité qui sert juste à faire beau. »

Donc : **chaque marche pose une pièce durable de l'architecture**. Une page
jolie posée sur rien n'est pas une marche. Chaque marche a quand même une
preuve qu'on peut voir.

| # | La marche | La pièce qu'elle pose | Ce qu'on voit quand elle est faite |
|---|---|---|---|
| 0 | **La stèle du moteur** (en cours) | une base qu'on peut tuer, rouvrir et croire | rien à l'écran ; les témoins du banc |
| 1 | **Les fiches de contexte** sur l'agent de code | le contexte tenu comme un état, pas comme un fil | une tâche longue où l'agent ne relit pas, n'édite pas sur du périmé, et où chaque édition porte sa raison |
| 2 | **Le backend déclaratif** | un contrôleur est un graphe ; ses entrées (route, outil, événement) ; les services en sous-graphes ; les briques de contrôle et les ports ; le rechargement à chaud | un vrai backend — celui du code, ou celui des decks — déclaré de bout en bout, appelé par une route comme par un agent, modifié et rechargé sans compiler |
| 3 | **Les vues déclarées** | la fin d'un graphe nomme une vue ; le contrat de données tiré des ports ; des graphes autour de composants (à concevoir) | une page réelle servie par ce backend : les liens en graphe, une exécution nœud par nœud, une fiche, tout cliquable |
| 4 | **Parler à une page** : l'éditer en place | le brouillon et la publication ; chaque changement est un commit de déclaration | Lucie dit « ajoute une colonne à cette liste », la page change dans son brouillon ; la voix en option |
| 5 | **Les vitrines** | le quota, l'authentification du service | le constructeur de decks ouvert à tous, des rejeux d'agent sur un dépôt connu |
| 6 | **À plusieurs** | les écritures parallèles (fin de la stèle), la présence entre agents | la page de démo que tout le monde édite, remise à zéro chaque jour |

Ensuite, quand une marche les réclame : les secrets et les intégrations, les
migrations en verbes, les contributions de modules, d'autres bases par les
dialectes, l'archiviste.

**La piste à concevoir pour la marche 3 — des graphes autour de composants.**
Pas encore de réponse ; une forme à débattre : un composant d'interface est
un **nœud à ports**. Ses entrées sont ses données (ce que le graphe lui
donne) ; ses sorties sont ses événements (un clic, une saisie), qui sont des
entrées de contrôleurs. Le graphe dit d'où viennent les données et où vont
les actions ; le composant garde ce qu'il sait faire, dessiner et réagir.
Les ports étant typés, le contrat entre le composant et le backend se
vérifie comme celui de deux nœuds. Ce n'est ni « tout l'écran est un
graphe », ni « une page à la main » : le graphe câble, le composant affiche.

**Et les clics qui n'ont qu'un effet d'écran ?** Lucie, 5 octobre : « on ne
veut pas systématiquement un clic qui a un effet backend, souvent c'est un
effet front aussi ; je me demande comment même les clics gérés côté front
sont des DAG. »

Une réponse à débattre (proposition de l'orchestration, rien d'essayé) :

- **Un clic n'est pas une boucle, c'est un tour.** La boucle « clic → état →
  écran → clic » est une boucle *dans le temps*. À l'intérieur d'un seul
  événement, le chemin est sans cycle : (l'événement, l'état d'avant) →
  l'état d'après → ce que chaque composant reçoit. C'est un graphe.
- **Trois sortes de nœuds côté écran** : des **cases d'état** (la sélection,
  un filtre, la page courante, la valeur d'un formulaire) ; des **nœuds
  purs** (une expression qui calcule l'état d'après ou une donnée dérivée) ;
  des **nœuds d'effet** (appeler un contrôleur, changer de page). Un clic
  purement d'écran ne traverse que les deux premières sortes ; un clic qui
  touche le backend passe par un nœud d'effet.
- **Le même moteur des deux côtés** — la phrase de départ de Lucie, « un
  moteur de DAG pour le back et le front » : le moteur de graphes tournerait
  aussi dans le navigateur (compilé pour lui), et le graphe d'une page se
  coupe tout seul là où un nœud a besoin de la base. Les nœuds purs et les
  composants tournent devant ; le reste derrière.
- **Ce que cela donne en plus** : l'exécution d'un clic se voit dans le même
  afficheur que celle d'un contrôleur (§ « voir une exécution comme un
  graphe ») ; et l'agent qui édite une page modifie des graphes, devant comme
  derrière.
- **La limite à garder** : l'état purement de présentation d'un composant
  (un menu ouvert, un survol) reste dans le composant. Le graphe porte l'état
  qui compte pour l'application, pas chaque pixel — même règle que « les
  nœuds pour le flux, le script pour l'expression ».
- **Ce qui est à vérifier** : ce que pèse le moteur dans un navigateur, quels
  nœuds y sont permis, et quel langage d'expression y tourne (c'est là que
  TypeScript a un avantage naturel sur rhai).

- **La contrainte connue** (Lucie, 5 octobre) : « le souci, c'est tokio,
  qu'on a remis récemment ; donc pas envisageable de suite, à moins de le
  désactiver sur option et de prendre luciole quand on est sur navigateur. »
  Deux voies, aucune pour maintenant : (a) le moteur d'exécution derrière une
  option — tokio en natif, luciole dans le navigateur ; (b) ne porter devant
  que le cœur des graphes (le modèle, les ports, la vérification, un
  exécuteur simple) dans une pièce sans moteur asynchrone : un tour de clic
  fait de nœuds purs s'exécute d'une traite, et seuls les nœuds d'effet
  attendent — ce que le navigateur sait faire seul. À regarder quand la
  marche 3 arrive ; d'ici là, la marche 2 doit seulement éviter de coller le
  cœur des graphes à tokio plus qu'il ne l'est.

- **Le playground WASM, « luxe ultime de marketing » (Lucie, 11 octobre)** :
  une vitrine qui tourne dans le navigateur, « regardez comme mes habits sont
  saillants ». Pas maintenant, mais la pile est connue : rag3db compilé en
  WASM (précédent Kuzu, emscripten, ses fils par workers) ; rag3weaver dessus
  avec tokio à un fil **dans chaque worker** (`rt`, `sync`, `macros` compilent
  pour wasm32 ; pas de réseau, de fichiers ni de processus) ; **luciole entre
  les workers** comme couche d'acteurs — la seule forme de parallélisme d'un
  navigateur sans mémoire partagée, pas un pis-aller ; embarquements et modèle
  par le réseau vers un service ; le navigateur n'est qu'un client mince de
  plus du démon. Ni fork de tokio, ni luciole réécrite en exécuteur : les deux
  sont complémentaires. Un projet à part, de l'ordre de quelques semaines,
  après ce qu'on fait.
- **Tranché par Lucie, 8 octobre** : le navigateur (WASM) est **abandonné
  pour l'instant** — « chiant de se passer d'async partout ». Plus aucune
  restriction sur la marche 2 pour le ménager : tokio partout, la boucle
  d'agent et le réacteur restent des fils, et ce qui est asynchrone chez les
  outils le reste. Le jour où le navigateur revient, ce sera **un projet pour
  améliorer luciole** (l'exécuteur asynchrone de Lucie), pas des restrictions
  dans rag3weaver. Les deux voies ci-dessus restent écrites pour ce jour-là.

Trois règles pour tenir ce cap :

- **Lucie est la première utilisatrice** : les marches 1 à 4 tournent d'abord
  en local, chez elle, sur ses projets. Une personne extérieure dès qu'une
  marche se montre.
- **La plomberie se choisit par la marche qu'elle sert** : un défaut du
  moteur qui corrompt ou qui perd se corrige toujours ; une optimisation ou
  un confort attend la marche qui en a besoin.
- **Rien qui ne sert qu'à faire beau** : une démonstration se bâtit sur une
  pièce posée, jamais à côté.

### Le démon, et après (Lucie, 11 octobre 2026)

Décidé : **le serveur MCP n'est qu'une interface vers un démon** ; le démon
possède le catalogue, les travaux longs (indexation, commande en fond) et le
réacteur ; le chat, l'atelier, la CLI et MCP sont des clients minces qui lui
parlent par une seule forme (page `docs/11-octobre-2026-arbre-principal/01-…`).
Un seul démon pour toutes les bases pour le moment. **À moyen terme, après ce
chantier** : un démon routeur et des démons spécialisés derrière lui — un par
base, ou portant plusieurs bases — pour qu'un démon qui tombe n'emporte pas
tout et que le tampon du moteur ne se partage plus entre tous les backends.
C'est aussi la voie Neo4j pour plusieurs processus : un processus tient une
base, les autres lui parlent ; le multi-processus simultané sur un même
fichier (mémoire partagée, PostgreSQL) reste une refonte non engagée.

### L'IR, et le chaînage des formes (Lucie et l'orchestration, 11 octobre 2026)

L'IR (`rag3weaver-ir`) n'est pas un langage qu'on écrit : c'est une structure
de données — des formes typées (`Hop`, `Select`, `Count`, `Write`, bientôt
`Tx`) dont les noms d'entités, de champs et de relations sont des valeurs.
Il n'y a pas de texte au-dessus des dialectes, donc pas d'échappement, pas
de conflit possible entre nos verbes et les noms déclarés ; le texte (Cypher,
SQL) naît dans le dialecte seul, qui cite chaque identifiant à sa façon. Le
format de transport (JSON quand le client mince parlera au démon) est à
part de la structure ; un seul suffit. Les limites de l'IR sont dans ce
qu'il sait dire, et chaque manque se comble par une forme de plus, que
chaque dialecte déclare savoir traduire ou refuse en la nommant.

**Le chaînage des formes** : aujourd'hui, c'est le graphe de traitement qui
compose, une forme par nœud, un aller-retour par forme. Le jour où un fil
sépare l'outil du moteur (le démon, PostgreSQL), le chaînage utile n'est pas
une forme « pipeline » de l'IR mais **un sous-graphe de formes confié entier
au dialecte** : le mermaid reste le même, une frontière y marque le
sous-graphe « au dialecte » ; un dialecte qui le sait le traduit en une seule
requête (une chaîne de MATCH, des tables temporaires nommées), un autre
l'exécute pas à pas comme aujourd'hui — même déclaration, deux exécutions.
C'est une capacité déclarée de plus. Le sous-graphe confié ne contient que
des formes de l'IR (un nœud scripté ou un nœud qui parle au système y est
refusé au chargement), et l'exécution enregistrée dit qu'il a été poussé
d'un bloc, avec le texte produit. À faire quand une mesure le demande.

### L'agent est un graphe à ports, et les ports portent des flux (Lucie, 11 octobre 2026)

« Un agent est un DAG ; un DAG est potentiellement un outil quand on a besoin
qu'il le soit, dont les agents ; ce que ça veut dire n'est pas forcément
d'attendre son retour : peut-être juste le lancer en fond avec un message
initial et la référence de qui je suis, qu'il puisse me parler. » Et : « à
tout moment pouvoir faire un appel structuré — selon ce qu'a dit
l'utilisateur une classification, un routage vers un autre agent, une
action ; selon ce que quelqu'un lui a envoyé, transmettre ou non ; selon un
retour d'outil ou une réponse d'agent, faire telle chose en plus — que tout
soit traduisible dans le graphe de l'agent, comme un flot de conversation
mais beaucoup plus permissif, en synergie avec notre moteur. »

**La boucle est un graphe déclaré, pas du code.** L'agent livré est un graphe
par défaut à ports nommés — message utilisateur entrant, message d'un pair
entrant, résultat d'outil entrant, réponse du modèle, appel d'outil sortant,
réponse finale, fin de tour (le retour au début) — et il se comporte, sans
rien de branché, comme la boucle d'aujourd'hui. Sur chaque port, n'importe
quel sous-graphe : un petit jeu de nœuds génériques (décider par un appel
structuré, router, filtrer, agir en plus) et le nœud scripté pour le reste.
Les cas cités sont des exemples, pas un catalogue ; il n'y a pas d'états,
il y a des ports et des sous-graphes. Chaque décision, message et route
retombe en base avec la trace, rejouable et mesurable, et un nœud de routage
peut interroger le graphe par l'IR avant de choisir. Deux verbes que le
modèle choisit, « envoyer » et « lancer en fond », sont aussi des nœuds
posables sur un port : le déclaré et le choisi, même mécanisme.

**Les ports portent des flux.** Un seul mécanisme : tout port porte un flux —
des éléments dans le temps, et une fin ; une valeur est un flux d'un seul
élément, marqué comme tel à la vérification du graphe. Le nœud dit sa
posture, élément par élément ou sur le flux complété, et la vérification
commune refuse un nœud « complet » sur un flux sans fin. Il s'ensuit :
interrompre, c'est fermer un flux avant sa fin ; la politique de livraison,
c'est quand le nœud modèle lit le flux des pairs ; un agent lancé vit tant
qu'un de ses flux d'entrée est ouvert ; la voix et le son (STT, TTS) sont
des ports de flux ordinaires ; le playground à acteurs en est l'exécuteur
naturel. **Ce qui ne doit rien perdre passe par un port, jamais par le bus** :
un port refoule jusqu'au producteur, le bus écrase les plus anciens et le dit
— il sert à observer (mesuré le 12 octobre, page
`docs/12-octobre-2026-recherche/04-la-boite-en-flux-ouvert.md`). Consigne de méthode, dite avec : « la complexité n'est jamais un
argument, l'important c'est que ce soit au top » — la parité prouvée avec la
boucle d'aujourd'hui en est la condition, pas la charge de l'exécuteur.
Chantier ouvert chez la session du proto tout déclaratif le 11 octobre
(page `docs/11-octobre-2026-everything-declarative/03-les-agents-se-parlent.md`).

### Un IDE adapté aux agents (Lucie, 11 octobre 2026)

« Le vrai but, c'est qu'avec ce MCP vous vous sentiez déjà comme dans un IDE
adapté à vous. S'il y a des choses à revoir pour ça, on reverra. Comme si
vous étiez des malvoyants qui avaient d'autres sens. »

Ce qu'un IDE donne à un humain, c'est voir sans lire : la structure, qui
appelle qui, ce qui casse si je touche, ce qui est verrouillé sur le chemin,
où en est l'index. Un agent n'a pas d'écran ; il a des outils qui rendent du
texte, et sa boîte. Le gabarit `code` par le serveur MCP et le démon est
cet IDE : `usages`, `impact`, `callees`, le voisinage, les verrous sur le
chemin, l'état de l'index, le refus qui dit quoi faire. La mesure n'est donc
pas « l'outil rend la bonne réponse » mais « un agent qui travaille sur
rag3db avec cet MCP ne retombe plus sur grep ni sur des lectures par
tranches ». Ce qui compte, c'est le reste : ce que l'agent demande et qu'il
n'a pas. D'où la règle, dès le premier index réel de nos dépôts : les
sessions qui travaillent le code posent leurs vraies questions par le MCP et
tiennent une page « ce qui manque à un agent dans cet IDE », avec chaque
retour forcé à grep et sa raison ; c'est « demander son avis au modèle », en
vrai, et c'est ce qui décide des outils suivants.

### L'atelier comme IDE des gabarits — la priorité (Lucie, 18 octobre 2026)

« Je me dis qu'on devrait vite arriver à notre petit pseudo-IDE de graphe de
code, et chargement à chaud tout déclaratif, des agents non pas de code mais
d'édition de programmes gabarits rag3weaver (dont aussi des composants React),
c'est vraiment le truc que je veux voir le plus vite ; et si je peux visualiser
les DAG, ça m'inspirera peut-être. » Et, en second : « la version avec vrai
code, qui explore du code indépendant de rag3weaver embarqué dedans et l'édite
sur un worktree, mais du coup interface web quand même pour ça. Les deux
profiteraient du multi-fenêtre des navigateurs modernes. »

Ce qu'on a déjà : la page vivante (`src/atelier.rs`, `ui/chat`, `serve
--atelier`) : le chat avec l'agent et ses outils de déclaration, la page
produite, le graphe de l'outil touché en mermaid, un flux d'événements
(`declared`, `refused`, `reloaded`) et une montre qui recharge à chaud. C'est le
germe : l'IDE se bâtit dessus, il ne repart pas de zéro.

Ce qu'il y faut, dans l'ordre :

1. **Voir** : chaque vue est une URL (un gabarit, le DAG d'un outil, le graphe
   du code, le journal d'un run), ouvrable dans sa fenêtre ; les fenêtres se
   tiennent à jour par le flux de l'hôte, jamais par un état propre à l'onglet.
   Le DAG se dessine vivant (les nœuds qui tournent, les ports et leurs flux,
   l'annulation), pas seulement en mermaid figé.
2. **Éditer par l'agent** : un agent dont l'objet est le gabarit, pas le code —
   il lit, propose, valide (la déclaration refuse ou recharge), et la page
   montre la différence ; un composant React est un gabarit comme un autre,
   compilé par l'hôte, rechargé à chaud (la graine des applications
   empaquetées).
3. **Le code d'à côté** : le même IDE sur un dépôt étranger, indexé par le
   gabarit `code`, édité par l'agent dans un worktree à lui, la différence
   relue avant toute fusion.

La règle de la stèle tient : tout est déclaré, et la page n'a pas d'état que
l'hôte n'ait pas. Le navigateur abandonné le 8 octobre était le moteur en WASM ;
ici le moteur reste dans l'hôte et le navigateur n'est qu'une vue.

### Un hôte d'équipe : un index par dépôt, un routeur, les fiches au routeur (Lucie, 18 octobre 2026)

Graine, née d'une question de Lucie sur une mémoire partagée par une équipe.
Une équipe a plusieurs dépôts ; on lui sert, par MCP, depuis un service à elle
(un pod), le code et la mémoire de tous :

- **un index par dépôt, sur la tête de sa branche de travail** (`develop`),
  tenu à jour à chaque push (montre sur un pull, ou un webhook) ; pas un index
  par commit : l'histoire passe par des fiches qui visent un commit
  (immuable) ;
- **un index maître, routeur**, qui passe une question aux index concernés et
  fusionne les réponses (le pont entre bases de la page des migrations, §12,
  en lecture seule) ; **les fiches vivent sur lui** et visent tout dépôt par
  le préfixe `<dépôt>:` de leurs références ;
- **chaque fiche porte son auteur**, posé par l'hôte d'après l'identité de la
  session (jamais déclaré par l'agent) : « le Claude de Jean sait qu'il est
  celui de Jean » ;
- **les droits ne s'inventent pas** : ils viennent de l'application où les
  membres sont déjà inscrits, selon leurs scopes (Lucie : « les admins déjà
  enregistrés sur l'app, selon leurs scopes ») ; l'hôte accepte un jeton de ce
  fournisseur d'identité et traduit ses scopes en lecture et écriture par
  dépôt et par mémoire ;
- **les fiches en fichiers** (la base n'est qu'un index reconstructible) se
  partagent par git, avec relecture et fusion.

Première marche plus simple : un seul hôte, plusieurs arbres dans une base
(`code-tree add`) et la mémoire, sans routeur ; le routeur quand le nombre de
dépôts, les droits par dépôt ou la taille l'exigent. Préalable : les correctifs
de la perte du COPY de relations (42a32bd52), que `code-tree add` déclenche.
L'outil dit toujours sa fraîcheur (« indexé à tel commit de develop »).

### Un produit d'audit de sécurité (Lucie, 18 octobre 2026)

Graine, notée à la lecture des prix de Vertex. CodeMender, l'agent de sécurité
du code de Google DeepMind (en préversion sur Vertex, dans la plateforme
d'agents Gemini Enterprise, sur Gemini 3.6 à 3.8 Flash), lit le source,
cherche les failles, prouve qu'elles sont exploitables par une attaque simulée
et propose un correctif validé, relu par un humain avant tout envoi ; il couvre
C, C++, Go, Java, Python, Rust et TypeScript. Lucie : « graine CodeMender oui,
pour un produit d'audit de sécu ».

Ce qu'on en tirerait : un produit rag3weaver d'audit de sécurité, déclaré comme
le reste. L'index de code donne déjà ce qu'un auditeur cherche sans lire :
`usages`, `impact`, `callees`, le voisinage, les entrées non sûres et les
chemins jusqu'à elles. Un agent d'audit (CodeMender appelé comme un outil, ou
notre propre agent sur les mêmes modèles) en tire des fiches de faille : la
preuve, le correctif proposé et la relecture, chaque faille devenant un ticket
et chaque correctif un lot. Premier essai à prévoir chez nous : un audit du
moteur C++ et de rag3weaver avant la sortie du paquet npm. À vérifier avant
tout envoi de code chez Google : les conditions de lecture et de conservation
du source. Et rien ne part vers les amonts : les correctifs restent nos lots.

### La voix sur toute interface de chat (Lucie, 25 octobre 2026)

Une graine, pour un chantier à part : tout produit qui expose une interface de
chat (l'atelier, l'IDE, l'application empaquetée dans Electron, un pod chez un
client) doit pouvoir s'écouter et parler, sans ajouter un environnement Python
pour ça. Ce que Lucie a relevé, à vérifier avant d'en faire une dépendance :

- **voicepipe** (crate Rust) : le pipeline de la parole, et non les moteurs.
  Détection de début et de fin de parole, VAD, transcription en flux, découpe
  des réponses en phrases pour la synthèse, génération anticipée des segments,
  interruption (barge-in) et annulation, et un trait `Llm` qu'on implémente
  nous-mêmes : c'est là que rag3weaver se branche, avec ses agents et ses
  DAG. Les moteurs STT et TTS se branchent à côté. Sa maturité est à mesurer.
- **Pocket TTS en Rust** (Kyutai) : des binaires autonomes, sans Python ni
  PyTorch, pour une synthèse vocale locale dans un processus dédié.
- **voxkit**, à regarder aussi : les abstractions VAD, STT, TTS, micro et
  lecture, mais moins complet sur les tours de parole.

L'architecture envisagée : Electron (micro, sortie audio, interface) →
voicepipe (VAD, tours de parole, barge-in, flux audio) → rag3weaver (les
agents et les DAG, par le trait `Llm`) → Pocket TTS Rust (la synthèse locale).
Ce qui resterait à écrire : le transport audio et le branchement des moteurs ;
ce qu'on éviterait d'écrire : toute la logique des interruptions et des tours
de parole. Rien n'est décidé ni commencé : c'est un chantier propre, qui vient
quand le chat lui-même (l'atelier dans l'IDE, l'agent qui code dans une tâche)
est stable.

### Le juge sur les outils que les agents déclarent : bac à sable et surveillance (Lucie, 25 octobre 2026)

Une graine, dite le soir où le juge de session est arrivé sur master
(4b64a98e5) et où son second rapport, le bilan des outils, se concevait
(juge-outils-1 : une fiche par outil, des défauts typés avec preuve, gravité
et forme proposée, le retour jugé en cinq genres). Lucie : « pourrait servir
aussi pour des bacs à sable / staging d'outils créés déclarativement par des
agents, hop ils provident un contexte de test et un juge vient regarder, ou
bien leurs prochains appels y a flag comme outil à surveiller, et le juge vient
observer la session et demander à quelqu'un d'améliorer ». Structuré ainsi,
tout en déclaratif, avec les pièces qui existent déjà (propositions, réactions,
tâches, fiches de la mémoire) :

- **Un outil déclaré naît sous surveillance.** La proposition qui ajoute un
  outil au manifeste d'un backend (un agent qui déclare un outil, un gabarit
  nouveau) pose sur lui une marque `a_surveiller`. Tant qu'elle est là, chaque
  run qui l'appelle déclenche le bilan des outils sur ce run, et pas seulement
  à la fin d'un tour du chat : le juge observe la session par le journal des
  runs, comme aujourd'hui, mais pour cet outil à chaque appel.
- **Le bac à sable : des essais fournis avec la déclaration.** Celui qui
  déclare l'outil fournit un contexte d'essai : quelques appels avec leurs
  arguments, et ce qu'il attend du retour (un champ présent, une forme, un
  compte). L'hôte les joue dans un backend de mise en scène (une base
  éphémère, un instantané reflink de la vraie, jamais la vraie), et le juge
  relit les retours avec ses genres : mal formé, incohérent d'un appel à
  l'autre, bavard, hors sujet, incomplet ou ambigu, paramètre manquant ou mal
  nommé. Aucun défaut bloquant : l'outil entre en service, sous surveillance.
  Un défaut bloquant : la proposition revient à son auteur avec la fiche du
  juge, elle ne s'applique pas.
- **La surveillance se lève par la fiche.** La fiche de l'outil (« outil :
  <nom> », ancrée outil:<nom>) accumule les défauts run après run, par
  gravité. Après un nombre fixé de runs sans défaut bloquant ni gênant, la
  marque tombe ; un défaut qui revient ensuite la repose. La tendance par
  genre de défaut, déjà prévue dans la page par outil de l'IDE, est ce qu'on
  regarde pour décider.
- **Le juge n'améliore pas, il adresse.** Une réaction déclarée sur « défaut
  bloquant ou récurrent sur un outil » ouvre une tâche pour l'auteur de
  l'outil, s'il est un agent, avec la fiche en contexte : la même tâche que
  celle où le chat code aujourd'hui (ae127d634), sur la branche de la tâche,
  jamais ailleurs, et sa proposition repasse par le bac à sable. Si l'outil est
  livré avec le produit, c'est un ticket dans docs/tickets, pour un humain. Le
  juge ne touche jamais ni l'outil ni le manifeste : il range, il compare, il
  demande.

- **L'outil garde qui l'a fait, et le contexte de ce moment.** Aujourd'hui une
  proposition porte son auteur et son instant (`auteur`, `at_ms`), mais l'outil
  appliqué au manifeste ne les garde pas, et le journal des runs est borné (200
  runs par backend, le milieu d'un long run tombe) : on sait qui et quand au
  moment de la déclaration, on ne retrouve pas le contexte plus tard sans l'avoir
  figé. Donc la déclaration estampille l'outil (auteur, run, instant) et fige
  dans la fiche de l'outil le contexte qu'avait l'agent à ce moment : le journal
  de ce run entier, hors de la borne, la tâche, les messages qui ont mené à la
  déclaration. Deux chemins pour la correction, au choix de la réaction :
  - **la session de l'auteur est encore ouverte et récente** : la suggestion du
    juge (la fiche, le défaut, la forme proposée) s'injecte directement dans sa
    session, comme un message du bus, et c'est lui qui corrige dans son fil ;
    cela suppose que les réactions puissent s'adresser aux agents, ce que Lucie
    doit encore blanchir (18 octobre) ;
  - **sinon** : l'agent est relancé dans une tâche neuve avec le contexte figé
    et la fiche, et sa correction repasse par le bac à sable. Automatique
    seulement si la correction automatique des outils est activée sur le backend
    (`autofix_outils`, éteint par défaut) ; sinon la tâche attend qu'un humain
    la lance.

Rien n'est décidé ni commencé au-delà de juge-outils-1. L'ordre naturel : le
bilan des outils d'abord (c'est lui qui fournit le jugement), puis la marque
`a_surveiller`, l'estampille de l'outil et la réaction qui adresse (ce sont des déclarations), puis le
bac à sable avec ses essais (c'est le seul morceau qui demande une mise en
scène de l'hôte). Ce qui reste à trancher avec Lucie : qui a le droit de poser
une marque, combien de runs lèvent la surveillance, et si un outil déclaré par
un agent peut écrire avant d'être sorti de surveillance.

## 8. Ce qui n'est pas décidé

- **S'intégrer à l'usage, une graine** (Lucie, 11 octobre 2026, à propos
  de agent-bridge, https://github.com/raysonmeng/agent-bridge, qui interface
  différents logiciels d'agents de code : « on pourrait s'inspirer après pour
  que Claude puisse parler à des agents du moteur, ou l'inverse, ou Codex ;
  l'adoption sera plus grande si on commence par s'intégrer à l'usage des
  gens, ils ne quitteront pas Claude en claquant des doigts »). Le serveur
  MCP est déjà cette entrée pour Claude Code (le cœur a eu son IDE sans
  changer d'outil). Ce qu'elle ajoute : le même serveur pour les autres
  agents de code (Codex, Cursor, tout client MCP) et un pont pour ceux qui
  n'en parlent pas ; dans l'autre sens, nos agents (des graphes) appelant
  Claude Code ou Codex comme un outil, lancé en fond avec la référence de
  l'appelant, comme un agent à nous. Le principe : entrer dans l'usage
  existant, et le moteur, les fiches et l'index deviennent ce que ces agents
  partagent entre eux. Pas de chantier.
- **Injecter dans Claude Code, les canaux** (Lucie, 11 octobre 2026 : « on ne
  peut pas altérer le contexte existant de Claude, mais lui en injecter petit
  à petit ; l'archiviste, des hooks pour des résumés »). Vérifié sur la
  documentation de Claude Code (hooks, channels, mcp, sessions, Agent SDK) :
  **les channels MCP** (`notifications/claude/channel` émise par le serveur,
  session lancée avec `--channels server:<nom>`, arrive au tour suivant dans
  une balise `<channel source=…>`) sont le seul canal poussé par le serveur
  quand il le décide ; **les hooks** (`SessionStart`, `UserPromptSubmit`,
  `PostToolUse`, `Stop`, `PreCompact`) injectent `additionalContext` jusqu'à
  10 000 caractères à leur moment ; les ressources MCP ne sont lues qu'à la
  demande ; l'Agent SDK n'entre pas dans une session interactive existante ;
  rien ne pousse sans hook ni channel, et toute injection consomme du
  contexte. Pour l'archiviste (vision du 4 octobre) : l'hôte, qui sait ce
  qui change (fiche à revoir, index en retard, un pair qui écrit), pousse par
  le channel au moment où ça arrive ; un hook avant chaque message demande à
  l'hôte les fiches pertinentes et les glisse en contexte. Graine, pas de
  chantier sans le mot de Lucie.
- **Des applications rag3weaver empaquetées, une graine** (Lucie, 12 octobre
  2026 au soir : « notre projet pourrait être exportable : t'es sur ton
  Windows, Mac, Linux, Android, tu peux lancer l'app quand même, des apps
  rag3weaver packageables ; une autre vision long terme »). Un produit
  (un atelier avec ses gabarits, ses graphes, ses vues, sa base) devient une
  application qu'on lance sur n'importe quel poste sans rien installer
  autour. Ce qui y mène déjà : le démon portable (lot C, vert sur Linux,
  macOS et Windows le 12 octobre), le paquet npm qui livre les binaires par
  système, la lib du moteur liée par identifiant de bâti, l'atelier qui n'est
  qu'une page servie et des déclarations rechargées à chaud. Ce qui manque :
  un seul binaire qui embarque l'hôte, le moteur et ses extensions, le
  gabarit du produit et sa base à côté ; les services d'embarquement en
  local ou distants selon le poste ; et Android, le plus loin (le moteur C++
  et le crate Rust pour l'ARM d'Android, des modèles petits). **Le frontend
  reste à penser** (Lucie, le même soir) : aujourd'hui c'est la page vivante
  de l'atelier, servie par l'hôte, déclarée et rechargée à chaud ; pour une
  application, la garder comme seul frontend enveloppé par système (une
  fenêtre native qui l'affiche, une vue web sur Android), ce qui tient
  « tout en gabarit » jusqu'à l'écran, ou rendre une part native un jour :
  question ouverte. Et le bout de la vision (Lucie, 12 octobre au soir) :
  « que même des apps three.js puissent être empaquetées, que j'aie
  vraiment tout : le mec construit son moteur de jeu en rag3weaver, ou son
  app de classeur Excel, ou n'importe quelle autre ». C'est « plusieurs
  produits, un seul atelier » poussé jusqu'au bout : les données de l'app
  dans le moteur, sa logique en graphes et en nœuds scriptés (TypeScript),
  ses écrans en vues déclarées ; une scène three.js ou une grille de
  classeur est une vue de plus, avec ses propres fichiers (un bundle) et son
  contrat de ports vers les graphes ; l'atelier qui sert la page devient le
  lanceur de l'application. Ce qui manque est précisément cela : l'atelier
  qui accepte un frontend arbitraire comme vue déclarée (le bundle, ses
  assets, ses ports), et l'empaquetage qui l'emporte avec l'hôte, le moteur
  et la base. Forme proposée par Lucie (même soir) : « un module React qui
  se recharge à chaud, embarquer la compilation sur commande ou en hot
  reload ; pour le frontend, que ça ne sorte jamais des données ». C'est le
  même chemin que les graphes : les composants et leurs sources sont une
  donnée du produit, l'hôte les compile lui-même (le moteur de script qui
  exécute déjà les nœuds TypeScript est la piste pour compiler du TSX en
  bundle, à la demande ou à chaque changement), et la page vivante recharge
  le module comme une vue ; l'écran est déclaré, stocké, versionné et
  rechargé par le même chemin que le reste, l'empaquetage n'emporte que la
  base. Les libs aussi (Lucie : « dont l'install de libs donc aussi »)  :
  le module déclare ses dépendances et leurs versions, l'hôte les va
  chercher sur commande seulement (jamais en silence : c'est du réseau
  sortant), les range dans un magasin par contenu comme les libs du moteur
  par identifiant de bâti et les modèles dans le cache, le compilateur
  embarqué les lie dans le bundle ; l'empaquetage emporte le magasin avec
  la base et l'application démarre sans réseau ; une mise à jour de lib est
  un changement déclaré, versionné, rechargé à chaud. Graine, pas de
  chantier sans le mot de Lucie.
- **Distribué, une graine** (Lucie, 11 octobre 2026 : « peut-être des visions
  avec des systèmes distribués comme ça, mais je ne sais pas encore ce qu'on
  pourrait en faire à part distribuer l'inférence »). La brique est posée le
  même jour : un hôte par poste, un client qui sait lequel, un jeton par
  hôte, le manifeste qui dit son hôte. Ce qu'elle porte sans chantier à
  part : l'index qui tourne là où est l'arbre ; l'embarquement et l'inférence
  là où est la carte ; le démon routeur et les spécialisés sur plusieurs
  postes ; « une équipe qui parle à un site déployé » (la condition 3 vue du
  réseau) ; les agents qui se parlent entre postes par la boîte durable en
  base ; une base gardée à deux endroits, sauvegarde avant réplication. Pas
  de cap « distribué » pour lui-même : chaque ligne arrive par un besoin.
  Et, de Lucie le même soir : « des échanges entre bases, un truc bizarre de
  réseaux : contrôler toute ton infra d'un seul agent, qui se répercute sur
  des nœuds qui eux-mêmes se répercutent après, comme un réseau de fourmis ;
  je ne sais pas encore quels cas exacts ». Chez nous c'est la réaction vue
  en réseau : un hôte s'abonne aux événements d'un autre (la boîte durable en
  base pour passer d'un hôte à l'autre, la réaction déclarée au manifeste
  pour répondre, `spawn` pour lancer un agent là où l'événement arrive),
  chaque hôte ne connaissant que ses voisins déclarés ; « contrôler l'infra
  d'un agent », c'est le backend déclaratif altérable depuis lui-même dont la
  déclaration se propage aux hôtes abonnés, chacun l'appliquant à sa façon ou
  refusant en nommant, chaque propagation tracée comme un run avec son
  parent. Premier cas sans rien inventer : nos deux postes (lib rebâtie,
  index rafraîchi, ticket fermé, dits d'un hôte à l'autre sans relais à la
  main).

- « sans IDE » dès le départ, ou « à côté de ton IDE » d'abord ;
- le prix exact, et ce que l'abonnement compte (une personne, un poste, un
  déploiement) — à écrire dans la LRSL ou à côté ;
- l'ordre des trois chantiers de la stèle ;
- où vit une référence, où vit une mémoire (une base chacune, ou une seule) ;
- les conditions d'une contribution dans un produit sous LRSL ;
- rag3db donné seul : Lucie, 10 octobre, « un jour, quand on publiera pour de
  vrai rag3weaver, on donnera rag3db en MIT à côté ; on réfléchira alors à
  remettre lucivy dessus, utilisable par Cypher ». Une intention, pas une
  décision ; en attendant, l'extension `fts` de l'amont est retirée du dépôt.
