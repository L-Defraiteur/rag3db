# Réconciliation des objectifs — où on allait, où on est allé, quoi faire maintenant

**1er octobre 2026.** Demandé par Lucie au retour de l'expérience MTG : « toi tu
connais le passé et moi j'ai pu oublier plein de visions qu'on avait ». Ce doc
remet côte à côte ce qu'on voulait (les seize docs de vision, du 25 août au
18 septembre), ce qui existe dans le code aujourd'hui, ce qui est nouveau, ce
qui est en suspens, et propose un ordre.

**Comment il a été fait.** Les docs de vision ont été relus en entier, et
chaque élément annoncé a été cherché dans le code de `mtg-experiments`
(`3886a458e`) : « fait », « partiel » ou « absent » ne sont pas des souvenirs.
Les détails de fichier et de fonction sont dans les deux relevés en annexe
(§9). Le compagnon de ce doc est le
[knowledge dump](02-knowledge-dump.md) ; le registre vivant est
[`docs/journal-des-chantiers.md`](../../../../docs/journal-des-chantiers.md).

## 1. En une page

**Où on allait.** Un agent qui vit dans une base, la gère, et construit des
backends avec la technologie dont il est fait. L'ordre posé : avaler le réel
(code, documents, tableurs), puis seulement la surface. Ta « meilleure cible
immédiate » du 29 août : un backend debout, produit par un agent à partir de
gabarits, avec un front.

**Où on est allé.** Du 6 au 18 septembre : la mécanique (ingestion en masse,
plusieurs modèles par index, KB repliées en entités dérivées, un seul chemin
de recherche, `Lifecycle`). Du 19 au 27 septembre : l'expérience MTG, qui a
construit **la surface qu'on avait repoussée** — backend déclaratif, harnais
de validation, pont MCP, chat — sur un vrai cas d'usage.

**Ce que ça change.** L'expérience MTG n'est pas un détour : elle a livré le
« backend = un ensemble de graphes plus une surface » de la vision, et le
processus hôte qui manquait le 18 septembre. Mais ce backend a été monté par
des scripts et par Codex, pas par un agent à partir de gabarits ; et les
entrées du réel (documents, tableurs, « indexer ce dépôt ») n'ont pas bougé.

**Ce que je propose** (détail au §8) :

1. **Mettre le travail à l'abri** : extraire les correctifs C++ et fusionner
   `mtg-experiments` dans `master`.
2. **La fiabilité qui bloque tout produit** : le WAL illisible après un arrêt
   brutal, la persistance des structures imbriquées.
3. **Fermer ce qui est ouvert côté recherche** : le pas C, une fois tes
   décisions prises.
4. **Brancher les deux moitiés** : l'agent de code servi par le backend
   déclaratif, avec l'entrée « indexer ce dépôt ».
5. **Ta cible du 29 août** : un backend produit par un agent à partir de
   gabarits, en mesurant ce qu'il a dû écrire lui-même.

## 2. La vision, dans tes mots

> Un chaos contrôlé, un truc qui peut servir à tout. C'est un agent de code,
> mais il peut gérer sa base de données dans laquelle il vit, il peut aussi
> construire des backends se servant de la même techno… il fait tout et
> n'importe quoi, un gloubiboulga du futur. — 25 août

Ce qui la tient debout : **tout est un graphe, et un graphe est une donnée**.
Un pipeline, un outil, un agent, un backend sont des graphes ; leur catalogue
est fait d'entités dans la base ; l'agent cherche ses capacités comme il
cherche un document.

Tes décisions qui en découlent, à ne pas reperdre :

- **Aucun Cypher pour les agents.** « Si une capacité manque par rapport à
  Cypher, on l'ajoute à l'abstraction — on ne régresse pas vers du Cypher qui
  nous enchaînerait à cette base pour toujours. »
- **« Une boucle étrange qui ne sert que kuzu ne sert pas les projets
  réels. »** D'où PostgreSQL, et Neo4j en dernier.
- **Un agent est un outil.** Et deux espaces de tags, dont un temporel.
- **Des catalogues de gabarits** : « un backend, il faut justement pas qu'ils
  aient à tout coder, donc on leur prépare d'avance des `user`,
  `conversation`, `product`… Pareil pour le front : on leur pré-fait des
  templates de composants React. » Et : « je pense que c'est le meilleur but
  car ça touche tout d'un coup, notre meilleure cible immédiate. »
