# Le proto « tout déclaratif » — parler à un backend, et le voir changer

10 octobre 2026, 22 h, orchestration, pour la session « everything
declarative » (chantier I) et pour Lucie. Page de conception : ce qu'on
bâtit, dans quel ordre, avec quels témoins. Rien de codé.

## 0. D'où ça vient

La vision générale (`../../../visions/00-vision-generale.md`) vise *tu parles
à ton code ou à ton site, et il change sous tes yeux* ; ses marches 2 et 3
sont le backend déclaré et les vues déclarées. Lucie, 10 octobre au soir :
ce qui l'intéresse le plus, ce sont les fiches mémoire, « le backend
déclaratif, purement altérable depuis lui-même », et les nœuds entièrement
scriptables ; sa question : « comment on va vite sur un proto du truc à la
Westworld ? ». Les choix sur le script sont déjà pris
(`../../../visions/2026-10-04-16h24-montrer-nos-produits.md`, §11) : le
moteur de script est **générique** (le langage est un point de branchement),
un nœud peut être déclaré **entièrement en script**, rhai est le moins bon
des choix pour des agents qui écrivent (lent, peu connu des modèles), et
TypeScript passe par un moteur JavaScript embarqué.

## 1. Le proto, en une phrase

Une seule page servie par rag3weaver : à gauche on parle à l'agent ; à droite
le graphe du backend dessiné en direct et la page que ce backend produit. On
dit « ajoute un champ », l'agent modifie une **déclaration** (un graphe, un
nœud scripté, un gabarit), le système recharge à chaud, le graphe et la page
changent sous les yeux. Rien d'autre.

Ce qu'on voit à la fin, et qui sert de recette : Lucie ouvre la page, tape
« ajoute une colonne "prix" à la liste », et en moins de dix secondes la
liste a sa colonne, le graphe montre le nœud modifié en surbrillance, et le
journal dit quel fichier a changé et pourquoi. Aucune compilation.

## 2. Ce qui existe et qu'on réutilise tel quel

| Pièce | Où | Ce qu'elle donne au proto |
|---|---|---|
| la boucle d'agent dans les graphes | `src/agent.rs`, `src/chat*.rs` | le chat, les outils tirés des schémas de nœuds, le bac à sable |
| le moteur de graphes | `src/dataflow/` (`graph.rs`, `runtime.rs`, `node.rs`, `port.rs`) | les graphes à ports typés, vérifiés, exécutés et enregistrés nœud par nœud |
| un sous-graphe comme nœud | `src/dataflow/graph_node.rs` | « un service est un petit graphe partagé » |
| le nœud rhai | `src/dataflow/rhai_node.rs` | le premier branchement du moteur de script (script en configuration, pas encore par les ports) |
| le réacteur sur le bus | `src/dataflow/reactor.rs`, `react_nodes.rs` | ce qui déclenche un graphe sur un événement, et ce qui poussera les changements à l'écran |
| les gabarits de rendu | minijinja (`Cargo.toml`), `render_nodes.rs` | du texte aujourd'hui, du HTML demain par les mêmes gabarits |
| les graphes en mermaid | `src/dataflow/mermaid.rs`, fichiers `.mmd` | le dessin du graphe, sans rien écrire : mermaid tourne dans le navigateur |
| tokio partout | depuis `execution-asynchrone-2` (10 oct.) | un serveur HTTP ne coûte rien |
| les manifestes et leur vérification | `src/manifest*.rs`, `templates/` | la forme de toute déclaration, et le filet qui la refuse en la nommant |

Ce qui n'existe pas : un moteur de script générique (rhai est câblé en
direct), la déclaration d'un nœud scripté sous un nom avec ses ports, le
rechargement à chaud (tout se charge au démarrage), un serveur HTTP et des
routes, un outil « déclarer », une page.

## 3. Les six lots, dans l'ordre

Chaque lot : page courte si besoin, **témoin rouge d'abord**, code, suites du
changement, rapport. Les lots 1 à 3 sont la pièce durable (marche 2) ; 4 à 6
sont ce qui la montre.

### Lot 1 — le moteur de script générique, et JavaScript en second branchement

- Une seule interface : préparer un script (le compiler, vérifier ses
  ports), l'appeler avec des entrées typées, recevoir des sorties typées ; un
  journal, une limite de temps et de mémoire par appel, et ce que le script
  a le droit d'appeler (rien par défaut : ni fichiers, ni réseau).
