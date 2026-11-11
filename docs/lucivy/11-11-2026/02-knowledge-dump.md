# lucivy — ce qu'un nouvel agent doit savoir (11 novembre 2026)

*Pour un agent sans aucune mémoire des échanges précédents. L'état du jour (version, PR, tickets,
reprise) est dans `01-rapport-de-session.md`, à côté.*

## 1. Ce qu'est lucivy

Un moteur de recherche plein texte écrit en Rust, dérivé de tantivy. Il fait :
- le classement BM25 ;
- la recherche de **sous-chaînes** à travers les mots (`contains`) ;
- le flou (distance d'édition) ;
- les expressions régulières et la syntaxe booléenne ;
- pour chaque résultat, la **position exacte en octets** de chaque occurrence (les surlignages).

Il sert à l'agent de code de rag3db. rag3weaver indexe les fichiers d'un dépôt dans lucivy, à côté de
la base de graphe rag3db et de ses vecteurs, puis fusionne les trois signaux.

Notions internes qu'on croise partout :

- **shard** : un index est découpé en plusieurs sous-index (4 par défaut chez rag3weaver), chacun
  servi par un **acteur**.
- **acteur, luciole** : `luciole` est le planificateur d'acteurs de lucivy (une boîte de messages par
  acteur, sur un planificateur global). Un `ShardedHandle` a un acteur par shard, des acteurs lecteurs
  et un routeur.
- **SFX** : l'index de suffixes (un FST, automate de clés) qui permet les sous-chaînes.
- **dictionnaire partagé** (`sfx_version` 4, le défaut) : un dictionnaire de textes par shard plutôt
  qu'un par segment.
- **segment** : un morceau d'index écrit par un commit, fusionné ensuite avec d'autres.
- **relâché / strict** : en séparateurs relâchés (le défaut), la ponctuation ASCII est retirée de la
  requête et du texte avant de comparer. En strict, elle doit correspondre.

## 2. Le dépôt et les paquets

- **Dépôt** : `https://github.com/L-Defraiteur/lucivy`, sur le compte GitHub personnel de Lucie,
  `L-Defraiteur`. Clones : `~/git_workspaces/lucivy` sur ce poste et sur luciepc.
- **Documents de référence du dépôt** :
  - `CLAUDE.md` est la référence vivante : versions publiées par registre, méthode, pièges ;
  - `RELEASE.md` décrit la publication ;
  - `CHANGELOG.md` décrit chaque version ;
  - `docs/<jour>/` contient les rapports.
