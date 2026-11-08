# Knowledge dump — l'arbre principal, 7 novembre 2026

Ce qu'une session qui reprend doit savoir sans relire la journée.

## Les scripts et où ils sont

- **luciepc, `~/.cache/rag3weaver-notes-A/`** :
  - `batir58.sh [<commit>]` : bâtit la lib dans `wt-lib-arbre` (détaché), la publie
    `--sans-courant`, écrit `lib58/id` (le nom du magasin, par le dernier commit moteur), les
    symboles et vtables avant/après, `lib58/fini` ;
  - `lib58/verdict58.sh` : la copie de `docs/procedures/scripts/verdict-e2e-code.sh` (worktree
    `~/git_workspaces/rag3db-plan`, target `target-A-plan`), à lancer avec `VERDICT_FORMAT=43`
    pour une lib du format 43 ; `lib58/v-etat` finit par `fin` ;
  - `mesure-vues/` : les scripts M3-M5 et `e2e_mesure_vues.rs` ; les copies sous
    `~/.local/share/rag3weaver/mesure-vues/`.
- **ici, le dossier de travail de la session** : `whois-push.sh` / `whois-push-fin.sh` (la passe
  et le push d'un lot de code : rebase, check_features, lib entière, navigateur, e2e, relecture de
  master avant le push, trois fois au plus), `docs-push.sh` (un lot de docs : rebase,
  `check_tickets.py`, refus hors `docs/` et `extension/rag3weaver/docs/`, trois contrôles).
- **`docs/procedures/scripts/controle-des-deux-sens.sh <étiquette> <id>`** : l'outil depuis
  `origin/master`, une seconde copie reflink des instantanés, rc 0/1/7.

## Les pièges rencontrés

- `RAG3WEAVER_MOTEUR_ANCIEN=1`, pas `MOTEUR_ANCIEN` ; `RAG3WEAVER_TARGET_REPRENDRE=1` quand un
  target a été bâti par un autre worktree (effacé).
- En Rhai, `trim()` change la chaîne en place et rend `()`.
- Une arête vers un `RhaiNode` remplace toute sa config : le script ne voit que l'entrée de
  l'arête (d'où le `said` générique de whois, jusqu'au pas 3 de 96).
- Un `PlanNode` n'expose qu'une forme sur `rows` ; un `collect` n'a qu'un pas, sans ordre ni
  limite ; un `select` peut partir d'une table enfant par un `through` entrant.
- La valeur d'une Ref est nue : `session:<uuid>` de genre `handle`, jamais `handle:session:…`.
- Un `pgrep -f` lancé par ssh se trouve lui-même ; un motif « fini » trouve « finished ». Une
  surveillance suit un pid, ou une ligne ancrée.
- Le shell de luciepc est fish : `ssh … "bash -c '…'"`.
- L'arbre de Lucie peut avoir du retard sur master : un outil se prend dans `origin/master`.

## Les chiffres à garder

- `code-principal` (instantané du 25 oct., luciepc) : 104 923 morceaux, 65 946 scopes, 6 312
  fichiers ; index `Scope_Chunk_vec__granite_278m`.
- Le contrôle des deux sens, version ancrée : 14 minutes pour `code-rag3db-depot`.
- La lib 17a094d12 : bâtie en 2 min 30 sur un `build/` déjà configuré (889 cibles), verdict en
  4 minutes.
