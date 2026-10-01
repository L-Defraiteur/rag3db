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

Dernière mise à jour : **1er octobre 2026**, après la fusion de `mtg-experiments`.

**À lire avec ce journal** : la réconciliation des objectifs et le knowledge
dump du 1er octobre 2026, dans
`extension/rag3weaver/docs/1-octobre-2026-22h47/` — ce qu'on voulait, ce qui
existe, ce qui est en suspens, et l'ordre proposé. Le rapport de session du même soir
(`05`) dit ce qui est en vol et comment le contrôler.

## 1. Branches ouvertes

*Aucune.* `correctif-wal-enregistrements-longs` a été fusionnée dans `master`
en avance rapide le 1er octobre 2026 (`955b1b136`, `84d783afc`), après
reconstruction de `build/lecteurs-csv` et des suites vertes.

`mtg-experiments` a été fusionnée dans `master` le 1er octobre 2026
(fusion `32d6e0b44`, `master` poussé à `d4f32f9e0`) ; la branche reste sur
`origin`, à supprimer quand Lucie le dira.

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
| Repli des KB en entités dérivées | `master`, pas A et B faits | **Pas C** : poids de fusion par entité, pondération par genre dans `Scope`, gabarits de dérivées au catalogue. Attend une décision (§4). |
| Chemin de masse des lots de naissances | `fa70cf8f3`, désactivé (`RAG3WEAVER_COPY_NAISSANCES`) | Trouver pourquoi le `COPY` des chunks croît avec la table. Pistes : reconstruction de l'index vectoriel à chaque lot (`ajuster_l_index_pour_le_retard`), relecture `select_node_ids`. |
| Deck builder MTG (produit) | `experiments/mtga`, `master` | Descriptions d'outils propres à chaque entité (description d'entité dans le manifeste, au lieu du même texte pour tous les `search_*`) ; compter artefacts et créatures de mana comme sources de couleur dans le harnais (accordé, pas fait) ; barre de défilement du chat dont la taille ne suit pas la liste (capture attendue) ; option Gemini via Vertex. |
| Synchronisation par identifiant : supprimer les lignes disparues d'un snapshot | moteur (`ingest_snapshot` / `EntityBatchNode`) | Décidé avec Lucie le 1er octobre, après les étapes D et E du WAL. Générique ; aujourd'hui seuls les fichiers de code le font (`reingest_file`). |
| Lever le refus d'écrire en lot sur une entité à `Lifecycle` | `backend_nodes.rs` (« bulk snapshot writes cannot bypass a lifecycle ») | Décidé avec Lucie le 1er octobre, après D et E. Depuis le 18 septembre `ingest_entities` garde les transitions lui-même : le refus peut devenir une vérification. |
| Champ `folds` des scopes | `1e5eea234` | Ré-ingérer le code pour le remplir. |
| Base MTG | poste | À reconstruire (environ 8 Go, dont 6 récupérables). |
| Récupération des lignes supprimées dans rag3db | proposé, pas fait | Les blobs d'index sont bornés par une purge côté rag3weaver (`d1aa7d296`) en attendant. |
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
8. ~~Fusionner `mtg-experiments` dans `master`~~ — **tranchée et faite le
   1er octobre 2026** : correctifs C++ extraits seuls (`9edf6f3b4`, avec le
   test de `ParsedParameterExpression::copy` prouvé rouge sans le correctif),
   fusion `32d6e0b44`, trois corrections de tests et du chat par-dessus, `master`
   vert puis poussé (`d4f32f9e0`).
Tranchée le 1er octobre 2026 : les trailers d'attribution à une IA
(`Co-Authored-By: Claude…`) ont été retirés des 17 commits de
`mtg-experiments` qui en portaient, par réécriture des messages et push en
force ; arbres, auteurs et dates inchangés, **hash changés** à partir de
`64a12b05b` (le commit de Codex `ab95c3a2d` garde le sien). Sur `master`,
quatre commits anciens en portent encore — un de février 2026, trois de
l'amont Kuzu de 2025 — et ne sont pas réécrits : cela changerait tous les
hash du dépôt public.

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

- **WAL illisible : la vraie cause est un bug d'écriture**, pas l'arrêt
  brutal (trouvé le 1er octobre 2026). `resizeBufferIfNeeded`
  (`src/storage/wal/checksum_writer.cpp`, même défaut dans
  `checksum_reader.cpp`) remplace le tampon de 4096 octets sans recopier ce
  qui y était : tout enregistrement de plus de 4 Kio perd son début, et sa
  somme de contrôle, calculée sur le tampon faux, ne voit rien. Un arrêt
  propre supprime le journal ; un arrêt brutal force le rejeu d'un journal
  déjà faux (`wal_record.cpp:79` ou `:76`). Hérité de l'amont (#5940), non
  corrigé chez Vela. **Le bug d'écriture est corrigé sur `master`** (`955b1b136`, étape D :
  recopie des deux côtés, message « journal corrompu », doc
  `docs/1-octobre-2026-23h37/01-…`). **Reste ouvert : la fin déchirée**
  (étape E, décidée par Lucie le 1er octobre : rouvrir au dernier COMMIT,
  écarter le reste en le copiant à côté) ; d'ici là un arrêt brutal pendant
  l'écriture d'un commit peut encore refuser l'ouverture. En attendant : arrêt par SIGTERM ou
  EOF, copie reflink avant une longue écriture, un seul processus par base.
  **La base MTG actuelle a un `.wal` déjà corrompu (30 enregistrements
  illisibles) : ne pas l'ouvrir, et jamais avec
  `throw_on_wal_replay_failure=false`** — le rejeu tronquerait le journal.
- **Persistance des `abilities` imbriquées** : après réouverture, des textes
  rattachés au mauvais élément. Bloquant pour les filtres sur ce champ.
- **SIGSEGV avec un buffer pool de 1 Gio** ; contournement : 8 à 15 Gio.
- La synchronisation MTG ne fait que des upserts : pas de suppression des
  cartes absentes d'un nouveau snapshot (chantier au §3).
- `list_filter.cpp:113` lit `inputVector.isNull(i)` au lieu de `pos`.

## 7. Ménage

- **Sur cette machine (ROG Flow Z13)**, les poids minilm et multilingual-minilm
  sont installés depuis le 1er octobre dans `~/.cache/rag3weaver/` (procédure
  de `extension/rag3weaver/generated/README.md`, sha256 vérifiés) : sans eux,
  16 tests e2e ne tournent pas et ne doivent pas être comptés verts.

- `git worktree prune` : trois worktrees dont les dossiers n'existent plus
  (`rag3db-embarquements`, `rag3db-recherche`, `rag3db-lifecycle`).
- Fichiers non suivis à la racine, à supprimer : `follows.csv`, `user.csv`,
  `user.parquet` (restes d'une démo du 7 septembre), `build-lecteurs-csv.log`,
  `build-rag3weaver.log`.
