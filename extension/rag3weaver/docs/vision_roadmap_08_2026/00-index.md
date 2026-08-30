# Vision et feuille de route — août 2026

Six documents écrits le 25 août 2026, un septième le 26, un huitième le 29 — à la fin d'une journée où beaucoup de
choses ont été décidées oralement. Ils existent pour que ces décisions ne se
perdent pas, et pour qu'on puisse les attaquer plutôt que se les rappeler.

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

**Ce qu'ils remplacent** : rien. Les documents datés de
`23-aout-2026-20h33/` restent la trace chronologique du travail (état des
lieux, mesures, passations). Ceux-ci sont la couche au-dessus : ce qu'on veut
faire et pourquoi.

**Ce qu'ils ne sont pas** : une spécification. Plusieurs points y sont
explicitement ouverts, et signalés comme tels.

## Où en est le moteur au moment où ils sont écrits

Livré et mesuré : recherche hybride (BM25 lucivy, vecteur HNSW, sparse WAND,
fusion RRF, rerank), multi-tenant org × project, six modèles sur burn
(embedders, rerankers, OCR PP-OCRv6), un dataflow de 28 nœuds avec undo et
checkpoints, une recherche composable (signaux étiquetés, fusion N-aire,
rerank comme nœud), les outils comme graphes, une boucle d'agent, un client cloud
(Vertex, AI Studio, tout endpoint compatible OpenAI) et **un agent qui tourne
hors ligne** sur un modèle de 996 Mo.

Manquant, et c'est le sujet du [06](06-la-feuille-de-route.md) : de quoi
**avaler le réel** — le code par `codeparsers`, les documents, les catalogues.
