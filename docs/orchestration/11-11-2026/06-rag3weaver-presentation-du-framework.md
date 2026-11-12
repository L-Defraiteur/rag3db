# rag3weaver — un atelier d’agents dont les capacités sont des données

*Présentation au 11 novembre 2026. À lire sans connaître le dépôt ni nos conversations précédentes.*

> **Tu parles à ton code, à tes documents ou à ton application ; le système agit,
> montre par où il est passé et conserve ce qu’il a appris.**

Lucie construit un environnement où les agents travaillent avec des données, des outils,
une mémoire et des traitements décrits dans un même modèle déclaratif. Le terrain d’épreuve
actuel est le développement logiciel : le framework sert déjà sur son propre dépôt.

La promesse est celle d’un atelier extensible : un domaine nouveau apporte ses entités,
ses opérations et ses rendus ; il réutilise la recherche, les graphes d’exécution,
les agents, la mémoire et les traces de l’ensemble.

Ce qui rend cette ambition intéressante, c’est qu’une partie importante du mécanisme tourne déjà.
La créature a des organes fonctionnels. Le système nerveux continue de passer des examens.

## 1. L’idée structurante : savoir et capacités dans un même système

Une base de connaissances décrit des entités, des champs, des relations, un découpage et des
signaux de recherche. Un traitement compose des nœuds à ports typés. Un outil expose un graphe
avec ses paramètres. Un backend rassemble dans un manifeste les données, les outils,
les modèles et le périmètre de travail qu’il rend disponibles.

Cette continuité permet de construire des comportements par composition :

- Une recherche hybride est un graphe qui interroge plusieurs signaux et fusionne leurs résultats.
- Un outil peut appeler un sous-graphe ; un traitement peut appeler un graphe pour chaque élément.
- Un appel de modèle peut prendre place dans un traitement et rendre une décision structurée.
- Une réaction peut lancer un graphe sur un événement du bus.
- Une mémoire est un backend déclaré, avec ses entités, ses liens et ses outils.

**La boucle d’agent assemble ces mécanismes.** Les outils destinés au modèle sont décrits à partir
des schémas ; leur exécution produit des rapports du runtime. Les traitements et les outils
partagent ainsi une représentation exploitable par le système.

Cela ne signifie pas que tout ajout est magiquement possible sans programmation. Les primitives
nouvelles ont un coût et peuvent demander du Rust ou un service spécialisé. L’intérêt est de
réutiliser ensuite ces primitives dans plusieurs domaines et plusieurs interfaces.

## 2. Les pièces, et pourquoi elles sont réunies

| Pièce | Fonction |
|---|---|
| **rag3db** | Moteur embarqué de graphe en C++, fork de Kuzu, avec transactions, journal de reprise et index vectoriel HNSW. |
| **lucivy** | Recherche plein texte, sous-chaînes, flou et positions ; utilisé aussi dans la chaîne de recherche creuse. |
| **rag3weaver** | Runtime Rust : ingestion, recherche, graphes de traitement, outils, modèles, agents, sessions et mémoire. |
| **codeparsers** | Analyse du code : scopes, symboles, usages et relations entre fichiers pour plusieurs langages. |
| **Hôte, IDE et terminal** | Accès aux backends et aux agents, suivi des actions et présentation des résultats. |

Le socle de données et le plein texte fonctionnent dans le processus. Les modèles peuvent être
locaux ou servis à distance ; des services d’embarquement sont déjà utilisés sur une autre machine.
Il faut donc distinguer **un système cohérent avec des services et des agents coordonnés** d’une
**base distribuée** : cette dernière n’est pas une capacité revendiquée ici.

## 3. Ce qui fonctionne déjà

### Ingérer, relier et chercher

Le système ingère des données, les découpe, crée leurs relations et calcule leurs représentations
vectorielles. La recherche combine le plein texte BM25, le dense et le creux, avec fusion des rangs,
filtres, extraits et surlignage. Les résultats exposent les contributions des signaux.