- Deux branchements : **rhai** (ce qui existe, déplacé derrière l'interface,
  ses tests inchangés) et **JavaScript** par un moteur embarqué léger et
  isolé (QuickJS par une crate Rust, à vérifier : taille, isolation, limites
  de temps). **TypeScript dès ce lot** (Lucie, 22 h 30 : son langage de
  cœur) : le langage déclaré est `typescript` ; un outil Rust retire les
  types à la volée au chargement (oxc ou swc en mode « retirer les types »
  seul, le plus petit qui suffit, à vérifier), puis le JavaScript obtenu va au
  moteur ; pas de vérification de types à la `tsc`, ce sont les ports du nœud
  qui vérifient entrées et sorties à la frontière ; `javascript` reste accepté
  (du TypeScript sans types).
- Témoins : le même nœud scripté en rhai et en JavaScript rend les mêmes
  sorties ; un script qui boucle est arrêté par sa limite ; un script qui
  tente un accès interdit est refusé en le nommant.
- Témoin de plus : un nœud en TypeScript annoté rend les mêmes sorties que le
  même nœud en JavaScript.
- Décision de Lucie (10 oct.) : TypeScript dès le proto, parce que c'est son
  langage et ce que les modèles écrivent le mieux.

### Lot 2 — le nœud entièrement scripté

- Une déclaration : un nom, des ports d'entrée et de sortie avec leur
  schéma, une configuration, un langage, le corps du script. Une seule
  fonction à écrire : elle reçoit les entrées et la configuration, elle rend
  les sorties.
- Vérifiée comme tout nœud (ports, schémas) avant d'entrer dans un graphe ;
  **réutilisable sous son nom** dans n'importe quel graphe, comme un nœud
  fourni ; ses entrées viennent des ports, pas seulement de la configuration
  (le défaut relevé par la session mémoire sur `RhaiNode` et
  `EntityBatchNode`).
- Ce qu'un script peut appeler, par ce qu'on lui donne : requêter (par le
  dialecte), journaliser, nommer un secret (jamais le lire). Pour le proto :
  requêter et journaliser.
- Témoins : un nœud déclaré en script, posé dans un graphe, exécuté,
  enregistré nœud par nœud comme les autres ; un port manquant refusé à la
  déclaration ; le même nœud appelé depuis deux graphes.

### Lot 3 — le rechargement à chaud

- Un fichier déclaré change (manifeste, graphe `.mmd`, nœud scripté,
  gabarit) : le système le revérifie comme au démarrage et remplace l'ancien
  **d'un coup** ; s'il est invalide, l'ancien reste en service et l'erreur
  dit quoi corriger ; une exécution en cours finit sur l'ancienne version.
- La montre sur les fichiers passe par le réacteur (c'est un événement comme
  un autre) ; le rechargement émet sur le bus « rechargé : tel fichier, telle
  version », ce que l'écran écoutera.
- Témoins : modifier un gabarit → la sortie change sans redémarrer ; poser un
  graphe invalide → l'ancien répond toujours et le refus est lisible ; une
  exécution lancée avant le rechargement rend le résultat de l'ancienne
  version.
- Limite dite : un changement de **schéma** (une entité qui change de forme)
  n'est pas un rechargement, c'est une migration (vision §12) — hors proto.

### Lot 4 — un contrôleur est un graphe, une route y mène

- Une table déclarée : une route (`GET /produits`) → un graphe ; la requête
  (chemin, paramètres, corps) entre par les ports du graphe ; **la fin du
  graphe nomme une vue** : un gabarit HTML qui reçoit le résultat typé. Le
  contrat de données sort des ports, il ne s'écrit pas deux fois.
- Un serveur HTTP minimal (tokio ; axum ou hyper, le plus petit qui suffit),
  une sous-commande du backend (`serve`), à côté de `mcp`.
- Trois façons d'entrer dans un même graphe, déjà vraies pour deux : l'outil
  (l'agent), l'événement (le réacteur), et maintenant la route. Pas
  d'authentification dans le proto (local, Lucie seule) ; la garde fournie
  viendra avec la vision §9.
- Témoins : une route déclarée rend une page ; une route vers un graphe
  inconnu est refusée au chargement, pas à l'appel ; le résultat du graphe
  et la page rendue sortent du même enregistrement d'exécution.

### Lot 5 — l'agent écrit des déclarations

- Un outil `declare` : poser ou modifier un manifeste, un graphe, un nœud
  scripté, un gabarit — **et rien d'autre** (jamais un fichier Rust, jamais
  hors du dossier des déclarations) ; vérification avant écriture (le filet
  des manifestes), rechargement après, et le refus revient au modèle avec sa
  raison et la ligne fautive.
