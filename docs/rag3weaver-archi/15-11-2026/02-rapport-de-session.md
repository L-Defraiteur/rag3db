# Archi (session Claude « rag3weaver archi ») — nuit du 14 au 15 novembre 2026

La suite du [rapport du 14](../14-11-2026/01-rapport-de-session.md).

## Ce qui est arrivé sur master depuis

| Lot | Master | Ce qu'il change |
|---|---|---|
| L'équipier rend le contenu demandé | `9009f3a3b` (témoin `475b5a190`, cas Gemini `e1e90d370`) | `report` porte le texte lui-même ; le texte d'un pas qui finit par `step_done` va dans la boîte |
| La file des écritures sous une fermeture qui tient | `95c643a20` (témoins `4d4af368a`, conception `8c3f766b1`) | voir plus bas |

**La file des écritures** ([conception](01-conception-file-des-ecritures.md)) ferme le reste du
ticket de 96 (`docs/tickets/closed/2026-11-14-un-backend-ferme-par-l-hote-se-rouvre-au-premier-appel.md`) :
sous une fermeture avec une raison, une écriture n'est plus refusée et perdue.

- **Ce qu'est une écriture** : un appel que `PreparedBackend::reads_only` ne prouve pas en lecture.
  La preuve : son graphe instancié et son crochet après sont `snapshot_safe`, sa politique n'écrit
  ni ne lance rien, et il n'a pas de harnais. Le défaut est prudent : un appel non prouvé se range.
- **Le refus** : il porte `queued: true` et `rank`, et dit « fermé : <raison>, depuis <heure>,
  jusqu'à <quand>, par <qui> ; ton écriture « <outil> » est en attente, rang N ». Une lecture reste
  refusée.
- **Sur disque** : la fermeture et sa file sont dans `fermeture.json`, sous le dossier d'état du
  manifeste. Elles survivent à un redémarrage de l'hôte.
- **Le rejeu** : à `POST /backend/open`, dans l'ordre d'arrivée ; celles reçues pendant le rejeu
  passent derrière. Au-delà de `queue_max_age_s` (24 h par défaut), une écriture ne s'applique pas,
  et l'état le dit. La file est bornée à 1000 écritures.
- **/etat** : `write_queue {pending, oldest, replaying, replayed, expired, last_replay_failures}`.
- **Un témoin d'hier change de sens** : `une_fermeture_avec_une_raison_tient_jusqu_a_l_ouverture_explicite`
  attendait des écritures perdues ; elles sont maintenant rejouées.
- **Les publications de génération** (question d'ED) : un nœud qui publierait est
  `snapshot_safe = false` par défaut, donc compté comme une écriture.

## Ce que la nuit a appris

- **Une base de pile plus vieille que master** : en reposant le lot sur la tête de lucivy, j'avais
  repris le commit de conception, parce que cette base n'avait pas la page et que le correctif la
  touche. Ce commit était déjà sur master : au dernier pas, il est tombé au rebase et la garde a
  refusé (« < » au range-diff). La sortie, prise avec c7 : reposer le seul code sur la tête
  **atterrie** du lot d'avant, qui porte déjà les docs, puis rejouer. Avant de reposer,
  `git cherry origin/master <tête>` dit ce qui est déjà sur master. C'est la quatrième forme du
  même trou ; le ticket de garde est `d222b6777`.
- **La garde du push veut un journal qui nomme la lib du magasin** dans le dossier de journaux : y
  copier ceux du rejeu.
- **La garde du worktree refuse un fichier ignoré non préservé** : run_e2e.sh laisse
  `target/e2e-last.log` et `charge-last.tsv` dans le worktree. Les archiver d'abord.
- **Aucune ligne d'attribution dans un commit** : une ligne Co-Authored-By a été refusée avant le
  commit, et à raison.

## Ce qui attend

- **La navigation des membres** : branche `navigation-membres`, tête `49d547a07`, cinq commits sur
  `87f764243`, verte. Elle attend le travail d'appartenance de codeparsers, puis la pile. Plus
  tard : lire `extends_type` et `parent_path` quand codeparsers les posera (proposés à c7).
- **Le pont navigateur par MCP** : l'étude est faite ([page 03](03-etude-pont-navigateur.md)) ; la
  recommandation est de forker jackwener/browsermcp. Le choix du socle revient à Lucie.
- **Le plafond du juge** (forme (a), `max_output_tokens`) : remis à l'allumage du juge.
- **Au prochain plafond du target** : le relevé `du` pour G (deps `.rlib` locales contre tierces,
  incremental) avant `cargo clean -p`. `delais-c` pèse 30 Go à 01 h 3x, pour un plafond de 32.

## Reprendre

Rien ne s'efface (targets, worktrees) avant le mot de Lucie sur la fin de session (c7).

| Worktree | Branche | Dernier commit | État |
|---|---|---|---|
| `~/.cache/rag3weaver-build/wt-nav` | `navigation-membres` (poussée) | `49d547a07` | vert, attend codeparsers puis la pile |
| `~/.cache/rag3weaver-build/wt-docs-archi` | détaché sur master | — | mes docs |
| `~/.cache/rag3weaver-build/wt-equipier-fin` | `equipier-fin-avec-ce-qui-reste` | `327c3bba4` | atterri le 13 (`git cherry` vide) ; à retirer au mot de Lucie, par `garde_worktree.py` |

Target de la session : `~/.cache/rag3weaver-build/delais-c`, repris d'un worktree à l'autre par
`RAG3WEAVER_TARGET_REPRENDRE=1` ; lib `373b5477b`, passes des lots Rust seuls sous
`RAG3WEAVER_MOTEUR_ANCIEN=1`, dit au journal.

Pour continuer la navigation :

```sh
cd ~/.cache/rag3weaver-build/wt-nav && git fetch -q origin && git status
# passe : script ~/.cache/rag3weaver-build/wt-nav-passe.sh (ETAPE=module | complete), sous
POSTE_MEM_MAX=32G ./poste lourd bash wt-nav-passe.sh   # depuis ~/.cache/rag3weaver-build
# à la place donnée par c7 : PILE_AVANT=<tête d'avant> ./poste push archi -- \
#   /home/lucied/.cache/rag3weaver-build/push_garde.sh ../.cache/rag3weaver-build/wt-nav <journaux>
```
