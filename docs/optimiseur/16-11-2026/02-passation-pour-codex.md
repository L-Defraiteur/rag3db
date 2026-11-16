# Passation de G (optimiseur) pour Codex — 16 novembre 2026

Pour une session Codex neuve, qui n'a que le dépôt, ses docs et ses tickets. Écrite par G le lundi 16 novembre 2026,
à la demande de Lucie par c7 (9 h 3x). Master lu à l'écriture : **96daa8f122**. Le reste du contexte est dans
`docs/optimiseur/15-11-2026/01-rapport-de-session.md` (section « Reprendre »),
`docs/optimiseur/11-11-2026/02-knowledge-dump.md` et `docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md`.

## 1. État

### Ce qui tourne (n'y touchez pas sans le mot de l'orchestration)

| quoi | où | valeur |
|---|---|---|
| lib `courant` du magasin | `~/.cache/rag3db-libs/courant`, les deux postes | `rag3db-build-373b5477b…` (librag3db 4fa330b5, libvector ce6b2db4) |
| hôte de Lucie (`rag3weaver-daemon`, socket 7970) | `~/.local/bin/rag3weaver-backend`, les deux postes | master 4802a147a5, md5 8d9e62c3, ancien en `.avant-4802a147` |
| services d'embarquement | luciepc, unités `rag3weaver-embeddings@granite-278m` (7878) et `@bge-m3` (7879) | `~/.local/bin/rag3weaver-embeddings` sha256 a70f1e462bd9df95 (f85977d09e) ; 7878 sous `RAG3WEAVER_EMBED_MAX_CHARS=60000` le temps de l'index B′ du banc (embarquement remet 12000) |
| tunnels du portable vers ces services | `rag3weaver-tunnels.service` (portable) | 7979 → 7878, 7980 → 7879 |

### Ce qui est au cache des deux postes (`~/.cache/rag3weaver-build/`), copié depuis master

