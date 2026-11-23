# Où part le temps d'une passe, faut-il tester en optimisé, et pourquoi sccache rate

23 novembre 2026, au soir, à luciepc. Trois questions de Lucie : « et niveau accélération des builds ? et tests ?
d'ailleurs on teste en release ? faudrait peut-être, non ? », puis « le cache de compilation partagé, on y va,
non ? ». Ce document répond par des mesures ; il ne change aucun code. Les lots suivent.

## 1. Où part le temps d'une passe du pousseur

Cinq passes du soir (de `host-state-cache-settled` à `tty-host-copy-once-2`), durée totale 856 à 934 s. Les étapes,
lues dans les journaux de chaque passe (heure de fin de chaque journal, et « Finished … in » de cargo) :

| Étape | Durée | dont compilation | dont exécution |
|---|---|---|---|
| témoin du lot (lib, filtre) | 60–75 s | 54–71 s | quelques s |
| `e2e_lectures` | 80–110 s | 54–84 s | ≈ 25 s |
| `e2e_code` | ≈ 90 s | < 1 s | ≈ 90 s |
| `e2e_demon_hote`, `e2e_tty`, tty | ≈ 30 s | < 4 s | ≈ 25 s |
| `check_features` (8 jeux) | 150–170 s | tout | — |
| lib entière (3 116 tests) | 310–350 s | 51–60 s | 230–257 s |
| témoins navigateur (75) | 160–175 s | 54–63 s | 103–107 s |

**Compilation ≈ 400 s, exécution ≈ 480 s, sur ≈ 900 s.** Chaque étape qui compile le crate paie ≈ 55 s : le crate
lui-même, en `-O0`, sans incrémental (`CARGO_INCREMENTAL=0`), recompilé une fois par jeu de features distinct (le
témoin, la lib, le navigateur, les e2e, et les huit `cargo check`). Les dépendances, elles, sont déjà là.

**Une lib rag3db neuve** ne recompile que `rust_api` et le crate : son chemin entre par le `build.rs` de `rust_api`
(`cargo:rustc-link-arg=-Wl,-rpath,…`), pas par `RUSTFLAGS`. Le rpath dans `RUSTFLAGS` n'existe que dans le script de
bâti des poses (`bati-lucie-luciepc.sh`), où il fait tout recompiler : lui seul est à corriger.

## 2. Tester en optimisé : la mesure dit non

La lib entière, mêmes features que le pousseur, même lib épinglée (3a2639714e), un target à part, `debug-assertions`
et `overflow-checks` gardés (le profil de test les garde ; seul `opt-level` change, par
`--config profile.test.opt-level=N`) :

