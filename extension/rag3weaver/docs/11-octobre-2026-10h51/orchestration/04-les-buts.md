# Les buts, clarifiés — 11 octobre 2026, 21 h

Demandé par Lucie avant de dormir : « les objectifs plus long terme, l'interface
à la Westworld, le projet qui s'édite depuis lui-même sans recharger, tout en
gabarit à 100 %, les langages de script interfacés comme on veut : ça sera la
suite ? ». Oui, c'est la suite, et une partie est déjà là. Cette page remet en
ordre ce qu'on vise, à trois horizons, avec les documents qui le disent et
l'état réel au soir du 11 octobre.

## 1. Le cap, en une phrase

« Tu parles à ton code ou à ton site, et il change sous tes yeux » (vision
générale §0). Tout le reste en découle : le moteur qui tient les écritures,
les dialectes pour parler à n'importe quelle base, l'IR structure, les agents
qui sont des graphes, le démon qu'on installe, l'IDE pour agents.

## 2. Les documents qui portent ces buts

| But | Où il est écrit | État |
|---|---|---|
| L'interface « à la Westworld » : on parle, la chose change sous les yeux, le journal dit quel fichier a changé et pourquoi | `10-octobre-2026-18h20/orchestration/03-le-proto-tout-declaratif.md` (la conception, six lots) ; `visions/2026-10-09-13h36-plusieurs-produits-un-seul-atelier.md` §4 « ce que comme dans les films veut dire ici » ; `visions/00-vision-generale.md` §0 | **le proto existe** : page vivante `/_atelier`, servie par `serve`, recette en trois commandes (`10-octobre-2026-18h20/everything-declarative/08-lot-6-la-page-vivante.md` §1) ; pas encore joué par Lucie |
| Le projet qui s'édite depuis lui-même, sans recharger | lot 3 du proto, `everything-declarative/05-lot-3-le-rechargement-a-chaud.md` ; lot 5 `07-lot-5-l-outil-declare.md` (`declare`, commit par chemins) ; vision des produits §7 « déclaré n'est pas exposé » | **fait pour les déclarations** (graphes, nœuds scriptés, gabarits, vues, réactions rechargés à chaud, l'atelier qui relit) ; ce qui reste : l'agent qui édite depuis la page elle-même sur un vrai produit |
| Tout en gabarit à 100 % | vision générale §2 « les étages » ; la stèle du produit (`11-octobre-2026-arbre-principal/02-la-stele-du-produit.md`) ; `genericite-des-schemas` (une organisation nouvelle se décrit dans la configuration, jamais en dur) | **très avancé** : entités, relations, découpe, réactions, montres, outils, agents (`AgentNode` et ses ports) sont des déclarations ; ce qui reste en dur : la liste des relations du code (en train de passer par le catalogue), quelques chemins des gabarits livrés (B4 de G), la cellule des montres |
| Les langages de script interfacés comme on veut | `visions/2026-10-04-16h24-montrer-nos-produits.md` §11 ; lot 1 du proto `03-lot-1-le-moteur-de-script.md` (le langage est un point de branchement : TypeScript/JS par moteur embarqué, rhai) ; lot 2 `04-lot-2-le-noeud-scripte.md` (un nœud déclaré entièrement en script) | **fait** : moteur de script générique, nœud scripté en TypeScript (le langage de cœur de Lucie), rechargé à chaud ; ce qui reste : d'autres langages derrière le même point de branchement si un gabarit le demande, et la mesure du coût d'un nœud scripté dans un flux |
| Les agents qui sont des graphes, qui se parlent, qu'on lance en fond | `11-octobre-2026-everything-declarative/03-les-agents-se-parlent.md` ; vision générale « L'agent est un graphe à ports, et les ports portent des flux » ; `vision_roadmap_09_2026/07` et `09` | **lots 1-3 sur master** (les sept ports de la boucle, Decide et Route, send et l'annuaire) ; restent AgentNode avec parent, spawn, la boîte de session dans l'atelier et le MCP, l'expérience à vrai modèle ; l'exécuteur de flux est là (lot 0 de la recherche clos) |
| Les fiches mémoire classées, le sujet préféré de Lucie | `11-octobre-2026-memoire-longue/04-les-fiches-classees-par-sortie-structuree.md` ; visions du 4 octobre (mémoires par gabarit, fiches de contexte) | **en expérimentation** (régime de Lucie : tentatives datées avec gabarit, note, coût, retours) ; deux tentatives faites, la troisième avec Gemini quand Decide a son plafond |
| Un IDE pour agents, « comme des malvoyants qui ont d'autres sens » | vision générale « Un IDE adapté aux agents » ; `10-octobre-2026-18h20/codeparsers/04-ce-qui-manque-a-un-agent-dans-cet-ide.md` (la page des manques, tenue par les sessions) | **en service** : le cœur et hotfixs travaillent avec le MCP sur rag3db (ici et sur luciepc) ; la page des manques décide des outils suivants ; **la priorité de Lucie** : chaque session sur son propre worktree, à jour quand elle édite |
| Le démon : « installer rag3weaver = installer un démon » | `11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md` (forme C, §9 les treize choix) | **marches 0 à 5 faites** : hôte par poste, sessions, client mince, découverte et lancement par le premier client, unité système en option, hôte distant par tunnel ; reste la portabilité (Windows, macOS : lot C de G) |
| Le moteur sûr sous écritures parallèles | la stèle du moteur (`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`), page 11 du cœur | **condition 3 : voie A complète, B2 faite** ; le mode multi-écrivains est le défaut depuis le 11 octobre ; B3 côté produit (l'hôte) en grande partie là ; les tickets font foi |
| Plusieurs produits, un seul atelier : decks, code, DXF, Blender, tisserande | `visions/2026-10-09-13h36-plusieurs-produits-un-seul-atelier.md` ; le magicien du chaos §6 | **à venir** : le code est le premier domaine réel ; le constructeur de decks « on l'oublie pour l'instant » ; la tisserande « très plus tard » |
| Le pont vers les autres agents, le distribué, le multimodal | vision générale §8, trois graines du 11 octobre | **graines**, pas de chantier |

## 3. Les buts à court terme (cette semaine), dans l'ordre

1. **Le worktree de chaque session indexé et à jour quand elle édite** (la priorité
   de Lucie du 11 octobre) : réconciliation de la montre (proto), identités
   relatives (G, B4), routes de l'hôte (faites), commande `code-tree add|remove`
   et manifeste stable `code-coeur` (codeparsers). Résultat visé : un seul
   serveur MCP par session, jamais recréé, qui suit son worktree.
2. **La fraîcheur sous la seconde** : les 6 s entre une édition et sa visibilité
   (mesure par phase du proto, le rendez-vous par les symboles du fichier et
   le délai d'embarquement chez l'arbre principal) ; et `usages` à quelques
   dizaines de millisecondes (le planificateur du moteur : les fils et le
   balayage sous masque, chez le cœur).
3. **Les manques de la page 04** fermés dans l'ordre que la page donne (le nom
   qualifié, les tests par requêtes non suivis, la portée, les verrous du
   chemin, l'envoi virtuel déjà fait).
4. **Master vert tout le temps** : le tour de push, les cinq jeux de features,
   le recompte des tickets, les règles de forme ; le magasin de libs par
   identifiant de bâti pour que la lib partagée ne couple plus personne.
5. **Le paquet npm prêt** (archives alpha.3 bâties, portables Windows et macOS),
   publié le jour où Lucie donne ses OTP.

## 4. Les buts à moyen terme (les prochaines semaines)

- **Le proto Westworld joué par Lucie** sur un vrai produit : la page vivante,
  un agent qui édite les déclarations depuis la page, le journal qui dit ce
  qui a changé ; puis les agents qui se parlent au complet (AgentNode, spawn,
  la boîte dans l'atelier), les premiers vrais flux (la sortie du modèle en
  jetons), le retrait par ensemble.
- **Les fiches mémoire classées** : tentatives datées jusqu'à ce qu'une forme
  tienne ; l'archiviste branché sur Claude Code par les channels MCP et les
  hooks (vision du 11 octobre, vérifiée sur la documentation).
- **Le démon partout** : portable (Windows, macOS), une configuration à lui,
  l'unité en option, le client qui survit à son redémarrage, un hôte par
  poste et un client qui sait lequel.
- **Le moteur** : I2 (la maintenance de l'index au commit, « un luxe plus
  tard »), les petites cardinalités au planificateur, l'index plein texte
  tenu par le point de reprise (comme Neo4j), les tickets moteur restants.
- **PostgreSQL comme second dialecte de plein exercice** : le gabarit code s'y
  indexe depuis ce soir ; les réactions et la mémoire doivent y tourner
  aussi ; le fanout quand un vrai déploiement distant le mesure.

### 4 bis. Deux questions ouvertes du 13 octobre (Lucie)

- **rag3weaver comme base d'une application à beaucoup de développeurs**, qui
  écrivent et migrent en même temps (« ajouter une colonne comme ils le
  feraient sur PostgreSQL, sur la même base »). L'architecture tient (un seul
  processus possède les fichiers, l'hôte, et tous s'y connectent, comme le
  postmaster), mais quatre écarts avec PostgreSQL sont à mesurer avant de
  raffiner le mode multi-écrivains : le DDL transactionnel (un CREATE INDEX
  refuse la transaction explicite ; l'ADD COLUMN n'est pas mesuré), le coût
  d'un ADD COLUMN sous le verrou du catalogue (chez PostgreSQL une écriture
  de catalogue seule), vingt écrivains simultanés dans l'hôte avec le point
  de reprise qui fait la queue, et les migrations comme objet (table des
  migrations, ordre, rejeu, refus). Scénario à poser au banc : vingt clients,
  petites transactions, un DDL par minute, un point de reprise en route ;
  décision sur les chiffres. Pas avant la reprise.
- **Le multi-vecteur et MaxSim** (ColBERT, bge-m3 le rend déjà) : une option
  plus tard, d'abord comme rerank sur les k premiers candidats, jamais une
  colonne de listes de vecteurs par défaut (trop lourd).

## 5. Les buts à long terme (les visions)

- **Plusieurs produits, un seul atelier** : le même atelier Westworld avec un
  domaine différent à chaque fois ; le magicien du chaos qui monte un produit
  et « te donne le lien de l'agent dédié » ; la tisserande (génération
  procédurale 3D, « mini Houdini à la ComfyUI »).
- **Le distribué comme réseau de fourmis** : un hôte par poste qui s'abonne aux
  événements d'un autre, la réaction vue en réseau, le backend déclaratif
  dont la déclaration se propage ; d'abord nos deux postes.
- **S'intégrer à l'usage** : le même serveur pour tous les agents de code, nos
  agents appelant Claude Code ou Codex comme un outil, l'adoption par l'usage
  existant plutôt que par le changement d'outil.
- **Le multimodal local** : images, audio dans le même espace de vecteurs que le
  texte, derrière le même service d'embarquement.
- **Le playground WASM** : « luxe ultime de marketing », anecdotique.

## 6. Ce qui ne change pas

Le moteur reste le moteur ; rag3weaver parle en dialectes ; l'IR est une
structure ; tout est déclaré ; chaque défaut connu est un ticket ; chaque
manque est dit par l'outil lui-même ; rien ne va aux amonts ; rien ne sort
sans le mot de Lucie ; la complexité n'est jamais un argument, la parité
prouvée en est la condition.