| fichier | sha256 (16) | source dans le dépôt |
|---|---|---|
| `push_garde.sh` | 95a1d85779f0105a | `tools/poste/push_garde.sh` |
| `passe_epinglee.sh` | ec1529fc2f3dcf07 | `tools/poste/passe_epinglee.sh` |
| `poste` | 79e3a8bbf457e3bc | `extension/rag3weaver/scripts/poste` |
| `required_suites.sh`, `.txt` | dc0879f04c53d3eb, 0d96a1d33eac2089 | `tools/poste/` (pour les témoins seulement : une vraie passe lit la table dans l'arbre du lot) |
| `lib_seule.sh` | 8f60e82f60a4a7b5 | aucun : remplacé par `run_e2e.sh --lib`, à retirer (ticket plus bas) |

Chaque copie a son ancienne à côté (`.avant-<lot>`). Une copie se refait **par lot atterri qui change le fichier**, aux
deux postes, sous cette forme :
```bash
C=~/.cache/rag3weaver-build/<fichier>; cp -p $C $C.avant-<lot> && install -m 755 <fichier du dépôt sur master> $C.neuf && mv $C.neuf $C
sha256sum $C | cut -c1-16; bash <témoin du dépôt> $C        # le témoin joué contre la copie
```
`.neuf` puis `mv` : une passe en cours garde l'ancien fichier, et rien ne réécrit le fichier qu'elle lit. luciepc se
pilote par `ssh luciepc 'bash -s' <<'EOF' … EOF` : son shell de login est fish.

### Ce qui est prêt et n'est pas posé : la bascule de la lib

Plan et état : `docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md`. La lib
`rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a…` (librag3db 697c7a0c, libvector 925b6b13) est au magasin des deux
postes, avec un verdict vert sans `RAG3WEAVER_MOTEUR_ANCIEN` (36/36, 27/27, 24/24 sur 3177d2489e). Les hôtes bâtis
contre elle (md5 d164e4da) sont en `~/.local/bin/rag3weaver-backend.nouveau` au portable et dans
`nuit-3177d2489e/` à luciepc. Le service d'embarquement de 3177d2489e (sha256 6196fbe7102ad533) attend dans
`nuit-3177d2489e/`. **La fenêtre part au mot de Lucie.** Codex ne la lance pas. Après elle vient le ticket
« épingler la lib » (entrée 6).

### Ce qu'il faut savoir pour ne rien casser

- **La garde du push** (`tools/poste/push_garde.sh`) juge chaque push : la passe VERTE sur le commit poussé, une seule
  lib dans les journaux (ou `TESTS_ONLY=1` et la ligne `tested binary: …`), aucun code apporté par master depuis la
  base testée (sauf la voie C++ transférable), le mode pile (`PILE_AVANT`), les **suites exigées** et les trois contrôles.
- **Les suites exigées** : `tools/poste/required_suites.txt`. Un lot qui touche `extension/rag3weaver/src/code.rs`,
  `code_sync.rs`, `catalog.rs` ou `catalog/*`, `generations.rs`, `marque_d_ecrivain.rs` ou
  `tests/e2e_code_sync.rs` joue `e2e_code_sync`. La passe l'ajoute d'elle-même, et la garde refuse une passe qui ne
  l'a pas jouée verte.
- **`poste`** : `poste lourd|mesure|push`. Une validation (`run_e2e.sh`, `temoins_navigateur.sh`,
  `passe_epinglee.sh`, `confiner_validation.py`) se lance sous `POSTE_MEM_MAX=32G` ; au-dessus, `poste` la refuse
  avant la file. Le défaut des autres lourds reste 40G.
- **`passe_epinglee.sh <worktree relatif à ~/git_workspaces> <nom du dossier de journaux> <filtre du témoin>`** : le
  2e argument est un nom sous `~/.cache/rag3weaver-build/`, jamais un chemin absolu (refusé). `SUITE_DU_TEMOIN=lib`
  joue un témoin de la lib par `run_e2e.sh --lib`. `SUITES=-` réduit aux témoins, check_features et la lib.

## 2. Travail pour Codex, dans l'ordre

Chaque entrée est un lot : une branche `cx-…`, un worktree à soi, un témoin rouge d'abord, un rejeu court, un push au
« à toi » de l'orchestration (c7, session `rag3db-5a`). Les entrées 1 à 5 se font tout de suite. 6 et 7 attendent ce
qui est dit.

**Le rejeu court d'un lot de scripts** (aucun Rust du crate, aucun C++ lié) est son témoin, joué dans le format que lit la
garde :
```bash
B=~/.cache/rag3weaver-build; J=$B/<journaux>; W=~/git_workspaces/<worktree>; rm -rf $J; mkdir -p $J
t=$(git -C $W rev-parse HEAD); git -C $W merge-base HEAD origin/master > $J/base
echo "lib : rag3db-build-373b5477b4fd413f96605061b8d09393439a1a30 (le lot ne lie aucune lib)" > $J/lib.log
if (cd $W && <témoin>) > $J/temoins.log 2>&1 && ! grep -qE '^(ROUGE|ÉCHEC|SAUTÉ)' $J/temoins.log; then
  printf 'teste %s (<témoin> vert)\nVERT\n' $t > $J/resume.log; else printf 'teste %s\nROUGE\n' $t > $J/resume.log; fi
```
puis, au « à toi » :
`if tail -1 $J/resume.log | grep -qx VERT; then $B/poste push <nom de session> -- bash $B/push_garde.sh <worktree> <journaux>; fi`
(`PILE_AVANT=<tête du lot d'avant>` devant `$B/poste` pour un lot empilé). Les lignes « lib : » valent tant que `courant`
est 373b5477b. Après la bascule, mettre l'id neuf.

### 1. Le témoin de `poste` cherche le confineur à côté du `poste` jugé — 1 h

Ticket : `docs/optimiseur/tickets/2026-11-16-le-temoin-de-poste-cherche-le-confineur-a-cote-du-poste-juge.md`.
- Faire : dans `extension/rag3weaver/scripts/test_poste.sh`, lire `confiner_validation.py` dans le dossier du témoin
  (`$(dirname "$(realpath "$0")")`) et non dans `$(dirname "$POSTE")`. Même chose dans
  `tools/poste/test_passe_epinglee.sh` pour `required_suites.sh` et `.txt` (aujourd'hui copiés depuis
  `$(dirname "$X")`). Ensuite, retirer `required_suites.sh` et `.txt` du cache des deux postes, chacun gardé en
  `.avant-retrait`.
- Ne pas toucher : `scripts/poste` lui-même, `passe_epinglee.sh`.
- Prouver : `bash extension/rag3weaver/scripts/test_poste.sh ~/.cache/rag3weaver-build/poste` est rouge aujourd'hui (34
  OK sur 35, « confiner_validation.py absent ») et vert après (35 OK, aucun SAUTÉ).
  `bash tools/poste/test_passe_epinglee.sh ~/.cache/rag3weaver-build/passe_epinglee.sh` reste vert sans la table au
  cache. Se joue poste libre (`~/.cache/rag3weaver-build/poste --qui`).

