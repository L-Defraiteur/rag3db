# Journal des chantiers

**À quoi il sert.** Plusieurs sessions (Claude Code, Codex) travaillent sur ce
dépôt, sur plusieurs machines. Ce fichier tient la liste de ce qui est ouvert,
pour qu'un travail non fusionné, non poussé ou en attente d'une décision ne
soit pas oublié. Il ne recopie pas ce que git sait dire : il porte ce que git
ne dit pas.

**Les deux règles.**
1. Une session qui ouvre une branche, ou un chantier hors de ce dépôt, ajoute
   sa ligne ici dans le même commit que son premier travail.
2. Elle met sa ligne à jour quand elle dit « prêt », quand c'est fusionné, et
   quand elle s'arrête en laissant quelque chose.

**La partie qui se recalcule.** Avant de se fier au tableau, relancer
l'inventaire ; s'il contredit le journal, c'est le journal qui a tort.

```bash
git fetch --all --prune
for b in $(git for-each-ref --format='%(refname:short)' refs/heads); do
  up=$(git rev-parse --abbrev-ref "$b@{upstream}" 2>/dev/null)
  echo "$b | derrière/devant master : $(git rev-list --left-right --count master...$b) \
| non poussés : ${up:+$(git rev-list --count $up..$b)}${up:-pas d'amont} | $(git log -1 --format=%cs $b)"
done
git stash list ; git worktree list ; git status --short
```

Dernière mise à jour : **1er octobre 2026**.

## 1. Branches ouvertes

| Branche | Qui | État | Ce qui reste |
|---|---|---|---|
| `mtg-experiments` | Codex (19-23 sept.), puis la session « rag3db Products Experiments » (25-27 sept.) | 18 commits devant `master`, **poussée le 1er octobre** (`a59ca01de`) | Fusion dans `master` à décider. Avant : extraire les quatre correctifs du moteur C++ dans un commit à part (voir §3). Doc de reprise : `extension/rag3weaver/docs/25-septembre-2026-18h27/00-reprise-branche-mtg-experiments.md`, puis `docs/27-septembre-2026-03h57/01-…`. |

Les neuf branches des sessions du 18 septembre (`heuristique-taille`,
`retrait-monolithe-recherche`, `nettoyage-apres-monolithe`,
`doc-dernier-chemin-parallele`, `banc-ponderation`, `banc-ponderation-suite`,
`lifecycle`, `embarquements`, `fts-lucivy-v3`) sont **toutes fusionnées** dans
`master` : elles n'ont rien devant lui et peuvent être supprimées.

## 2. Ce qui est dans git, et ce qui n'y est pas (expérience MTG)

Le dépôt est public. La règle, posée par Lucie le 1er octobre 2026 : **on
pousse tout sauf les données**. Le code des branchements a sa place dans git
même s'il n'est réellement branché que sur le poste.

| Dans git | Hors de git, sur le poste seulement |
|---|---|
| Le moteur, le backend déclaratif, le harnais, le chat, les scripts de préparation et d'ingestion | `experiments/mtga/data/` : bases, snapshots, collection capturée, decks générés, journaux de conversation |
| Le manifeste `backend.json`, les schémas, les graphes, les règles Rhai, le gabarit de rendu | `backend/harness/cards.json` et `wildcards.json` : faits extraits, régénérés par `scripts/prepare_deck_harness.py` |
| Les branchements vers les sources locales : `mtga-reader`, `Player.log`, les SQLite du client Arena | `.venv`, `node_modules`, `rag3bridge/target`, les poids des modèles |
| Les docs des expériences, noms de cartes compris | Les identifiants : `.vault` |

Un chemin absolu du poste traîne dans `experiments/mtga/chat/chat.json`
(`backend_command`) : c'est un branchement local, pas une donnée ; à rendre
relatif le jour où quelqu'un d'autre lance le chat.

À ne pas confondre avec la question des droits, qui reste ouverte avant toute
diffusion d'un produit : `mtga-reader` est sous GPL-3.0 et les conditions de
Wizards ne sont pas clarifiées (`extension/rag3weaver/docs/20-09-2026/15-…`).

## 3. Reste à faire sur du travail déjà fusionné ou poussé

