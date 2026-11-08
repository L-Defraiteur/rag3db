# Les branches distantes de rag3db, pour décision

*Relevé du 8 novembre 2026 contre `origin/master` = `f7df88003`, après le `git fetch --prune`. 732 branches distantes (hors `master`). Aucune n'a été effacée : la page propose, Lucie décide.*

## Comment lire

- **dernier commit** : la date de commit de la tête de la branche.
- **dans master** : `oui` si la tête est un ancêtre de `master` (`git merge-base --is-ancestor`).
- **+ master** : les commits de la branche absents de l'historique de `master` (`git rev-list master..branche`).
- **non atterris** : parmi eux, ceux dont ni le patch (`git cherry`) ni le sujet ne se retrouvent dans `master` — le travail qui n'a vraiment pas atterri. Un commit rebasé avec un conflit résolu change de patch mais garde son sujet : il compte comme atterri.
- **session** : d'après le préfixe du nom ; sinon d'après la partie du dépôt que les commits propres touchent (moteur, rag3weaver, IDE, codeparsers) ; « Kuzu (amont) » pour tout ce qui est d'avant août 2026, les branches héritées du fork. La colonne *par* dit laquelle des deux a servi : un rattachement par la partie touchée est une supposition.
- **proposition** :
  - *effacer* — entièrement dans master, ou tout son travail atterri, ou remplacée par une sœur de numéro plus haut (`-2`, `-3`…) atterrie ;
  - *garder* — la tête de sa famille, du travail non atterri depuis le 25 octobre ;
  - *à demander* — une sœur plus récente pas encore atterrie, du travail probablement repris sous d'autres commits (voir plus bas), du travail non atterri d'avant le 25 octobre, un nom de brouillon (préfixe `ancien/` ou `archive/`, suffixe `-wip`, `essai-…`, `…-avant-renommage`, `…-sur-<hash>`), ou une branche de l'amont.
- **probablement reprise** : une branche dont un commit `feat`/`fix`/`perf`/`refactor`/`wip` non atterri a, dans master depuis octobre, un sujet proche (au moins 0.45 des mots de quatre lettres et plus en commun, indice de Jaccard). C'est le cas d'un lot atterri par un autre jeu de commits (réécrit, regroupé, repris après relecture) : ni le patch ni le sujet ne le prouvent. Une proximité n'est pas une preuve — la colonne *proche dans master* donne le hash et le score, pour vérifier. Sous le seuil, une branche reprise passe pour du travail à garder : c'est le sens de l'erreur choisi, rien ne s'efface par cet indice.
- **tenue** : `hooks-essai` est gardée d'office, une session vit dessus (son worktree `rag3db-hooks-essai`).
- La date limite du 25 octobre est la reprise de toutes les sessions après le nettoyage à 30 jours.

## Totaux

| session | branches | effacer | garder | à demander |
|---|---|---|---|---|
| IDE (React) | 40 | 33 | 3 | 4 |
| arbre principal (73) | 60 | 53 | 2 | 5 |
| banc | 45 | 43 | 0 | 2 |
| codeparsers | 28 | 26 | 2 | 0 |
| embarquement | 2 | 1 | 0 | 1 |
| everything declarative (I) | 64 | 52 | 2 | 10 |
| hooks, boîte et pont (archi, hooks-essai) | 29 | 27 | 2 | 0 |
| moteur (cœur C++ ou hotfixs) | 147 | 134 | 5 | 8 |
| mémoire (96) | 59 | 28 | 4 | 27 |
| optimiseur (G) | 5 | 4 | 0 | 1 |
| rag3weaver, non attribuée | 13 | 3 | 1 | 9 |
| recherche (97) | 18 | 8 | 0 | 10 |
| tty | 16 | 16 | 0 | 0 |
| rag3weaver, atterries, session non déduite | 111 | 111 | 0 | 0 |
| Kuzu (amont) | 95 | 1 | 0 | 94 |
| **total** | **732** | **540** | **21** | **171** |

## IDE (React) — 40 branches (33 à effacer, 3 à garder, 4 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `chat-lisible` | 2026-11-07 | non | 11 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | refactor(équipier): une seule règle d'état — teammate_list et teammate_states lisent team… |
| `chercher-sans-dossier` | 2026-11-07 | non | 11 | 6 | nom | **effacer** | remplacée par `chercher-sans-dossier-2`, atterrie | `08d10babf` (0.29) | test(ide): le témoin de « chercher sans dossier » attend l'index par search_code avec ses… |
| `chercher-sans-dossier-2` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(ide): le témoin de « chercher sans dossier » attend l'index par search_code avec ses… |
| `ide-chat-1` | 2026-10-25 | non | 4 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(ide): K2 — la vue du chat de l'atelier dans l'IDE : le fil, les jetons du tour, ses … |
| `ide-chat-2` | 2026-10-25 | non | 5 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(ide): K3 — l'agent de l'IDE propose un gabarit sur Gemini, jugé sur le résultat : l'… |
| `ide-chat-k3-1` | 2026-10-25 | non | 5 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(ide): K3 — l'agent de l'IDE propose un gabarit sur Gemini, jugé sur le résultat : l'… |
| `ide-js-1` | 2026-10-25 | non | 4 | 2 | nom | **effacer** | remplacée par `ide-js-3`, atterrie | `ca53f8e3f` (0.80) | wip(ide): le témoin de l'atelier lit son code servi ; les routes page.js dans la doc des … |
| `ide-js-2` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(ide): la vue outline — la fiche d'un fichier montre le plan rendu par outline, chaqu… |
| `ide-js-3` | 2026-10-25 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | refactor(ide): le code des pages de l'IDE et de l'atelier en fichiers servis — ide/page.j… |
| `ide-outline-repli-1` | 2026-10-25 | non | 3 | 2 | nom | **effacer** | remplacée par `ide-outline-repli-2`, atterrie | `400813249` (0.26) | test(ide): l'entrée par un lien venu d'ailleurs — l'accueil d'un hôte sans backend dit qu… |
| `ide-outline-repli-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(ide): la marge « Dans ce fichier » se replie sans l'outil outline, et le cookie de l'… |
| `ide-react-1` | 2026-10-25 | non | 9 | 9 | nom | **effacer** | remplacée par `ide-react-4`, atterrie | `7179b330e` (0.28) | wip(ide): le chat relit son état au flux rouvert avant tout filtre de backend |
| `ide-react-3` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(ide): l'IDE en React, vues 1 et 2 — la chaîne de bâti (bundle commité, embarqué, vér… |
| `ide-react-4` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(ide): l'IDE en React, vues 1 et 2 — la chaîne de bâti (bundle commité, embarqué, vér… |
| `ide-react-markdown` | 2026-10-25 | non | 10 | 9 | nom | **effacer** | remplacée par `ide-react-markdown-4`, atterrie | `067f63a96` (0.64) | wip(ide): le HTML du Markdown est rogné — pas de nœud texte de fin dans .body |
| `ide-react-markdown-2` | 2026-10-25 | non | 8 | 6 | nom | **effacer** | remplacée par `ide-react-markdown-4`, atterrie | `7c7a69578` (0.58) | feat(ide): le Markdown des réponses du chat se rend (marked 18.1.1) et s'assainit (dompur… |
| `ide-react-markdown-3` | 2026-10-25 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(navigateur): le harnais attend l'appel en vol, pas une durée — vingt secondes de cal… |
| `ide-react-markdown-4` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(navigateur): le harnais attend l'appel en vol, pas une durée — vingt secondes de cal… |
| `ide-react-vue3` | 2026-10-25 | non | 3 | 3 | nom | **effacer** | remplacée par `ide-react-vue3-3`, atterrie | `7c7a69578` (0.58) | docs(everything declarative): l'IDE en React — ce que la vue 3 pose |
| `ide-react-vue3-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(ide): l'IDE en React, vue 3 — la vue d'un fichier et sa marge « Dans ce fichier », l… |
| `ide-react-vue3-3` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(ide): l'IDE en React, vue 3 — la vue d'un fichier et sa marge « Dans ce fichier », l… |
| `ide-react-vue4` | 2026-10-25 | non | 7 | 6 | nom | **effacer** | remplacée par `ide-react-vue4-3`, atterrie | `7c7a69578` (0.58) | wip(ide): la vue de la recherche lit chunk.highlights (le commit précédent ne portait que… |
| `ide-react-vue4-2` | 2026-10-25 | non | 4 | 3 | nom | **effacer** | remplacée par `ide-react-vue4-3`, atterrie | `7c7a69578` (0.58) | feat(ide): l'IDE en React, vue 4 — la recherche sur le JSON convenu avec la session reche… |
| `ide-react-vue4-3` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(ide): l'IDE en React, vue 4 — la recherche sur le JSON convenu avec la session reche… |
| `ide-react-vue5` | 2026-10-26 | non | 4 | 4 | nom | **effacer** | remplacée par `ide-react-vue5-2`, atterrie | `43d7489ee` (0.25) | wip(ide): les champs des tâches, de la relecture et de l'équipier ne sont pas contrôlés p… |
| `ide-react-vue5-2` | 2026-10-26 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(ide): l'IDE en React, vue 5 — les tâches de l'agent et la relecture d'une tâche, ave… |
| `mes-recherches` | 2026-11-08 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(ide): mes recherches, avec les clics — celles de l'IDE et des agents, lues dans le j… |
| `mes-recherches-2` | 2026-11-08 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(ide): mes recherches, relue par everything declarative — un clic droit n'est pas un c… |
| `mes-recherches-suite` | 2026-11-08 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(ide): mes recherches, la suite de la relecture d'everything declarative — une page li… |
| `mes-recherches-suite-2` | 2026-11-08 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(ide): mes recherches, la suite de la relecture d'everything declarative — une page li… |
| `relais-ide-1` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(ide): le relais du flux de l'IDE a ses propres fils — il attendait derrière les graph… |
| `relais-ide-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(ide): le relais du flux de l'IDE a ses propres fils — il attendait derrière les graph… |
| `relais-ide-3` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(ide): le relais du flux de l'IDE a ses propres fils — il attendait derrière les graph… |
| `ide-react-vue3b-2` | 2026-10-26 | non | 8 | 6 | nom | **garder** | 6 commit(s) non atterri(s), tête de sa famille | `08d10babf` (0.29) | feat(ide): l'IDE en React, vue 3b — l'édition d'un fichier dans la page neuve ; l'ancienn… |
| `ide-react-vue6-3` | 2026-10-26 | non | 5 | 4 | nom | **garder** | 4 commit(s) non atterri(s), tête de sa famille | `43d7489ee` (0.25) | feat(ide): l'IDE en React, vue 6 — les bilans de session par fiche et la fiche par outil |
| `ide-react-vue7-2` | 2026-10-26 | non | 7 | 6 | nom | **garder** | 6 commit(s) non atterri(s), tête de sa famille | `08d10babf` (0.29) | feat(ide): l'IDE en React, vue 7 — l'arbre, le symbole rendu section par section, le grap… |
| `ide-react-vue3b` | 2026-10-26 | non | 10 | 10 | nom | **à demander** | dépassée par `ide-react-vue3b-2`, pas encore atterrie | `08d10babf` (0.29) | docs(ide): page 08 — la vue 3b, l'édition dans la page neuve |
| `ide-react-vue6` | 2026-10-26 | non | 4 | 4 | nom | **à demander** | dépassée par `ide-react-vue6-3`, pas encore atterrie | `08d10babf` (0.29) | wip(ide): l'IDE en React, vue 6 — les bilans de session par fiche (notes, tendance, appor… |
| `ide-react-vue6-2` | 2026-10-26 | non | 6 | 6 | nom | **à demander** | dépassée par `ide-react-vue6-3`, pas encore atterrie | `08d10babf` (0.29) | docs(everything declarative): l'IDE en React — ce que la vue 6 pose |
| `ide-react-vue7` | 2026-10-26 | non | 8 | 8 | nom | **à demander** | dépassée par `ide-react-vue7-2`, pas encore atterrie | `08d10babf` (0.29) | wip(ide): call() déplie l'enveloppe { status, result } d'une lecture du catalogue, comme … |

## arbre principal (73) — 60 branches (53 à effacer, 2 à garder, 5 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `agent/journaux-annulation-tmp-3` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | nettoyer_apres_fin : supprimer les fichiers *.undo.json à la fin d'un graphe (témoin fin_… |
| `agent/journaux-annulation-tmp-3-2` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | nettoyer_apres_fin : supprimer les fichiers *.undo.json à la fin d'un graphe (témoin fin_… |
| `agent/journaux-annulation-tmp-3-3` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | nettoyer_apres_fin : supprimer les fichiers *.undo.json à la fin d'un graphe (témoin fin_… |
| `agent/journaux-annulation-tmp-3-4` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | nettoyer_apres_fin : supprimer les fichiers *.undo.json à la fin d'un graphe (témoin fin_… |
| `agent/racine-disparue` | 2026-11-08 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(racine disparue): le message commun dit quoi faire (« … : rouvrez le backend sur une … |
| `arbre-principal/ancien-l4` | 2026-10-19 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(lectures): L4 — la recherche et les nœuds de lecture passent par la vue, plus par le… |
| `arbre-principal/ancien-l5` | 2026-10-19 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(démon): L5 — le slot d'un backend passe en RwLock : les appels d'un même backend ne … |
| `arbre-principal/ancien-l6` | 2026-10-19 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(lectures): L6 — un instantané READ ONLY par appel de lecture, sur un pool de connexi… |
| `arbre-principal/ancien-l6b` | 2026-10-19 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(lectures): RAG3WEAVER_READ_POOL=0 coupe les instantanés de L6, et l'écart d'usages m… |
| `arbre-principal/ancien-plafond` | 2026-10-19 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(lectures): le témoin du plafond de l'instantané ne dépend plus d'une attente — un pl… |
| `arbre-principal/ancien-plan` | 2026-10-19 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(plan): l'IR du Plan déclaré et le PlanNode — un gabarit lit par un Plan JSON (plans/… |
| `arbre-principal/bascule-procedure` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(procédure): bascule de lib — le contrôle des deux sens (tools/check_rel_directions s… |
| `arbre-principal/controle-ancre` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(procédure): l'étape 8 prend tools/check_rel_directions dans origin/master (l'arbre d… |
| `arbre-principal/delai-partout` | 2026-11-08 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(délai): le témoin de l'arrêt borné passe en suite e2e (e2e_arret_borne) — il tient v… |
| `arbre-principal/delai-partout-2` | 2026-11-08 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(délai): la procédure dit que la lib entière se joue avec ou sans RAG3WEAVER_EMBED_SE… |
| `arbre-principal/delai-partout-3` | 2026-11-08 | non | 5 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): le témoin de la porte de l'équipier lit le second retour avant qu'il arriv… |
| `arbre-principal/juge-claude-code` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(juge): un tour de Claude Code se juge — ses appels de l'hôte lus dans tours[].tool_c… |
| `arbre-principal/juge-claude-code-2` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(juge): un tour de Claude Code se juge sur la tâche que whois rend — le journal du ho… |
| `arbre-principal/juge-claude-code-3` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(juge): le faux juge d'un tour de Claude Code prend un PreparedBackend — la signature… |
| `arbre-principal/juge-claude-code-4` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(juge): le faux juge d'un tour de Claude Code prend un PreparedBackend — la signature… |
| `arbre-principal/juge-claude-code-5` | 2026-11-07 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(dataflow): les paramètres du parent ne se substituent plus dans le graphe cité — la v… |
| `arbre-principal/juge-claude-code-6` | 2026-11-07 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(dataflow): les paramètres du parent ne se substituent plus dans le graphe cité — la v… |
| `arbre-principal/juge-claude-code-7` | 2026-11-07 | non | 5 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(dataflow): la garde de bijection ne lit plus les paramètres d'un graphe cité — templa… |
| `arbre-principal/juge-claude-code-8` | 2026-11-07 | non | 5 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(dataflow): la garde de bijection ne lit plus les paramètres d'un graphe cité — templa… |
| `arbre-principal/ligne-en-json` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(plan): une ligne d'un Plan déclaré qui ne se rend pas en JSON refuse en nommant sa co… |
| `arbre-principal/ligne-en-json-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(plan): une ligne d'un Plan déclaré qui ne se rend pas en JSON refuse en nommant sa co… |
| `arbre-principal/ordre-par-extreme` | 2026-10-25 | non | 7 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(plan): l'ordre par extrême se joue sur le moteur et sur PostgreSQL — le dernier L1 o… |
| `arbre-principal/ordre-par-extreme-2` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(plan): l'ordre par extrême se joue sur le moteur et sur PostgreSQL — le dernier L1 o… |
| `arbre-principal/ordre-par-extreme-3` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(plan): l'ordre par extrême se joue sur le moteur et sur PostgreSQL — le dernier L1 o… |
| `arbre-principal/pagination-sans-ignore` | 2026-10-25 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | style(ir): un mut de trop dans le liage d'un Plan déclaré — le seul warning de rag3weaver… |
| `arbre-principal/pagination-sans-ignore-2` | 2026-10-25 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | style(ir): un mut de trop dans le liage d'un Plan déclaré — le seul warning de rag3weaver… |
| `arbre-principal/plafond-l6` | 2026-10-25 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(lectures): le témoin du dense filtré passe au lot de recherche ; le témoin du plafon… |
| `arbre-principal/pli-et-pour-chaque` | 2026-10-26 | non | 7 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): le pli impossible d'un flux est corrigé — le ticket passe dans closed/, sa… |
| `arbre-principal/pli-et-pour-chaque-2` | 2026-10-25 | non | 6 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(dataflow): un manifeste qui cite un sous-graphe absent ou faux se refuse au chargeme… |
| `arbre-principal/pli-et-pour-chaque-3` | 2026-10-26 | non | 7 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): le pli impossible d'un flux est corrigé — le ticket passe dans closed/, sa… |
| `arbre-principal/pli-et-pour-chaque-4` | 2026-10-26 | non | 7 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): le pli impossible d'un flux est corrigé — le ticket passe dans closed/, sa… |
| `arbre-principal/rapport-8-nov` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(arbre): le rapport de session, la nuit du 7 au 8 novembre — le juge et le graphe cit… |
| `arbre-principal/ticket-figements` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): le cas de l'arbre principal au ticket des figements de la lib entière — le… |
| `arbre-principal/ticket-figements-2` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(tickets): les figements de la lib entière — la piste RAG3WEAVER_EMBED_SERVICE (verte… |
| `arbre-principal/ticket-figements-3` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(tickets): les figements de la lib entière — la cause trouvée : un service d'embarque… |
| `arbre-principal/vues-lecture` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(arbre): ce que le moteur doit au modèle des vues — une lecture pour c7 et everything… |
| `arbre-principal/vues-lecture-2` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tickets): le sparse filtré résout ses offsets sur le parent et cherche dans les morc… |
| `arbre-principal/whois` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(memory): whois lit la ligne doing: par valeur bornée, les plus longues d'abord et mas… |
| `arbre-principal/whois-2` | 2026-11-07 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): le témoin du chat rougit à la première exécution après un bâti, ici aussi,… |
| `code-tree-from` | 2026-11-07 | non | 3 | 1 | nom | **effacer** | remplacée par `code-tree-from-2`, atterrie | `0683cc474` (0.28) | wip(code-tree): le poste du témoin nomme son arbre comme add le fait (name = son nom) |
| `code-tree-from-2` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(code-tree): --from par un nom d'arbre ou un chemin de manifeste, quel que soit le nom… |
| `ordre-des-verrous` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(stockage): la compaction des listes CSR ne prend plus le verrou des blocs sous l'inde… |
| `ordre-des-verrous-2` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(stockage): l'ordre des deux verrous du groupe CSR, et pourquoi la compaction lit sa … |
| `ordre-des-verrous-3` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(stockage): l'ordre des deux verrous du groupe CSR, et pourquoi la compaction lit sa … |
| `synchronisation-par-perimetre` | 2026-10-03 | non | 4 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(synchronisation): relecture — marque dans chaque régime, sessions concurrentes, migra… |
| `synchronisation-par-perimetre-2` | 2026-10-03 | non | 8 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(annulation): une colonne nulle partout ne part plus dans la liste typée d'une annulat… |
| `synchronisation-par-perimetre-3` | 2026-10-03 | non | 10 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(synchronisation): un périmètre est borné à la cellule courante |
| `synchronisation-par-perimetre-4` | 2026-10-03 | non | 10 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(synchronisation): un périmètre est borné à la cellule courante |
| `agent/tests-bases-dans-tmp` | 2026-11-08 | non | 5 | 5 | nom | **garder** | 5 commit(s) non atterri(s), tête de sa famille |  | docs(tickets): la forme attendue pour les six témoins de dossier_temporaire — qu'il rende… |
| `arbre-principal/mesure-vues` | 2026-11-07 | non | 2 | 2 | nom | **garder** | 2 commit(s) non atterri(s), tête de sa famille |  | test(mesure): e2e_mesure_vues ouvre sa copie en écriture — PROJECT_GRAPH_CYPHER est refus… |
| `ancien/qui-dans-l-hote` | 2026-10-18 | non | 1 | 1 | nom | **à demander** | probablement reprise sous d'autres commits : `b3418b0a1` ≈ `6e647dcb9` de master (0.92) | `6e647dcb9` (0.92) | feat(démon): les runs en fond se voient dans l'état de l'hôte et tombent avec lui — /etat… |
| `arbre-principal/ancien-l4b-essai` | 2026-11-07 | non | 1 | 1 | nom | **à demander** | nom de brouillon ou d'archive | `77216bb36` (0.14) | wip(lectures): l'essai de L4 laissé non commité le 18 octobre 2026 dans rag3db-l4b — péri… |
| `arbre-principal/graphe-cite-intact` | 2026-11-07 | non | 1 | 1 | nom | **à demander** | probablement reprise sous d'autres commits : `4be482a00` ≈ `2fe3c114f` de master (0.94) | `2fe3c114f` (0.94) | fix(dataflow): les paramètres du parent ne se substituent plus dans le graphe cité — la v… |
| `catalogue-dit-tout` | 2026-10-19 | non | 1 | 1 | nom | **à demander** | dépassée par `catalogue-dit-tout-2`, pas encore atterrie | `6fe94ba9c` (0.16) | feat(catalogue): le catalogue dit tout — chaque chemin d'écriture publie sur catalog, en … |
| `catalogue-dit-tout-2` | 2026-10-19 | non | 1 | 1 | nom | **à demander** | 1 commit(s) non atterri(s), rien depuis le 2026-10-19 | `6fe94ba9c` (0.16) | feat(catalogue): le catalogue dit tout — chaque chemin d'écriture publie sur catalog, en … |

