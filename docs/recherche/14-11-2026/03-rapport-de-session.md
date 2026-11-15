# Rapport de session — recherche, nuit du 14 au 15 novembre 2026

Session recherche (97). File en pile (`docs/orchestration/14-11-2026/03-la-file-en-pile.md`), lib
épinglée 373b5477b. Les lots dont le C++ des sources est plus neuf que cette lib se sont joués sous
`RAG3WEAVER_MOTEUR_ANCIEN=1`, dit au résumé de chaque passe.

## Lots sur master (le plus récent d'abord)

| hash | lot | preuve |
|---|---|---|
| 68b1454d8 | **L'erreur d'origine d'un BEGIN refusé** (`read_pool.rs`, `backend.rs`) : `Phase::Rompu` porte la cause du BEGIN et dit si la base est à rouvrir. La requête rompue et la réponse la disent ; sur une base à rouvrir, l'appel ne recommence pas. | rouge en 48de02f99 (cause absente ; second essai lu sur une base à rouvrir), vert en 4501cd38c ; passe entière verte sur 766e5bff2 |
| 262be6fc4 | **Les orphelins du plein texte** (`catalog.rs`, `record_nodes.rs`) : un `FlushNode` suit `DeleteRecordNode`, et l'offset d'une ligne inconnue du cache se lit par `select_node_ids`. | deux témoins de lib, avec et sans redémarrage, rouges puis verts ; banc au bit près sur la base migrée : écart File de 1 à 0, et +203 → 0 à l'index entier |
| 9fd946ec3 | **Le filtre `_uuid IN` par la clé primaire** (`read_view.rs`, `dialect.rs`) : page `02-le-filtre-in-par-la-cle.md`. | témoin 1,10 s → 0,28 s ; banc 30/30 au bit près ; sonde d'ED, étape 3 : 5 008 → 113,6 ms |

**La cause des orphelins, corrigée en route.** Ma première lecture était le cache uuid → offset vide
après un redémarrage. La trace l'a démentie : le témoin rougissait aussi avec le cache plein. Le drain
ne validait jamais au plein texte une table qui ne fait que perdre des lignes, puisqu'aucun
`FlushNode` ne suivait `DeleteRecordNode`. Le `delete_by_node_id` restait en attente, puis se perdait
à la fermeture. Le cache vide est la seconde moitié du défaut : après un redémarrage, il n'y avait
aucun offset à retirer.

**Deux têtes données trop tôt.** Sur le lot du BEGIN, j'ai donné ma tête avant check_features, contre
la règle de la pile. 96 a vu le rouge du jeu sans feature (un import `AtomicBool` inutilisé). Le jeu
`rag3db-native,daemon` a rougi ensuite sur un crochet de test inutilisé. Les deux sont corrigés par
des commits ajoutés, et la tête n'a été redonnée qu'après check_features vert sur les 8 jeux.

## Tickets

- Ouvert, à moi : `tickets/2026-11-14-le-temoin-du-chat-d-une-tache-rougit-sous-charge.md`. Un seul
  témoin navigateur sur 45 est rouge sous la charge d'autres passes (la page reste vide, rien en vol),
  et vert trois fois sur trois quand il est joué seul.
- Fermé par G (une base Postgres par passe) :
  `2026-11-14-deux-passes-concurrentes-partagent-la-base-postgres-de-test.md`.
- Laissé ouvert pour le cœur, avec ma ligne d'état écrite par ED :
  `docs/everything-declarative/tickets/2026-11-14-un-in-sur-la-cle-primaire-balaie-toute-la-table.md`.
  Le contournement rag3weaver est sur master ; le planificateur reste à corriger.

## Ce que j'ai appris en passe

- Le filtre du témoin de `passe_epinglee.sh` doit désigner un test **e2e** : `run_e2e.sh` est strict,
  et un témoin de lib se joue à l'étape lib. Un « le filtre n'a désigné aucun test » peut venir d'un
  bâti échoué : lire `^error` dans `temoins.log`.
- `CHECK_FEATURES_LOGS=<dossier>` garde le journal d'un jeu rouge, qui part sinon dans `/dev/null`.
- Un `RUSTFLAGS="-D warnings"` global prend aussi les dépendances (`tools/rust_api`) et ne reproduit
  pas check_features : passer par `scripts/cargo_garde.py check …`.
- Une sonde sous `poste mesure` attend sa place derrière les lourds en cours : l'heure de la mesure
  se lit au journal, pas au lancement (la sonde de l'IN, lancée à 2 h 51, a tourné à 3 h 37).

## Ce qui reste sur disque

À **luciepc** (rien n'est effacé avant le mot de Lucie, selon c7) :
- les worktrees détachés `~/git_workspaces/rag3db-filtre` (9fd946ec3, avec la sonde d'ED reprise sans
  commit : `tests/sonde_in_par_cle.rs` et son entrée de Cargo.toml), `rag3db-orphelins` (262be6fc4) et
  `rag3db-begin` (68b1454d8), tous atterris ;
- les targets `target-97f` (11 Go), `target-97g` (20 Go), `target-97h` (16 Go) et `target-97h-cf`
  (0,5 Go) ;
- les journaux de passe `~/.cache/rag3weaver-build/passe-{filtre,orphelins,begin}-*`.

Les deux copies de la base migrée de la sonde sont effacées.

Sur le **portable** : les worktrees `rag3db-filtre` (branche `begin-erreur-d-origine-pile`, atterrie),
`rag3db-orphelins` (branche `orphelins-pile-3`, atterrie) et `rag3db-docs11` (docs). Tous sont propres.

## Reprendre

**Aucun lot de code inachevé.** Les trois lots de la nuit sont sur master. Une seule branche n'a pas
atterri : `mesure-exclusions` (8be832525), avec le test de mesure
`e2e_code_sync::mesure_ce_que_index_exclude_retire_du_plein_texte`. Ses chiffres sont déjà dans
`01-ce-qu-on-n-indexe-pas.md` ; la faire atterrir ou non reste une décision à prendre.

Pour le ménage, au mot de Lucie (bash) :

```bash
ssh -T lucied@luciepc bash -s <<'X'
cd ~/git_workspaces/rag3db
for w in rag3db-filtre rag3db-orphelins rag3db-begin; do git -C ~/git_workspaces/$w status --short; done
# vides sauf rag3db-filtre (la sonde d'ED, sur sa branche sonde-in-par-cle) : alors
git worktree remove --force ~/git_workspaces/rag3db-filtre
git worktree remove ~/git_workspaces/rag3db-orphelins
git worktree remove ~/git_workspaces/rag3db-begin
rm -rf ~/.cache/rag3weaver-build/target-97f ~/.cache/rag3weaver-build/target-97g ~/.cache/rag3weaver-build/target-97h ~/.cache/rag3weaver-build/target-97h-cf
X
```

La prochaine piste de la session est le ticket du témoin du chat sous charge : lire ce que la page
attend quand elle reste vide sans rien en vol.