| Chantier | Où | Ce qui reste |
|---|---|---|
| Correctifs du moteur C++ (lambdas sur structs, quantificateurs, `ParsedParameterExpression::copy`, `StringChunkData::finalize`) | dans `ab95c3a2d`, sur `mtg-experiments` | Les extraire sur `master` ; ajouter un test pour `copy`. |
| Repli des KB en entités dérivées | `master`, pas A et B faits | **Pas C** : poids de fusion par entité, pondération par genre dans `Scope`, gabarits de dérivées au catalogue. Attend une décision (§4). |
| Chemin de masse des lots de naissances | `b7ae0a683`, désactivé (`RAG3WEAVER_COPY_NAISSANCES`) | Trouver pourquoi le `COPY` des chunks croît avec la table. Pistes : reconstruction de l'index vectoriel à chaque lot (`ajuster_l_index_pour_le_retard`), relecture `select_node_ids`. |
| Avertissement « 0 relue » de `split_unchanged` | `b7ae0a683` l'a rendu muet pour les lots de naissances | Le remettre quand le chemin de masse n'est pas actif — accordé par Lucie le 1er octobre, en cours dans la session Products Experiments. |
| Champ `folds` des scopes | `98a9f4dac` | Ré-ingérer le code pour le remplir. |
| Base MTG | poste | À reconstruire (environ 8 Go, dont 6 récupérables). |
| Récupération des lignes supprimées dans rag3db | proposé, pas fait | Les blobs d'index sont bornés par une purge côté rag3weaver (`94555c18b`) en attendant. |
| Budget de reprise du lecteur en lecture seule | `master`, mesuré (`20a8f6ee8`) | 250 ms de budget contre un pic à 567 ms sous charge : relever le budget, ou tester l'invariant par `read_only_patient`. Attend une décision. |

## 4. Décisions en attente de Lucie

Posées le 18 septembre 2026, non tranchées depuis :

1. La forme du pas C (fusion pesée par entité, pondération par valeur de champ).
2. Les poids de fusion par défaut : 0,6 / 0,4 du gabarit `search_base`, en
   vigueur depuis `63d4b86b5`, ou 0,3 / 0,7 d'avant.
3. Le texte embarqué : le nom avec le corps (environ +0,06 de MRR au banc).
4. Supprimer `fuse_results` à trois listes et ses onze tests, ou la garder.
5. Supprimer la grappe d'exploration après recherche (`search_with_explore`,
   `explore_bfs`).
6. Supprimer `search_with_strategy` (aucun appelant de production).

Depuis :

7. Le budget de reprise du lecteur (§3).
8. Fusionner `mtg-experiments` dans `master`, et quand.

## 5. Hors de ce dépôt

À confirmer par qui s'en souvient : ces lignes viennent de la mémoire des
sessions, pas d'une vérification.

| Chantier | Où | État connu |
|---|---|---|
| Forks burn, cubecl, cubek | `github.com/L-Defraiteur/{burn,cubecl,cubek}`, branche `rag3weaver/pre.3` | Utilisés tels quels (burn `630c546c`). |
| PR amont burn / cubek | à ouvrir | Sept préparées par la session optimiseur au 18 septembre ; **aucune envoyée**, elles attendent le mot de Lucie. |
| lucivy | crates.io | 4.3.0 utilisée par `mtg-experiments`. |
| Amont Vela | remote `vela`, branche `storage/concurrent-checkpoint-recovery` (27 septembre) | À lire : elle touche la reprise après checkpoint, donc peut-être le WAL illisible (§6). |

## 6. Bugs connus, non corrigés

- **WAL illisible après un arrêt brutal** (`wal_record.cpp:79`). En attendant :
  arrêt par SIGTERM ou EOF, copie reflink avant une longue écriture, un seul
  processus par base.
- **Persistance des `abilities` imbriquées** : après réouverture, des textes
  rattachés au mauvais élément. Bloquant pour les filtres sur ce champ.
- **SIGSEGV avec un buffer pool de 1 Gio** ; contournement : 8 à 15 Gio.
- La synchronisation MTG ne fait que des upserts : pas de suppression des
  cartes absentes d'un nouveau snapshot.
- `list_filter.cpp:113` lit `inputVector.isNull(i)` au lieu de `pos`.

## 7. Ménage

- `git worktree prune` : trois worktrees dont les dossiers n'existent plus
  (`rag3db-embarquements`, `rag3db-recherche`, `rag3db-lifecycle`).
- Fichiers non suivis à la racine, à supprimer : `follows.csv`, `user.csv`,
  `user.parquet` (restes d'une démo du 7 septembre), `build-lecteurs-csv.log`,
  `build-rag3weaver.log`.
