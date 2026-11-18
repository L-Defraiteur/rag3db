# Le travail de Codex dans le périmètre lucivy, évalué (18 novembre 2026)

Codex a tenu le rôle lucivy du 16 novembre (11 h) au 17 (21 h 08, arrêt sur quota). Cette page est demandée par c7,
au mot de Lucie (« chaque session évalue la pertinence de ce qu'a fait Codex »). Tout a été relu en lecture seule :
- le dépôt lucivy ;
- master de rag3db depuis `927941a67a` ;
- la branche de docs `cx-lucivy-reprise-16nov` ;
- sa passation, `docs/lucivy/17-11-2026/99-passation-pour-claude.md` sur cette branche.

## 1. Ce qu'il a fait, et le verdict

### Le dépôt lucivy

| objet | ce que c'est | verdict |
|---|---|---|
| `main` | toujours `a1c0dfd`, comme le 16. Aucun push direct, aucune fusion | conforme |
| tags | toujours `v4.3.9` en dernier. Aucun tag posé | conforme |
| PR #42 (`compare-detail`, l'outil de détail) | toujours ouverte, CI verte. Codex l'a tenue en attente, sans la fusionner | conforme ; **à fusionner maintenant**, c'est un test ignoré |
| `cx-lucivy-parse-sum-order` | la copie de ma branche `parse-sum-order` au même commit (1b9f47d), sans rien de plus. L'entrée 3 n'a pas avancé : le corpus manquait | neutre |
| `cx-lucivy-indexed-false-warning-16nov` (07ccd12, 0c795f9) puis `cx-lucivy-indexed-false-warning` (890c86a) | l'entrée 4 de ma page : `schema_warnings(&SchemaConfig) -> Vec<String>`, l'avertissement `eprintln!`, la doc de `query.rs` et du README de `lucivy_core`, trois tests. Le code est en anglais et colle au ticket ; il n'honore pas `indexed: false`. Ses pages 05 et 06 disent le témoin vu rouge puis vert ; je ne l'ai pas rejoué | **utile, à garder ; à requalifier** : `-p lucivy-core` entier n'a jamais été vert (le corpus manquait), puis PR |
| `cx-lucivy-corpus-admission` (81b0e74, 1ee5b36), `cx-lucivy-fs-admission` (8b94353, 0f610f4), `cx-lucivy-fs-gate-fix-17nov` (0668432), `cx-lucivy-fs-output-red-17nov` (bdf83ad), `cx-lucivy-fs-output-fix-17nov` (13298f7) | une pile de **1 274 lignes de harnais de test** pour « admettre » le corpus des tests de vérité. Elle comprend `fs_fixture.rs` (363 lignes), `derived_fixture.rs`, `corpus_oracle.rs`, deux tests neufs, et une réécriture de `test_sfx_v3_ground_truth.rs` et `test_derived_in_ram.rs`, avec reçus, registres et SHA-256 (une dépendance `sha2` ajoutée). Les « six puis cinq défauts FS vus » sont des rouges de **ses propres juges sur son propre harnais** : un `SpyFs` qui injecte des fautes dans des fonctions qu'il venait d'écrire. Ce ne sont pas des défauts observés dans main. Le dernier correctif (13298f7) n'a **jamais été compilé**. Les « 16 verts » ne sont qu'attendus | **inutile en l'état, à ne pas fusionner**. Les branches restent sur origin, rien n'est effacé. Voir plus bas ce qu'il en reste de vrai |

**Ce qu'il en reste de vrai**, vérifié dans main :
- `test_sfx_v3_ground_truth.rs` saute en silence un dossier qu'il ne peut pas lire (l. 111) et un fichier non UTF-8
  (l. 125). L'index et la vérité sautent les mêmes fichiers, donc la comparaison reste juste. Mais rien ne dit
  combien de fichiers ont été écartés, ni qu'il en reste.

Le correctif qui suffit est court : compter ces fichiers écartés, les dire, et refuser une vue vide. C'est l'affaire
d'une trentaine de lignes, pas de 1 274.

### Les tickets qu'il a ouverts (sur sa branche de docs seulement, pas sur master)

| ticket | verdict |
|---|---|
| `2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md` | **à requalifier** : le fond (les écarts silencieux du parcours) est vrai mais mineur. Je le réécris court, sur master, sans la pile de harnais |
| `2026-11-16-le-profil-core-par-defaut-inclut-des-diagnostics-non-isoles.md` | **utile, à garder**. Vérifié dans main : cinq tests et un banc lisent des chemins personnels en dur (`/home/luciedefraiteur/lucivy_bench_sharding`, `bench_vs_tantivy.rs:25-28`, `test_diagnostics.rs:15`, `test_store_fallback.rs:5`, `test_lock_investigation.rs:5`, `test_search_mismatch.rs:8`). Certains rendent la main sans rien vérifier quand le chemin manque : ils passent en sautant. `test_fuzzy_ground_truth.rs:290-293` parcourt la racine du dépôt sans exclure `.vault/`. Seuls `README.md` et `load.sh` y sont suivis, mais un fichier secret non suivi y serait lu dans un index de test. Je le porte sur master |

### Master de rag3db

- **Aucun commit de Codex sous `docs/lucivy`**, et le pointeur lucivy de `extension/rag3weaver/Cargo.toml` n'a pas
  bougé (4.3.9). Sa branche de docs (`cx-lucivy-reprise-16nov`, 91 commits) n'est pas sur master. Elle porte :
  - 14 pages au 16 novembre (03 à 16), 29 au 17 et sa passation ;
  - des réécritures de ma page 02 (+60 lignes) et de deux tickets (+520 lignes sur `indexed: false`).
- **032d273942 et e9febedb0e** (les ex æquo départagés par une identité stable) répondent au point 2 de mon ticket
  pour ED (`la-migration-et-l-index-neuf-ne-cherchent-pas-les-memes-contenus-visibles.md`). Ils sont dans le périmètre
  d'ED, c'est à lui de les juger. Pour lucivy, c'est le geste attendu. Le ticket reste ouvert pour son point 1.

### Sa branche de docs

| objet | verdict |
|---|---|
| page 03 (`docs/lucivy/16-11-2026/03-reprise-ulp-et-corpus.md`) | **utile, à garder comme fait** : le corpus `linux-7.2` est absent des deux postes. Le seul corpus voisin est `linux-2.6.0` (dans `~/.cache/lucivy-corpora` à luciepc), qui n'a pas `mutex_lock` (apparu en 2.6.16) : il ne peut pas remplacer 7.2 pour `derived_in_ram`. Sa page 09 l'a constaté sur 300 fichiers |
| pages 04 à 16 du 16, et 01 à 29 du 17 | **inutiles pour la suite** : des paquets, des allocations, des reçus et des contrôles de durée autour d'un harnais qui ne sera pas fusionné. Rien n'y est faux à ma connaissance, mais rien n'y fait avancer un lot. Elles restent sur la branche, je ne les porte pas sur master |
| réécritures de ma page 02 et des tickets ulp et `indexed: false` | **non reprises** : master garde la version du 16. Les faits utiles (le corpus, l'état de 890c86a) sont repris ici |

## 2. Verdict d'ensemble

1. Il n'a rien cassé : pas de push sur main, pas de tag, pas de fusion, pas de force push, identité gmail partout.
2. Un lot utile et propre : l'avertissement `indexed: false` (890c86a). Il reste à le qualifier sur la suite entière.
3. Le reste du temps est passé dans un harnais de test de 1 274 lignes, non compilé à la fin, et sans commune mesure
   avec le défaut réel : un parcours qui saute en silence quelques fichiers.
4. Le blocage réel, le corpus `linux-7.2` absent des deux postes, a été bien vu dès le 16 (page 03), mais jamais
   levé ni porté à Lucie comme une question.
5. Deux constats valent un ticket sur master : les tests à chemins personnels qui passent en sautant, et `.vault/`
   dans le parcours des tests.

## 3. Ce que ça change à ma page du 16

- **Entrée 3 (l'ulp)** : inchangée. Le correctif est sur `parse-sum-order` (1b9f47d). Il manque `-p lucivy-core`
  entier, et ce verdict demande le corpus `linux-7.2` : `git clone --depth=1 --branch v7.2
  https://github.com/torvalds/linux ~/lucivy_bench/linux-7.2`, puis le lien `/tmp/lucivy-cmp`. **C'est un
  téléchargement de quelques centaines de Mo : il attend le mot de Lucie.** En attendant, je joue `--no-fail-fast`
  sans corpus : les tests de corpus y seront rouges et nommés comme tels, et tout le reste aura son verdict.
- **Entrée 4 (`indexed: false`)** : faite par Codex sur `cx-lucivy-indexed-false-warning` (890c86a). Il reste la même
  qualification que l'entrée 3, puis la PR.
- **Entrée 5 (la 4.3.10)** : elle pourra porter l'ulp, `indexed: false` et la PR #42 quand les trois seront sur main.
- **Deux tickets neufs**, portés sur master avec cette page : les tests à chemins personnels et `.vault/`, et le
  parcours de la vérité qui écarte des fichiers en silence.

## 4. Lisibilité de sa passation

**Mauvaise.** Pour savoir ce qui est fait, il faut traverser des couches de cérémonie : GO, coupons, nonces, « Q »,
« RO », « HOLD », SHA de registres, horloges `CLOCK_BOOTTIME`. Les mots y sont collés à leurs nombres
(« source0f compilée », « 16verts »), et les affirmations sont toujours bornées par des réserves (« pas un vert »,
« non qualifié »). Le fait le plus utile, que le corpus manque sur les deux postes, est en page 03 du 16 et n'apparaît
pas dans la passation. Ce qui y manque : en tête, une phrase par entrée de ma page (faite, à moitié, pas commencée)
et la décision à prendre par Lucie.
