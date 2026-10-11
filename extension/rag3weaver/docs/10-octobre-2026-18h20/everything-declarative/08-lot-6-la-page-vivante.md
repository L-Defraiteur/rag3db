# Lot 6 — la page vivante (l'atelier)

11 octobre 2026, session « everything declarative » (chantier I). On parle à
l'agent ; le backend change sous les yeux : la page produite, le graphe de
l'outil touché, le journal des rechargements.

## 1. La recette, pour Lucie (trois commandes)

Sur le poste principal, depuis l'arbre principal à jour (la lib du moteur
déjà bâtie dans `build/lecteurs-csv`). Les commandes passent dans fish comme
dans bash.

**1. Bâtir** le binaire, avec le chat (feature `openai-llm`) :

```sh
cd ~/git_workspaces/rag3db/extension/rag3weaver; and ~/.cache/rag3weaver-build/poste lourd env RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=../../build/lecteurs-csv/src RAG3DB_INCLUDE_DIR=../../build/lecteurs-csv/src cargo build -j 8 --release --features rag3db-native,code,daemon,openai-llm --bin rag3weaver-backend
```

**2. Copier le jouet hors du dépôt** et lui déclarer son modèle (Gemini par
Vertex ; le projet est celui de `.vault`, la clé n'entre dans aucun fichier) :

```sh
cp -r templates/proto/boutique ~/boutique; and python3 -c 'import json,os; p=os.path.expanduser("~/boutique/backend.json"); m=json.load(open(p)); m["vector_extension"]=os.path.abspath("../vector/build/libvector.rag3db_extension"); m["models"]={"llm":{"provider":"compatible","protocol":"vertex","model":"gemini-2.5-flash","params":{"project":"lr-hub-472010","location":"global"}}}; json.dump(m,open(p,"w"),indent=2)'
```

**3. Servir et ouvrir** (la clé du compte de service par
`GOOGLE_APPLICATION_CREDENTIALS`, seulement dans l'environnement) :

```sh
env GOOGLE_APPLICATION_CREDENTIALS=<le fichier de clé du compte de service> LD_LIBRARY_PATH=../../build/lecteurs-csv/src target/release/rag3weaver-backend serve --manifest ~/boutique/backend.json --atelier & sleep 3; xdg-open http://127.0.0.1:7777/_atelier
```

Puis, dans la page : ajouter un produit par le formulaire (à droite), et
écrire dans le chat **« ajoute une colonne prix à la liste »**. L'agent lit
`views/products.html` (`read_declaration`), la déclare avec la colonne
(`declare`, sa raison pour message) ; le journal dit « déclaré
views/products.html (version 1, commit …) », la page se recharge avec la
colonne, sans compilation. `git -C ~/boutique log` montre le commit avec la
raison. Arrêter : `curl -X POST http://127.0.0.1:7777/arret` (la base se ferme
proprement).

Sans `models.llm`, ou avec un binaire bâti sans `openai-llm`, la page s'ouvre
et dit pourquoi le chat est éteint. Le jeton Vertex est pris à la connexion
et dure une heure (limite de `connect_llm`).

## 2. Ce que c'est

`rag3weaver-backend serve --manifest … --atelier` ajoute, sur la boucle locale
seulement, sous le préfixe réservé `/_atelier` (hors des routes déclarées) :

| chemin | ce qu'il rend |
|---|---|
| `GET /_atelier` | la page : le chat à gauche ; à droite la page produite (une route GET du backend, d'abord une qui a une vue), le graphe mermaid de l'outil de la route montrée, le journal |
| `GET /_atelier/events` | un seul flux d'événements serveur : `token`, `tool_start`, `tool_end`, `answer` (le chat) ; `declared`, `refused`, `reloaded` (l'hôte) |
| `GET /_atelier/state` | nom, version, routes, vues, outils et leurs graphes, chat allumé ou pourquoi éteint |
| `GET /_atelier/graph?tool=…&highlight=…` | le graphe d'un outil en mermaid, les nœuds d'un type en surbrillance |
| `POST /_atelier/chat` | un tour de chat ; la réponse et les outils passent par le flux |

- **Le chat** : `ChatSession::run` avec les outils du manifeste en processus
  (`BackendTools`, la même présentation que le binaire de chat) et les trois
  de la déclaration (`DeclarationTools`) ; un `declare` accepté ou refusé est
  publié au flux. Le modèle est **déclaré** (`models.llm`, provider
  « compatible », protocole openai ou vertex) ; sans lui, pas d'agent.
- **La session** est gardée à chaque tour dans
  `<dossier du backend>/.atelier/session.json`, avec un `.gitignore` posé par
  le serveur : `declare` ne peut pas l'écrire (dossier caché), git ne la
  commite pas.
- **La montre** sonde les déclarations deux fois par seconde (dates et
  tailles, aucune dépendance) : une modification faite à la main dans
  l'éditeur est revérifiée et rechargée (`reloaded`), ou dite (`refused`,
  l'ancienne version reste) ; un `declare` met la montre à jour, elle ne
  recharge pas une seconde fois.
- **Le flux** est tenu par l'hôte lui-même (`tiny_http` retiendrait de petits
  événements dans ses tampons) : l'en-tête et chaque événement sont écrits et
  vidés sur la connexion ; un abonné parti est oublié.
- La page : du HTML et ~150 lignes de JavaScript intégrés au binaire ; pas de
  React, pas d'empaquetage. **Mermaid est chargé du réseau (jsdelivr) pour le
  proto ; le produit l'embarquera** — un atelier local ne doit pas dépendre du
  réseau à terme.
- Le binaire n'ajoute pas `openai-llm` à ses features requises : le paquet npm
  décide (session du paquet).

## 3. Les témoins

Rouges à la compilation, puis verts (`src/backend_reload_tests.rs`) :

- le backend donne ses outils à un agent en processus (définitions, appel,
  outil inconnu) ; le graphe d'un outil se lit pour le dessiner ;
- la page `/_atelier` se sert sur un vrai port, son état dit « chat éteint »
  et pourquoi, le graphe d'un outil arrive avec la surbrillance, les routes
  déclarées répondent à côté, le chat éteint refuse en le disant, `.atelier/`
  porte son `.gitignore` ;
- le flux d'événements atteint un abonné sur une vraie connexion ;
- la montre recharge une vue modifiée à la main (`reloaded`, la page change)
  et dit le refus d'une vue cassée (l'ancienne répond) ;
- un tour de chat avec un modèle enregistré déclare une vue : le flux porte
  `tool_start`, `declared`, `answer` ; la page change ; la montre ne recharge
  pas une seconde fois ; le commit porte la raison ; la session est gardée et
  n'est pas commitée.

Et le binaire, réellement : `serve --atelier` sur une copie du jouet ouverte
sur une base : état (chat éteint, sans `models.llm`), page, formulaire,
liste, arrêt propre (sortie 0). Le témoin final reste la recette du §1,
jouée par Lucie.

## 4. Ce qui reste

- ~~`declare` et la montre par la porte de la session mémoire~~ fait le
  11 octobre (`apply_reload_with_reactions`) : les réactions de l'ancienne
  version s'arrêtent et remontent ; le journal dit les curseurs oubliés et les
  erreurs des réactions arrêtées.
- Le retour en arrière d'une déclaration ; plusieurs sessions de chat ;
  mermaid embarqué ; la surbrillance d'un nœud scripté touché (aujourd'hui :
  le graphe de la route montrée).