- **Des agents qui se parlent** : « c'est l'agent qui touche réellement à
  l'écriture du code qui est en auto ; l'agent de contexte ne fait que
  discuter avec l'agent de code ; l'agent de design sert d'user. » Plus un
  agent vision qui « perd jamais la vision initiale utilisateur ».
- **La parole** : reportée, pas oubliée — elle se fera avec l'interface du
  produit agent de code.
- **Wasm abandonné pour rag3weaver** (26 août). lucivy garde le sien.
- **Généricité** : une organisation nouvelle se décrit dans la config, jamais
  en dur pour un cas.

## 3. Les ordres de travail successifs

| Date | Ce qui a été posé |
|---|---|
| 25 août | « On ne livre pas, il manque de quoi avaler le réel. » Ordre : codeparsers, `grep` / `read`, lecture des documents, normalisation des tableurs, rapport de validation, **puis seulement** la surface (MCP), le modèle local, la parole. |
| 29 août | Les catalogues de gabarits, « notre meilleure cible immédiate ». |
| 30 août | Les rôles (vision, design, contexte, code), les commandes données à un agent, l'ingesteur de base étrangère, le schéma comme artefact. |
| 3-5 sept. | Le moteur devient multi-backend. Reste nommé : lecture des documents, tableurs, le monolithe de recherche. |
| 7 sept. | Ton ordre : replier les KB en entités dérivées → un index à plusieurs embarquements → 278m par défaut et l'heuristique de taille → **l'entrée produit « indexer ce dépôt »** → les mesures HNSW. |
| 18 sept. | Les quatre produits et leur empaquetage. « Pour avancer on n'a pas besoin de penser à tout ça tout de suite. » |
| 19-27 sept. | L'expérience MTG : deck builder, backend déclaratif, harnais, chat. |

## 4. Ce qu'on voulait, contre ce qui existe

### Fait

| Voulu | Où |
|---|---|
| codeparsers branché, l'agent lit son propre code | `code.rs`, `e2e_code` |
| Les outils de l'agent : `grep`, `read`, `edit`, `list`, `search`, `schema`, `run`, `run_bg`, `wait`, `place`, `adopt` | `templates/tools/`, `code_tools.rs` |
| Donner des commandes sans donner la machine : argv, trois modes, verdict structuré, liste blanche de session | `commande.rs` |
| Deux backends entiers (rag3db, PostgreSQL), les cinq organes | `dialect.rs`, `postgres_*` |
| « Une absence se nomme », « une recherche dit quand rien n'est probant » | `search.rs::marquer_la_confiance`, `acces.rs` |
| Un seul chemin de recherche, composable ; le monolithe retiré | `Catalog::rechercher`, 18 sept. |
| Bus d'événements, identité des runs, réacteur, boîtes de réception, trace cherchable | `events.rs`, `reactor.rs`, `trace_nodes.rs` |
| Catalogue de gabarits : chercher, poser, adopter ; gabarits `user`, `conversation`, `product` ; motif « versionné » | `template.rs`, `templates/entities/` |
| KB repliées en entités dérivées (pas A et B) | `derived_kb.rs`, 18 sept. |
| Un index à plusieurs embarquements ; 278m par défaut | `embedding_storage.rs` |
| Plusieurs processus lecteurs | `read_only_patient`, marque d'eau |
| La surface : backend déclaratif, pont MCP, chat | `backend.rs`, `scripts/serve_backend_mcp.py`, `chat.rs` (MTG) |

### Partiel

| Voulu | Ce qui existe | Ce qui manque |
|---|---|---|
| L'heuristique du premier index | écrite et testée (`embedding_choice`) | **aucun appelant** : elle attend l'entrée produit |
| Le pas C du repli | la fusion déclarée par entité prime sur le gabarit | la pondération par genre, les gabarits de dérivées |
| Catalogue d'outils comme graphe | `GraphTool` en mémoire, fiches `Template`, `Trace` | `GraphVersion`, `Invocation`, la **promotion sur preuve** |
| « Aucun Cypher pour les agents » | tenu pour les graphes composés par un modèle dans un backend | l'agent de code tourne encore sans cette frontière |
| Le rapport de validation à l'ingestion | le harnais (`ValidationReport`) valide une **soumission** | rien ne valide une **ingestion** de données |
| Synchronisation par identifiant | inchangés comptés ; disparus supprimés pour le code seulement | rien de générique (MTG ne fait que des upserts) |
| Le poids par champ en plein texte | le gabarit `weighted_search.mmd` | il n'est branché nulle part, aucune mesure du gain |
| Mémoire de l'agent bornée | `Absorb::Stale`, `Session::recall` | la coupe « aux deux dernières manches de design » |
| Les quatre produits | agent, démon, backend, chat, crate | pas de MCP natif, pas d'API multi-locataire, pas de roue ni de paquet npm |

