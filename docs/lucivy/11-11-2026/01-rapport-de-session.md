# lucivy — rapport de session au 11 novembre 2026

*Écrit pour l'agent qui reprendra lucivy sans aucune mémoire des échanges précédents. Les termes
propres au projet sont définis la première fois qu'ils servent ; le reste du contexte est dans
`02-knowledge-dump.md`, à côté.*

**lucivy** est le moteur de recherche plein texte (BM25, sous-chaînes, flou, regex, positions exactes
des occurrences) que rag3weaver utilise à côté de la base rag3db. Son dépôt est séparé :
`https://github.com/L-Defraiteur/lucivy`, cloné sur ce poste dans `~/git_workspaces/lucivy` et sur
l'autre poste, **luciepc** (la machine de calcul de Lucie, joignable par `ssh lucied@luciepc`), dans
`~/git_workspaces/lucivy`.

## 1. La version publiée

**4.3.2, publiée le 7 novembre 2026** sur crates.io (`ld-lucivy`, `lucivy-core`, `luciole`,
`lucistore`, `sparse-vector` ; `lucivy-fst` reste en 0.1.1), npm (`lucivy`, ses cinq paquets de
plateforme, `lucivy-wasm`) et PyPI (`lucivy`, cinq wheels et le sdist). Tag `v4.3.2` sur le commit
`dd70c04` de `main`. Elle porte quatre correctifs de recherche (le détail et les chiffres sont dans
`CHANGELOG.md` du dépôt lucivy) :

1. un passage exact d'un document est retrouvé à toute longueur (il cessait de l'être vers
   48 octets : la chaîne de morceaux qui épelle la requête était coupée à 8 positions) ;
2. une requête floue longue ne coûte plus en O(n²) de sa longueur (200 mots à distance 1 sur un
   index à quatre shards : 15,8 s → 96 ms) ;