Pour le code, on dispose d’un graphe navigable : scopes, symboles, usages, impact et relations entre
fichiers. Une modification peut déclencher une réindexation. Les cas de résolution sont confrontés
à des oracles de langage ; les résultats douteux et les régressions font l’objet de témoins.

**Exemple concret :** retrouver une fonction, voir ses usages et les dépendances concernées,
ouvrir son contenu, modifier le fichier puis réindexer ce changement. La relation entre les
éléments apporte une information que la seule similarité entre morceaux de texte ne donne pas.

### Exécuter des traitements observables

Les graphes sont exécutables, avec ports typés, sous-graphes, itération par élément et rapports
nœud par nœud. Le runtime dispose de mécanismes de point de reprise et d’annulation ; leurs
frontières et celles du stockage sont vérifiées par des tests, notamment sous interruption.

Un appel d’outil laisse donc davantage qu’une phrase « j’ai fait quelque chose » : il produit
une exécution dont on peut examiner les entrées, les sorties, les étapes et les erreurs.
Les réactions sur le bus permettent aussi de déclencher du travail à partir d’événements.

### Faire travailler un agent dans un espace délimité

Un agent peut chercher, lire, modifier et utiliser les commandes disponibles selon les capacités
et la politique de son espace. L’équipier travaille par étapes sur une tâche, avec mini-plan,
boîte de dialogue avec le superviseur et journal des runs. Les questions et accords en attente
peuvent suspendre son travail jusqu’à une réponse.

Un juge relit les runs et produit des bilans sur la session et sur les outils, avec des défauts
identifiés et leurs preuves. Ces bilans alimentent la mémoire. Le juge reste lui-même un agent
à évaluer, pas une autorité qui transforme une opinion en vérité.

**Limite importante :** des cas de clôture prématurée sans livrable ont été observés.
Le mécanisme de travail existe ; la garantie qu’une tâche ne se déclare terminée qu’après
livraison complète reste un chantier actif.

### Utiliser un véritable atelier et des interfaces

L’hôte sert un IDE React : chat, fichiers, recherche, arbre, graphes, tâches et relecture.
Un client terminal suit l’agent et ses outils, permet de lui parler et de l’arrêter ; il fonctionne
aussi par SSH.

L’état de l’hôte et la fin des tours du chat sont désormais poussés par le flux d’événements
(`8d5139f9e`). L’usage du modèle est commun à l’API, à la barre et au récapitulatif ; après une
panne, les actions et résultats déjà obtenus restent visibles et le chat peut reprendre.

Le lot contextuel `68b9ede17` rend aussi la recherche et la navigation accessibles depuis le
chat de l’atelier, dans un dépôt explicitement choisi, ainsi que le suivi des commandes longues
par `wait_output`. Les résultats portent leur cible en métadonnées structurelles : les liens
ouvrent le bon dépôt après troncature ou rechargement de la page. Les parcours navigateur ont
éprouvé deux dépôts avec le même chemin et le même symbole, ainsi qu’une commande réellement
différée. Le runtime vérifie et exécute une même version des déclarations lors d’une invocation.

L’atelier sait notamment attacher un worktree, ouvrir une branche dans un worktree voisin,
lire un fichier ou un scope, modifier du texte ou des lignes et rendre le diff.
Les modifications d’un worktree attaché peuvent déclencher sa réindexation.
Les demandes d’accès ou de commande passent par des cartes d’accord selon le périmètre.

La modification de gabarits depuis le chat avec aperçu a déjà été démontrée, par exemple pour
faire avancer un compteur par cinq. Cela donne un premier exemple concret d’un atelier qui
agit sur les déclarations utilisées par son propre environnement.

**Limite actuelle :** les capacités ne sont pas encore toutes accessibles de manière uniforme
à chaque chat. Compléter et éprouver le parcours de code de bout en bout dans l’IDE est une
priorité explicite, pas une capacité que cette présentation considère comme entièrement finie.