- **Paquets publiés**, tous au même numéro (`X.Y.Z`, 4.3.2 aujourd'hui) :
  - **crates.io** : `ld-lucivy` (le moteur), `lucivy-core` (l'API haut niveau, `ShardedHandle`,
    `QueryConfig`), `luciole`, `lucistore`, `sparse-vector`. Deux exceptions :
    - `lucivy-fst` a sa propre version (0.1.1) ;
    - huit petits crates `ld-lucivy-*` / `ld-ownedbytes` sont **gelés en 0.27.0** et absents de la
      boucle de publication. En modifier un exige de monter sa version et de l'ajouter à la boucle.
  - **npm** : `lucivy` (Node.js, plus cinq paquets de plateforme) et `lucivy-wasm` (navigateur).
  - **PyPI** : `lucivy`, en wheels `abi3` pour cinq plateformes, plus le sdist.
  - **la démo dans le navigateur** : `https://l-defraiteur.github.io/lucivy/`, faite de
    `playground/`. Elle est déployée par `pages.yml` à chaque push de `main` qui touche `playground/`.

## 3. Bâtir et tester

Les compilations lourdes se font sur luciepc, avec `CARGO_BUILD_JOBS=8` au plus, sous
`~/.cache/rag3weaver-build/poste lourd <commande>`. Ce verrou partagé limite les travaux lourds
simultanés : 2 à luciepc, 1 sur ce poste.

```bash
cargo clippy --lib -- -D warnings
cargo clippy -p lucivy-core -p lucistore -p luciole -p sparse-vector --lib -- -D warnings
cargo test --lib                                   # ~1 480 tests
cargo test --lib --no-default-features
cargo test -p lucivy-core --no-fail-fast           # ~216 tests ; voir le piège du corpus (§ 7)
cargo test -p lucivy-cpp
cd bindings/python && bash build.sh && .venv/bin/python -m pip install pytest && .venv/bin/python -m pytest tests -q
cd bindings/nodejs && npm install && npm run build && node test.mjs && for t in tests/*.mjs; do node "$t" "$PWD/lucivy.node"; done
bash bindings/emscripten/build.sh                  # le WASM, copié dans playground/pkg et estampillé dans la page
```

**La CI** (`.github/workflows/ci.yml`) joue sur chaque PR vers `main` et chaque push de `main` :
- les tests de la lib, sous trois jeux de features ;
- clippy ;
- `lucivy-core` et `lucivy-cpp` ;
- la vérité terrain sur le dépôt lui-même ;
- l'audit des `thread::spawn` ;
- Python et Node.

**Le bâti** (`build.yml`) produit wheels, addons et WASM sur les cinq plateformes. Ses runners sont
`ubuntu-latest`, `macos-latest` et `windows-latest`, jamais des images numérotées : GitHub les retire,
et `macos-14` a cassé une PR le 7 novembre.

**Pas de workflow planifié** : Lucie ne veut pas consommer le quota d'Actions.

## 4. Publier

**Pousser un tag `v*` publie.** `release.yml` rejoue alors la CI, bâtit tout et attache 12 artefacts
à la release GitHub. Il publie ensuite PyPI, npm et crates.io par trusted publishing, sans jeton ni
code. Les crates passent en dernier, dans l'ordre : `lucivy-fst`, `luciole`, `lucistore`, `ld-lucivy`,
`lucivy-core`, `sparse-vector`. Une version publiée sur crates.io ne se retire pas.

**Aucun tag sans le mot explicite de Lucie dans la session.** Le 25 octobre puis le 7 novembre, elle a
autorisé la session à fusionner et taguer une version qu'elle avait demandée, une fois tout vert.

Le chemin d'une publication :
1. PR vers `main`, puis tous les checks verts.
2. Fusion par rebase.
3. Commit de publication sur `main` : le CHANGELOG daté, plus les en-têtes et la section « What's
   new » des cinq README (racine, `bindings/nodejs`, `bindings/python`, `bindings/cpp`,
   `bindings/emscripten`).
4. CI verte sur le SHA exact de `main`.
5. `git tag -a vX.Y.Z <sha> && git push origin vX.Y.Z`.
6. Suivre `release.yml`.
7. Vérifier chaque registre : `npm view`, l'API JSON de PyPI, l'API de crates.io **avec un en-tête
   User-Agent générique**.
8. Noter les versions dans le `CLAUDE.md` du dépôt.

Le numéro se change dans neuf `Cargo.toml` (avec leurs dépendances internes), sept `package.json` et
`bindings/python/pyproject.toml`.

Le détail est dans `docs/25-10-2026/01-pipeline-de-publication.md`, sur la branche
`docs-pipeline-publication` du dépôt lucivy.

## 5. Comment rag3weaver utilise lucivy

- **Une dépendance publiée.** `extension/rag3weaver/Cargo.toml` prend `lucivy-core = "4.3.2"`,
  `ld-lucivy = "4.3.2"` et `sparse-vector = "4.3.2"` de crates.io. Il n'y a plus de chemin local ni de
  `[patch.crates-io]` : monter la version de lucivy dans rag3weaver est un lot de rag3weaver.
- **La recherche par défaut** envoie la **requête entière** à lucivy comme un seul `contains` relâché,
  à distance 1 jusqu'à 900 caractères. En parallèle part une recherche par mots (`parse`, un OU des
  mots, au plus 64 termes). Ce que la requête entière trouve passe devant la fusion.
- **Les surlignages.** Un `HighlightSink` passé à la recherche rend, par champ, des intervalles
  `[début, fin)` en **octets UTF-8** dans la valeur stockée. rag3weaver les convertit en unités UTF-16
  pour la page.
- **`query_warnings(&QueryConfig)`** dit ce qu'une requête fera vraiment : par exemple, une requête
  `parse` sans syntaxe devient un OU de mots, ou une clé est ignorée.
- La note complète sur les options de recherche, avec l'appel Rust exact de chacune, est dans
  `docs/25-10-2026/03-options-de-recherche.md`, sur la branche `docs-pipeline-publication` du dépôt
  lucivy.

## 6. Les règles

- **Tag = publication partout** : jamais sans le mot de Lucie.
- **Fusion dans `main` de lucivy** : au mot de Lucie (sa règle du 13 septembre pour lucivy). Une
  session qui relaie un accord ne remplace pas le sien.
- **Identité git personnelle de Lucie** (son adresse gmail, déjà posée dans la config git des clones),
  jamais son adresse professionnelle. **Aucune ligne d'attribution** dans les commits ni les PR, et
  aucune mention d'IA dans le code et les docs.
- **Deux comptes `gh`** : `gh auth switch -u L-Defraiteur` avant toute action GitHub sur lucivy, puis
  revenir au compte actif d'origine.
- **Les langues** : code, identifiants et commentaires en anglais ; docs, commits et rapports en
  français.
- **Les branches** :
  - chaque commit est poussé dans la minute sur une branche à son nom ;
  - jamais de push en force : après un rebase qui réécrit l'historique, pousser sous un nom neuf
    (`-2`) ;
  - ne jamais effacer une branche distante sans Lucie ;
  - les branches de travail s'empilent par fonctionnalité finie et se nomment `<version>.N-nom`, par
    exemple `4.3.2.1-corpus-absent`.