- Chaque écriture est un **commit** avec sa raison (le message de l'agent) :
  c'est la première fiche de contexte, et c'est ce qui permet de revenir en
  arrière d'un geste.
- Le backend du proto est **un jouet** (décision à confirmer par Lucie,
  recommandée) : trois entités (produit, catégorie, commande), une liste, une
  fiche, un formulaire ; pour que le Westworld se voie sur quelque chose qui
  change vite, sans dépendre du backend de code.
- Témoins : « ajoute un champ » → la déclaration change, le commit porte la
  raison, la page le montre ; une déclaration invalide proposée par l'agent
  est refusée et l'agent corrige au tour suivant (témoin avec un modèle
  enregistré, pas un vrai appel).

### Lot 6 — la page vivante

- Une page : le chat à gauche ; à droite le graphe du backend (mermaid tiré
  des `.mmd`, le nœud touché en surbrillance), la page produite par le
  contrôleur courant, et le journal des rechargements. Le bus est poussé au
  navigateur (événements serveur) : tout se redessine sans recharger.
- Une donnée, deux rendus : ce que l'agent lit en texte et ce que la page
  dessine sortent du même résultat ; aucun lien inventé par le modèle.
- Pas de React, pas d'empaquetage : du HTML servi par les gabarits, mermaid
  et quelques lignes de JavaScript pour écouter le bus. Ce n'est pas un cadre
  web ; c'est la vitrine de la pièce posée.
- Témoin : la recette du §1, jouée par Lucie.

## 4. Ce qui est décidé, ce qui ne l'est pas

Décidé par Lucie : le moteur de script générique ; le nœud 100 % scripté ;
TypeScript dès le proto (JavaScript exécuté) ; les produits sont des paquets de déclarations
(« des plugins ») que l'agent propose de télécharger — hors proto, mais
chaque déclaration du proto doit pouvoir voyager (un dossier, pas des
chemins en dur) ; une session dédiée, sur luciepc.

À confirmer par Lucie : le backend jouet plutôt que le backend de code pour
le proto (recommandé) ; QuickJS comme moteur JavaScript (après vérification
par la session).

Hors proto, nommé pour ne pas y tomber : les fiches de contexte (marche 1,
sujet entier ; le proto n'en pose que le commit avec sa raison) ; les secrets
et intégrations (§9) ; les migrations en verbes (§12) ; l'authentification ;
React ; WebAssembly ; le navigateur comme moteur de graphes.

## 5. Les fichiers, et la coordination

- Neuf : `src/script/` (l'interface et ses deux branchements), `src/http/`
  (serveur, routes), `src/dataflow/scripted_node.rs`, `src/reload.rs` (ou
  dans le réacteur), `templates/proto/` (le jouet : manifeste, graphes,
  gabarits, nœuds scriptés), `tests/e2e_proto_*.rs`.
- Touchés : `src/dataflow/rhai_node.rs` (déplacé derrière l'interface),
  `node_registry.rs` / `node_factories.rs` (le nœud déclaré sous un nom),
  `reactor.rs` (la montre sur les fichiers), `tools.rs` (l'outil `declare`),
  le binaire (`serve`).
- La session **recherche** (`rag3db-97`) possède `src/dataflow/` et vient de
  livrer l'exécution asynchrone : elle brieffe la session du proto en un
  message (runtime, ports, enregistrement des exécutions, réacteur, RunScope)
  et relit ce qui touche `reactor.rs` et `runtime.rs`. La session
  **mémoire** (`rag3db-96`) a écrit la sous-commande `mcp` : même forme pour
  `serve`. La session **embarquements** (`rag3db-6f`) possède l'IR : un
  script qui requête passe par le dialecte, jamais par du Cypher.
- Branche `proto-declaratif`, un worktree sur luciepc, un target unique,
  lots fusionnés en avance rapide un par un (les lots 1 à 3 sont utiles seuls).

## 6. Le coût, en passes

Six lots ; aucun plus gros que l'exécution asynchrone de cette semaine. Pour
une session seule, de l'ordre d'une semaine de passes d'agent (« un jour »
d'agent ≈ 1 h 30 réelle), avec deux ou trois décisions de Lucie en chemin.