### Garder une mémoire qui existe hors du contexte du modèle

Les fiches de mémoire vivent dans des fichiers Markdown, avec identité, état, sources et liens.
La base de mémoire peut être reconstruite depuis son dossier. Des outils permettent de rappeler
les fiches et de retrouver celles qui citent un fichier, un symbole ou une autre référence.

Le gabarit d’archivage décompose un document en sections et décide vers quelles fiches les ranger.
Ce traitement est exprimé par des graphes et des scripts du gabarit. Des hooks apportent déjà
la mémoire à des sessions d’agent externes, dans un périmètre d’essai.

La qualité dépend aussi du contenu accumulé : un rappel vide ne se résout pas uniquement en
changeant de modèle. L’archivage à plus grande échelle et la qualité du rappel restent travaillés.

## 4. Pourquoi la « boucle étrange » a un contenu technique

Le système possède des représentations de ses capacités : manifestes, graphes, schémas d’outils
et gabarits. Ces représentations peuvent être lues, modifiées, testées et versionnées.
L’atelier sait déjà intervenir sur certaines d’entre elles.

On commence donc à franchir la frontière entre « un framework qui exécute un agent » et
« un environnement dont l’agent peut faire évoluer une partie des capacités ».

Le cycle visé est : **exprimer un besoin → composer ou modifier une déclaration → l’exécuter →
observer le résultat → conserver la déclaration et les connaissances utiles**.

La généralisation complète de cette boucle reste une direction d’architecture. Il ne faut pas
lui attribuer aujourd’hui une autonomie générale, une auto-amélioration sans supervision ou la
capacité de créer n’importe quel produit sans nouvelles primitives.

Le pari est néanmoins concret : **plus les capacités sont composables et décrites dans le système,
plus une extension peut devenir une nouvelle composition plutôt qu’un nouveau sous-système.**

## 5. Le prochain cap : un index, plusieurs mondes de travail

Aujourd’hui, une tâche peut encore coûter une copie de base et une seconde instance de backend.
Lucie veut un index commun au dépôt et des vues par branche ou worktree, sous forme de couches.

- L’ancre contient la version de référence.
- Une couche porte les différences de sa branche ou de son worktree.
- Une couche peut hériter d’une autre couche : les branches empilées sont un cas normal.
- Les suppressions masquent les éléments hérités ; les recherches respectent la vue choisie.
- Un contenu identique peut partager ses morceaux et ses vecteurs, tout en gardant des occurrences,
  des positions et des relations distinctes.

**État au 12 novembre :** le socle du partage est intégré (`08086ba36`). Dans une cellule,
plusieurs occurrences peuvent réutiliser une version de contenu, ses morceaux et ses vecteurs.
La recherche publique restitue leurs identités, données et positions distinctes ; les masques
s’appliquent avant la sélection des meilleurs résultats. La déclaration est accessible depuis
un vrai manifeste et les parcours natif/PostgreSQL sont éprouvés : 225 tests dans quatorze suites,
avec 2 139 tests de bibliothèque et 39 tests navigateur réussis.

Cette capacité prépare les vues ; elle n’active pas encore le partage dans le gabarit code de
production. La migration déclarée sur copies est la suite prévue, puis le raccord aux branches
et worktrees. La pyramide complète sans copie d’index reste à livrer. Les transformations de
texte sans remappage de coordonnées sont refusées, et la rétention des versions orphelines
reste séparée du partage.
En parallèle, le chat de l’IDE doit permettre un parcours complet : explorer, chercher, lire,
modifier, exécuter les tests, examiner le diff, reprendre après une erreur ou un accord et livrer.
Ces deux chantiers se rejoignent : **un agent doit disposer des bons outils dans la bonne version
du monde, avec les bons droits et une trace de ce qu’il a réellement fait.**

## 6. Quelques preuves, sans promesse de laboratoire universel

