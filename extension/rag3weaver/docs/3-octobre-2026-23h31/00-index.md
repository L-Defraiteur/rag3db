# Rapports de session et connaissances — 3 octobre 2026, 23 h 31

Demande de Lucie : chaque session rend **régulièrement**, pas seulement à
l'arrêt, un rapport de session et un relevé de ce qu'elle sait, pour que rien
ne dépende de ce qu'une session a encore en tête.

## La règle

Chaque session tient ici un dossier à son sujet, avec deux fichiers :

- `01-rapport-de-session.md` — ce qui a été fait, décidé et pourquoi, ce qui
  est en cours, ce qui attend quelqu'un, et **comment reprendre** (branches,
  commits, commandes, pièges).
- `02-knowledge-dump.md` — ce que la session sait de sa partie : architecture,
  fichiers, tests, mesures, défauts connus, ce qui a été essayé sans succès.

Ils se **mettent à jour sur place** : après chaque lot fusionné, et au moins
toutes les deux heures de travail. Un fichier à jour vaut mieux que trois
fichiers datés. Une session qui sent sa mémoire de conversation se remplir
(réponses plus lentes, résumé automatique) met à jour avant de continuer.

Le journal des chantiers (`docs/journal-des-chantiers.md`) reste le registre
commun ; ces dossiers disent ce que le journal ne dit pas.

## Les dossiers

| Dossier | Sujet |
|---|---|
| `orchestration/` | cadrage des sessions, décisions de Lucie, état d'ensemble |
| `arbre-principal/` | ingestion du code, synchronisation, chargement en masse, requêtes par lot |
| `coeur-cpp/` | moteur : reprise après arrêt, index vectoriel, verrous |
| `banc-de-concurrence/` | banc, témoins des verrous, arrêts brutaux |
| `embarquements/` | service distant, estimation, indexation, modèles déclarés, mesures |
| `recherche/` | backend de code, outils d'agent, passes d'agent, crochet après outil |
| `codeparsers/` | analyseur, genre d'usage, tests, imports, `usages`, `impact`, banc des relations |
| `memoire-longue/` | mémoire longue, banc, arrêt brutal côté produit |
| `optimiseur/` | modèles de décision, envois à tracel-ai |

Les rapports du 2 octobre sont dans `../2-octobre-2026-01h07/`.