- **Les défauts** : un défaut connu non corrigé devient un ticket. Il se prouve par un **témoin rouge
  d'abord** (un test qui échoue sur l'ancien code), puis viennent le correctif et le témoin vert.

## 7. Les pièges appris

- **Le corpus des tests.** Les tests de vérité terrain lisent le noyau Linux sous `/tmp/lucivy-cmp`
  et `/tmp/lucivy-cmp-90k`, ou un clone de rag3db sous `/tmp/rag3db-bench` (ou le chemin donné par
  `V3_CORPUS`). Sans ce corpus, sur `main`, ils passent sans rien vérifier, sur un synthétique de
  onze mots ou une liste vide ; la PR #22 les fait échouer. Pour recréer le corpus :
  - `git clone --depth=1 --branch v7.2 https://github.com/torvalds/linux ~/lucivy_bench/linux-7.2`
    (le comparatif est épinglé sur le commit `8d3ae59288f1`) ;
  - `ln -sfn ~/lucivy_bench/linux-7.2 /tmp/lucivy-cmp`, et de même pour `-90k`.

  Lire « corpus: N files » dans la sortie avant de croire un vert.
- **`/tmp` se vide** de ce qui a plus de dix jours sur ce poste : rien de durable n'y vit.
- **L'outil Bash est zsh** : une commande rangée dans une variable ne se découpe pas, et
  `[ a \> b ]` n'existe pas. Les boucles et les scripts passent par un fichier `bash`.
- **Des tests qui partagent un dossier temporaire** se battent quand cargo les joue en parallèle
  (`LockBusy`) : un nom de dossier par test.
- **Les défauts de concurrence ne sortent que sous charge.** Le défaut corrigé en 4.3.1 n'échouait
  pas hors charge (0 sur 15), et échouait 11 fois sur 80 sous charge. Avant de conclure, rejouer une
  série de plusieurs dizaines de passes, avec plusieurs binaires en même temps. Lire la panique réelle
  dans la sortie (`--nocapture`), pas seulement le compte.
- **Un correctif « atterri » peut l'avoir été sous d'autres commits** : un rebase ou un regroupement
  change les hash, et parfois les sujets.
- **`npm publish` à la main** répond un faux 404 quand la session npm a expiré : lancer `npm whoami`
  d'abord. C'est normalement inutile, puisque la CI publie.
- **Le `cargo clippy` de la CI suit Rust stable** : un nouveau lint peut faire rougir `main` sans
  changement de code. Le 7 novembre, `fetch_update` a été déprécié en Rust 1.99 ; il est gardé par
  `#[allow(deprecated)]`, parce que la `rust-version` du dépôt est 1.85.
- **Un push vers `main` peut échouer** si `main` a bougé entre le `fetch` et le push. Avant de dire
  « poussé », relire la sortie du push et faire les trois contrôles :
  - `git status` vide ;
  - aucun rebase en cours ;
  - `git log -1 origin/main`, après `fetch`, montre le bon hash.

## 8. Les tickets ouverts de lucivy

Sur la branche `tickets-2026-11-07` du dépôt lucivy, dans `docs/tickets/` :

- `2026-11-07-apostrophe-typographique.md` — `’` contre `'` : corrigé à la requête en 4.3.2. Reste à
  décider la normalisation à l'indexation, qui change le format.
- `2026-11-07-cles-de-requete-ignorees.md` — `boost`, `offset`, `slop` ignorés : avertis en 4.3.2.
  Reste à décider le refus, qui serait une 4.4.0.
- `2026-11-07-un-passage-exact-long-n-est-plus-trouve.md` — corrigé en 4.3.2, à fermer.
- `2026-11-08-un-test-de-corpus-sans-son-corpus-passe-vert.md` — corrigé par la PR #22, pas
  fusionnée.
- `2026-11-08-close-sort-sur-l-erreur-sans-arreter-ses-pools.md` — corrigé par la PR #23, pas
  fusionnée.

Dans `docs/tickets/` de rag3db : `2026-11-07-une-requete-floue-de-lucivy-coute-en-proportion-de-ses-termes.md`,
corrigé par la 4.3.2, à fermer. L'orchestration dira quand déplacer les tickets.

À ouvrir : dans `luciole`, un acteur qui meurt devrait répondre une erreur à chaque message de sa
boîte, au lieu de les jeter.

## 9. Où lire la suite

- Le `CLAUDE.md` du dépôt lucivy ; pour l'histoire du moteur, `docs/14-09-2026/01-rapport-nuit-4.3.0.md`
  et les dossiers `docs/<jour>/`.
- La mémoire de la session lucivy : `~/.claude/projects/-home-lucied-git-workspaces-lucivy/memory/`,
  un fichier par règle ou par piège, avec son index dans `MEMORY.md`.