- Le banc d’indexation du dépôt a documenté une médiane de **78 secondes au 8 octobre**, contre
  753 secondes sur la configuration de septembre, pour environ 7 000 fichiers. Chiffre situé :
  machine, corpus et configuration précis ; ce n’est pas une garantie sur tout dépôt.
- Le banc de classement de fiches a mesuré **76 % de bonnes premières et AUC 0,92** avec
  Qwen3-4B en GGUF Q8_0 et cache de préfixe, autour de **1,08 seconde par groupe de cinq fiches**.
  Il démontre une possibilité de décision locale ; son intégration généralisée reste distincte.
- Des tests navigateur font réellement lire et modifier un dépôt depuis l’IDE.
- Les traces exactes d’un défaut de relations du moteur ont été rejouées sans écart sous le
  correctif. D’autres défauts restent ouverts, dont une annulation sous pool mémoire plein.
- Lucivy **4.3.3 est publiée et intégrée dans rag3weaver** (`f96acc6cf`), avec arrêt des acteurs
  même si close() échoue. Le même témoin public échoue avec4.3.2 et passe avec4.3.3 : erreur
  conservée, nouvelles écritures refusées, sortie normale de l’enfant. Le format d’index reste
  inchangé. L’arrêt des backends et son résultat conservé sont également intégrés (`80b467dd3`).

La maturité est celle d’un système en développement, utilisé sur son propre code, avec des
capacités déjà démontrées et des garanties encore en consolidation. « Test ciblé vert »,
« passe complète verte », « sur master » et « posé chez Lucie » désignent quatre états différents.

## 7. La portée de la vision

Le code est le domaine principal actuel. La vision est celle de plusieurs ateliers partageant
le même socle : code, documents, catalogues, puis éventuellement dessin, scènes et géométrie.
Les decks de cartes ont déjà servi d’expérimentation ; DXF, Blender et la génération procédurale
restent des pistes, pas des produits livrés par le framework.

Le backend d’une application qui porte son propre atelier de modification fait également partie
du cap. L’idée est qu’un même modèle serve à construire l’application, à l’exploiter et à la faire
évoluer, avec des frontières de capacités et d’exposition explicites.

Pour comprendre Lucie, il faut garder cette échelle en tête : elle cherche une cohérence de
représentation qui rende davantage de comportements exprimables. Les détails actuels d’un agent
de code sont le terrain où cette cohérence doit faire ses preuves.

**La bonne question à poser au projet : quelles nouvelles capacités peut-on exprimer avec les
primitives présentes, et quel invariant manque quand une composition ne suffit plus ?**

## Repères pour vérifier ou approfondir

Les chemins ci-dessous sont relatifs au dépôt rag3db. Les pages anciennes décrivent parfois un
état historique ; l’état des lots se lit dans les rapports datés des sessions.

- `README.md` et `extension/rag3weaver/README.md` : moteur, runtime et capacités existantes.
- `docs/nouveautes.md` : étapes effectivement intégrées sur master.
- `extension/rag3weaver/visions/00-vision-generale.md` : ambition générale.
- `extension/rag3weaver/visions/2026-10-09-13h36-plusieurs-produits-un-seul-atelier.md` : domaines.
- `docs/arbre-principal/11-11-2026/03-contrats-et-lots-des-vues.md` : vues et couches.
- `docs/orchestration/11-11-2026/05-cap-produit-vues-et-chat.md` : priorités produit confirmées.
- `docs/banc/11-11-2026/03-le-juge-en-gguf.md` : protocole et mesures du juge local.
- `docs/orchestration/11-11-2026/01-rapport-de-session.md` : suivi des atterrissages et validations.
- `docs/memoire/11-11-2026/05-archiviste-automatique-du-chat.md` : conception du classement et
  du rappel automatiques aux frontières de tour, avec abstention et hiérarchie mémoire ;
  ce parcours M9 reste à réaliser.
