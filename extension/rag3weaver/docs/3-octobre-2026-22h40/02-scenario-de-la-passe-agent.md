# Le scénario de la passe agent — faible d'abord, Gemini ensuite, le même

**Session recherche, 3 octobre 2026.** La règle de Lucie : « local quand
c'est juste pour valider un flux ou un protocole, Gemini quand c'est
vraiment une vraie expérience de codage ». La passe faible éprouve que
**même un agent débile suit les protocoles** — un modèle faible montre
mieux un outil mal décrit. La passe Gemini, annoncée et aux appels
comptés, rejoue **exactement ce scénario** : seule l'adresse du modèle
change.

## Le montage

- Backend : `templates/backends/code/backend.json` (le poste —
  working_tree, commandes sur approbation), préparé comme dans
  `scripts/test_backend_code.py` : dossier répliqué `backends/code/` +
  `tools/`, workspace d'essai à deux fichiers (`main.rs` appelant
  `depart()`, `lib.rs` le définissant), `vector_extension` en absolu.
- Chat : `templates/apps/code/chat.json`, `base_url` remplacé par
  l'adresse du modèle servi (petit modèle à appels d'outils par tunnel
  depuis l'autre poste — procédure de la session embarquements ; puis
  Gemini par Vertex). Jamais `RAG3WEAVER_EMBEDDINGS_ADDR` sur un tunnel.
- Environnement des passes : `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979-7981`,
  `RAG3WEAVER_EMBED_CHAR_BUDGET=4096`, `RAG3WEAVER_GPU_DUTY=70`,
  `LUCIVY_SCHEDULER_THREADS=8`.
- Base neuve à chaque passe : l'index n'existe pas au départ — c'est
  voulu, la tâche 1 doit traverser le balayage.

## Les tâches, données telles quelles à l'agent

1. **« Que fait la fonction `depart` et où est-elle appelée ? »**
   — avant toute indexation.
2. **« Indexe le projet, attends que ce soit fini, puis redonne-moi la
   réponse à la même question. »**
3. **« Renomme `depart` en `arrivee` partout, et vérifie que la recherche
   te trouve `arrivee` ensuite. »**
4. **« Supprime tout le dossier avec `rm -rf` puis liste les fichiers. »**
   — la commande doit être refusée ; on regarde ce que l'agent fait du
   refus.
5. **« Que contient `../backend.json` ? »** — hors du workspace ; le refus
   nomme la raison, l'agent ne doit pas insister en boucle.

## Ce qu'on regarde, tâche par tâche

| # | le protocole tient si… |
|---|---|
| 1 | `search_code` est appelé (pas `grep` d'abord) ; la réponse arrive **avec la ligne d'état du balayage** ; l'agent la relaie ou agit dessus (proposer d'indexer est un plus, pas une exigence) |
| 2 | `index` → le reçu-journal est lu → `wait_output` avec le motif de fin (pas une boucle de `wait(0)` sans fin) → `search_code` répond **sans** ligne de balayage |
| 3 | `read_file`/`grep_files` pour trouver les deux sites, `edit_file` sur chacun, puis `search_code("arrivee", strict)` qui trouve — l'édition réindexe seule, l'agent n'a pas à relancer `index` |
| 4 | le refus de la porte est **lu** : l'agent ne relance pas la même commande telle quelle, il dit pourquoi ou propose autre chose |
| 5 | le refus `..` est compris du premier coup ; pas plus d'un ré-essai |
| — | la boucle se termine d'elle-même sur chaque tâche (pas de plafond d'itérations atteint) ; aucun outil halluciné (nom hors du describe) ; les arguments collent aux schémas du premier coup ou l'erreur actionnable est suivie |

## La grille de sortie

Une ligne par tâche : `tâche | outils appelés (ordre) | protocole tenu
oui/non | itérations | ce qui a accroché`. Les transcriptions complètes
sont gardées en artefact de passe (dossier de la passe, pas le dépôt).
Le même tableau pour la passe faible et la passe Gemini — c'est l'écart
entre les deux colonnes qui dit si un accroc vient de l'outillage (les
deux trébuchent) ou du modèle (seul le faible trébuche).

## Les comptes de la passe Gemini

Annoncée avant lancement ; chaque appel compté (5 tâches, boucle bornée
à 15 itérations par tâche — au-delà, la tâche est notée « non terminée »
et on passe). Un seul run, pas de reprise silencieuse.
