# Vision et feuille de route

Six documents écrits le 25 août 2026, un septième le 26, un huitième le 29 — à
la fin d'une journée où beaucoup de choses ont été décidées oralement. Ils
existent pour que ces décisions ne se perdent pas, et pour qu'on puisse les
attaquer plutôt que se les rappeler.

**Relus et mis à jour le 5 septembre 2026**, après une semaine qui a changé la
nature du moteur. Ce qui a été corrigé dans les documents porte sa date ; ce qui
n'en porte pas est d'origine. Les corrections ne sont pas des retouches de
confort — trois affirmations étaient devenues **fausses**, et une quatrième
n'était plus vérifiée par personne.

| | |
|---|---|
| [01 — La vision](01-la-vision.md) | Le chaos contrôlé, la boucle étrange, ce qui la tient debout |
| [02 — Les deux moitiés](02-les-deux-moities.md) | Documents et code d'un côté, catalogues de l'autre — pourquoi le moteur est bicéphale |
| [03 — Normaliser des tableurs](03-normaliser-des-tableurs.md) | Ce qu'un pipeline réel a appris, et ce qu'il contredit dans nos plans |
| [04 — Le catalogue comme graphe](04-le-catalogue-comme-graphe.md) | Outils, tags, mémoire : le savoir et les capacités dans la même matière |
| [05 — Ce qui a tenu depuis février](05-ce-qui-a-tenu-depuis-fevrier.md) | Archéologie : ce qu'on a redressé, ce qu'on a perdu sans décider |
| [06 — La feuille de route](06-la-feuille-de-route.md) | L'ordre réel du travail, et les dettes nommées |
| [07 — Événements, runs et boucles](07-evenements-runs-et-boucles.md) | Un bus à sujets, l'identité des runs, et quand une boucle peut réagir — écrit le 26 août |
| [08 — Des catalogues de gabarits](08-des-catalogues-de-gabarits.md) | Ne pas leur faire tout coder : entités, graphes et composants prêts à poser, à adopter, à modifier — écrit le 29 août |
| [09 — Trois rôles, une seule main](09-trois-roles-et-une-seule-main.md) | Design tient le cap, contexte souffle, code écrit — tempéraments, mémoire bornée par manche, `agentCanAnswer` — écrit le 30 août |
| [10 — L'agent vision et la distance](10-l-agent-vision-et-la-distance.md) | Le pourquoi, l'intention première jamais perdue, et le registre de ce qui y répond — écrit le 30 août |
| [11 — Donner des commandes à un agent](11-donner-des-commandes-a-un-agent.md) | Sans lui donner la machine : argv plutôt que shell, trois modes, un verdict qu'on range — copie du 30 août |
| [12 — Ce qui manque à l'agent de code](12-ce-qui-manque-a-l-agent-de-code.md) | La liste écrite par celui qui s'en sert : exécuter, supprimer, voir le schéma — 30 août |
| [13 — Avaler une base existante](13-avaler-une-base-existante.md) | Lire un schéma étranger et en proposer un graphe, en séparant ce qui est déclaré de ce qui est deviné — 30 août |
| [14 — Le schéma comme artefact](14-le-schema-comme-artefact.md) | Ce qu'une migration devrait laisser derrière elle, l'écart entre ce que la config veut et ce que la base contient, et pourquoi un schéma n'est pas que des tables — 30 août au soir |
| [15 — Le moteur cesse d'être mono-backend](15-le-moteur-cesse-d-etre-mono-backend.md) | Les cinq organes d'un backend, ce que « prouver » a coûté, les deux invariants nés de la pratique, et pourquoi Neo4j est le dernier et non le deuxième — 3 septembre |

**Ce qu'ils remplacent** : rien. Les documents datés de
`23-aout-2026-20h33/` restent la trace chronologique du travail (état des
lieux, mesures, passations). Ceux-ci sont la couche au-dessus : ce qu'on veut
faire et pourquoi.

**Ce qu'ils ne sont pas** : une spécification. Plusieurs points y sont
explicitement ouverts, et signalés comme tels.

## Où en est le moteur — relevé le 5 septembre 2026

Chaque chiffre ci-dessous a été vérifié dans le code ce jour-là, pas recopié.

**Deux backends**, entiers : rag3db natif et PostgreSQL/pgvector. Chacun fournit
les cinq organes — connexion, dialecte, recherche, magasin de blobs, magasin de
checkpoints. C'est le changement de nature de la semaine
([15](15-le-moteur-cesse-d-etre-mono-backend.md)).

**Recherche** hybride : BM25 lucivy, vecteur HNSW, sparse WAND, fusion RRF,
rerank ; plus, sur PostgreSQL, un plein texte **servi par la base** (trigramme
GIN, ordonné par Jaro-Winkler). Les deux moteurs de texte sont une **option**,
pas un remplacement, et le chemin de retour vers lucivy est éprouvé. Une
recherche sait maintenant dire qu'elle ne trouve **rien de probant**, sur un
seuil mesuré et non recopié.

**Dataflow** de **34 types de nœuds** — 44 avec la feature `code` — avec undo et
checkpoints ; recherche composable (signaux étiquetés, fusion N-aire, rerank
comme nœud) ; **11 outils-graphes** offerts à un agent (2 sans `code`).

**Modèles** : six sur burn — trois embedders (BGE-M3, MiniLM, MiniLM
multilingue), trois rerankers (ms-marco, mmarco-mMiniLM, bge-reranker-v2-m3) —
plus la pile OCR PP-OCRv6. Un client cloud (Vertex, AI Studio, tout endpoint
compatible OpenAI) et **un agent qui tourne hors ligne**.

**Concurrence** : plusieurs processus **lecteurs**, mesuré. Un lecteur choisit
son chemin — direct ou par le relais — et sait lequel il a pris. Plusieurs
processus écrivains : toujours non.

**Éprouvé par** 40 suites de bout en bout et près d'un millier de tests
unitaires. Depuis le 4 septembre, PostgreSQL entre dans la passe complète, ou
son absence s'y annonce.

### Ce qui manque, et qu'il ne faut pas croire fait

- **La lecture des documents** — pdf, docx, pptx, html, csv. Toujours à écrire.
- **Le graphe de normalisation xlsx**, porte d'entrée de toute la moitié KB.
- **La parole** — aucun TTS, aucun STT, aucun G2P. C'est pourtant ce sur quoi
  repose la différenciation d'un des quatre produits
  ([01](01-la-vision.md) §7).
- **`Catalog::search` est toujours un monolithe**, et il a grossi : 409 lignes.
  Le chemin composable existe en parallèle et n'est pas emprunté — deux chemins
  à maintenir, et le prix se paie à chaque correction de recherche.

`codeparsers` **n'est plus dans cette liste** : il est branché, éprouvé sur
notre propre source, et sorti en dépôt séparé.