## banc — 45 branches (43 à effacer, 0 à garder, 2 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `banc-a4-temoins` | 2026-10-10 | non | 8 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(banc): l'attente de LockBench se prouve contre le début de la fin de transaction du … |
| `banc-a4-temoins-2` | 2026-10-10 | non | 11 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(banc): SameVectorForEveryRow/SameIndexFirstCopy passe aux probabilistes — vert neuf … |
| `banc-b2-temoins` | 2026-10-11 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(B2): le second processus écrivain attend au fichier — trois témoins, rouges sur mast… |
| `banc-call-supprimes` | 2026-10-11 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(banc): le CALL de la recherche vectorielle ne rend aucun nœud supprimé — le témoin d… |
| `banc-commit-detruit` | 2026-10-18 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): l'index dit le COMMIT après un point de reprise en panne confirmé, avec so… |
| `banc-de-concurrence` | 2026-10-02 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): banc, étape 2 EN COURS — triple vérification, C4 à C9, lanceur Process… |
| `banc-de-concurrence-sur-7928974` | 2026-10-03 | non | 5 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): la transaction en échec en rouge, C5 est une course du moteur, premièr… |
| `banc-fiches-classees` | 2026-10-11 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(banc, en attente): le banc des fiches classées — jeu v1 (10 sujets, 39 fiches dont 8… |
| `banc-fils` | 2026-10-18 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): la famine du bâti d'index à huit fils — tous les acteurs retenus 2,2 s et … |
| `banc-hors-graphe` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence): la mesure de la recherche sur des lignes hors du graphe — 120 000 vect… |
| `banc-hors-graphe-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence): la mesure de la recherche sur des lignes hors du graphe — 120 000 vect… |
| `banc-hors-graphe-3` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence): la mesure de la recherche sur des lignes hors du graphe — 120 000 vect… |
| `banc-hors-graphe-4` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence): le témoin hors graphe tient sa préparation — tampon de 4 Gio, arrêt si… |
| `banc-hors-graphe-mesure` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(tickets): l'état du ticket des lignes hors du graphe dit la mesure faite (8 novembre… |
| `banc-i1-temoins` | 2026-10-11 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(banc, I1): sous l'index tenu en exclusif, celui qui valide le premier écrit le premi… |
| `banc-i2` | 2026-10-18 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): les témoins de I2 joués — deux SET et deux suppressions voisines s'att… |
| `banc-lecteur-journal` | 2026-10-11 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(journal): la liste attendue du scénario connu, relevée sur la première passe (N tabl… |
| `banc-lecteur-journal-avant-amende` | 2026-10-11 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(journal): le lecteur du journal, premier jet (liste attendue à fixer sur la première… |
| `banc-masque` | 2026-10-11 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(banc): le témoin du petit masque à l'échelle du cas réel — 216 000 MENTIONS, pour qu… |
| `banc-merge-et-csv` | 2026-10-04 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(copy): un champ CSV vide entre guillemets se lit comme la chaîne vide, plus comme NULL |
| `banc-merge-et-csv-2` | 2026-10-04 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(copy): un champ CSV vide entre guillemets se lit comme la chaîne vide, plus comme NULL |
| `banc-modele-de-decision` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(banc): un petit modèle de décision, la première mesure — sur 1 328 paires (prompt, f… |
| `banc-modele-de-decision-2` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(banc): la page du modèle de décision compte deux 429, pas un |
| `banc-remarques-de-relecture` | 2026-10-03 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): les remarques de la relecture — raison de chaque rouge épinglée, faux … |
| `banc-remedes-checkpoint` | 2026-10-04 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(transactions): une validation au seuil reporte son point de reprise sans l'attendre |
| `banc-reprise-ordre` | 2026-10-04 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(reprise): le journal supprimé avant le fichier fantôme, et les pages rejouées synchro… |
| `banc-reprise-sans-arret` | 2026-10-18 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): les témoins du point de reprise qui ne fige personne, joués rouges tro… |
| `banc-retenue` | 2026-10-18 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): SCENARIO_DDL_THREADS règle le nombre de fils des DDL du scénario |
| `banc-rlimit-relatif` | 2026-10-18 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence): le fils de RolledBackCopyThenCheckpoint borne son tas à 3 Gio au-delà … |
| `banc-rlimit-relatif-2` | 2026-10-18 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): le fils de RolledBackCopyThenCheckpoint borne son tas à 3 Gio au-delà … |
| `banc-rlimit-relatif-3` | 2026-10-18 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): le fils de RolledBackCopyThenCheckpoint borne son tas à 3 Gio au-delà … |
| `banc-rollback` | 2026-10-18 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): les lignes CHECKPOINT et BEGIN du témoin s'écrivent après leur résultat |
| `banc-same-rows-first-copy` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): SameRowsFirstCopy passe de known_red à probabilistic — sorti vert une … |
| `banc-scan-filtre` | 2026-10-12 | non | 1 | 1 | nom | **effacer** | remplacée par `banc-scan-filtre-2`, atterrie |  | test(banc): le balayage filtré d'une colonne de chaînes ou de listes à cheval sur deux se… |
| `banc-scan-filtre-2` | 2026-10-12 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(banc): le balayage filtré d'une colonne de chaînes à cheval sur deux segments — le t… |
| `banc-temoin-e11` | 2026-10-18 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): sur une table indexée par HNSW, un écrivain qui garde sa transaction o… |
| `banc-temoins-v22` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): deux témoins de plus pour V2-2 — le lecteur ouvert voit encore une lig… |
| `banc-ticket-juge` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(tickets): le juge du rappel des hooks coupe sa réponse sous un plafond de 400 jetons… |
| `banc-vingt-clients` | 2026-10-18 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence): le scénario sans avertissement d'initialisation |
| `banc-vingt-clients-2` | 2026-10-18 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence): le scénario sans avertissement d'initialisation |
| `banc-vingt-clients-3` | 2026-10-18 | non | 5 | 2 | nom | **effacer** | remplacée par `banc-vingt-clients-5`, atterrie |  | test(concurrence): le témoin du BEGIN refusé en known_red, joué rouge (signal 11) ; ses l… |
| `banc-vingt-clients-4` | 2026-10-18 | non | 6 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): le scénario trace aussi la fin de chaque DDL, pour situer les lignes [… |
| `banc-vingt-clients-5` | 2026-10-18 | non | 8 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(concurrence): le scénario garde le point de reprise automatique jusqu'à la minute 6 … |
| `archive/banc-elagage-hnsw` | 2026-10-10 | non | 2 | 1 | nom | **à demander** | nom de brouillon ou d'archive | `c841d507c` (0.17) | wip(hnsw): la taille d'un vecteur sans symbole non exporté ; le témoin des vecteurs ident… |
| `banc-elagage-en-cours` | 2026-10-10 | non | 2 | 1 | nom | **à demander** | 1 commit(s) non atterri(s), rien depuis le 2026-10-10 | `c841d507c` (0.17) | wip(hnsw): la taille d'un vecteur sans symbole non exporté ; le témoin des vecteurs ident… |

## codeparsers — 28 branches (26 à effacer, 2 à garder, 0 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `codeparsers-champs-test` | 2026-10-03 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(code): un scope de test le dit dans ses champs — test_role, test_certainty, test_name |
| `codeparsers-mentions-type` | 2026-10-03 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(code): une mention typée choisit sa méthode parmi les homonymes — PROPOSITION pour l… |
| `codeparsers-pile` | 2026-10-03 | non | 5 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | chore(codeparsers): pointeur sur a569417 — imports Rust, Python et C++, et leurs biblioth… |
| `codeparsers-pointeur` | 2026-10-03 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | chore(codeparsers): pointeur sur f0faa82 — résolution déterministe, fausses arêtes retiré… |
| `codeparsers-pointeur-3` | 2026-10-03 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | chore(codeparsers): pointeur sur 9a1fae3 — les appels sur une variable dont le type se lit |
| `codeparsers-pointeur-4` | 2026-10-03 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | chore(codeparsers): pointeur sur 1f9d653 — les fonctions d'un module Rust deviennent des … |
| `codeparsers-pointeur-5` | 2026-10-03 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | chore(codeparsers): pointeur sur 137b9d2 — chaque scope dit s'il est un test |
| `codeparsers-pointeur-6` | 2026-10-03 | non | 4 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | chore(codeparsers): pointeur sur 594fc61 — describe, it et test deviennent des scopes de … |
| `codeparsers-pointeur-7` | 2026-10-03 | non | 5 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | chore(codeparsers): pointeur sur a569417 — imports Rust, Python et C++, et leurs biblioth… |
| `codeparsers-pointeur-8` | 2026-10-03 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | chore(codeparsers): pointeur sur 56d039f — appels typés par un champ ou un retour déclaré |
| `codeparsers-rendez-vous-champs` | 2026-10-03 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(code): un accès de champ ne prend pas de rendez-vous avec une fonction homonyme — PRO… |
| `codeparsers-transport` | 2026-10-03 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(code): les arêtes d'usage portent comment la source se sert de la cible, et à quelle… |
| `codeparsers/allegement-boucle-langages` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | perf(code): le garde-fou des langages lit la famille une fois par scope, et ne recopie le… |
| `codeparsers/docs-graine-lsp` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(codeparsers): page 10 — les outils d'un serveur de langage en gabarits sur notre gra… |
| `codeparsers/garde-pointeur` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tickets): la marque « nom » relie un appel de méthode standard à un homonyme — ouver… |
| `codeparsers/garde-pointeur-2` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tickets): la marque « nom » relie un appel de méthode standard à un homonyme — ouver… |
| `codeparsers/garde-pointeur-3` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): la marque « nom » relie un appel de méthode standard à un homonyme — ouver… |
| `codeparsers/includes` | 2026-10-25 | non | 13 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | perf(code): les déclarants se lisent seulement dans les en-têtes des fermetures du lot (I… |
| `codeparsers/includes-2` | 2026-10-26 | non | 17 | 1 | nom | **effacer** | remplacée par `codeparsers/includes-4`, atterrie |  | docs(tickets): la lecture des déclarants coûte autant que le choix des cibles, pour un lo… |
| `codeparsers/includes-3` | 2026-11-07 | non | 19 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(codeparsers): la page 10 aux chiffres du juge strict (218 justes, 14 fausses sur 300… |
| `codeparsers/includes-4` | 2026-11-07 | non | 19 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(codeparsers): la page 10 aux chiffres du juge strict (218 justes, 14 fausses sur 300… |
| `codeparsers/outline` | 2026-10-25 | non | 5 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(plan): les commentaires de check et de lire_le_plan reviennent au-dessus de leur fon… |
| `codeparsers/tree-sitter-027` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | build(codeparsers): tree-sitter 0.27 et les grammaires les plus récentes — ce que la sond… |
| `codeparsers/tree-sitter-027-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | build(codeparsers): tree-sitter 0.27 et les grammaires les plus récentes — ce que la sond… |
| `codeparsers/tree-sitter-027-3` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(e2e): la sonde de PostgreSQL lit les deux formes de RAG3WEAVER_PG, l'URL et les mots-… |
| `codeparsers/tree-sitter-027-4` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(e2e): la sonde de PostgreSQL lit les deux formes de RAG3WEAVER_PG, l'URL et les mots-… |
| `codeparsers/oracle-clangd` | 2026-10-25 | non | 1 | 1 | nom | **garder** | 1 commit(s) non atterri(s), tête de sa famille | `8c5b0894d` (0.19) | feat(scripts): clangd en oracle de mesure des arêtes C++ — jamais dans le produit, il jug… |
| `codeparsers/oracle-lsp` | 2026-10-25 | non | 2 | 2 | nom | **garder** | 2 commit(s) non atterri(s), tête de sa famille |  | docs(oracle): Rust rendu — 253 justes, 19 faux, 27 muets sur 300 ; les 162 muets de la pr… |

