# Arrêt du 2 octobre 2026, 01 h 07 — rapports de session et knowledge dumps

Lucie arrête la séance pour la nuit. Chaque session active rend ici, dans un
dossier à son sujet, deux documents, puis cesse de travailler :

- `01-rapport-de-session.md` — ce qui a été fait cette nuit, ce qui est en
  cours et **où exactement le travail s'est arrêté** (branche, commit, dossier
  de travail, commande à relancer), ce qui attend une décision de Lucie, ce
  qu'il faut faire en premier à la reprise.
- `02-knowledge-dump.md` — tout ce que la session sait du projet, orienté sur
  sa partie : architecture, fichiers, façon de construire et de tester,
  pièges, ce qui est vérifié et ce qui ne l'est pas.

| Dossier | Session | Sujet |
|---|---|---|
| `orchestration/` | rag3weaver archi | cadrage, contrôle par git, décisions de Lucie, état de tous les chantiers |
| `moteur-concurrence/` | cœur C++ | reprise après panne, lecteur d'un autre processus, plan des écritures parallèles, verrous |
| `banc-de-concurrence/` | banc | banc de concurrence et vérificateur d'intégrité |
| `optimiseur/` | optimiseur burn | forks, poids granite, PR amont vers tracel-ai |
| `produit/` | produit (arbre principal) | livraisons sur master, journal d'écriture, synchronisation par périmètre, lot sous Lifecycle |
| `recherche/` | recherche | pas C : pondérations dans les graphes |

Le registre des chantiers reste `docs/journal-des-chantiers.md` à la racine :
c'est lui qu'on lit en premier en reprenant.
