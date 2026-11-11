# lucivy — rapport de session au 11 novembre 2026

*Écrit pour l'agent qui reprendra lucivy sans aucune mémoire des échanges précédents. Les termes
propres au projet sont définis la première fois qu'ils servent ; le reste du contexte est dans
`02-knowledge-dump.md`, à côté.*

**lucivy** est le moteur de recherche plein texte (BM25, sous-chaînes, flou, regex, positions exactes
des occurrences) que rag3weaver utilise à côté de la base rag3db. Son dépôt est séparé :
`https://github.com/L-Defraiteur/lucivy`, cloné sur ce poste dans `~/git_workspaces/lucivy` et sur
l'autre poste, **luciepc** (la machine de calcul de Lucie, joignable par `ssh lucied@luciepc`), dans
`~/git_workspaces/lucivy`.

## 1. La version publiée : 4.3.3

Mise à jour du 11 novembre : **Lucie annonce la 4.3.3 publiée et vérifiée sur
crates.io, npm et PyPI**, PR #22 à #25 fusionnées, `main` à `663975a` avant le tag
`v4.3.3`, workflow de publication vert, puis note `CLAUDE.md` à `e805235`.
La démo reste en 4.3.2. Ces vérifications de registres et de workflow sont celles
transmises par Lucie via l’orchestration ; cette session ne les a pas rejouées.

Vérifications locales de cette session : lecture du `CHANGELOG.md` au tag
`v4.3.3` (commit `663975adf11fc4a9d6cadf90e70ee68f6c9db075`),
du commit `e805235` et de la présence du témoin amont nommé ci-dessous. Le clone
principal local reste sur une ancienne tête : sa branche courante ne représente
pas la version publiée. Le témoin ciblé de fermeture a ensuite été exécuté ; preuve ci-dessous.

La 4.3.3 conserve le format d’index. Elle corrige `ShardedHandle::close()` :
chaque écriture est tentée, la première erreur est conservée, les pools lecteurs,
routeur et shards sont arrêtés et attendus avant de rendre cette erreur.
L’arrêt du routeur est lui aussi attendu. Le garde-fou des corpus absents est
intégré : les tests concernés échouent sans corpus, sauf permission explicite
`LUCIVY_CORPUS_ABSENT_OK=1` ; cette permission ne prouve aucune vérité terrain.

La 4.3.2 (7 novembre) reste la version des correctifs du passage exact long,
du coût quadratique du flou long, des apostrophes à la requête et des avertissements
de clés inconnues. La 4.3.1 corrigeait les paniques sous écritures concurrentes.

## 2. Intégration dans rag3weaver

Le manifeste relu par cette session prend encore `lucivy-core`, `ld-lucivy` et
`sparse-vector` en **4.3.2**, depuis le registre. **La session recherche tient le
lot dépendances et Cargo.lock vers 4.3.3** ; la publication amont ne signifie pas
que ce lot est déjà livré dans rag3db.

Coordination avec recherche, recentrée ensuite par Lucie via l’orchestration :
lot distinct `cx-recherche-lucivy-4-3-3`, après stabilisation du lot d’arrêt,
limité aux minimums des dépendances et au lock ciblé. **Aucun nouveau témoin de
compatibilité 4.3.2 → 4.3.3, banc ou migration** : le format est inchangé.
La réouverture et la recherche existantes restent dans la passe habituelle.

La seule preuve dédiée porte sur `close()` en erreur : pools et acteurs arrêtés,
processus terminé proprement. Point de départ amont :
`a_failed_close_still_stops_its_pools` dans
`lucivy_core/src/sharded_handle.rs` (dossier d’index supprimé avant `close()`,
erreur rendue et acteurs libérés). Preuve amont exécutée ensuite par cette session sur luciepc :

```bash
~/.cache/rag3weaver-build/poste lourd timeout --signal=TERM --kill-after=30s 1800 env CARGO_BUILD_JOBS=8 CARGO_TARGET_DIR=/home/lucied/.cache/rag3weaver-build/cx-lucivy-target cargo test -j8 -p lucivy-core --lib a_failed_close_still_stops_its_pools -- --nocapture
```