3. les deux apostrophes `'` et `’` se trouvent l'une l'autre (la valeur est cherchée sous ses deux
   formes, à la requête, sans toucher l'index) ;
4. les clés de requête inconnues (`boost`, `offset`, `slop`, une coquille) sont nommées par
   `query_warnings` au lieu d'être ignorées en silence.

La version d'avant, 4.3.1 (25 octobre), corrigeait une recherche qui paniquait sous écritures
concurrentes et tuait l'acteur d'un shard (documents perdus derrière des commits « réussis », gels).

rag3weaver prend `lucivy-core = "4.3.2"` du registre (`extension/rag3weaver/Cargo.toml`), sans
chemin local ni `[patch.crates-io]`.

## 2. La branche `main` de lucivy et ce qui n'est pas publié

`main` est à `b359f50` (le `CLAUDE.md` du dépôt mis à jour après la publication). Rien de code n'est
sur `main` sans être publié.

**Deux PR ouvertes, vertes (32 checks sur 32), en attente du mot de Lucie pour être fusionnées :**

| PR | branche | ce qu'elle fait |
|---|---|---|
| #22 | `4.3.2.1-corpus-absent` (`b98a731`) | un test de vérité terrain qui ne trouve pas son corpus échoue au lieu de passer ; `LUCIVY_CORPUS_ABSENT_OK=1` permet l'absence par son nom (posé dans le job `core` de la CI) |
| #23 | `4.3.2.2-close-arrete-ses-pools` (`c040e0f`) | `ShardedHandle::close()` arrête et attend ses acteurs même quand une écriture échoue (cause d'un SIGSEGV à la sortie des e2e de rag3weaver) |

Ni l'une ni l'autre n'est publiée : elles iront dans une 4.3.3, que seule Lucie décide.

**Pourquoi « au mot de Lucie »** : la mémoire de la session lucivy porte sa règle du 13 septembre
2026 — fusionner dans `main` de lucivy est sa décision. Une autre session (l'orchestration) a invoqué
sa règle du 3 octobre (« fusionner dès que fini et vert », écrite pour rag3db) ; la question lui a été
posée dans la session lucivy et n'a pas encore de réponse. Un accord relayé par une autre session ne
vaut pas le sien.

**Branches de docs non fusionnées** (rien de code) : `docs-pipeline-publication` (la page du pipeline
de publication, la relecture de la 4.3.1, la note sur les options de recherche pour rag3weaver) et
`tickets-2026-11-07` (le dossier `docs/tickets/` de lucivy, voir § 5).

## 3. La CI publie sur un tag

`.github/workflows/release.yml` se déclenche sur le **push d'un tag `v*`** : il rejoue la CI, bâtit
tout, puis publie sur PyPI, npm et crates.io par « trusted publishing » (authentification OIDC de
GitHub Actions, sans jeton ni code à usage unique). **Pousser un tag, c'est publier, partout, sans
retour possible sur crates.io.** Aucun tag sans le mot explicite de Lucie dans la session.

## 4. `~/lucivy_bench` effacé le 8 novembre

Sur luciepc, `~/lucivy_bench` (97 Gio) a été effacé le 8 novembre par la session d'orchestration, au
mot de Lucie, pour libérer le disque plein à 85 % — sans attendre la réponse de la session lucivy à
la question de ce qui n'était pas régénérable (l'orchestration l'a reconnu). Partis :

- le noyau Linux `linux-7.2` et `linux-2.6.0` (régénérables, voir le knowledge dump) ;
- tous les index de banc (`idx-*`, `compare-4.1`, `lucivy_bench_sharding`, …), régénérables par les
  bancs ;
- **perdus pour de bon** : les scripts de mesure écrits à la main dans `scratch-positions/`
  (`cold.py`, `compat.mjs`, `ab.sh`, `after-fix.sh`, `compact-bench/`) et leurs relevés. Leurs
  méthodes et leurs chiffres restent dans les docs du dépôt lucivy (`docs/11-09-2026/`,
  `docs/13-09-2026/`). S'ils se réécrivent : dans le dépôt (`benches/` ou `playground/tools/`).

Conséquence : les liens `/tmp/lucivy-cmp` et `/tmp/lucivy-cmp-90k` (qui pointaient vers le noyau)
n'existent plus. **Tant que la PR #22 n'est pas fusionnée**, un test de vérité terrain lancé sans
corpus passe vert sans rien vérifier ; aucune mesure lucivy ne se dit verte sans avoir lu le compte
de fichiers du corpus dans la sortie.

## 5. Les tickets

Un ticket est un fichier qui décrit un défaut connu non corrigé (convention de rag3db). lucivy a son
propre dossier `docs/tickets/`, **sur la branche `tickets-2026-11-07` seulement** (pas encore sur
`main`). Leur état réel :

| ticket (branche `tickets-2026-11-07`) | état réel |
|---|---|
| `2026-11-07-apostrophe-typographique.md` | corrigé dans 4.3.2 (à la requête) ; le fichier dit encore « ouvert » ; reste une décision : normaliser aussi à l'indexation (change le format) |
| `2026-11-07-cles-de-requete-ignorees.md` | corrigé dans 4.3.2 (avertissement) ; le fichier dit encore « ouvert » ; reste une décision : refuser ces clés (une 4.4.0, incompatible) |
| `2026-11-07-un-passage-exact-long-n-est-plus-trouve.md` | corrigé dans 4.3.2 ; le fichier dit encore « ouvert » |
| `2026-11-08-un-test-de-corpus-sans-son-corpus-passe-vert.md` | ouvert, corrigé par la PR #22 (pas fusionnée) |
| `2026-11-08-close-sort-sur-l-erreur-sans-arreter-ses-pools.md` | ouvert, corrigé par la PR #23 (pas fusionnée) |

Dans `docs/tickets/` de **rag3db**, un ticket est à la session lucivy :
`2026-11-07-une-requete-floue-de-lucivy-coute-en-proportion-de-ses-termes.md` — corrigé par la 4.3.2
(le flou long) ; à fermer, pas encore déplacé (l'orchestration dira quand). Les trois tickets lucivy
de rag3db du 19 et du 25 octobre sont déjà dans `docs/tickets/closed/`.

## 6. Ce qui est bloqué, et sur quoi

- **Les PR #22 et #23** : le mot de Lucie pour fusionner.
- **Toute mesure de lucivy** : le corpus du noyau à recloner sur luciepc (et la PR #22 pour que son
  absence se voie).
- **Les deux décisions de format** (apostrophe à l'indexation, refus des clés inconnues) : Lucie.

## 7. Les trois prochains pas

1. Au mot de Lucie : fusionner #22 puis #23 dans `main` (rebase, CI verte sur le SHA exact de
   `main`), puis passer les trois tickets corrigés en « fermé » sur `tickets-2026-11-07` et proposer
   de fusionner cette branche et `docs-pipeline-publication`.
2. Ouvrir le ticket du second garde-fou proposé pendant la 4.3.1 : dans `luciole` (le planificateur
   d'acteurs de lucivy), un acteur qui meurt devrait répondre une erreur à chaque message encore dans
   sa boîte, au lieu de les jeter (« Reply dropped without send() »).
3. Recloner le noyau sur luciepc et rejouer la vérité terrain complète avant toute 4.3.3.

## 8. Reprendre

Chaque travail non fini, à reprendre de son point et non de `main`. Les tests lourds se jouent sur
luciepc, sous `~/.cache/rag3weaver-build/poste lourd <commande>` (le verrou qui limite le nombre de
compilations simultanées sur la machine), avec `CARGO_BUILD_JOBS=8` au plus. `gh` a deux comptes :
avant toute commande `gh` sur lucivy, `gh auth switch -u L-Defraiteur`, et revenir ensuite au compte
actif d'origine (`gh auth switch -u luciedefraiteur-sairen`).

### PR #22 — les tests de corpus échouent sans leur corpus

- **Chemin** : `~/git_workspaces/lucivy-corpus` (ce poste), branche `4.3.2.1-corpus-absent`, dernier
  commit `b98a731`, poussé sur `origin`.
- **Fait** : `lucivy_core/tests/common/mod.rs` (`usable`, `recipe`), les quatre fichiers de corpus
  convertis, le témoin `lucivy_core/tests/test_missing_corpus.rs`, la variable dans `.github/workflows/ci.yml`
  (job `core`). Mesuré sans corpus : rouge sans la variable, vert avec. CI de la PR : 32 sur 32.
- **Reste** : la fusion, au mot de Lucie seulement.
- **Reprendre** :
  ```bash
  cd ~/git_workspaces/lucivy-corpus && git fetch origin && git status && git log -1 --oneline
  git rebase origin/main            # si main a bougé ; puis repousser sous un nom neuf si l'historique change (jamais de push en force)
  LUCIVY_CORPUS_ABSENT_OK=1 cargo test -p lucivy-core --no-fail-fast      # sur luciepc, sous poste lourd
  cargo test -p lucivy-core --test test_dictionary_index                  # sans corpus : doit échouer en nommant la recette
  gh pr checks 22 --repo L-Defraiteur/lucivy
  gh pr merge 22 --repo L-Defraiteur/lucivy --rebase                      # SEULEMENT au mot de Lucie ; jamais --delete-branch
  ```

### PR #23 — `close()` arrête ses pools même en échec

- **Chemin** : `~/git_workspaces/lucivy-close` (ce poste), branche `4.3.2.2-close-arrete-ses-pools`,
  dernier commit `c040e0f`, poussé sur `origin`.
- **Fait** : `close()` dans `lucivy_core/src/sharded_handle.rs` tente chaque écriture, garde la
  première erreur, arrête et attend les acteurs puis rend l'erreur ; le témoin
  `a_failed_close_still_stops_its_pools` (dans le même fichier), rouge avec l'ancien `close()`, vert
  3 fois sur 3 ; `lucivy-core` 216 passés ; clippy vert ; CI de la PR : 32 sur 32.
- **Reste** : la fusion, au mot de Lucie seulement. Si #22 est fusionnée d'abord, rebaser #23 sur
  `main` et rejouer sa CI.
- **Reprendre** :
  ```bash
  cd ~/git_workspaces/lucivy-close && git fetch origin && git status && git log -1 --oneline
  cargo test -p lucivy-core --lib a_failed_close_still_stops_its_pools   # sur luciepc, sous poste lourd
  gh pr checks 23 --repo L-Defraiteur/lucivy
  gh pr merge 23 --repo L-Defraiteur/lucivy --rebase                      # SEULEMENT au mot de Lucie
  ```

### Après une fusion dans `main`

```bash
gh run list --repo L-Defraiteur/lucivy --commit <sha de main> --json name,status,conclusion
```
La CI doit être verte sur le SHA exact de `main`. Aucun tag : une 4.3.3 se publie au seul mot de
Lucie (voir le knowledge dump, « Publier »).

### Branches de docs

`tickets-2026-11-07` (dernier commit `5951858`) et `docs-pipeline-publication` (dernier commit
`a49e18c`) : aucun worktree ouvert. Pour mettre à jour l'état des tickets :
`git worktree add ../lucivy-tickets origin/tickets-2026-11-07 -B tickets-2026-11-07`, éditer, commit
par chemins, `git push origin tickets-2026-11-07`, puis `git worktree remove ../lucivy-tickets`.

## 9. Ce qui reste sur disque

- Ce poste : `~/git_workspaces/lucivy` (clone principal, `main` ; des suppressions non commitées sous
  `bindings/nodejs/node_modules` et `.gitignore` appartiennent à Lucie, ne pas y toucher) ;
  `~/git_workspaces/lucivy-corpus` et `~/git_workspaces/lucivy-close` (les worktrees des PR #22 et
  #23, propres, poussés) ; `~/git_workspaces/lucivy-figement-G` (worktree de la session
  « optimiseur », pas à la session lucivy).
- luciepc : `~/git_workspaces/lucivy` (clone principal) et `~/git_workspaces/lucivy-figement-G`
  (à l'optimiseur). Rien d'autre de lucivy ; plus de `~/lucivy_bench`.
- La mémoire de la session lucivy : `~/.claude/projects/-home-lucied-git-workspaces-lucivy/memory/`.