### Absent

Classé par ce que les docs présentaient comme important.

1. **L'entrée par tableur** : le graphe de normalisation xlsx, « la porte
   d'entrée de toute la moitié catalogue ». Rien.
2. **La lecture des documents** : pdf, docx, pptx, html. Rien. (L'OCR prend
   une image ; il ne lit pas un PDF.)
3. **L'entrée produit « indexer ce dépôt »**, et le consentement au
   téléchargement des poids.
4. **Les rôles d'agents** : vision, design, contexte, code ; leurs
   tempéraments ; `agentCanAnswer` ; le registre de la distance ; la vision
   initiale gardée mot pour mot.
5. **L'ontologie de tags** : le tag comme concept, le reranker qui la
   maintient, les deux espaces dont un temporel.
6. **« Un agent est un outil »** : rien n'enveloppe un agent comme un outil.
7. **Les composants React** et la **WorkPalette** du catalogue de gabarits ;
   la mesure de « ce que l'agent a dû écrire lui-même ».
8. **L'ingesteur de base étrangère** : lire un schéma, déduire les tables de
   liaison, proposer un graphe avec sa provenance.
9. **Le schéma comme artefact** : migrations rejouables du chemin induit,
   écart entre config et base, fonctions RPC et politiques RLS (Supabase).
10. **L'inférence zéro configuration** et la détection de l'identifiant unique.
11. **L'empaquetage** : roues, npm, binaire, liaison statique des extensions.
12. **La parole.** Reportée par décision.
13. Petits outils demandés par le modèle lui-même : supprimer et déplacer un
    fichier, inspecter un gabarit avant de le poser.
14. Plusieurs processus écrivains.

## 5. Ce qui est nouveau depuis le 18 septembre

Ce que l'expérience MTG a ajouté, et que les visions ne prévoyaient pas sous
cette forme :

- **Le backend déclaratif** : un manifeste JSON, des outils qui sont des
  graphes, un binaire hôte en JSONL. C'est la première réalisation de « un
  backend = un ensemble de graphes plus une surface ».
- **Le harnais** : `before` / `after` / `on_accept`, des règles Rhai, un
  contrat de résultat. Leçon mesurée : sans lui, aucun modèle local n'a
  produit un deck conforme ; avec lui, le modèle corrige et resoumet.
- **Les verbes de recherche** : `search`, `select`, `follow`, `within`, `fuse`,
  compilés vers le même DAG.
- **Le chat** web et TUI, avec réflexion visible et journal des conversations.
- **Des payloads typés imbriqués** (listes, structs) et des filtres lisibles
  par un agent, avec schéma et vocabulaire générés.
- **Des extraits aux vraies lignes du fichier** (`sourceLines`, replis).
- **Une revue d'architecture externe** et un design non implémenté : des
  accès déclarés et une optimisation adaptative des index
  (`docs/19-09-2026/06` et `07`). Sa thèse : une recherche définie
  sémantiquement, plusieurs réalisations physiques.
- **Une nouvelle machine**, des modèles locaux de 120 milliards de paramètres.

*Ajouts de la session Products Experiments (27 septembre) :*

- **Une surface pensée pour un agent qui n'est pas Claude** : le refus du
  harnais arrive comme une erreur de requête MCP ; la livraison acceptée
  devient le texte même de la réponse de l'outil (le texte d'import Arena,
  pas un JSON à recopier) ; les sorties sont élaguées sans perte (anglais
  seul, champs vides omis, `contentKind: record`). Ces changements sont dans
  le moteur, génériques ; aucun n'est écrit pour gpt-oss.
- **Les conversations en base, au fil de l'eau**, avec l'heure de chaque
  événement : c'est la première trace d'agent que le moteur stocke lui-même,
  et un début pour « l'agent qui se relit » des visions.
- **Des faits extraits pour raisonner** : les capacités d'une carte décomposées
  en coûts, déclencheurs, effets et restrictions, déclarés comme vocabulaire
  dans le schéma. C'est un cas concret de « structurer le texte pour que
  l'agent filtre au lieu de lire », réutilisable hors de Magic.
- **Le stockage des index lucivy borné** (`d1aa7d296`), qui a révélé qu'une
  ligne supprimée n'est jamais récupérée par rag3db.

