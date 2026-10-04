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

## 7. Les marches — proposées par l'orchestration, à valider par Lucie

Chaque marche se montre : elle a une preuve qu'on peut voir, et la liste de
ce qu'elle demande à la plomberie. On ne commence pas une marche pour sa
plomberie seule.

| # | La marche | Ce qu'on voit quand elle est faite | Ce qu'elle demande |
|---|---|---|---|
| 0 | **La stèle du moteur** (en cours) | rien à l'écran : une base qu'on peut tuer, rouvrir et croire | les tickets bloquants, le chargement journalisé, puis les verrous |
| 1 | **Les fiches de contexte** sur l'agent de code | une tâche longue où l'agent ne relit pas, n'édite pas sur du périmé, et où chaque édition porte sa raison | l'assemblage du prompt du chat, le registre des lectures, les commits de fiche ; rien du moteur au-delà de la marche 0 |
| 2 | **Voir** : une première page servie par rag3weaver | l'arbre des liens en graphe, une exécution nœud par nœud, une fiche, tout cliquable — en lecture seule | une vue déclarée, un afficheur, le pas d'exécution comme adresse |
| 3 | **Parler à une page** : l'éditer en place | Lucie dit « ajoute une colonne à cette liste », la page change dans son brouillon, le commit et sa raison sont là ; la voix en option | le contrôleur déclaré, le rechargement à chaud, le brouillon et la publication |
| 4 | **Les vitrines** | le constructeur de decks Magic ouvert à tous, des rejeux d'agent sur un dépôt connu | la base Magic refaite, un quota, l'authentification du service |
| 5 | **À plusieurs** | la page de démo que tout le monde édite, remise à zéro chaque jour | les écritures parallèles (fin de la stèle), la présence entre agents |

Ensuite, quand une marche les réclame : les secrets et les intégrations, les
migrations en verbes, les contributions de modules, d'autres bases par les
dialectes, l'archiviste.

Deux règles pour tenir ce cap :

- **Lucie est la première utilisatrice** : les marches 1 à 3 tournent d'abord
  en local, chez elle, sur ses projets. Une personne extérieure dès qu'une
  marche se montre.
- **La plomberie se choisit par la marche qu'elle sert** : un défaut du
  moteur qui corrompt ou qui perd se corrige toujours ; une optimisation ou
  un confort attend la marche qui en a besoin.

## 8. Ce qui n'est pas décidé

- « sans IDE » dès le départ, ou « à côté de ton IDE » d'abord ;
- le prix exact, et ce que l'abonnement compte (une personne, un poste, un
  déploiement) — à écrire dans la LRSL ou à côté ;
- l'ordre des trois chantiers de la stèle ;
- où vit une référence, où vit une mémoire (une base chacune, ou une seule) ;
- les conditions d'une contribution dans un produit sous LRSL.