### 2. Le témoin de `run_e2e.sh` compte l'attente du poste dans son délai — 3 h

Ticket : `docs/optimiseur/tickets/2026-11-16-le-temoin-de-run-e2e-compte-l-attente-du-poste-dans-son-delai.md`.
- Faire : `extension/rag3weaver/scripts/test_run_e2e.sh` lance chaque cas sous `timeout 120`. Chaque cas passe par
  `confiner_validation.py lancer`, donc par `poste lourd`, qui n'a qu'une place au portable. Le délai doit ne compter
  que le travail du cas. Piste à préférer : le témoin prend une fois la place (`poste lourd bash -c '…'`), et ses cas
  héritent du poste tenu (le confineur reconnaît un poste tenu par `poste_tenu`) ; le `timeout 120` reste par cas.
- Ne pas toucher : la règle « aucun drapeau d'environnement seul ne saute la vérification » du confineur.
- Prouver : rouge d'abord. Lancer un lourd factice qui tient la place (`~/.cache/rag3weaver-build/poste lourd sleep 300 &`,
  et le tuer par son pid `$!`), jouer le témoin : aujourd'hui, des 124. Après le correctif, le témoin, lancé
  pendant le lourd, attend la place puis rend 22 OK.

### 3. La voie des suites C++ transférables ignore `tools/shell` — 1 h 30

Ticket : `docs/optimiseur/tickets/2026-11-16-la-voie-cpp-transferable-ignore-tools-shell.md`. Tout y est dit :
`CHEMINS_MOTEUR` et le `case` de `cpp_transferable` dans `tools/poste/push_garde.sh`, deux cas au témoin
`tools/poste/test_push_garde.sh` (son mode `--code-apporte` et ses dépôts jetables montrent la forme). Puis la recopie
de la garde aux deux postes.

### 4. Le juge known_red rougit sur un cas qui passe une fois — 5 h

Ticket : `docs/optimiseur/tickets/2026-11-15-un-cas-known-red-qui-passe-une-fois-rougit-le-juge.md`.
- Faire : dans `test/transaction/concurrence/compare_known_red.cmake`, un cas connu rouge qui passe dans le banc
  ne fait plus échouer (`FATAL_ERROR`). Il devient une ligne `STATUS` « known red passed this time (intermittent?):
  <nom> », comptée. Le retrait de la ligne de `known_red.txt` ne s'exige qu'après N passes vertes consécutives du cas
  joué **seul**. Il faut pour cela un script neuf à côté, `test/transaction/concurrence/confirm_known_red_fixed.sh
  <binaire du banc> <nom du cas> [N=3]`, qui joue `--gtest_filter=<nom>` N fois, chaque fois dans un processus neuf,
  et dit « fixed: remove the line » ou « still red (k/N) ».
- Ne pas toucher : `known_red.txt` (ses étiquettes `nom a,b`), `probabilistic.txt`, la ligne INCOMPLETE des cas
  longs (0274c53f14).
- Prouver : `test/transaction/concurrence/test_compare_known_red.sh` (faux banc gtest JSON, `cmake -P`) gagne
  trois cas, rouges sur le juge de master : un connu rouge vert une fois rend 0 avec la ligne STATUS ; un connu rouge
  rouge reste muet ; le script de confirmation, sur un faux binaire, rend « fixed » après N passes vertes et « still red »
  sinon. Le chemin `test/` est du moteur : ce lot passe par la voie C++ transférable.

### 5. La base « un seul sens » au témoin de `check_rel_directions` — 3 h, à luciepc

Ticket : `docs/optimiseur/tickets/2026-11-16-la-base-un-seul-sens-n-est-pas-au-temoin-de-check-rel-directions.md`.
La base, ses comptes exacts et la forme du mode `--one-direction` y sont. Le témoin se joue par son nom seulement, à
luciepc, sous `poste lourd`, sur une copie reflink de `~/.cache/optimiseur-mesure/un-seul-sens-copie/base.kz`.