Ce que ça a révélé du moteur : quatre défauts du cœur C++ (lambdas,
quantificateurs), un `COPY` qui coûte la taille de la table, des blobs
d'index qui grossissaient sans fin, un WAL illisible après un arrêt brutal.

## 6. Ce qui est en suspens

**Décisions qui t'attendent** (le détail est au journal, §4) :

1. La forme du pas C.
2. Les poids de fusion par défaut : 0,6 / 0,4 ou 0,3 / 0,7.
3. Le texte embarqué : le nom avec le corps.
4. Supprimer `fuse_results` à trois listes.
5. Supprimer la grappe d'exploration après recherche.
6. Supprimer `search_with_strategy`.
7. Le budget de reprise du lecteur (250 ms contre un pic à 567 ms).
8. Fusionner `mtg-experiments` dans `master`.
9. Les sept PR amont burn / cubek, jamais envoyées.
10. Le design des accès déclaratifs : l'adopter, le réduire, ou le laisser.

**Fils laissés ouverts** :

- Le rapport de bugs à `tracel-ai` (16 entrées prêtes).
- Le HNSW sur codes quantifiés (note du 7 septembre, à la racine).
- Les autres moteurs de plein texte : Elasticsearch, Tantivy.
- L'évaluation : on mesure enfin la recherche de code (les bancs du
  18 septembre), pas encore celle d'un catalogue.
- La question des droits pour un produit MTG (GPL de `mtga-reader`, Wizards).
- « Demander son avis au modèle » : à refaire maintenant que les outils ont changé.

## 7. Les tensions à regarder en face

1. **Deux produits tirent dans deux directions.** L'agent de code (le produit 1
   du 18 septembre, « il tire tout le reste ») et le deck builder. Le second
   a fait avancer la surface ; le premier attend toujours son entrée.
2. **Le backend MTG n'est pas né de la boucle.** Il a été écrit par des
   scripts Python qui régénèrent `backend.json`. Ta cible était qu'un agent
   le compose à partir de gabarits. Les briques existent des deux côtés
   (gabarits, `place` / `adopt` ; manifestes, harnais) et ne se touchent pas.
3. **Le vocabulaire MTG a deux chemins de données.** La synchronisation ne
   fait que des upserts, alors que le moteur sait dériver, rattraper et
   supprimer. Le produit contourne des mécanismes qu'on a construits.
4. **La fiabilité.** Un produit qu'on met entre des mains ne peut pas perdre
   sa base sur un arrêt brutal. C'est devenu le premier risque.
5. **Le chemin de masse s'est dégradé hors de son cas.** Rapide sur une table
   vide, plus lent que l'ancien chemin dès que la table grandit.