Worktree isolé `cx-lucivy-close-433`, tag `v4.3.3` au commit
`663975adf11fc4a9d6cadf90e70ee68f6c9db075`, sources propres après exécution.
**1 passé, 0 échec, 0 ignoré, 96 filtrés**, 0,16 s ; processus terminé code 0.
Compilation en 45,36 s, sept avertissements ; ce n’est pas un passage de clippy.
Le tag ne suit pas de Cargo.lock : résolution générée localement, sans modifier
les sources. Le planificateur annonce 24 threads à l’exécution ; la compilation
est bien limitée à huit jobs.

Journal complet : [preuves/close-4.3.3.log](preuves/close-4.3.3.log), SHA-256
`3a2208790e67d2216a55d1c2c9d2a30bde79cb56ff0569054838562a29e69274`.
Copie luciepc : `~/.cache/rag3weaver-build/journaux-lucivy/close-433.log`.
Cette preuve couvre le témoin amont et sa sortie propre, pas encore l’intégration
rag3weaver ni une passe complète. Résultat transmis à recherche.

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

Conséquence historique : les liens de corpus ont disparu lors de ce ménage.
Leur recréation sur luciepc n’a pas été vérifiée dans cette reprise. Depuis la
4.3.3, l’absence du corpus est signalée par les tests concernés ; lire tout de
même le nombre de fichiers réellement jugés avant de parler de vérité terrain.

## 5. Tickets

Les correctifs des tickets de corpus absent (#22) et de fermeture sur erreur
(#23) sont désormais publiés en 4.3.3, selon la transmission de Lucie.
L’état rédactionnel actuel des tickets du dépôt lucivy n’a pas été relu ; ne
pas reprendre l’ancienne branche `tickets-2026-11-07` pour refaire les fusions
#22 à #25 déjà annoncées comme terminées.

Le ticket rag3db du flou long est déjà dans
`docs/tickets/closed/2026-11-07-une-requete-floue-de-lucivy-coute-en-proportion-de-ses-termes.md`,
avec l’état corrigé en 4.3.2 (chemin et contenu vérifiés dans cette reprise).

## 6. Décisions et limites restantes

- L’attente de fusion #22/#23 et de publication 4.3.3 est levée par les faits
  transmis par Lucie. Aucune nouvelle fusion, aucun tag ni republication demandé.
- La normalisation des apostrophes à l’indexation et le refus des clés inconnues
  restent des décisions distinctes ; la 4.3.3 ne change pas le format d’index.
- Corpus luciepc et vérité terrain complète : non revérifiés ici.
- Le garde-fou des messages en attente lorsqu’un acteur meurt reste à examiner ;
  son état actuel n’a pas été vérifié dans cette reprise.

## 7. Prochain pas

Recherche porte la montée des dépendances et du lock, sa passe habituelle
et sa preuve dédiée de fermeture en erreur. Reporter ses preuves lorsqu’elles sont disponibles.
La session lucivy actualise les présentes docs sans dupliquer ce lot.

## 8. Reprendre

Ne pas exécuter les anciennes commandes de fusion des PR #22/#23 : elles sont
terminées selon Lucie. Pour tout nouveau lot, branches et worktrees préfixés
`cx-`, jamais de force-push ni de suppression de branche distante. Aucun lot
nouveau n’est requis uniquement pour renommer les anciens worktrees.

Les compilations lourdes restent sur luciepc sous `poste lourd`, au plus
`CARGO_BUILD_JOBS=8`. Un test sans son corpus ne vaut pas une preuve de recherche.
La publication ou la fusion d’un futur lot reste soumise aux règles du dépôt.

## 9. Inventaire historique sur disque (non revérifié dans cette reprise)

- Ce poste : `~/git_workspaces/lucivy` (clone principal, `main` ; des suppressions non commitées sous
  `bindings/nodejs/node_modules` et `.gitignore` appartiennent à Lucie, ne pas y toucher) ;
  `~/git_workspaces/lucivy-corpus` et `~/git_workspaces/lucivy-close` (les worktrees des PR #22 et
  #23, propres, poussés) ; `~/git_workspaces/lucivy-figement-G` (worktree de la session
  « optimiseur », pas à la session lucivy).
- luciepc : `~/git_workspaces/lucivy` (clone principal) et `~/git_workspaces/lucivy-figement-G`
  (à l'optimiseur). Rien d'autre de lucivy ; plus de `~/lucivy_bench`.
- La mémoire de la session lucivy : `~/.claude/projects/-home-lucied-git-workspaces-lucivy/memory/`.
