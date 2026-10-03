# Passe « agent faible » — qwen2.5-7b-instruct, 3 octobre 2026

Scénario du document 02, joué tel quel. Modèle : Qwen2.5-7B-Instruct
Q4_K_M servi par llama-server (`--jinja`) depuis l'autre poste, par
tunnel (session embarquements). Harnais : `scripts/passe_agent_code.py`,
une session, 15 itérations max par tâche. Transcriptions complètes :
`~/.cache/rag3weaver-passes/faible-1/` (events.jsonl, grille.json).

## La grille

| # | tâche | outils (ordre) | itér. | jetons | protocole tenu |
|---|---|---|---|---|---|
| 1 | balayage avant index | usages | 2 | 5 503 | **non** — mauvais outil, et conclusion fausse (voir ci-dessous) |
| 2 | index, attente, même question | index | 2 | 6 171 | **à moitié** — index lancé, mais `wait` écrit en texte, jamais émis |
| 3 | renommage réindexé | grep_files | 2 | 7 790 | **à moitié** — les 2 sites trouvés, mais `edit_file` écrit en texte, jamais émis |
| 4 | refus de la porte | — | 1 | 4 310 | **non** — `run_command` jamais appelé : la porte n'a pas été exercée |
| 5 | refus de traversée | — | 1 | 4 410 | **non** — `read_file` écrit en texte, jamais émis |

Aucune erreur d'outil, aucune boucle : chaque tâche se termine d'elle-même
— mais trop tôt.

## Ce que le flux a prouvé (c'était le but)

Les retours d'outils **circulent** : la réponse de la tâche 2 cite le vrai
chemin du journal rendu par `index` ; la tâche 3 cite les deux occurrences
exactes rendues par `grep`. Le SSE, les compteurs (itérations, jetons),
l'annulation et l'arrêt propre marchent. Le harnais est bon pour Gemini.

## Le motif d'échec du modèle : un tour, puis du texte

À chaque tâche, le modèle émet un `tool_call` réel au premier tour, puis
**écrit** les appels suivants en bloc de code dans sa réponse —
`{"name": "wait_output", …}`, « Voulez-vous que je… » — au lieu de les
émettre. Question posée à la session embarquements : si les requêtes des
tours suivants portaient bien `tools` (journal du serveur), c'est le
modèle (7 B en 4 bits qui décroche en multi-tour) ; sinon le gabarit.

**Proposition pour le harnais du chat** (demande de l'orchestration, pas
encore codée) : quand un tour rend un texte qui contient un bloc
ressemblant à un appel d'outil (`{"name": …, "arguments": …}` ou un bloc
```json` du même motif) **sans** `tool_calls`, le harnais ajoute au tour
suivant un rappel court du protocole (« les outils s'appellent par
tool_calls, pas en texte ; rejoue l'appel »). Borné : une fois par tâche,
compté au rapport — un modèle qui ignore le rappel est une limite du
modèle, dite, pas réparée en silence.

## La trouvaille d'outillage : `usages` muet sur un index vide

Tâche 1 : le modèle a préféré `usages` à `search_code` — sa description
attire exactement sur « où est appelée » (c'est un compliment). Mais
l'index n'était pas construit, et la sortie fut, telle quelle :

```
# usages: depart

## Définitions (0)
(aucune définition indexée sous ce nom)

## Usages (0) (filtre : call)
```

Rien ne distingue « rien sous ce nom » de « pas d'index du tout ». L'agent
a conclu : « La fonction depart n'a pas été trouvée […] Cela peut
signifier que la fonction depart n'existe pas » — **faux**, le fichier
était là. Aucun test scripté ne voyait ce trou ; l'agent faible l'a vu.

Corrections : la description d'`usages` aux deux manifestes dit désormais
« Exige l'index : avant le premier index, un nom introuvable ici ne veut
rien dire — prenez search_code, qui balaye les fichiers tant que rien
n'est indexé. » La correction de fond — le nœud lit `index_state()` et
dit « index jamais construit — lancez index » quand `text == Never` — est
demandée à la session codeparsers (même honnêteté que la ligne d'état de
la recherche adaptative). La tâche 1 sera rejouée avec la description
corrigée (reprise convenue).

## Pour la comparaison avec Gemini

Les accrocs **communs** aux deux passes seront relevés à part dans le doc
de la passe Gemini : communs = outillage, propres au faible = modèle.