## embarquement — 2 branches (1 à effacer, 0 à garder, 1 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `embarqueur-absent-portes` | 2026-10-11 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(backend): le reçu dit pourquoi l'embarqueur manque — rien de déclaré (le donner), ser… |
| `embarqueur-absent` | 2026-10-10 | non | 3 | 1 | nom | **à demander** | probablement reprise sous d'autres commits : `eaa5dbe19` ≈ `700324232` de master (0.67) | `700324232` (0.67) | wip(état de l'index): Level::NotDeclared — une entité sans vecteurs déclarés le dit, au l… |

## everything declarative (I) — 64 branches (52 à effacer, 2 à garder, 10 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `chat-code-1` | 2026-10-25 | non | 3 | 2 | nom | **effacer** | remplacée par `chat-code-3`, atterrie | `ae127d634` (0.62) | wip(chat): le témoin du chat qui code, sur Gemini — jugé sur le résultat, avec sa mesure |
| `chat-code-2` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(chat): le chat code dans une tâche, sur un dépôt étranger — il lit, cherche et édite… |
| `chat-code-3` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(chat): le chat code dans une tâche, sur un dépôt étranger — il lit, cherche et édite… |
| `commandes-declarees` | 2026-11-08 | non | 1 | 1 | nom | **effacer** | remplacée par `commandes-declarees-6`, atterrie | `ad496c462` (0.45) | wip(équipier): les commandes par la porte déclarée — run_command gardé dans une tâche don… |
| `commandes-declarees-2` | 2026-11-08 | non | 1 | 1 | nom | **effacer** | remplacée par `commandes-declarees-6`, atterrie | `ad496c462` (0.45) | wip(équipier): les commandes par la porte déclarée — run_command gardé dans une tâche don… |
| `commandes-declarees-3` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(équipier): les commandes par la porte déclarée — dans une tâche dont le workspace a … |
| `commandes-declarees-4` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(équipier): les commandes par la porte déclarée — dans une tâche dont le workspace a … |
| `commandes-declarees-5` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(équipier): les commandes par la porte déclarée — dans une tâche dont le workspace a … |
| `commandes-declarees-6` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(équipier): les commandes par la porte déclarée — dans une tâche dont le workspace a … |
| `equipier-1` | 2026-10-25 | non | 16 | 12 | nom | **effacer** | remplacée par `equipier-3`, atterrie | `96d5060fe` (0.17) | docs(everything declarative): rapport et knowledge dump du soir — le juge des outils, le … |
| `equipier-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(équipier): un agent sur Gemini qui accomplit une tâche seul, par pas, et parle à un … |
| `equipier-3` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(équipier): un agent sur Gemini qui accomplit une tâche seul, par pas, et parle à un … |
| `equipier-attente` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(équipier): l'état d'attente — une question ou une commande demandée sans réponse sus… |
| `equipier-attente-2` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(équipier): l'état d'attente — une question ou une commande demandée sans réponse sus… |
| `equipier-bac-a-sable` | 2026-11-08 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(commandes): le bac à sable d'une tâche de code joue cargo — la chaîne Rust en lectur… |
| `equipier-bac-a-sable-2` | 2026-11-08 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(commandes): le bac à sable d'une tâche de code joue cargo — la chaîne Rust en lectur… |
| `equipier-bac-a-sable-3` | 2026-11-08 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(commandes): le bac à sable d'une tâche de code joue cargo — la chaîne Rust en lectur… |
| `equipier-bac-a-sable-4` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(commandes): le bac à sable d'une tâche de code joue cargo — la chaîne Rust en lectur… |
| `equipier-etanche` | 2026-11-07 | non | 8 | 4 | nom | **effacer** | remplacée par `equipier-etanche-4`, atterrie | `75d8d6d1e` (0.33) | docs(équipier): la page dit ce que le lot 1 pose (la politique au manifeste, le modèle pr… |
| `equipier-etanche-2` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(équipier): l'étanchéité au moteur — le pas prend le backend à chaque appel d'outil e… |
| `equipier-etanche-2-wip` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | wip(équipier): pending rend l'auteur de chaque item (from), pour le pont d'archi |
| `equipier-etanche-3` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(équipier): l'étanchéité au moteur — le pas prend le backend à chaque appel d'outil e… |
| `equipier-etanche-4` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(équipier): l'étanchéité au moteur — le pas prend le backend à chaque appel d'outil e… |
| `equipier-fil-par-pas` | 2026-11-08 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(hôte): un équipier en route retient l'arrêt d'inactivité — au travail ou en attente d… |
| `equipier-fil-par-pas-2` | 2026-11-08 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(équipier): le fil par pas suit la demande en cours — la dernière reçue, et les step_d… |
| `equipier-jamais-muet` | 2026-11-08 | non | 1 | 1 | nom | **effacer** | remplacée par `equipier-jamais-muet-2`, atterrie | `aff3611fb` (0.88) | wip(équipier): jamais en silence — un pas sans step_done écrit sa réponse finale en repor… |
| `equipier-jamais-muet-2` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(équipier): jamais en silence — un pas sans step_done écrit sa réponse finale en repor… |
| `equipier-quota` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(équipier): un refus du fournisseur n'est pas un pas raté — 429 et 503 attendent une … |
| `equipier-reprise` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(équipier): la reprise après un arrêt de l'hôte — au démarrage, l'équipier dont la de… |
| `equipier-reprise-2` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(équipier): la reprise après un arrêt de l'hôte — au démarrage, l'équipier dont la de… |
| `equipier-reprise-3` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(équipier): la reprise après un arrêt de l'hôte — au démarrage, l'équipier dont la de… |
| `journal-poids-1` | 2026-10-25 | non | 5 | 2 | nom | **effacer** | remplacée par `journal-poids-2`, atterrie | `82db9a463` (0.85) | wip(journal): les mesures de la page 06 |
| `journal-poids-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(hôte): le journal des runs borné par poids (5 Gio, tous backends) et par âge (90 jou… |
| `journal-rendu-1` | 2026-10-25 | non | 3 | 2 | nom | **effacer** | remplacée par `journal-rendu-2`, atterrie | `c4472d61a` (0.52) | test(hôte): le rendu journalisé est ce que l'appelant a reçu, octet pour octet (read_file… |
| `journal-rendu-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(hôte): le rendu journalisé d'un appel /call est entier — une borne ne coupe que les é… |
| `juge-1` | 2026-10-25 | non | 3 | 2 | nom | **effacer** | remplacée par `juge-5`, atterrie |  | docs(everything declarative): le juge de session — la forme de la fiche relue par la mémo… |
| `juge-2` | 2026-10-25 | non | 17 | 11 | nom | **effacer** | remplacée par `juge-5`, atterrie | `4b64a98e5` (0.30) | wip(juge): le modèle du juge à part (with_juge_model, Vertex par défaut) ; le témoin sur … |
| `juge-3` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(juge): le juge de session — un agent déclaré qui relit un run, rejoue ce qui lit, ou… |
| `juge-4` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(juge): le juge de session — un agent déclaré qui relit un run, rejoue ce qui lit, ou… |
| `juge-5` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(juge): le juge de session — un agent déclaré qui relit un run, rejoue ce qui lit, ou… |
| `juge-outils-1` | 2026-10-25 | non | 5 | 3 | nom | **effacer** | remplacée par `juge-outils-2`, atterrie | `bdf5929c7` (0.48) | wip(juge): la vue des bilans trouve aussi les fiches d'outils, et les apports par le nom … |
| `juge-outils-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(juge): le juge des outils — deux rapports (les outils, puis la session), des défauts… |
| `juge-plafond` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | wip(juge): bilans.max_per_hour — au plus N runs jugés par heure glissante et par backend … |
| `juge-plafond-2` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(juge): bilans.max_per_hour — au plus N runs jugés par heure glissante et par backend… |
| `outline-json-1` | 2026-10-25 | non | 5 | 3 | nom | **effacer** | remplacée par `outline-json-4`, atterrie | `cbb41d312` (0.56) | wip(outline): en json=rows, text porte l'objet — la réponse reste un objet, la ligne de f… |
| `outline-json-2` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(ide): la vue outline — la fiche d'un fichier montre le plan rendu par outline, chaqu… |
| `outline-json-3` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(ide): la vue outline — la fiche d'un fichier montre le plan rendu par outline, chaqu… |
| `outline-json-4` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(ide): la vue outline — la fiche d'un fichier montre le plan rendu par outline, chaqu… |
| `raisonnement-1` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(llm): R1 — le budget de raisonnement se déclare par modèle (models.llm.reasoning), a… |
| `raisonnement-2` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(llm): R1 — le budget de raisonnement se déclare par modèle (models.llm.reasoning), a… |
| `raisonnement-3` | 2026-10-25 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(llm): R1 — le budget de raisonnement se déclare par modèle (models.llm.reasoning), a… |
| `raisonnement-4` | 2026-10-25 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(llm): R1 — le budget de raisonnement se déclare par modèle (models.llm.reasoning), a… |
| `atelier-attach-7` | 2026-11-08 | non | 1 | 1 | nom | **garder** | 1 commit(s) non atterri(s), tête de sa famille | `754e863a9` (0.13) | feat(atelier): ouvrir un worktree existant par un mot — attach_worktree et list_worktrees… |
| `juge-genres` | 2026-11-08 | non | 1 | 1 | nom | **garder** | 1 commit(s) non atterri(s), tête de sa famille |  | test(juge): le fichier obligatoire se défend sous deux genres — le témoin du juge des out… |
| `atelier-attach` | 2026-11-08 | non | 2 | 1 | nom | **à demander** | dépassée par `atelier-attach-7`, pas encore atterrie | `754e863a9` (0.13) | feat(atelier): ouvrir un worktree existant par un mot — attach_worktree et list_worktrees… |
| `atelier-attach-2` | 2026-11-08 | non | 1 | 1 | nom | **à demander** | dépassée par `atelier-attach-7`, pas encore atterrie | `754e863a9` (0.13) | feat(atelier): ouvrir un worktree existant par un mot — attach_worktree et list_worktrees… |
| `atelier-attach-3` | 2026-11-08 | non | 1 | 1 | nom | **à demander** | dépassée par `atelier-attach-7`, pas encore atterrie | `754e863a9` (0.13) | feat(atelier): ouvrir un worktree existant par un mot — attach_worktree et list_worktrees… |
| `atelier-attach-4` | 2026-11-08 | non | 1 | 1 | nom | **à demander** | dépassée par `atelier-attach-7`, pas encore atterrie | `754e863a9` (0.13) | feat(atelier): ouvrir un worktree existant par un mot — attach_worktree et list_worktrees… |
| `atelier-attach-5` | 2026-11-08 | non | 1 | 1 | nom | **à demander** | dépassée par `atelier-attach-7`, pas encore atterrie | `754e863a9` (0.13) | feat(atelier): ouvrir un worktree existant par un mot — attach_worktree et list_worktrees… |
| `atelier-attach-6` | 2026-11-08 | non | 1 | 1 | nom | **à demander** | dépassée par `atelier-attach-7`, pas encore atterrie | `754e863a9` (0.13) | feat(atelier): ouvrir un worktree existant par un mot — attach_worktree et list_worktrees… |
| `commande-en-fond` | 2026-10-10 | non | 3 | 2 | nom | **à demander** | dépassée par `commande-en-fond-3`, pas encore atterrie | `3e7d88c18` (0.62) | fix(commande-en-fond): la lib verte — un dossier de journaux par processus hors run, Tail… |
| `commande-en-fond-2` | 2026-10-10 | non | 5 | 2 | nom | **à demander** | dépassée par `commande-en-fond-3`, pas encore atterrie | `3e7d88c18` (0.62) | feat(commande en fond): branchée sur la boucle — la poignée de l'appel, la fin postée dan… |
| `commande-en-fond-3` | 2026-10-10 | non | 9 | 2 | nom | **à demander** | probablement reprise sous d'autres commits : `c2e2cf4b4` ≈ `3e7d88c18` de master (0.62) | `3e7d88c18` (0.62) | fix(commande en fond): la mort confiée à la portée après le registre, et par la commande … |
| `temoins-plan-declare` | 2026-10-18 | non | 3 | 1 | nom | **à demander** | 1 commit(s) non atterri(s), rien depuis le 2026-10-18 |  | test(plan déclaré): le moment de chaque refus, fixé avec l'arbre principal — au chargemen… |

## hooks, boîte et pont (archi, hooks-essai) — 29 branches (27 à effacer, 2 à garder, 0 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `boite-adressee` | 2026-11-07 | non | 4 | 3 | nom | **effacer** | remplacée par `boite-adressee-2`, atterrie | `b9b47ee12` (0.83) | wip(équipier): le témoin de la boîte adressée ne compte plus le numéro du pas d'après l'a… |
| `boite-adressee-2` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(équipier): la boîte adressée — un id stable par ligne, to et reply_to, les champs de… |
| `hooks-3-ter-c` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(hooks): 3 ter (c) — la requête laisse tomber les pronoms, les mots d'adresse et les v… |
| `hooks-3-ter-d` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(hooks): 3 ter (d) — le client du hook donne à l'hôte son échéance (deadline_ms à 95 … |
| `hooks-3-ter-d-b` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | feat(hooks): 3 ter (d) — le client du hook donne à l'hôte son échéance (deadline_ms à 95 … |
| `hooks-3-ter-e` | 2026-11-08 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(hooks): 3 ter (e) — les fiches du juge (via: juge, sinon bilan : / outil :) ne sont p… |
| `hooks-3-ter-e-b` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(tickets): la sonde de débit d'embarquement rougit dans la lib entière — un témoin de… |
| `hooks-3-ter-f` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | chore(hooks): 3 ter (f) — recall_min_score retiré du réglage (la logique ne le lisait plu… |
| `hooks-3-ter-f-b` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | chore(hooks): 3 ter (f) — recall_min_score retiré du réglage (la logique ne le lisait plu… |
| `hooks-3-ter-g` | 2026-11-08 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(hooks): 3 ter (g) — le juge sait quelle session il sert (sa clé dans la consigne et d… |
| `hooks-3-ter-g-b` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | fix(hooks): 3 ter (g) — le juge sait quelle session il sert (sa clé dans la consigne et d… |
| `hooks-3-ter-juge` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(hooks): 3 ter (b) — le rappel qui dépasse son budget finit en fond (hook --rappel-di… |
| `hooks-3-ter-juge-2` | 2026-11-07 | non | 5 | 2 | nom | **effacer** | remplacée par `hooks-3-ter-juge-3`, atterrie |  | feat(hooks): 3 ter (b) — le rappel qui dépasse son budget finit en fond (hook --rappel-di… |
| `hooks-3-ter-juge-3` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(hooks): 3 ter (b) — le rappel lit les signaux par ligne (le hook prenait le score du … |
| `hooks-claude-code-1` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(hooks): la commande mince des hooks de Claude Code — rag3weaver-backend hook : l'évé… |
| `hooks-claude-code-2` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(hooks): les blocs de lecture — SessionStart (qui je suis, ce que je tiens, le plan e… |
| `hooks-claude-code-3` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(hooks): les écritures mécaniques et la correction à la volée — PostCompact range le … |
| `hooks-claude-code-4` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(hooks): lot 3 bis — la table des serveurs du run, le témoin sur un vrai gabarit memo… |
| `hooks-claude-code-3b` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(hooks): la fiche d'une session se trouve par la valeur nue session:<uuid> de genre ha… |
| `hooks-essai-trois-lots` | 2026-11-07 | non | 27 | 0 | nom | **effacer** | contenu dans master (même patch) |  | tickets: l'État des huit tickets fermés dit ce qui les ferme (le correctif, son témoin, r… |
| `hooks-essai-trois-lots-2` | 2026-11-07 | non | 27 | 0 | nom | **effacer** | contenu dans master (même patch) |  | tickets: l'État des huit tickets fermés dit ce qui les ferme (le correctif, son témoin, r… |
| `hooks-pont-5` | 2026-11-07 | non | 1 | 1 | nom | **effacer** | remplacée par `hooks-pont-9`, atterrie | `1a0c8d64d` (0.54) | feat(hooks): le pont, première moitié — mail send\|reply\|pending\|seen, les fils Codex (… |
| `hooks-pont-6` | 2026-11-07 | non | 4 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(hooks): le pont — le canal (§ 4 bis : contrat, identité par le pid de claude, mode p… |
| `hooks-pont-7` | 2026-11-07 | non | 8 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tickets): un test de la lib qui parle au service d'embarquement attend un service ré… |
| `hooks-pont-8` | 2026-11-07 | non | 8 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tickets): un test de la lib qui parle au service d'embarquement attend un service ré… |
| `hooks-pont-9` | 2026-11-08 | non | 8 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): un test de la lib qui parle au service d'embarquement attend un service ré… |
| `hooks-pont-7-b` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(tickets): un test de la lib qui parle au service d'embarquement attend un service ré… |
| `hooks-3-ter` | 2026-11-07 | non | 4 | 2 | nom | **garder** | 2 commit(s) non atterri(s), tête de sa famille |  | docs(tickets): le ticket de l'hôte sous les ouverts du produit |
| `hooks-essai` | 2026-11-08 | non | 34 | 30 | nom | **garder** | la branche de la session d'essai des hooks (worktree rag3db-hooks-essai) |  | Merge remote-tracking branch 'origin/master' into hooks-essai |

## moteur (cœur C++ ou hotfixs) — 147 branches (134 à effacer, 5 à garder, 8 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `a2-remappage-des-offsets-au-commit` | 2026-10-03 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(transaction): au commit, un offset définitif n'est plus pris pour une ligne locale |
| `a5-suppression-sure-entre-fils` | 2026-10-03 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(stockage): les informations de version et l'index des relations en mémoire sont proté… |
| `a5-suppression-sure-entre-fils-2` | 2026-10-03 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(transaction): un seul écrivain qui supprime pendant que des lecteurs lisent — le tém… |
| `annulation-tampon-plein` | 2026-11-07 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): dans le témoin de l'annulation d'un COPY sous tampon plein, B remplit le tamp… |
| `bloc-annule` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(stockage): un bloc entièrement annulé garde ses versions, lignes marquées annulées — … |
| `bloc-annule-2` | 2026-11-07 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(transaction): le témoin du bloc annulé rejoue la course telle qu'elle est — le compt… |
| `bloc-annule-3` | 2026-11-07 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(transaction): le témoin du bloc annulé rejoue la course telle qu'elle est — le compt… |
| `bloc-annule-4` | 2026-11-07 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(transaction): le témoin du bloc annulé rejoue la course telle qu'elle est — le compt… |
| `champ-nul-d-une-liste-de-structures` | 2026-10-03 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(api): une liste de lignes en paramètre, un champ nul partout — par CREATE, ça passe |
| `ci-cassert-geo` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(geo): rtree.cpp n'inclut plus <cassert>, qu'il n'employait pas — le contrôle de la CI… |
| `colonnes-stables` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): les numéros de colonnes ne changent jamais — une colonne supprimée laisse… |
| `colonnes-stables-2` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): les numéros de colonnes ne changent jamais — une colonne supprimée laisse… |
| `colonnes-stables-3` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): les numéros de colonnes ne changent jamais — une colonne supprimée laisse… |
| `colonnes-stables-4` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): les numéros de colonnes ne changent jamais — une colonne supprimée laisse… |
| `colonnes-stables-5` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): les numéros de colonnes ne changent jamais — une colonne supprimée laisse… |
| `commit-transaction-partie` | 2026-10-19 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(transaction): un COMMIT dont le point de reprise échoue après la validation n'annule … |
| `controle-de-bati` | 2026-10-10 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(extensions): le témoin du rejeu charge l'extension de ce bâti avant de chercher |
| `controle-de-bati-2` | 2026-10-10 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(extensions): le témoin du rejeu charge l'extension de ce bâti avant de chercher |
| `copie-journalisee-double` | 2026-10-11 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(stockage): les exceptions ALP d'un segment lu à un décalage non nul du vecteur sont r… |
| `create-concurrently` | 2026-10-19 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence): le témoin d'acceptation de (A), le CREATE concurrent — sous quarante é… |
| `create-concurrently-2` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence): le témoin d'acceptation de (A), le CREATE concurrent — sous quarante é… |
| `create-concurrently-etat` | 2026-10-25 | non | 7 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(concurrence, vector): les témoins voisins suivent le CREATE concurrent — la reconstr… |
| `create-concurrently-etat-2` | 2026-10-25 | non | 8 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(stockage): le commentaire de reclaimDroppedIndexes suit V2-1c — aucun écrivain ouver… |
| `create-concurrently-etat-3` | 2026-10-25 | non | 8 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(stockage): le commentaire de reclaimDroppedIndexes suit V2-1c — aucun écrivain ouver… |
| `create-concurrently-etat-4` | 2026-10-25 | non | 8 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(stockage): le commentaire de reclaimDroppedIndexes suit V2-1c — aucun écrivain ouver… |
| `create-concurrently-etat-5` | 2026-10-25 | non | 8 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(stockage): le commentaire de reclaimDroppedIndexes suit V2-1c — aucun écrivain ouver… |
| `create-concurrently-etat-6` | 2026-10-25 | non | 8 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | docs(stockage): le commentaire de reclaimDroppedIndexes suit V2-1c — aucun écrivain ouver… |
| `create-concurrently-etat-7` | 2026-10-26 | non | 8 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | docs(stockage): le commentaire de reclaimDroppedIndexes suit V2-1c — aucun écrivain ouver… |
| `csr-listes-temoin` | 2026-11-07 | non | 9 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(stockage): le cas du COPY refusé faute de tampon suit le filet du crate — au refus, … |
| `csr-listes-temoin-2` | 2026-11-07 | non | 10 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(banc), docs(tickets): le témoin des deux sens d'une table de liens entre au banc par… |
| `elagage-hnsw-2-avant-rebase` | 2026-10-10 | non | 4 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | docs(tickets): la ligne lointaine et les COPY successifs, fermés par l'élagage redressé —… |
| `etendue-du-fichier` | 2026-10-10 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(stockage): le fichier de la base a une étendue connue — les pages d'un COPY tué ou re… |
| `fils-proportionnes` | 2026-10-12 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(processeur): les fils d'une tâche proportionnés à la cardinalité estimée (page coeur… |
| `fin-de-journal-dechiree` | 2026-10-02 | non | 4 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(wal): en lecture seule, une fin incomplète ne se dit pas |
| `format-41` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): la version de format est posée à la main, l'en-tête de base et le journal… |
| `format-41-2` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): la version de format est posée à la main, l'en-tête de base et le journal… |
| `format-41-3` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(stockage): la version de format est posée à la main, l'en-tête de base et le journal… |
| `g1-decalages-provisoires` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(transaction): témoin rouge attendu — un COPY de relations vers des nœuds locaux gard… |
| `g2-blobs` | 2026-11-08 | non | 7 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(copy): le budget en octets du COPY porte sur ce qui croît au-delà d'un groupe vide — … |
| `g2-blobs-2` | 2026-11-08 | non | 7 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(copy): le budget en octets du COPY porte sur ce qui croît au-delà d'un groupe vide — … |
| `g2-copy-refuse-a` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(copy), docs(tickets): ce qui suit, dans la même session, un COPY refusé faute de tam… |
| `hnsw-point-d-entree-apres-rollback` | 2026-10-03 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(vector): un DELETE annulé par ROLLBACK ne laisse plus l'index HNSW muet |
| `hnsw-point-d-entree-apres-suppression` | 2026-10-03 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(vector): après des suppressions, l'index HNSW ne répondait plus — trois défauts de no… |
| `ignore-errors-cle-disque` | 2026-10-10 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(journal): la clé 17 en double garde l'une ou l'autre de ses deux lignes |
| `instruction-en-echec` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(transactions): une instruction en échec ne condamne plus une transaction en lecture s… |
| `instruction-en-echec-2` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(transactions): une instruction en échec ne condamne plus une transaction en lecture s… |
| `instruction-en-echec-3` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(transactions): une instruction en échec ne condamne plus une transaction en lecture s… |
| `instruction-en-echec-4` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(transactions): une instruction en échec ne condamne plus une transaction en lecture s… |
| `instruction-en-echec-5` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(transactions): une instruction en échec ne condamne plus une transaction en lecture s… |
| `locks-via` | 2026-10-11 | non | 7 | 0 | sujet | **effacer** | contenu dans master (même patch) |  | feat(verrous): la marque via sur LOCKS (champ ou appel), le titre « Verrous pris sur le c… |
| `preparees-portent-leur-requete` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(main): une requête préparée porte sa requête liée et la rend en mourant — le magasin … |
| `preparees-portent-leur-requete-2` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(main): une requête préparée porte sa requête liée et la rend en mourant — le magasin … |
| `preparees-portent-leur-requete-3` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(main): une requête préparée porte sa requête liée et la rend en mourant — le magasin … |
| `preparees-portent-leur-requete-4` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(main): une requête préparée porte sa requête liée et la rend en mourant — le magasin … |
| `projection-lecture-seule` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(transactions): un graphe projeté est un objet de la connexion, pas une écriture — PRO… |
| `projection-lecture-seule-2` | 2026-10-26 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(transactions): un graphe projeté est un objet de la connexion, pas une écriture — PRO… |
| `projection-lecture-seule-3` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(transactions): un graphe projeté est un objet de la connexion, pas une écriture — PRO… |
| `projection-lecture-seule-4` | 2026-11-07 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(transactions): un graphe projeté est un objet de la connexion, pas une écriture — PRO… |
| `projection-lecture-seule-5` | 2026-11-07 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(graphe projeté): DROP_PROJECTED_GRAPH honore skip_if_not_exists — l'option du produit… |
| `projection-lecture-seule-6` | 2026-11-07 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(graphe projeté): DROP_PROJECTED_GRAPH honore skip_if_not_exists — l'option du produit… |
| `projection-lecture-seule-7` | 2026-11-07 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): DeathDuringDeletesAndVectorSearchKeepsTheIndexExact à probabilistic.txt — une… |
| `projection-lecture-seule-8` | 2026-11-07 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): DeathDuringDeletesAndVectorSearchKeepsTheIndexExact à probabilistic.txt — une… |
| `projection-lecture-seule-9` | 2026-11-07 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): DeathDuringDeletesAndVectorSearchKeepsTheIndexExact à probabilistic.txt — une… |
| `rattrapage-points-d-entree` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(vector): le rattrapage d'un CREATE sort ensemble les lignes déplacées et les supprimé… |
| `rattrapage-points-d-entree-2` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(vector): le rattrapage d'un CREATE sort ensemble les lignes déplacées et les supprimé… |
| `refus-apres-point-de-reprise-echoue` | 2026-10-03 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(point de reprise): après un point de reprise échoué, la base refuse tout jusqu'à sa r… |
| `rejeu-relations-copiees` | 2026-11-08 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(banc): LockBench.AutoCheckpointIsNotPostponedForeverUnderWritersWithoutAGap dans pro… |
| `reprise-apres-panne-index-cle-primaire` | 2026-10-02 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(stockage): la panne pendant la phase de stockage, à chaque allocation du point de re… |
| `statistiques` | 2026-10-10 | non | 3 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(coeur-cpp-tickets): la page 03 et le rapport à l'arrêt du nettoyage du disque |
| `statistiques-2` | 2026-10-10 | non | 4 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(statistiques): le recalage sur les lignes vivantes, pour les groupes de nœuds seuleme… |
| `statistiques-3-avant-rebase` | 2026-10-10 | non | 5 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | docs(coeur-cpp-tickets): la mesure de CALL analyze sur 100 000 lignes, et la reprise sur … |
| `systemconfig-abi` | 2026-10-11 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(api): la signature d'avant B2 du constructeur de SystemConfig reste, en surcharge qui… |
| `systemconfig-abi-224357` | 2026-10-11 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(api): la signature d'avant B2 du constructeur de SystemConfig reste, en surcharge qui… |
| `t0-transaction-en-echec` | 2026-10-03 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(transaction): une erreur de syntaxe met le bloc en échec comme les autres |
| `test-c-api-octets-magiques` | 2026-10-03 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(c_api): GetStorageVersion lit autant d'octets magiques que le moteur en écrit |
| `tests-tmp-par-passe` | 2026-10-11 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(tests): les bases gtest du moteur vivent sous un parent par passe, /tmp/rag3db/<pid d… |
| `v21b-balayages` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(stockage): un balayage tient ses blocs — l'instantané de la liste en propriété parta… |
| `v21b-balayages-2` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(stockage): un balayage tient ses blocs — l'instantané de la liste en propriété parta… |
| `v21b-bloc-neuf` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): au point de reprise, un segment de colonne changé est rebâti sur des page… |
| `v21b-bloc-neuf-2` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): au point de reprise, un segment de colonne changé est rebâti sur des page… |
| `v21b-bloc-neuf-3` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(stockage): au point de reprise, un segment de colonne changé est rebâti sur des page… |
| `v21b-bloc-neuf-4` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(stockage): au point de reprise, un segment de colonne changé est rebâti sur des page… |
| `v21b-compteur` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(profil): le point de reprise dit combien d'octets il écrit |
| `v21b-compteur-2` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(profil): le point de reprise dit combien d'octets il écrit |
| `v21b-compteur-3` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(profil): le point de reprise dit combien d'octets il écrit |
| `v21c-porte` | 2026-10-25 | non | 5 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): ReadDuringACheckpointDoesNotWait sort de known_red — V2-1c le rend vert (lect… |
| `v22-generations` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(stockage): les générations précédentes se retirent à chaque point de reprise, dans to… |
| `v22-generations-2` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(e2e): CheckpointTimeoutErrorTest dit ce qui est depuis V2-2 — un CHECKPOINT derrière… |
| `v22-generations-3` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(e2e): CheckpointTimeoutErrorTest dit ce qui est depuis V2-2 — un CHECKPOINT derrière… |
| `v22-retrait` | 2026-10-25 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): AutoCheckpointPostponedWhileATransactionStaysOpen contrôle que son écrivain o… |
| `v22-retrait-2` | 2026-10-26 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): AutoCheckpointPostponedWhileATransactionStaysOpen contrôle que son écrivain o… |
| `v22-retrait-3` | 2026-11-07 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): AutoCheckpointPostponedWhileATransactionStaysOpen contrôle que son écrivain o… |
| `v22-retrait-4` | 2026-11-07 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): AutoCheckpointPostponedWhileATransactionStaysOpen contrôle que son écrivain o… |
| `v22-retrait-5` | 2026-11-07 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): AutoCheckpointPostponedWhileATransactionStaysOpen contrôle que son écrivain o… |
| `v22-retrait-6` | 2026-11-07 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): AutoCheckpointPostponedWhileATransactionStaysOpen contrôle que son écrivain o… |
| `v22-retrait-7` | 2026-11-07 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): AutoCheckpointPostponedWhileATransactionStaysOpen contrôle que son écrivain o… |
| `v22-retrait-8` | 2026-11-07 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc): AutoCheckpointPostponedWhileATransactionStaysOpen contrôle que son écrivain o… |
| `validation-a-moitie` | 2026-11-07 | non | 6 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(transaction): l'index d'essai du témoin porte sur la clé, pour que la mise à jour de… |
| `vecteur-du-noeud-copie` | 2026-11-08 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(vector): la taille d'un élément du vecteur sans PhysicalTypeUtils, que la lib n'expor… |
| `vecteur-du-noeud-copie-2` | 2026-11-08 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(transaction): le témoin du use-after-free de shrinkForNode (écrit par hotfixs) — un … |
| `vecteur-du-noeud-copie-3` | 2026-11-08 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(transaction): le témoin du use-after-free de shrinkForNode (écrit par hotfixs) — un … |
| `vecteur-du-noeud-copie-4` | 2026-11-08 | oui | 0 | 0 | partie touchée | **effacer** | entièrement dans master |  | test(transaction): le témoin du use-after-free de shrinkForNode (écrit par hotfixs) — un … |
| `vector-ml-narrowing` | 2026-10-25 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(vector): le degré passé à HNSWDeleteState est converti explicitement — config.ml est … |
| `verrous-a3` | 2026-10-10 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(verrous, marche A3′): la liste jouée, known_red mis à jour (59 → 50 rouges), le témo… |
| `verrous-a3-2` | 2026-10-10 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(verrous, marche A3′): les trois témoins demandés à la relecture du banc, la forme fo… |
| `verrous-a3-reprise` | 2026-10-11 | non | 1 | 1 | partie touchée | **effacer** | remplacée par `verrous-a3-reprise-5`, atterrie |  | test(verrous, A3′ reprise): le témoin rouge — un journal validé qui porte un doublon de c… |
| `verrous-a3-reprise-2` | 2026-10-11 | non | 3 | 3 | partie touchée | **effacer** | remplacée par `verrous-a3-reprise-5`, atterrie | `b08396988` (0.19) | test(verrous, A3′ reprise): known_red — LockBench.RecoveryOfAJournalWithADuplicateKeyKeep… |
| `verrous-a3-reprise-3` | 2026-10-11 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(verrous, A3′ reprise): au rejeu, une clé en double validée au journal reçoit une pie… |
| `verrous-a3-reprise-4` | 2026-10-11 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(verrous, A3′ reprise): au rejeu, une clé en double validée au journal reçoit une pie… |
| `verrous-a3-reprise-5` | 2026-10-11 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(verrous, A3′ reprise): known_red — LockBench.RecoveryOfAJournalWithADuplicateKeyKeep… |
| `verrous-a4` | 2026-10-10 | non | 6 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(verrous, marche A4′): les huit cas Cypher WWConflict* attendent désormais le verrou … |
| `verrous-a4-2` | 2026-10-10 | non | 6 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(verrous, marche A4′): les huit cas Cypher WWConflict* attendent désormais le verrou … |
| `verrous-a9` | 2026-10-11 | non | 1 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(verrous, A9): la mesure du coût du mode multi-écrivains — trois cas joués mode étein… |
| `verrous-a9b` | 2026-10-11 | non | 5 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(verrous, A9): le COPY d'un CREATE TABLE … AS ne cherche plus l'index de sa table avan… |
| `verrous-a9b-2` | 2026-10-11 | non | 5 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(verrous, A9): le COPY d'un CREATE TABLE … AS ne cherche plus l'index de sa table avan… |
| `verrous-a9b-2-171305` | 2026-10-11 | non | 5 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(verrous, A9): le COPY d'un CREATE TABLE … AS ne cherche plus l'index de sa table avan… |
| `verrous-b2` | 2026-10-11 | non | 4 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(B2): le second écrivain voit, à l'ouverture, les lignes validées par le premier pend… |
| `verrous-b2-202358` | 2026-10-11 | non | 4 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(B2): le second écrivain voit, à l'ouverture, les lignes validées par le premier pend… |
| `verrous-c7` | 2026-10-11 | non | 1 | 1 | partie touchée | **effacer** | remplacée par `verrous-c7-4`, atterrie |  | test(verrous, C7): le témoin d'un fil — une relation créée dans une transaction, sa cible… |
| `verrous-c7-2` | 2026-10-11 | non | 2 | 1 | partie touchée | **effacer** | remplacée par `verrous-c7-4`, atterrie |  | fix(rejeu): le DETACH DELETE rejoué donne aux vecteurs voisin et relation leur propre éta… |
| `verrous-c7-3` | 2026-10-11 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(rejeu): le DETACH DELETE rejoué, et la pierre tombale, donnent aux vecteurs voisin et… |
| `verrous-c7-4` | 2026-10-11 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(rejeu): le DETACH DELETE rejoué, et la pierre tombale, donnent aux vecteurs voisin et… |
| `verrous-i1` | 2026-10-11 | non | 5 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(verrous, marche I1): known_red — H3 et les deux IndexLockBench sortent, I1 les fait … |
| `verrous-i1-2` | 2026-10-11 | non | 6 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(verrous, marche I1): le cas Cypher des deux transactions ouvertes qui insèrent des v… |
| `verrous-i1-3` | 2026-10-11 | non | 6 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(verrous, marche I1): le cas Cypher des deux transactions ouvertes qui insèrent des v… |
| `verrous-i1-4` | 2026-10-11 | non | 6 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(verrous, marche I1): le cas Cypher des deux transactions ouvertes qui insèrent des v… |
| `verrous-pk-lock` | 2026-10-11 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(verrous): lockKeyOfRow lève l'erreur de sérialisation nommée quand la ligne n'est plu… |
| `verrous-pk-lock-182355` | 2026-10-11 | non | 2 | 0 | partie touchée | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(verrous): lockKeyOfRow lève l'erreur de sérialisation nommée quand la ligne n'est plu… |
| `verrous-pk-lock-185011` | 2026-10-11 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(verrous): lockKeyOfRow lève l'erreur de sérialisation nommée quand la ligne n'est plu… |
| `verrous-rel2` | 2026-10-11 | non | 3 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc, V2): AnnouncedWritersOfOneIndexedTableWaitThenBothCommit se borne à commit:sta… |
| `verrous-rel2-2` | 2026-10-11 | non | 2 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(verrous, A4′ pour les relations): une relation vue par l'instantané mais supprimée ou… |
| `verrous-t0` | 2026-10-11 | non | 5 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | fix(verrous, T0): un BEGIN ou un CHECKPOINT refusé dans un bloc en échec ne le ferme plus… |
| `verrous-v2` | 2026-10-11 | non | 4 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | feat(verrous, marche V2): l'annonce sur une table indexée prend l'index en exclusif, sans… |
| `verrous-v2-2` | 2026-10-11 | non | 5 | 0 | partie touchée | **effacer** | contenu dans master (même patch) |  | test(banc, V2): l'annonce « l'une après l'autre » se borne au début du commit du premier … |
| `g2-copy-refuse` | 2026-11-07 | non | 4 | 2 | partie touchée | **garder** | 2 commit(s) non atterri(s), tête de sa famille |  | test(copy): le témoin des COPY de blobs sous un petit tampon (repris de copy-de-blobs, fe… |
| `g2b-melange-hnsw` | 2026-11-08 | non | 7 | 4 | partie touchée | **garder** | 4 commit(s) non atterri(s), tête de sa famille |  | test(copy): le témoin du mélange sous 512 Mio — sous 256, le COPY butait sur le tampon pl… |
| `g3-extensions-a-l-ouverture-3` | 2026-11-08 | non | 5 | 5 | partie touchée | **garder** | 5 commit(s) non atterri(s), tête de sa famille | `15fcc3474` (0.18) | docs(G3): l'accord de Lucie pour toujours rouvrir les extensions notées, transmis par c7 … |
| `g5-statistiques` | 2026-11-08 | non | 10 | 10 | partie touchée | **garder** | 10 commit(s) non atterri(s), tête de sa famille | `dff1adddc` (0.32) | style: clang-format-18 sur les lignes du lot (la version de la CI ; le reste des fichiers… |
| `versement-verrou` | 2026-11-08 | non | 3 | 3 | partie touchée | **garder** | 3 commit(s) non atterri(s), tête de sa famille | `f85e94a4e` (0.15) | style: clang-format-18 sur les lignes du lot (la version de la CI ; le reste des fichiers… |
| `essai-fin-d-instruction-filtre` | 2026-10-11 | non | 3 | 3 | partie touchée | **à demander** | nom de brouillon ou d'archive |  | vector(essai I2, non fusionné): variante union — chaque ligne mise à jour reçoit la reche… |
| `essai-fin-d-instruction-maj` | 2026-10-11 | non | 1 | 1 | partie touchée | **à demander** | nom de brouillon ou d'archive |  | vector(essai, non fusionné): la fin d'instruction de la mise à jour — gardée pour mémoire… |
| `g3-extensions-a-l-ouverture` | 2026-11-08 | non | 3 | 3 | partie touchée | **à demander** | dépassée par `g3-extensions-a-l-ouverture-3`, pas encore atterrie | `15fcc3474` (0.18) | test(ouverture): le témoin du refus d'écrire sans l'extension retire la liste — toute ouv… |
| `g3-extensions-a-l-ouverture-2` | 2026-11-08 | non | 5 | 5 | partie touchée | **à demander** | dépassée par `g3-extensions-a-l-ouverture-3`, pas encore atterrie | `15fcc3474` (0.18) | docs(G3): l'accord de Lucie pour toujours rouvrir les extensions notées, transmis par c7 … |
| `lecteur-reverifie-a-l-ouverture` | 2026-10-02 | non | 3 | 1 | partie touchée | **à demander** | 1 commit(s) non atterri(s), rien depuis le 2026-10-02 | `97874c094` (0.12) | wip(lecteur): le lecteur revérifie à l'ouverture — EN COURS, suites complètes non rejouées |
| `lecteur-reverifie-a-l-ouverture-sur-7928974` | 2026-10-03 | non | 4 | 2 | partie touchée | **à demander** | nom de brouillon ou d'archive | `97874c094` (0.12) | test(lecteur): la marche 1 est close — passe complète, vingt passes de charge, coût mesuré |
| `lecture-en-memoire` | 2026-10-18 | non | 1 | 1 | partie touchée | **à demander** | 1 commit(s) non atterri(s), rien depuis le 2026-10-18 | `e5f3095f0` (0.12) | wip(lecture en mémoire): troisième forme bâtie, non mesurée — à reformuler avant le push |
| `v22a-instantane` | 2026-10-25 | non | 1 | 1 | partie touchée | **à demander** | probablement reprise sous d'autres commits : `9101ec942` ≈ `57085cd35` de master (0.54) | `57085cd35` (0.54) | feat(stockage): le point de reprise fond jusqu'au plus vieux lecteur et porte le reste — … |

## mémoire (96) — 59 branches (28 à effacer, 4 à garder, 27 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `archive-96-wt-p1r-96` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | wip(mémoire): l'état non commité de wt-p1r-96 au 25 octobre, gardé tel quel avant l'effac… |
| `lot-96-banc-tentative-4` | 2026-10-18 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(fiches classées): la tentative 4 préparée — une décision par fiche (l'un des candida… |
| `lot-96-e2e-features` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(e2e): run_e2e.sh joue par défaut sous le jeu de features de la procédure (…, openai-l… |
| `lot-96-env` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(tests): un test ne règle plus le produit pour tout le processus — le seuil du COPY de… |
| `lot-96-filet` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(liens): le filet de copier_les_liens tombe — le COPY nomme toutes les propriétés décl… |
| `lot-96-filet-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(liens): le filet de copier_les_liens tombe — le COPY nomme toutes les propriétés décl… |
| `lot-96-filet-3` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(liens): le filet de copier_les_liens tombe — le COPY nomme toutes les propriétés décl… |
| `lot-96-format-fichiers` | 2026-10-25 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(fichiers): la version du format n'est pas un champ que les fichiers manqueraient |
| `lot-96-format-fichiers-2` | 2026-10-25 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(fichiers): la version du format n'est pas un champ que les fichiers manqueraient |
| `lot-96-m4` | 2026-10-25 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(dataflow): SplitSections — les sections d'un Markdown, une par titre, en flux d'élém… |
| `lot-96-m4-2` | 2026-10-25 | non | 3 | 1 | nom | **effacer** | remplacée par `lot-96-m4-4`, atterrie |  | feat(dataflow): SplitSections — les sections d'un Markdown, une par titre, en flux d'élém… |
| `lot-96-m4-3` | 2026-10-26 | non | 5 | 1 | nom | **effacer** | remplacée par `lot-96-m4-4`, atterrie |  | fix(dataflow): SplitSections — ConfigParam porte choices et json_schema depuis master |
| `lot-96-m4-4` | 2026-10-26 | non | 2 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(dataflow): SplitSections — ConfigParam porte choices et json_schema depuis master |
| `lot-96-rep` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(reprise): une reprise de drain garde les liens vers les lignes nées dans le même drai… |
| `lot-96-schema-json` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(llm): le témoin du mode strict vise un json sans défaut, l'objet libre qu'il décrit … |
| `lot-96-schema-json-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(llm): le témoin du mode strict vise un json sans défaut, l'objet libre qu'il décrit … |
| `lot-96-t1` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(catalogue): les écritures qui se taisaient s'annoncent — l'annulation d'une fin qui t… |
| `lot-96-t1-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(catalogue): les écritures qui se taisaient s'annoncent — l'annulation d'une fin qui t… |
| `lot-96-t2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(liens): l'annulation d'un lot de liens ne retire que les paires qu'il a créées, et re… |
| `lot-96-t2-2` | 2026-10-25 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(liens): l'annulation d'un lot de liens ne retire que les paires qu'il a créées, et re… |
| `memoire-longue` | 2026-10-03 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(rapports): la ligne d'âge de la bibliothèque est proposée à rag3db-50 |
| `memoire-longue-2` | 2026-10-10 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | test(gabarits): chaque gabarit livré se charge, et la lib rougit sinon |
| `memoire-longue-3` | 2026-10-10 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(run_e2e): laquelle des variables évite le bâti cmake — la nuance coûtait vingt minut… |
| `memoire-longue-4` | 2026-10-10 | non | 7 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(mcp): brancher rag3weaver sur Claude Code — la recette, et ce qu'elle fait à un dépô… |
| `memoire-longue-5` | 2026-10-10 | non | 8 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tickets): « unknown entity: File » ne dit pas d'indexer — le premier mur de tout usa… |
| `memoire-longue-6` | 2026-10-10 | non | 10 | 1 | nom | **effacer** | remplacée par `memoire-longue-8`, atterrie |  | test(démon): N backends sur une base par le démon — le témoin qui décide du mode --demon … |
| `memoire-longue-7` | 2026-10-10 | non | 10 | 1 | nom | **effacer** | remplacée par `memoire-longue-8`, atterrie |  | test(démon): N backends sur une base par le démon — le témoin qui décide du mode --demon … |
| `memoire-longue-8` | 2026-10-10 | non | 10 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(journal): la portée du neuvième piège était trop large — « le même arbre », pas « le… |
| `lot-96-e2e-features-4` | 2026-11-07 | non | 3 | 1 | nom | **garder** | 1 commit(s) non atterri(s), tête de sa famille | `91543d7f0` (0.29) | fix(run_e2e): le compte des suites jouées nomme chaque suite demandée et ne compte pas le… |
| `lot-96-m4-5` | 2026-11-07 | non | 3 | 1 | nom | **garder** | 1 commit(s) non atterri(s), tête de sa famille |  | test(dataflow): BUILTIN_NODE_COUNT compte SplitSections — les six témoins du registre et … |
| `lot-96-m4-archive-13` | 2026-11-08 | non | 4 | 4 | nom | **garder** | 4 commit(s) non atterri(s), tête de sa famille | `faa13a77c` (0.23) | fix(mémoire): le rapport des décisions de l'archive sort par un nœud terminal à lui — rel… |
| `lot-96-m4-llm` | 2026-10-26 | non | 3 | 1 | nom | **garder** | 1 commit(s) non atterri(s), tête de sa famille |  | test(e2e_memoire_modele): le graphe d'essai sous la feature openai-llm, comme son témoin … |
| `archive-96-wt-p1-96` | 2026-10-25 | non | 2 | 2 | nom | **à demander** | nom de brouillon ou d'archive | `77216bb36` (0.20) | wip(mémoire): l'état non commité de wt-p1-96 au 25 octobre, gardé tel quel avant l'efface… |
| `lot-96-cited-by` | 2026-10-25 | non | 1 | 1 | nom | **à demander** | dépassée par `lot-96-cited-by-4`, pas encore atterrie | `665a4a97c` (0.69) | feat(mémoire): cited_by — les fiches qui citent une chose, ancrées à elle ou par un appor… |
| `lot-96-cited-by-2` | 2026-10-25 | non | 5 | 4 | nom | **à demander** | dépassée par `lot-96-cited-by-4`, pas encore atterrie | `665a4a97c` (0.69) | docs(tickets): l'index nomme le témoin du sous-graphe de valeurs annulé |
| `lot-96-cited-by-3` | 2026-10-25 | non | 6 | 4 | nom | **à demander** | dépassée par `lot-96-cited-by-4`, pas encore atterrie | `665a4a97c` (0.69) | feat(mémoire): cited_by range les fiches par leur dernier apport — une fiche ancienne qui… |
| `lot-96-cited-by-4` | 2026-10-25 | non | 7 | 5 | nom | **à demander** | probablement reprise sous d'autres commits : `8da6f428c` ≈ `665a4a97c` de master (0.69) | `665a4a97c` (0.69) | docs(tickets): le témoin du sous-graphe de valeurs annulé est fermé par 2e205927a — il an… |
| `lot-96-e2e-features-2` | 2026-11-07 | non | 3 | 1 | nom | **à demander** | dépassée par `lot-96-e2e-features-4`, pas encore atterrie | `91543d7f0` (0.29) | fix(run_e2e): le compte des suites jouées nomme chaque suite demandée et ne compte pas le… |
| `lot-96-e2e-features-3` | 2026-11-07 | non | 3 | 1 | nom | **à demander** | dépassée par `lot-96-e2e-features-4`, pas encore atterrie | `91543d7f0` (0.29) | fix(run_e2e): le compte des suites jouées nomme chaque suite demandée et ne compte pas le… |
| `lot-96-m1-avant-renommage` | 2026-10-18 | non | 1 | 1 | nom | **à demander** | probablement reprise sous d'autres commits : `36e31d934` ≈ `e48858d8b` de master (0.80) | `e48858d8b` (0.80) | feat(mémoire): M1, le modèle — la fiche est le sujet (Fiche), un apport par ce qu'une sou… |
| `lot-96-m3` | 2026-10-25 | non | 4 | 3 | nom | **à demander** | dépassée par `lot-96-m3-4`, pas encore atterrie | `80ea996c2` (0.47) | feat(mémoire): M3c — le dossier se compare à la base à l'ouverture, et la vérification à … |
| `lot-96-m3-2` | 2026-10-25 | non | 6 | 3 | nom | **à demander** | dépassée par `lot-96-m3-4`, pas encore atterrie | `80ea996c2` (0.47) | fix(mémoire): la vérification à blanc prend son répertoire jetable sans tempfile, qui n'e… |
| `lot-96-m3-3` | 2026-10-25 | non | 6 | 3 | nom | **à demander** | dépassée par `lot-96-m3-4`, pas encore atterrie | `80ea996c2` (0.47) | fix(mémoire): la vérification à blanc prend son répertoire jetable sans tempfile, qui n'e… |
| `lot-96-m3-4` | 2026-10-25 | non | 6 | 3 | nom | **à demander** | probablement reprise sous d'autres commits : `4251bc204` ≈ `80ea996c2` de master (0.47) | `80ea996c2` (0.47) | fix(mémoire): la vérification à blanc prend son répertoire jetable sans tempfile, qui n'e… |
| `lot-96-m4-archive` | 2026-10-26 | non | 42 | 2 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `faa13a77c` (0.23) | feat(mémoire): l'archiviste, l'outil archive — un Markdown entre, ses sections sont rangé… |
| `lot-96-m4-archive-2` | 2026-10-26 | non | 3 | 1 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `faa13a77c` (0.23) | feat(mémoire): l'archiviste, l'outil archive — un Markdown entre, ses sections sont rangé… |
| `lot-96-m4-archive-3` | 2026-11-07 | non | 3 | 1 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `faa13a77c` (0.23) | feat(mémoire): l'archiviste, l'outil archive — un Markdown entre, ses sections sont rangé… |
| `lot-96-m4-archive-4` | 2026-11-07 | non | 4 | 2 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `faa13a77c` (0.23) | feat(mémoire): l'archiviste, l'outil archive — un Markdown entre, ses sections sont rangé… |
| `lot-96-m4-archive-5` | 2026-11-07 | non | 4 | 4 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `faa13a77c` (0.23) | fix(mémoire): les arêtes des graphes de l'archive nomment leurs ports d'entrée (element, … |
| `lot-96-m4-archive-7` | 2026-11-07 | non | 4 | 4 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `0ee414b61` (0.38) | fix(mémoire): les réglages de la décision voyagent par chaque section, et le graphe cité … |
| `lot-96-m4-archive-8` | 2026-11-07 | non | 6 | 6 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `0ee414b61` (0.38) | fix(mémoire): les scripts de l'archive en Rhai — let est mutable (pas de let mut), trim c… |
| `lot-96-m4-archive-9` | 2026-11-08 | non | 8 | 8 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `0ee414b61` (0.38) | fix(mémoire): l'outil archive ne déclare plus de métadonnées d'étage — un étage qui ne re… |
| `lot-96-m4-archive-10` | 2026-11-08 | non | 2 | 2 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `faa13a77c` (0.23) | feat(mémoire): recall_fiches rend via et author — le hook écarte les fiches du juge (via:… |
| `lot-96-m4-archive-11` | 2026-11-08 | non | 2 | 2 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `faa13a77c` (0.23) | feat(mémoire): recall_fiches rend via et author — le hook écarte les fiches du juge (via:… |
| `lot-96-m4-archive-12` | 2026-11-08 | non | 3 | 3 | nom | **à demander** | dépassée par `lot-96-m4-archive-13`, pas encore atterrie | `faa13a77c` (0.23) | fix(mémoire): l'archive cite ses graphes par @builtin (une copie du gabarit, celle du tém… |
| `lot-96-memoire-fichiers` | 2026-10-25 | non | 4 | 2 | nom | **à demander** | dépassée par `lot-96-memoire-fichiers-2`, pas encore atterrie | `d72bcf5e0` (0.48) | fix(mémoire): la valeur qui valide un champ du serveur absent tient la borne d'une révisi… |
| `lot-96-memoire-fichiers-2` | 2026-10-25 | non | 4 | 2 | nom | **à demander** | probablement reprise sous d'autres commits : `41f2cd76d` ≈ `d72bcf5e0` de master (0.48) | `d72bcf5e0` (0.48) | fix(mémoire): la valeur qui valide un champ du serveur absent tient la borne d'une révisi… |
| `lot-96-pas1-politique` | 2026-11-07 | non | 3 | 2 | nom | **à demander** | dépassée par `lot-96-pas1-politique-2`, pas encore atterrie | `ec9a333e3` (0.53) | fix(backend): le modèle déclaré (models.llm) se tient sur le Backend et se sert aux graph… |
| `lot-96-pas1-politique-2` | 2026-11-07 | non | 4 | 2 | nom | **à demander** | probablement reprise sous d'autres commits : `34906008c` ≈ `ec9a333e3` de master (0.53) | `ec9a333e3` (0.53) | test(e2e_memoire_modele): l'élément étiqueté se lit sous result, comme toute réponse d'ou… |

## optimiseur (G) — 5 branches (4 à effacer, 0 à garder, 1 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `coupure` | 2026-11-08 | non | 5 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): la durabilité au 8 novembre — le crochet de coupure, les témoins, la valid… |
| `coupure-2` | 2026-11-08 | non | 5 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): la durabilité au 8 novembre — le crochet de coupure, les témoins, la valid… |
| `coupure-3` | 2026-11-08 | non | 5 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): la durabilité au 8 novembre — le crochet de coupure, les témoins, la valid… |
| `coupure-4` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(tickets): la durabilité au 8 novembre — le crochet de coupure, les témoins, la valid… |
| `paquet-npm` | 2026-10-11 | non | 77 | 55 | nom | **à demander** | probablement reprise sous d'autres commits : `13821a7d2` ≈ `72fc2037b` de master (0.67) | `72fc2037b` (0.67) | chore(npm): 0.0.1-alpha.3 — la tête et ses trois plateformes ; le témoin « MCP de zéro » … |

## rag3weaver, non attribuée — 13 branches (3 à effacer, 1 à garder, 9 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `annulation-barriere-1` | 2026-10-25 | non | 1 | 1 | partie touchée | **effacer** | remplacée par `annulation-barriere-3`, atterrie |  | test(annulation): le sous-graphe est annulé quand son nœud intérieur attend — plus 50 ms … |
| `csr-listes-temoin-3` | 2026-11-07 | non | 12 | 1 | partie touchée | **effacer** | remplacée par `csr-listes-temoin-5`, atterrie |  | test(banc): C7 à travers un arrêt brutal entre à probabilistic.txt — 2 rouges sur 10 seul… |
| `envoi-virtuel` | 2026-10-11 | non | 2 | 1 | partie touchée | **effacer** | remplacée par `envoi-virtuel-2`, atterrie |  | test(e2e_usages): le relevé des appels de merge_port_values suit le corpus vivant — concu… |
| `reparer-demi-arete` | 2026-10-25 | non | 2 | 2 | partie touchée | **garder** | 2 commit(s) non atterri(s), tête de sa famille |  | tools(repair_half_edge): l'en-tête de FlatTuple, que la lecture d'un compte demande |
| `copy-de-blobs` | 2026-10-11 | non | 3 | 1 | partie touchée | **à demander** | 1 commit(s) non atterri(s), rien depuis le 2026-10-11 |  | docs(copy-de-blobs): la page de la forme (a) — un budget en octets dans NodeBatchInsert, … |
| `marche-1-bis` | 2026-10-11 | non | 1 | 1 | partie touchée | **à demander** | 1 commit(s) non atterri(s), rien depuis le 2026-10-11 |  | test(scope): la même fiche dans deux cellules fait deux lignes — rouge : la seconde écrit… |
| `memoire-temoin-demon` | 2026-10-10 | non | 11 | 2 | partie touchée | **à demander** | dépassée par `memoire-temoin-demon-2`, pas encore atterrie | `9205b3e51` (0.36) | fix(démon, journal): le témoin exigeait un schéma d'entité, et ma note du neuvième piège … |
| `memoire-temoin-demon-2` | 2026-10-11 | non | 12 | 3 | partie touchée | **à demander** | 3 commit(s) non atterri(s), rien depuis le 2026-10-11 | `9205b3e51` (0.36) | test(démon): le témoin est NON CONCLUANT, et ses défauts sont les miens |
| `resolveur-unique` | 2026-10-04 | non | 1 | 1 | partie touchée | **à demander** | 1 commit(s) non atterri(s), rien depuis le 2026-10-04 | `2600aa573` (0.23) | wip(code): le résolveur unique côté rag3weaver — pour reproduction par la session codepar… |
| `temoin-fuite` | 2026-10-10 | non | 1 | 1 | partie touchée | **à demander** | dépassée par `temoin-fuite-4`, pas encore atterrie | `33c3d42de` (0.19) | wip(témoin de la fuite): une mort pendant un COPY journalisé, rejouée à la réouverture, p… |
| `temoin-fuite-2` | 2026-10-10 | non | 2 | 2 | partie touchée | **à demander** | dépassée par `temoin-fuite-4`, pas encore atterrie | `33c3d42de` (0.19) | fix(témoin de la fuite): le point de reprise automatique coupé, et la garde exige le paqu… |
| `temoin-fuite-3` | 2026-10-11 | non | 3 | 3 | partie touchée | **à demander** | dépassée par `temoin-fuite-4`, pas encore atterrie | `33c3d42de` (0.19) | wip(témoin de la fuite): la sonde du chemin — le petit corpus (16 x 250) garde le premier… |
| `temoin-fuite-4` | 2026-10-11 | non | 3 | 3 | partie touchée | **à demander** | 3 commit(s) non atterri(s), rien depuis le 2026-10-11 | `33c3d42de` (0.19) | wip(témoin de la fuite): la sonde du chemin — le petit corpus (16 x 250) garde le premier… |

## recherche (97) — 18 branches (8 à effacer, 0 à garder, 10 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `recherche-trois-lots` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(recherche): une requête longue est bornée, et le dit — la requête entière perd son fl… |
| `recherche-trois-lots-2` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(recherche): une requête longue est bornée, et le dit — la requête entière perd son fl… |
| `recherche-trois-lots-3` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(recherche): une requête longue est bornée, et le dit — la requête entière perd son fl… |
| `spans-creux` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | fix(backend): la vérification à blanc ouvre sa base par une fonction à deux faces — sous … |
| `spans-creux-2` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(creux): l'espace de tête d'une plage (le Metaspace de BGE-M3 range l'espace dans le … |
| `spans-creux-3` | 2026-11-07 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(creux): l'espace de tête d'une plage (le Metaspace de BGE-M3 range l'espace dans le … |
| `spans-creux-4` | 2026-11-08 | non | 3 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(creux): l'espace de tête d'une plage (le Metaspace de BGE-M3 range l'espace dans le … |
| `spans-creux-5` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(creux): l'espace de tête d'une plage (le Metaspace de BGE-M3 range l'espace dans le … |
| `archive/fts-lacher` | 2026-10-19 | non | 2 | 2 | nom | **à demander** | nom de brouillon ou d'archive |  | mesure(fts): try_read |
| `archive/fts-sobre` | 2026-10-19 | non | 1 | 1 | nom | **à demander** | nom de brouillon ou d'archive | `5800e9349` (0.14) | wip(fts): le défaut sobre des écrivains de lucivy (mesure b) |
| `borne-requete-longue` | 2026-11-07 | non | 4 | 2 | nom | **à demander** | dépassée par `borne-requete-longue-2`, pas encore atterrie |  | test(sonde): la sonde de la requête longue copie tous les gabarits de backend (memory emp… |
| `borne-requete-longue-2` | 2026-11-07 | non | 6 | 4 | nom | **à demander** | probablement reprise sous d'autres commits : `84137113f` ≈ `77d0b0676` de master (0.52) | `77d0b0676` (0.52) | fix(recherche): une requête longue est bornée, et le dit — la requête entière perd son fl… |
| `creux-par-l-hote` | 2026-11-07 | non | 6 | 5 | nom | **à demander** | probablement reprise sous d'autres commits : `1e361eedc` ≈ `c5a40d376` de master (0.70) | `c5a40d376` (0.70) | test(lectures): le témoin compteur prouve que la réindexation a travaillé — le texte neuf… |
| `filtre-du-creux` | 2026-11-07 | non | 3 | 1 | nom | **à demander** | probablement reprise sous d'autres commits : `11cb1bfba` ≈ `aaf475d4b` de master (0.66) | `aaf475d4b` (0.66) | test(dialect): les noms de table du filtre des morceaux sont cités par cypher_quote (acce… |
| `fts-lacher-mesure` | 2026-10-19 | non | 2 | 2 | nom | **à demander** | 2 commit(s) non atterri(s), rien depuis le 2026-10-19 |  | mesure(fts): try_read |
| `fts-sobre-1` | 2026-10-19 | non | 1 | 1 | nom | **à demander** | 1 commit(s) non atterri(s), rien depuis le 2026-10-19 | `5800e9349` (0.14) | wip(fts): le défaut sobre des écrivains de lucivy (mesure b) |
| `scan-filtre-chaines` | 2026-10-12 | non | 1 | 1 | nom | **à demander** | dépassée par `scan-filtre-chaines-2`, pas encore atterrie | `f6179ade0` (0.92) | fix(stockage): le balayage filtré d'une colonne STRING ou LIST écrit toute la fenêtre qua… |
| `scan-filtre-chaines-2` | 2026-10-12 | non | 1 | 1 | nom | **à demander** | probablement reprise sous d'autres commits : `f9b3d53e0` ≈ `f6179ade0` de master (0.92) | `f6179ade0` (0.92) | fix(stockage): le balayage filtré d'une colonne STRING ou LIST écrit toute la fenêtre qua… |

## tty — 16 branches (16 à effacer, 0 à garder, 0 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `tty-1` | 2026-11-07 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tty): la conception — bases Rust étudiées (ratatui recommandé, iocraft, termion, Ope… |
| `tty-2` | 2026-11-07 | non | 8 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | build(tty): la vue derrière la feature vue (par défaut) ; rag3weaver prend le crate sans … |
| `tty-3` | 2026-11-07 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(tty): lot tty-3 — le Daemon Code Summoning en cellules (Cercle, Transmutation, Glitc… |
| `tty-1-2` | 2026-11-07 | non | 6 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tty): rapport et knowledge dump de la session, et le ticket de la lib rag3weaver qui… |
| `tty-1-3` | 2026-11-07 | non | 7 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | build(tty): rag3weaver prend le crate du terminal sans ses features par défaut — la vue d… |
| `tty-1-4` | 2026-11-07 | non | 8 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tickets): l'index produit compte 48 — le ticket du bâti sans rag3db-native et celui … |
| `tty-2-2` | 2026-11-07 | non | 10 | 0 | nom | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tty): la mesure du lot tty-2 dans la page, avec ce qui la produit — 28 o par token à… |
| `tty-2-3` | 2026-11-07 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tty): la durée d'e2e_tty varie (4 s puis 52,9 s, mêmes comptes) — noté, à mesurer au… |
| `tty-2-4` | 2026-11-07 | non | 4 | 0 | nom | **effacer** | contenu dans master (même patch) |  | docs(tty): la durée d'e2e_tty varie (4 s puis 52,9 s, mêmes comptes) — noté, à mesurer au… |
| `tty-code` | 2026-11-08 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | wip(tty): coloration du code — la palette code du thème, les blocs tenus jusqu'à leur clô… |
| `tty-code-2` | 2026-11-08 | non | 2 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(tty): les blocs de code colorés par les spans de l'hôte (POST /highlight) dans la pa… |
| `tty-code-3` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | feat(tty): les blocs de code colorés par les spans de l'hôte (POST /highlight) dans la pa… |
| `tty-docs-code` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | docs(nouveautés, tty): le code se colore dans le terminal avec le thème commun (f2e6e2521… |
| `tty-theme` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(tty): le thème daemon branché sur le fichier commun templates/themes/daemon.json (Lu… |
| `tty-theme-2` | 2026-11-08 | non | 1 | 0 | nom | **effacer** | contenu dans master (même patch) |  | feat(tty): le thème daemon branché sur le fichier commun templates/themes/daemon.json (Lu… |
| `tty-theme-3` | 2026-11-08 | oui | 0 | 0 | nom | **effacer** | entièrement dans master |  | feat(tty): le thème daemon branché sur le fichier commun templates/themes/daemon.json (Lu… |

## rag3weaver, atterries, session non déduite — 111 branches (111 à effacer, 0 à garder, 0 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `analyze-racine` | 2026-10-11 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(code): une racine relative d'une source locale se résout et se dit — plus de chemins … |
| `annulation-barriere-2` | 2026-10-25 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | test(annulation): trois témoins annulent sur une barrière — le nœud qui attend est entré … |
| `annulation-barriere-3` | 2026-10-25 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | test(annulation): trois témoins annulent sur une barrière — le nœud qui attend est entré … |
| `anthropic-llm` | 2026-10-10 | non | 4 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(llm): protocole anthropic dans connect_llm (retour en Box<dyn Llm>), et les trois co… |
| `anthropic-llm-2` | 2026-11-07 | non | 5 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(llm): connect_llm rend un Arc<dyn Llm> pour tous les protocoles, le client Anthropic… |
| `appel-a-delai` | 2026-11-07 | non | 3 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | tickets: une recherche continue sur l'hôte après l'abandon de son client, fermé par deadl… |
| `barre-noms` | 2026-11-08 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | Merge remote-tracking branch 'origin/master' into barre-noms |
| `best-signal` | 2026-11-08 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(recherche): le score d'une ligne fusionnée dit son échelle et le signal qui la porte |
| `best-signal-2` | 2026-11-08 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(recherche): le score d'une ligne fusionnée dit son échelle et le signal qui la porte |
| `boite-destinataire` | 2026-11-07 | non | 2 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | wip(équipier): seule une ligne adressée à l'équipier le lance et lui parvient à sa fronti… |
| `callees` | 2026-10-11 | non | 4 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(callees): include_by_name au même schéma qu'impact ; le README MCP dit treize outils |
| `chaines-de-champs` | 2026-10-11 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(code): une chaîne de champs dans un type différé se suit pas à pas — tableInfo.table… |
| `chat-et-delai` | 2026-11-07 | non | 18 | 0 | — | **effacer** | contenu dans master (même patch) |  | Merge remote-tracking branch 'origin/master' into chat-et-delai |
| `chat-ordre-page` | 2026-11-07 | non | 19 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | ticket: le premier jeton perdu en direct — la piste d'everything declarative (les relais … |
| `chat-ordre-rust` | 2026-11-07 | non | 4 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(réactions): role sur la déclaration d'une réaction, porté par sa montre jusqu'au Run… |
| `code-tree` | 2026-10-11 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(code-tree): une base ne se copie que fermée — si l'hôte tient --from ouvert, /backend… |
| `coloration-b` | 2026-11-08 | non | 5 | 0 | — | **effacer** | contenu dans master (même patch) |  | build(coloration): le Cargo.lock porte tree-sitter-highlight et tree-sitter-javascript, a… |
| `cpp-anonymes` | 2026-10-11 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | chore(codeparsers): pointeur 35faa0f — une classe exportée est une classe, les C++ anonym… |
| `creux-par-les-spans` | 2026-11-07 | non | 6 | 0 | — | **effacer** | contenu dans master (même patch) |  | test(recherche): le creux passe par les spans du factice d'embarquement (MockSparseEmbedd… |
| `creux-par-les-spans-2` | 2026-11-08 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | test(recherche): le creux passe par les spans du factice d'embarquement (MockSparseEmbedd… |
| `csr-listes-temoin-4` | 2026-11-07 | non | 11 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tickets), test(banc), docs(procédures): un rang déplacé pendant un CREATE n'est pas … |
| `csr-listes-temoin-5` | 2026-11-07 | non | 11 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(tickets), test(banc), docs(procédures): un rang déplacé pendant un CREATE n'est pas … |
| `defauts-bascules` | 2026-10-05 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(défauts, en préparation — non fusionné) : le plein texte en fichiers pour une base n… |
| `defauts-bascules-2` | 2026-10-10 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(défauts, en préparation — non fusionné) : le plein texte en fichiers pour une base n… |
| `dense-filtre-sous-l6` | 2026-11-08 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(recherche): la recherche filtrée revient sous l'instantané de son appel — le contourn… |
| `docs-96-m4-3` | 2026-10-25 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(mémoire longue): page 06 §3 — ce que 73 a lu : le pli d'un flux sur une entrée de va… |
| `docs-96-roadmaps-3` | 2026-10-25 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(mémoire longue): page 03 — la phase aussi se clôt par l'agent qui l'a accomplie (Luc… |
| `docs-essais-equipier` | 2026-11-08 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(équipier): le ticket des essais réels sur Gemini — l'attente qui tue la tâche, la ré… |
| `docs-i1` | 2026-10-25 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(coeur-cpp): page 19 — V2-2, le point de reprise lit un instantané et les autres cont… |
| `docs-i10` | 2026-11-08 | oui | 0 | 0 | — | **effacer** | entièrement dans master |  | docs(tickets): le second SIGSEGV de l'hôte demande le même bloc d'états de page (80 724) … |
| `docs-i11` | 2026-11-08 | oui | 0 | 0 | — | **effacer** | entièrement dans master |  | docs(tickets): l'ASAN du SIGSEGV de l'hôte — V2 et CSR propres, un débordement dans le CR… |
| `docs-i4` | 2026-11-07 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(tickets): une validation et un lecteur HNSW se figent l'un l'autre sur les deux verr… |
| `docs-i9` | 2026-11-08 | oui | 0 | 0 | — | **effacer** | entièrement dans master |  | docs(tickets): un SIGSEGV dans optimisticRead sous lecture concurrente pendant une indexa… |
| `docs-rapport-8-nov` | 2026-11-08 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(everything-declarative): le rapport de la nuit du 8 novembre — six lots poussés, le … |
| `docs-ticket-juge-genre` | 2026-11-08 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(tickets): le témoin du juge des outils exige un genre que Gemini ne tient pas — troi… |
| `docs-verrous-a3` | 2026-10-10 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(coeur-cpp): A3′ en cours — page 07 §5 (ce que le code a appris : index partagé/exclu… |
| `docs-verrous-a4` | 2026-10-10 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(coeur-cpp): A4′ en cours — le tableau des sémantiques en tête de la page 07 (lignes … |
| `envoi-virtuel-2` | 2026-10-11 | non | 3 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(code): la limite d'une base indexée avant template_params, dite là où elle agit — la… |
| `estimation-et-avancement` | 2026-10-03 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(embarquements): estimate et l'avancement de l'index — les deux premières briques d'«… |
| `estimation-et-avancement-2` | 2026-10-03 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(embarquements): EstimateNode et sa fiche d'outil — estimate, côté agent |
| `etape-navigateur-1` | 2026-10-25 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | wip(témoins): les témoins navigateur en étape à part (feature navigateur, scripts/temoins… |
| `etape-navigateur-2` | 2026-10-25 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | test(navigateur): les témoins navigateur sortent de la lib entière et deviennent leur pro… |
| `execution-asynchrone` | 2026-10-10 | non | 5 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(chat): le chat branche le bus et sa boîte — la fin d'un outil en fond revient dans l… |
| `execution-asynchrone-2` | 2026-10-10 | non | 6 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(backend code): l'attendu du crochet « Avant d'éditer » dit les deux dépendants — le … |
| `filtre-impact` | 2026-10-04 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(impact): le filtre des arêtes devinées est allumé dans impact et l'impact d'un fichi… |
| `filtre-resolution` | 2026-10-04 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | test(usages): usages_rendu connaît l'usage deviné, et le rend « (par le nom) » |
| `flux-declarations` | 2026-10-11 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(dataflow): les déclarations de flux sur les ports, et les sorties requises respectée… |
| `impact-surs` | 2026-10-11 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(impact): summary() compte pareil, et include_by_name montre les usages par le nom se… |
| `impl-pas-definisseur` | 2026-10-11 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(code): un bloc impl ne définit pas son nom — `Marteau` n'est plus ambigu partout où u… |
| `impl-pas-definisseur-2` | 2026-10-11 | non | 3 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | chore(codeparsers): pointeur 06d6e92 — la variable d'une boucle C++ sur intervalle a le t… |
| `index-des-tickets` | 2026-11-07 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(tickets): la garde passe dans l'en-tête de chaque ticket et l'index se génère — check… |
| `index-en-fond` | 2026-10-03 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(embarquements): IndexNode — indexer en fond, le reçu est un journal |
| `journal-hote-horodate` | 2026-11-07 | non | 5 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(hôte): WRITE_PATIENCE à 50 ms, lue sur une mesure — la reprise d'une sortie après EAG… |
| `liens` | 2026-10-03 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(liens): LinksNode — comment des résultats se relient, en attente du choix de longueur |
| `lucivy-publiee-1` | 2026-10-25 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | build(lucivy): rag3weaver prend lucivy 4.3.1 publiée par Lucie — le [patch.crates-io] tem… |
| `lucivy-publiee-2` | 2026-11-07 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | build(lucivy): rag3weaver prend lucivy 4.3.1 publiée par Lucie — le [patch.crates-io] tem… |
| `memoire-nommee` | 2026-11-08 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | Merge remote-tracking branch 'origin/master' into memoire-nommee |
| `montee-lucivy-4-3-2` | 2026-11-07 | non | 5 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tickets): la mémoire longue non remesurée sur 4.3.2 — sa copie, prise sous l'écrivai… |
| `montee-lucivy-4-3-2-2` | 2026-11-08 | non | 5 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(tickets): la mémoire longue non remesurée sur 4.3.2 — sa copie, prise sous l'écrivai… |
| `nom-seul` | 2026-10-11 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | chore(codeparsers): pointeur a6111cc — un paramètre n'est pas une référence du bloc impl,… |
| `nouveautes-chercher-sans-dossier` | 2026-11-07 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(nouveautés): chercher sans avoir ouvert un dossier |
| `nouveautes-recherche-8-nov` | 2026-11-08 | oui | 0 | 0 | — | **effacer** | entièrement dans master |  | docs(nouveautés): une phrase collée retrouve son fichier, et le surlignage ne demande plu… |
| `nouveautes-recherches` | 2026-11-08 | oui | 0 | 0 | — | **effacer** | entièrement dans master |  | docs(nouveautés): mes recherches avec les clics (et la course du journal des runs), le sc… |
| `page-atelier-depots` | 2026-11-07 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(everything declarative): l'atelier ouvre les dépôts — page de conception de la premi… |
| `page-equipier-agent` | 2026-11-07 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(everything declarative): l'équipier, agent de code — les trois réponses de c7 (comma… |
| `passage-exact-long` | 2026-11-07 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(lectures): témoin rouge — une ligne collée de 120 caractères retrouve son fichier pr… |
| `pile-relais-outline-js-1` | 2026-10-25 | non | 4 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | refactor(ide): le code des pages de l'IDE et de l'atelier en fichiers servis — ide/page.j… |
| `planchers-verdict-1` | 2026-10-25 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(magasin): un plancher que git ne peut juger n'est jamais tenu — verifier rend 3 et di… |
| `planchers-verdict-2` | 2026-10-25 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(magasin): verifier courant juge la lib que le lien désigne, et un « non jugé » dit sa… |
| `portage-b1-2` | 2026-10-12 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(portabilité): les dépendances de total_memory sous macOS (libc) et Windows (windows-s… |
| `portage-b2` | 2026-10-12 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(dialect): les chemins des deux COPY FROM cités pour Cypher (cypher_path_literal) — un… |
| `portage-b3` | 2026-10-12 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(rust_api): l'identifiant de bâti sous Windows — quand rag3db-src est une copie sans .… |
| `portage-c` | 2026-10-12 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs(tickets): la clôture du mode démon dit sa preuve — cycle vert sous macOS, vert sous … |
| `poste-n-detenteurs` | 2026-11-07 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(poste): au plus N lourds à la fois, les suivants dans l'ordre d'arrivée |
| `premier-jeton` | 2026-11-07 | non | 11 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(hôte): le dépôt d'un backend est celui de son espace de travail, sinon celui de son m… |
| `premier-jeton-2` | 2026-11-07 | non | 11 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(hôte): le dépôt d'un backend est celui de son espace de travail, sinon celui de son m… |
| `preuve-nature-rouge` | 2026-11-07 | non | 7 | 0 | — | **effacer** | contenu dans master (même patch) |  | test(hôte): le témoin de la nature crée son dossier de configuration avant d'y ranger les… |
| `profil-ingestion` | 2026-10-03 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | perf(profil): l'ingestion ventilée en entier — un accumulateur par étape, publié à la fin |
| `proto-declaratif` | 2026-10-10 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(everything-declarative): ouverture du chantier I — rapport (où l'on travaille, ce qu… |
| `prototypes-c` | 2026-10-11 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | chore(codeparsers): pointeur 9531860 — un prototype C du niveau fichier est un membre, un… |
| `readme-8-novembre` | 2026-11-08 | oui | 0 | 0 | — | **effacer** | entièrement dans master |  | docs(nouveautés): deux corrections — la recherche sans ses vecteurs porte ses commits de … |
| `recettes-p1` | 2026-10-11 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | test(usages): le type d'un champ déclaré dans un autre fichier se lit — ticket fermé (B2b… |
| `receveurs-non-types` | 2026-10-11 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(code): un lien posé par le nom seul dans le fichier se marque « nom », pas « fichier… |
| `reexport-au-site` | 2026-10-11 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(code): une réexportation part du scope de son pub use — à sa ligne, sur l'arête que l… |
| `reexportations` | 2026-10-11 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(code): un import suit les réexportations — `crate::dataflow::X` atteint dataflow/run… |
| `reexportations-2` | 2026-10-11 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | feat(code): un import suit les réexportations — `crate::dataflow::X` atteint dataflow/run… |
| `regime-carte-partagee` | 2026-10-03 | non | 4 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(embarquements): RAG3WEAVER_EMBED_SERVICE — s'attacher à un service d'embarquement dé… |
| `regime-carte-partagee-2` | 2026-10-03 | non | 9 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | docs+scripts(embarquements): trois services, la variable dans les scripts Python, le pièg… |
| `relcopy-sous-refus` | 2026-10-10 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(coeur-cpp-tickets): rapport et relevé de la seconde session cœur C++ ; le lot RelCop… |
| `rendu-gabarit-livre` | 2026-11-08 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(crochets): un gabarit nommé se lit dans le gabarit livré quand le dossier des gabarit… |
| `rendu-gabarit-livre-2` | 2026-11-08 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(crochets): un gabarit nommé se lit dans le gabarit livré quand le dossier des gabarit… |
| `repro-recherche-dense-apres-table-videe` | 2026-10-03 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | test(recherche): reproduction — après une table vidée puis repeuplée par le chemin de mas… |
| `resolution-marque` | 2026-10-04 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | refactor(code): le choix de la cible au rendez-vous dit comment il a choisi — type, impor… |
| `run-e2e-compte` | 2026-11-08 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(run_e2e): le compte des suites lit les suites demandées — `--test e2e` est le binaire… |
| `second-ecrivain` | 2026-11-07 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(connexion): un second écrivain refusé sait qui tient la base — l'ouverture en écritur… |
| `self-types` | 2026-10-04 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(code): par self, le type englobant d'abord — sans exclure l'héritage |
| `seuil-hors-graphe` | 2026-11-08 | oui | 0 | 0 | — | **effacer** | entièrement dans master |  | docs(tickets): le seuil de reconstruction de l'index vectoriel, proposé par la recherche … |
| `sonde-horloge` | 2026-11-08 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | docs(tickets): le témoin du réacteur each_runs_per_ring… rougit sous la lib entière (réve… |
| `sonde-horloge-b` | 2026-11-08 | oui | 0 | 0 | — | **effacer** | entièrement dans master |  | docs(tickets): le témoin du réacteur each_runs_per_ring… rougit sous la lib entière (réve… |
| `surlignage-creux-5` | 2026-11-07 | non | 7 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | fix(recherche): un passage creux s'étend à son mot entier — la sonde réelle surlignait « … |
| `surlignage-creux-6` | 2026-11-07 | non | 8 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(recherche): BUILTIN_NODE_COUNT à 56 — SplitSections (6e1dc868e) l'a porté à 55 pendan… |
| `t5-fermetures` | 2026-10-11 | non | 1 | 0 | — | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | chore(codeparsers): pointeur d90a1f6 — l'appel à une fermeture locale est relié à elle, u… |
| `tache-hors-depot` | 2026-11-07 | non | 5 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(tâches): hors dépôt, la tâche reçoit tout templates/ (le premier ancêtre de ce nom du… |
| `ticket-garde-match` | 2026-11-07 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | ticket: un appel dans une garde de match Rust n'est pas relevé — vue_portee 0 usage (ide.… |
| `tickets-lot-fermeture` | 2026-11-08 | oui | 0 | 0 | — | **effacer** | entièrement dans master |  | fix(tickets): check_tickets --lot lit « fermé » un ticket dont la fermeture réécrit le co… |
| `tx-par-paquet` | 2026-10-04 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | proto(sync): la transaction par paquet, derrière RAG3WEAVER_TX_PAR_PAQUET=1 — pour mesure… |
| `types-differes` | 2026-10-10 | non | 2 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(code): un receveur typé par une déclaration d'un autre fichier — le type différé au … |
| `types-differes-par-source` | 2026-10-11 | non | 4 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(code): les types différés se résolvent dans la source et le dépôt de la mention |
| `unknown-entity` | 2026-10-10 | non | 1 | 0 | — | **effacer** | contenu dans master (même patch) |  | fix(surface de code): « unknown entity: File » dit comment avoir le schéma du code — la d… |
| `verrous` | 2026-10-10 | non | 4 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(code): les verrous qu'un scope prend — la relation LOCKS vers le symbole Classe::cha… |
| `verrous-dans-impact` | 2026-10-11 | non | 3 | 0 | — | **effacer** | contenu dans master (même patch) |  | feat(impact): les verrous pris sur le chemin — un relevé déclaré au gabarit |

## Kuzu (amont) — 95 branches (1 à effacer, 0 à garder, 94 à demander)

| branche | dernier commit | dans master | + master | non atterris | par | proposition | pourquoi | proche dans master | dernier sujet |
|---|---|---|---|---|---|---|---|---|---|
| `play` | 2024-05-11 | non | 1 | 0 | date | **effacer** | atterrie après rebase (mêmes sujets dans master) |  | X |
| `0.10.1` | 2025-06-27 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Backport Java changes and update Maven Central workflow |
| `0.11.2` | 2025-08-20 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Bump version to 0.11.2 |
| `0.11.3` | 2025-10-10 | non | 20 | 18 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Add project URL to POM description in build.gradle |
| `0.6.0` | 2024-08-16 | non | 10 | 3 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Update api doc for java, nodejs, python and add pytest for in-mem mode (#4095) |
| `0.6.1` | 2024-10-14 | non | 55 | 27 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Turn off automatic grammar generation |
| `0.7.1` | 2024-12-20 | non | 57 | 3 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Bump all version number |
| `0.8.1` | 2025-02-21 | non | 42 | 4 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Bump version to 0.8.1 |
| `0.8.2` | 2025-02-24 | non | 45 | 7 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Bump version to 0.8.2 and update related files |
| `add-list-functions` | 2025-09-26 | non | 13 | 13 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | ci: auto code format |
| `agg-hash-memory-fix` | 2025-04-17 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Destroy AggregateState if an error occurs before the aggregation finishes |
| `algo-betweenness-centrality` | 2025-08-22 | non | 56 | 49 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | add comment on TODO's |
| `always-generate-query-ID` | 2025-08-18 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | ci: auto code format |
| `bench-test` | 2025-06-12 | non | 5 | 5 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update |
| `better-param-handling` | 2025-08-04 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Better param handling |
| `bundle-duckdb-extension` | 2025-02-19 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Bundle httpfs,delta extension in duckdb lib |
| `chunked-group-append-opt` | 2024-08-08 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Avoid re-initializing SelectionVector in ChunkedNodeGroup::append |
| `ci-fix` | 2025-05-16 | non | 11 | 11 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Better search |
| `cidr-2023` | 2023-06-27 | non | 22 | 18 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update README |
| `cmake-refactor-2` | 2025-06-01 | non | 8 | 8 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | ci: refactor setting threads |
| `comparison-between-nodes-and-between-rels` | 2023-12-22 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | fix formatting |
| `cpp-server` | 2025-07-04 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | x |
| `decimal-fix` | 2024-07-29 | non | 5 | 5 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | progress |
| `decouple-table-entry-copy-from` | 2025-05-09 | non | 15 | 15 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Clean NodeBatchInsert::initGlobalStateInternal |
| `drop-macros` | 2025-09-09 | non | 14 | 12 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Merge remote-tracking branch 'refs/remotes/origin/drop-macros' into drop-macros |
| `duplicate-eviction-candidates` | 2025-04-07 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Clear eviction candidates from the queue which have already been evicted |
| `emcc-test` | 2024-04-07 | non | 4 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Add file embedding |
| `expose-db-conn-error` | 2025-05-01 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | x |
| `extension-signature` | 2025-08-29 | non | 6 | 6 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update |
| `external-table-base` | 2024-08-26 | non | 3 | 3 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Run clang-format |
| `fts-chs-tokenizer` | 2025-08-13 | non | 8 | 8 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update |
| `fts-dd` | 2025-06-16 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update |
| `fts-new-bench` | 2025-06-11 | non | 3 | 3 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update |
| `fts-one-table` | 2025-06-24 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | fts-one-table |
| `fts-profile` | 2025-06-16 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update |
| `fts-wild-card-pattern` | 2025-09-11 | non | 7 | 7 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | ci: auto code format |
| `gaurav8297/acorn` | 2025-03-02 | non | 196 | 196 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update benchmark script |
| `gaurav8297/hnsw_index` | 2025-02-25 | non | 231 | 231 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Some more changes |
| `gaurav8297/hnsw_index_directed` | 2025-02-08 | non | 212 | 212 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Some more changes |
| `gaurav8297/hnsw_index_exp` | 2025-01-04 | non | 106 | 106 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Use buffer manager to read vectors during index building |
| `gaurav8297/hnsw_index_fix_issue` | 2025-01-24 | non | 196 | 196 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Some more changes |
| `gaurav8297/hnsw_index_global_selectivity` | 2025-02-09 | non | 212 | 212 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Some more changes |
| `gaurav8297/hnsw_index_no_one_hop` | 2025-02-11 | non | 213 | 213 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | some more changes |
| `gaurav8297/hnsw_index_no_zc` | 2025-02-11 | non | 214 | 214 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Some more changes |
| `gaurav8297/hnsw_index_random` | 2025-02-07 | non | 207 | 207 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Some more changes |
| `gaurav8297/hnsw_index_random_32` | 2025-02-07 | non | 210 | 210 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Some more changes |
| `gds-refactor` | 2025-03-11 | non | 7 | 7 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Run clang-format |
| `hash-index-optimistic-read-fix` | 2024-09-19 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Fix optimistic reads in the hash index to not use external state |
| `improve-copy-subquery-memory` | 2025-07-14 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Improve copy subquery memory |
| `improvements-over-table_info` | 2025-09-11 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | add support to print connection info for rel_tables |
| `invocable` | 2025-06-25 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Use std::invocable to enforce function arguments to templated functions |
| `io-benchmark-io-uring` | 2023-12-22 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | finish io-uring |
| `java-api-test` | 2025-06-11 | non | 4 | 4 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Fix includes |
| `libtorch` | 2025-06-13 | non | 4 | 4 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Refactor ArraySquaredDistance to utilize MPS device for Torch tensor operations |
| `libuv-async` | 2023-12-22 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | libuv-sync |
| `lockfree-mm-cache` | 2025-04-21 | non | 4 | 4 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Add benchmarks |
| `louvain-fixes` | 2025-05-09 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | gds(louvain): fix self loop handling, configure threshold |
| `macro-viewing` | 2025-09-04 | non | 15 | 6 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | change the function of getting transaction to fit the modification in master |
| `merge-free-pages-after-rollback` | 2025-08-22 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | ci: auto code format |
| `minors-tests-05st` | 2025-07-08 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Add tests |
| `msvc-errors` | 2025-06-18 | non | 3 | 3 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Temporarily change extension test |
| `null-handling-refactor` | 2025-05-08 | non | 38 | 35 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | null handling refactor |
| `openCypher-expression-other` | 2023-12-22 | non | 4 | 3 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | temporal test |
| `optional-params-other-fcn-types` | 2025-09-08 | non | 16 | 14 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Merge branch 'master' into optional-params-other-fcn-types |
| `parser-extension` | 2024-06-20 | non | 7 | 7 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update |
| `postgres-test` | 2025-07-08 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | update |
| `pr5811` | 2025-09-10 | non | 13 | 13 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Cleanup |
| `preserve-internal-ids-on-export` | 2025-07-28 | non | 35 | 28 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | comment on failing tests |
| `python-linting` | 2025-07-21 | non | 6 | 5 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Remove quotes around hints |
| `python-storage-version-info` | 2025-08-22 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | ci: auto code format |
| `readme` | 2025-10-10 | non | 8 | 6 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | remove logo |
| `recursive-pattern-label-pruning` | 2024-12-01 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | recursive relationships support rel label pruning (#4396) |
| `redesign-path-length-interface` | 2025-03-21 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Redesign path length interface |
| `rel-decouple-table-entry` | 2025-05-12 | non | 14 | 14 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Changes and clean up based on feedback from comments |
| `relcopyfrom-decouple-catalog-entry` | 2025-05-12 | non | 21 | 21 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Attempt to decouple catalog entry from mapCopyRelFrom |
| `revert-5871-fts-chs-tokenizer` | 2025-08-13 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Revert "feat: simp-chinese support in fts index extension (#5871)" |
| `royi/more-plan-improvements` | 2025-01-06 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Add test for hash join flatten fix |
| `royi/remove-chunk-checkpoint-state` | 2025-09-17 | non | 83 | 79 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Two pointers optimization for looping through chunk checkpoint states |
| `rust-convert-types` | 2025-04-21 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Implement TryFrom<Value> for the different types |
| `segmentation` | 2025-06-27 | non | 20 | 20 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | WIP Compress/decompress whole segments |
| `skipUUIDTest` | 2025-09-10 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Skipping the FSM Leak check for the UUID test. |
| `slow-tests` | 2025-10-01 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | updates |
| `stream-query-result` | 2025-08-11 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Implement stream query result |
| `table-func-optional-param-cast` | 2025-07-04 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Support optional parameter casting in fts |
| `tck/carm/label-as-bool` | 2025-09-15 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | spelling corrections |
| `tck/carm/num-hops-support-expressions` | 2025-09-17 | non | 4 | 4 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | fix test |
| `tck/carm/properties-func` | 2025-09-15 | non | 10 | 9 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | tidy |
| `test-rewriter-fix-regex` | 2025-07-23 | non | 3 | 3 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | try |
| `testdoc` | 2025-03-27 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Rename test.txt to test.csv |
| `try-decouple-enumeration` | 2024-06-25 | non | 2 | 2 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Run clang-format |
| `union-casts-2` | 2025-07-03 | non | 43 | 42 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | ci: auto code format |
| `update-fts` | 2025-06-02 | non | 3 | 3 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | implement delete |
| `update-msf-algo-logical-plan` | 2025-08-14 | non | 8 | 5 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | ci: auto code format |
| `uv-scripts` | 2025-07-19 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | Add scripts to build from source for uv-managed Python |
| `uv-sync` | 2023-12-22 | non | 1 | 1 | date | **à demander** | branche de l'amont Kuzu, héritée du fork |  | libuv sync |