| `opt-level` du crate | compiler le crate seul | exécuter la lib | binaire de test |
|---|---|---|---|
| 0 (aujourd'hui) | 62 s | 256 s | 516 Mo |
| 1 | 174 s | 260 s | 416 Mo |
| 2 | 181 s | 226 s | 402 Mo |

Les 3 116 tests passent aux trois niveaux. **`-O1` coûte deux minutes de compilation de plus et ne gagne rien à
l'exécution ; `-O2` gagne au plus 30 s, l'écart naturel d'une passe à l'autre (230 à 257 s le même soir).** Chaque
étape qui compile le crate paierait ces deux minutes : quatre à cinq fois par passe, soit huit à dix minutes perdues
pour une demi-minute gagnée.

Pourquoi : ce qui calcule dans les tests est déjà optimisé : les dépendances en `-O3` depuis le 6 septembre (burn,
tokenizers, lucivy, tree-sitter) et la lib C++ en Release. Le temps d'exécution est ailleurs : entrées-sorties,
bases ouvertes et fermées, attentes bornées, hôtes lancés. **La piste utile est de savoir quels tests le prennent** :
`cargo nextest` donne la durée de chaque test ; à mesurer sur une passe, puis à traiter les plus longs un par un.

## 3. Pourquoi sccache rate (8,2 % de succès)

`sccache --show-stats` à luciepc, serveur démarré le 23 à 11 h 10 : 35 937 demandes, **31 622 (88 %) non
candidates**, et sur les 4 312 restantes, 354 succès.

- **`crate-type` : 20 542** — sccache ne met en cache que les bibliothèques : les binaires de test (`--test`, un
  par e2e et par lib), les sondes des build.rs, n'y entrent jamais. C'est la majorité de ce qu'une passe compile.
- **`incremental` : 9 338** — une crate compilée en incrémental n'est pas mise en cache. Les passes coupent
  l'incrémental (`run_e2e.sh`), mais pas les bâtis à la main ni les scripts qui ne le posent pas.
- **Le cache est plein : 40 Gio sur 40.** Avec une douzaine de targets et autant de jeux de features, il évince ce
  qu'il vient d'écrire : d'où les échecs sur des dépendances qui devraient toucher.
- **Aucun répertoire de base** (`Base directories (none)`) : les crates par chemin (le crate, codeparsers,
  rust_api) changent de clé d'un worktree à l'autre. Les dépendances de crates.io, elles, ont le même chemin partout.
- **57 scripts de passe coupent sccache eux-mêmes** (`RUSTC_WRAPPER=`).

**Ce que ça veut dire** : sccache ne peut presque rien gagner sur nos passes, parce que ce qu'elles recompilent
est le crate (incrémental ou binaire de test, jamais en cache) et que les dépendances sont déjà dans chaque target.
Son seul gain possible est le premier bâti d'un target neuf (8 min 43 s à froid, mesuré ici) : à condition que le
cache ne soit pas plein.

## 4. Ce qui accélère vraiment, chiffré

| Piste | Gain attendu par passe | Coût |
|---|---|---|
| **Moins de jeux de features distincts** : le témoin, les e2e et la lib sous le même jeu, donc un seul bâti du crate | 2 à 3 compilations du crate en moins, ≈ 2–3 min | à vérifier : un e2e sous plus de features doit rester vert |
| **`check_features` en `cargo check` partagé** : un target à part, gardé, pour les huit jeux | ≈ 1–2 min (aujourd'hui 150–170 s) | un target de plus au registre |
| **Les tests les plus longs** (nextest, durée par test) | inconnu avant la mesure | une passe mesurée |
| sccache : cache à 80 Gio, `SCCACHE_BASEDIRS`, scripts sans `RUSTC_WRAPPER=` | premier bâti d'un target neuf seulement (≈ 9 min à froid) | disque |
| rpath hors de `RUSTFLAGS` dans le bâti des poses | une pose avec lib neuve : un bâti complet (dizaines de minutes) devient incrémental (≈ 3 min mesurées à lib égale) ; à mesurer | un lot |
| mold : déjà là (`poste`) ; lld inutile | — | — |
| crate en `-O1`/`-O2` pour les tests | **négatif** (+ 8 à 10 min par passe) | — |

## 5. La mémoire, pendant qu'on y est

Le 23 à 18 h 47, oomd a tué la session graphique de luciepc : quatre passes, pics jusqu'à 17,7 Gio (médiane 10,7,
p90 13,2 sur 398 passes), et le swap à 90 %. Ticket « poste compte les cœurs, pas la mémoire » : un budget mémoire à
l'admission et une enveloppe par portée. Toute accélération qui met plus de passes en parallèle passe par lui.

## Ordre proposé des lots

1. Le budget mémoire de `poste` (ticket du 23).
2. La mesure nextest des tests les plus longs, puis leurs correctifs.
3. Un seul jeu de features pour le témoin, les e2e et la lib.
4. `check_features` dans un target partagé.
5. Le rpath hors de `RUSTFLAGS` dans le bâti des poses.
6. sccache dimensionné, seulement s'il reste utile après 3 et 4.