### 6. Épingler la lib de la bascule dans les passes — 1 h 30, **après la bascule**

Ticket : `docs/optimiseur/tickets/2026-11-16-les-passes-epinglent-373b5477b-apres-la-bascule.md`. Bloqué par le mot
de Lucie sur l'heure de la bascule, et par le « courant posé » de G ou de c7. Le lot passe alors en tête de file.

### 7. Le défaut de 40G de `poste lourd` — 2 h, **après deux relevés**

Ticket : `docs/optimiseur/tickets/2026-11-16-le-plafond-par-defaut-de-poste-lourd-reste-a-40g.md`. Il attend le pic d'un
bâti à froid de `build/moteur` (session cœur) et d'un bâti à froid de la lib du magasin, puis le mot de c7. Si les
deux tiennent sous 24 Gio, le défaut passe à 32G ; sinon, le ticket se ferme sur les chiffres.

### Hors de Codex, pour mémoire

- `docs/optimiseur/tickets/2026-11-08-le-plein-texte-de-code-lourd-compte-plus-de-documents-que-de-lignes.md` est aux
  sessions recherche et lucivy.
- Les deux tickets de la chaîne Rust (`2026-11-14-la-chaine-rust-de-la-ci-flotte-celle-des-postes-non.md`,
  `2026-11-14-monter-la-chaine-rust-a-la-derniere-stable.md`) attendent une décision de Lucie (partie 3).

## 3. Ce qui attend Lucie (Codex n'y touche pas)

- **L'heure de la bascule de lib** (`docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md`) : environ 40 min sans hôte
  à luciepc et 20 au portable. G ou c7 la conduit.
- **La chaîne Rust** : la CI flotte, pas les postes (`rust-toolchain.toml` 1.98.1). Monter à la dernière stable se
  fait une nuit calme, à sa décision.
- **Tout `sudo`**, la carte **card0** de luciepc (celle de seat1, où Lucie joue) et ses unités systemd personnelles :
  rien sans elle.
- **Les copies de mesure** de luciepc (`~/.cache/optimiseur-mesure/`, environ 38 Go apparents) : rien ne s'efface
  avant son mot.

## 4. Règles du périmètre

- **Langue** : tout identifiant neuf en anglais (variables, fonctions, options, clés, noms de scripts, de tests, d'unités,
  étiquettes de known_red). La prose, les messages à l'écran, les commits et les tickets restent en français. Pas de lot de
  renommage : on ne renomme que ce qu'on touche.
- **Arbre et branches** : jamais dans `~/git_workspaces/rag3db` (l'arbre de Lucie : rien n'y est commité ni basculé).
  Un worktree à soi, des branches `cx-<sujet>`, un target cargo à soi
  (`CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-cx-<sujet>`), plafond 32 Go.
- **La passe et le push** : `docs/procedures/push-d-un-lot.md`. Un push seulement au « à toi » de l'orchestration,
  sous `poste push`, dans un `if … grep -qx VERT`. Puis les trois contrôles : status vide, aucun rebase en cours,
  HEAD = FETCH_HEAD. Un rebasé se pousse sous un nouveau nom de branche. Jamais de force push, jamais de suppression de
  branche distante. Chaque commit se pousse dans la minute.
- **Rouge d'abord** : chaque témoin est vu rouge sur l'outil d'avant, puis vert. Un test qui saute n'est pas vert, et une
  suite non jouée est rouge.
- **Un défaut connu non corrigé** s'écrit en ticket sous `docs/optimiseur/tickets/` (en-tête : État, Famille
  moteur|produit, Gravité, Atteignable en service, Touche rag3weaver), contrôlé par
  `python3 extension/rag3weaver/scripts/check_tickets.py`. On le ferme par `git mv` vers `docs/tickets/closed/`, dans
  le lot qui le corrige.
- **Identité** : `user.email = luciedefraiteur@gmail.com`, jamais une adresse pro. Aucune mention d'un assistant ni
  ligne d'attribution dans les commits.
- **La machine** : `-j 8` au plus, jamais `-j$(nproc)`. Tout bâti ou toute suite lourde passe par
  `~/.cache/rag3weaver-build/poste lourd`, une mesure par `poste mesure`. On ne tue que ses propres pids (`$!`),
  jamais par motif.
