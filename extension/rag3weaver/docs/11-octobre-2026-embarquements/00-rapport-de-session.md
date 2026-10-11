# Session embarquements (chantier F) — rapport du 11 octobre 2026, après-midi

*Suite du rapport de nuit (`11-octobre-2026-01h14/embarquements/01-rapport-de-session.md`).*

## Ce qui est sur master

| hash | lot |
|---|---|
| `26216e3be` | la page du chaînage (`01-chainage-des-formes.md`), sans code |
| `76cf86cbc` | chaînage, lot 1 : le graphe, la vérification commune, le repli découpé, le refus nommé, le rapport `atomic` / `instructions` / `fused` |
| `56a680169` | PostgreSQL : le plancher de rappel `pg_trgm` voyage avec la requête (`SET LOCAL`), ticket fermé |
| `7da725287` | chaînage, lot 2 : la fusion rag3db, prouvée contre le repli |

Décisions de Lucie (11 octobre) : pas d'arête d'ordre seul pour commencer, mais
l'arête porte zéro ou plusieurs colonnes ; un graphe non atomique s'exécute et le
dit, refusé seulement si l'appelant exige l'atomicité.

## Ce qui reste

- **La fusion PostgreSQL** (lot 3) : traduire d'abord le saut (`Hop`, refusé
  aujourd'hui par P3), puis fondre en CTE, contre le scénario d'`e2e_chainage`
  joué sur le conteneur (port 5433).
- L'entrée `$items` d'une écriture nourrie par une arête, et l'arête d'ordre seul :
  refusées en le disant, à ouvrir quand un appelant en aura besoin.
- Brancher des appelants réels sur `Catalog::run_plan` (aujourd'hui, seuls les
  témoins s'en servent).
- En fin de session : effacer `target-F` ici et `target-embarquements` sur luciepc.

## Ce qu'il faut savoir pour reprendre

- Worktree `~/git_workspaces/rag3db-embarquements`, branche `chainage-1`, poussée
  sur master par `git push origin HEAD:master`. Sous-modules à initialiser dans un
  worktree neuf (`git submodule update --init`), sinon cargo ne trouve pas
  `codeparsers`.
- Scripts : `~/.cache/rag3weaver-build/embarquements/` — `chainage.sh <étiquette>`
  (lib, check --tests, e2e_chainage ; `FILTRE=…` pour la lib, `SANS_E2E=1`),
  `plancher.sh` (e2e_postgres sur le conteneur), `deux-suites.sh <suites…>`. Tout
  sous `~/.cache/rag3weaver-build/poste lourd`.
- Le dialecte rag3db s'appelle `rag3db`, celui de PostgreSQL `postgresql` (le nom
  qui paraît dans les refus).
- Une fusion Cypher repose sur le `WITH` qui fait sortir de portée les variables
  d'amont : l'aval redéclare `m`, `d`, `r`, `u` sans conflit, et les prédicats
  compilés (`Predicate::Compiled`, sur l'alias `m`) restent justes.
- Un ticket qui se ferme : `git mv` vers `closed/` **et** sa ligne déplacée dans
  `docs/tickets/README.md`, dans le même commit.