*Note de la session Products Experiments sur 2 et 3.* Le constat est juste ;
deux nuances. (2) Les scripts Python ne font que **déclarer** : ils écrivent
schémas, vocabulaires et entrées de `backend.json`, puis tout passe par le
moteur (graphes, harnais, nœuds d'écriture). C'est donc un manifeste écrit à
la main par procuration, pas un contournement du moteur — mais oui, rien ne
vient des gabarits ni de `place` / `adopt`. (3) La synchronisation passe par
`EntityBatchNode`, que Codex a conçu exprès pour les snapshots externes
(identités explicites, lot validé avant écriture) : c'est un chemin du moteur,
pas un détour. Ce qui manque est réel : la suppression des cartes absentes
d'un nouveau snapshot, et le fait que ces entités n'utilisent ni dérivées ni
`Lifecycle`. J'ajouterais une sixième tension :

6. **Une partie du travail produit se décide face à un modèle faible.** Lucie
   a posé la règle le 27 septembre : pas de rustine pour la médiocrité d'un
   modèle, seulement des changements qui servent en général. Le harnais, les
   schémas générés et les sorties élaguées passent ce filtre ; le contrat de
   complétion forcé ne l'a pas passé et a été retiré de MTG. Tant que les
   essais se font sur gpt-oss-120b, chaque correction est à juger contre cette
   règle, et l'essai Gemini via Vertex dira ce qui relevait du modèle.

## 8. Les objectifs immédiats que je propose

Dans cet ordre. Chaque point dit pourquoi il passe avant le suivant.

**1. Mettre le travail à l'abri.** Extraire les quatre correctifs du moteur
C++ dans un commit à part sur `master` (avec un test pour
`ParsedParameterExpression::copy`), puis fusionner `mtg-experiments`. Tant
que 20 commits vivent sur une branche, tout ce qui suit se fait sur un tronc
périmé. Coût : une journée, surtout de vérification.

**2. La fiabilité qui bloque tout produit.** Le WAL illisible après un arrêt
brutal d'abord : lire la branche amont `vela/storage/concurrent-checkpoint-recovery`
du 27 septembre avant d'écrire quoi que ce soit, elle touche peut-être
exactement ça. Puis la persistance des structures imbriquées, que seul Codex
a vue : à reproduire avant de la croire. C'est pour la session cœur C++.

**3. Fermer la recherche.** Le pas C, après tes décisions 1 à 6. Le banc du
18 septembre en est le juge (ligne G : 0,328 → 0,385). C'est petit, et ça
clôt un chantier ouvert depuis le 7 septembre.

**4. Brancher les deux moitiés : l'agent de code par le backend déclaratif.**
C'est la réconciliation elle-même. Le 18 septembre il manquait un hôte et une
surface ; MTG les a construits. Un manifeste `code` qui expose `search`,
`grep`, `read`, `schema` par le même binaire et le même pont MCP donne
l'entrée « indexer ce dépôt » : lire les sources, choisir le modèle par
l'heuristique (qui trouve enfin son appelant), ingérer, servir. C'est ton
ordre du 7 septembre, arrivé à son quatrième point, avec les moyens du 27.

**5. Ta cible du 29 août.** Un agent qui compose un backend à partir de
gabarits : étendre `place` / `adopt` aux manifestes, aux outils et aux règles
de harnais, et mesurer ce qu'il a dû écrire lui-même. Le deck builder devient
le premier cas de test : le refaire par la boucle, et comparer.

**Ensuite, sans ordre arrêté** : la lecture des documents et les tableurs
(l'entrée du réel, toujours à zéro) ; les rôles d'agents ; l'ingesteur de
base étrangère ; l'empaquetage ; la parole.

**Ce que je ne ferais pas maintenant** : le design des accès déclaratifs et
de l'optimisation adaptative. Il est bien pensé, mais il optimise des accès
qu'aucun produit n'a encore sous charge.

### Avis de la session Products Experiments sur l'ordre

D'accord sur le fond et sur 1, 3, 4, 5. Trois réserves :

- **Dans 2, mettre la place disque avec le WAL.** La récupération des lignes
  supprimées est un défaut du même cœur, du même ordre de gravité pour un
  produit : sans elle, une base qui vit (index lucivy, journal des
  conversations) grossit sans fin, et le contournement côté rag3weaver ne
  couvre que les blobs. La base MTG actuelle (~8 Go dont ~6 récupérables)
  ne se rétrécira qu'en la reconstruisant.
- **Avant 4, un petit pas générique tiré de MTG** : la description d'entité
  dans le manifeste, reprise par les descriptions d'outils générées. Le
  constat du 27 septembre (des `search_*` qui disent tous la même chose, une
  requête décrite comme « nom de carte ») vaudra exactement pour un manifeste
  `code` : un agent qui ne sait pas ce qu'un outil couvre appelle le mauvais.
  C'est une journée, et 4 en hérite.
- **Le deck builder ne doit pas attendre 5 pour rester utilisable.** Ses
  restes sont petits (sources de mana des artefacts et créatures dans le
  harnais, barre de défilement du chat, essai Gemini) ; les faire entre deux
  pas évite de juger la boucle de 5 sur un produit de référence dégradé.

## 9. Annexe — les relevés

Les deux relevés complets (élément, section du doc, état, fichier et fonction
qui le prouve) ont été faits le 1er octobre par deux agents de repérage :
[03 — docs 02 à 08](03-releve-des-visions-02-a-08.md) et
[04 — docs 09 à 15 et produits](04-releve-des-visions-09-a-15.md). Ce doc en
est la synthèse. Ce qui y est marqué « absent » l'a été après recherche
explicite, et la recherche est nommée dans le relevé.

Trois constats de ces relevés qui méritent d'être gardés :

- `title_boost`, `content_boost` et `special_ops` sont toujours acceptés et
  jamais appliqués ; ils émettent maintenant un avertissement.
- Le mode `approbation` des commandes n'a aucun canal de réponse : une
  commande « en attente » ne peut pas être accordée par l'utilisateur.
- L'affirmation « l'OCR couvre déjà les PDF scannés » des docs de vision
  n'est pas étayée par le code : rien ne rastérise un PDF.
