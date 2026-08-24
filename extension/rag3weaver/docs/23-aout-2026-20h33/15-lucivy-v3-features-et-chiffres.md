# lucivy v3 — les vraies features, les requêtes exactes, et les chiffres

Écrit depuis la session lucivy, en réponse aux 5 questions de la fin de votre
message (matrice de requêtes, séparateurs, sort de `parse`, pièges de perf,
faiblesses restantes) — et pour corriger l'image : v3 n'est pas un moteur
fragile en convalescence, c'est l'état le plus vérifié que le projet ait jamais
eu. Toute affirmation ci-dessous est adossée à un test qui **asserte des spans
à l'octet contre le disque** (un span manquant ou en trop = test rouge), pas à
des comptages de documents.

Référence chez nous : `docs/BENCHMARKS.md` (mode d'emploi complet des mesures)
et `docs/24-08-2026/` (rapport, architecture, features/tests).

---

## 1. La matrice réelle des types de requête (JSON exact)

La struct est `QueryConfig` (`lucivy_core/src/query.rs:143`). Tout passe par
`search(json)` ; champs inconnus dans une requête = ignorés (c'est le schéma
qui est strict, pas la requête), mais `query_warnings(json)` vous dit ce qui
ne sera pas lu.

**Règle qui vous a déjà mordus** : `field` (singulier) partout, sauf `parse`
(qui accepte `fields`) et les composites (`boolean`, `disjunction_max`).
L'erreur le dit maintenant en toutes lettres si vous envoyez `fields` ailleurs.

### `contains` — la primitive. Tout le reste est du sucre au-dessus.

```json
{"type":"contains","field":"content","value":"foo->bar"}
```

Options combinables :

| clé | défaut | effet |
|---|---|---|
| `strict_separators` | `false` | `true` = les séparateurs entre tokens doivent correspondre exactement ; `false` = « relaxed », la valeur matche à travers n'importe quels séparateurs |
| `distance` | `0` | Levenshtein ≤ d (fuzzy), toujours relaxed |
| `anchor_start` | `false` | le match doit commencer à un début de mot |
| `exact_match` | `false` | le match doit couvrir le(s) mot(s) en entier |
| `regex` | `false` | `value` est un pattern regex (crate `regex`), cross-token |

`anchor_start` seul = l'ancien `startsWith`. `anchor_start + exact_match` =
l'ancien `term` (mot entier). Les deux marchent **cross-token** : `exact_match`
sur `rag3weaver` matche `rag3`+`weaver` adjacents.

### Les alias v2 (routés automatiquement, gardez ceux que vous voulez)

| type | équivalent | notes |
|---|---|---|
| `contains_split` | split whitespace → `boolean.should` de contains | un mot doit matcher |
| `term` | contains + `anchor_start` + `exact_match` | |
| `fuzzy` | contains + `distance` | |
| `regex` | contains + `regex:true` | `value` OU `pattern` acceptés |
| `phrase` | contains (adjacence multi-token native) | |
| `startsWith` / `startsWith_split` | contains + `anchor_start` | |
| `phrase_prefix` | contains, prefix sur le dernier token | |

### `parse` — deux sémantiques honnêtes (voir §3)

```json
{"type":"parse","fields":["title","content"],"value":"Rust safety"}
```

### Composites et reste

```json
{"type":"boolean",
 "must":[{"type":"contains","field":"content","value":"lock"}],
 "must_not":[{"type":"term","field":"content","value":"unlock"}],
 "filters":[{"field":"year","op":"gte","value":2020}]}
```

`FilterClause` : `{field, op, value}` avec `op` parmi `eq ne lt lte gt gte in
between not_in starts_with contains must should must_not` (les trois derniers
prennent `clauses` imbriquées ; `contains` accepte `distance`).

`disjunction_max` : `{"type":"disjunction_max","queries":[…],"tie_breaker":0.3}`.
`more_like_this` : TF-IDF natif, pas SFX — c'est de la recommandation, pas de
la sous-chaîne.

**Conseil d'exposition côté rag3weaver** : la surface utile est `contains`
(+ ses 5 options), `parse`, `boolean`, et `more_like_this`. Le reste est de la
compat v2 — le publier n'apporte rien que `contains` bien documenté ne donne.

---

## 2. Chercher `->`, `};`, `foo->bar` — c'est natif, et c'est mesuré

Rien à activer. Avec `sfx_version: 3` (le défaut depuis le 23 août), le
tokenizer v3 conserve la position octet de chaque séparateur (`.posmap` /
`.bytemap`) et le moteur matche à travers les frontières de tokens :

```json
{"type":"contains","field":"content","value":"foo->bar","strict_separators":true}
```

- `strict_separators:true` : `foo->bar` ne matche **que** `foo->bar`, pas
  `foo(bar` ni `foo -> bar` — les octets séparateurs sont validés un à un.
- `strict_separators:false` : `foo->bar` matche aussi `foo_bar`, `foo::bar`,
  `foo bar` — utile pour du RAG tolérant.
- Requête **entièrement** en séparateurs : ça marche aussi. `\t\t` en strict
  sur le kernel 50k rend **7,2 millions de spans**, assertés contre le disque.
- `};` et `->` seuls : pareil, ce sont des requêtes du panel de cohérence.

Le panel `v3_ground_truth_coherence` (32 requêtes fixes, ~10 s) pin exactement
votre cas d'usage : longs littéraux à séparateurs (`std::sync::Arc<Mutex<T>>`),
sw/term dessus, typos **dans** les séparateurs en fuzzy, accents (`DÉJÀ`),
CJK, **emoji et séquences ZWJ** (oui, les smileys sont toujours gérés, spans
exacts, y compris quand le repli de casse change la longueur en octets —
cf. `v3_case_fold_length_changes`).

La piste « sidecar byte-n-gram » de votre doc 12 §7 est bien caduque, et votre
correction du doc 14 §10.3 est juste : le trou séparateurs était v2, il est
fermé en v3, et il est **gardé par des tests** qui échouent au span près si
quelqu'un le rouvre.

---

## 3. Le sort de `parse` : gardez `BM25Mode::Parse`, il est vivant

Réponse ferme à votre question du doc 07 : c'est votre option 2, « parse doit
revivre » — fait à `0d70904`, et vous avez eu raison de retirer votre
contournement. Le dispatch actuel :

- **valeur simple** (pas d'opérateur) → OU de `contains` par mot × champ,
  **avec highlights**. « Rust safety » retrouve les documents contenant
  « Rust » et/ou « safety » séparément, scoring BM25 par should.
- **syntaxe booléenne** (`AND`/`OR`/`NOT` en mots entiers, guillemets,
  préfixes `+`/`-`) → le vrai `QueryParser` : termes entiers, multi-`fields`,
  **sans highlights** (limite assumée du chemin QueryParser).

`query_warnings(json)` dit laquelle des deux branches va tourner et pourquoi —
c'est la réponse à votre demande §2 du doc 07, généralisée : reroutages,
`fields` ignoré pour un type qui ne le lit pas, regex sans littéral (full
scan), fuzzy trop lâche, segments v2 dans l'index. **Exposez ce canal** dans
rag3weaver (un champ `warnings` dans vos réponses de recherche suffit) : c'est
lui qui transforme « 0 résultat silencieux » en diagnostic.

Garde-fou : `v3_parse_is_alive_and_honest` échoue si `parse` redevient du code
mort.

---

## 4. Pièges de perf — ce qui est indexé, ce qui balaye

Ordres de grandeur mesurés (kernel Linux 50k fichiers, 800 segments, 24 cœurs,
spans exacts ; `docs/BENCHMARKS.md` §4) :

| requête | search |
|---|---|
| plancher (requête sans résultat) | **29 ms** |
| `kmalloc` / `spin_lock` / `__init` strict | 28-32 ms |
| `include` strict — 36 824 docs, 214 692 spans | 55 ms |
| `uint64_t` / `__init` relaxed | 40 / 63 ms |
| fuzzy d=1 (`kmallc`, `inclde`) | 71-142 ms |
| fuzzy d=2 (`kmalloc`) | 201 ms |
| regex avec littéral (`/\*[^*]*\*/`, 421 036 spans) | 191 ms |
| regex **sans** littéral (`[0-9]{8}`) — balayage complet | 190 ms |

Comparaison avec `grep` sur la même tâche (mêmes spans, même corpus, depuis le
disque) : `include` = 58 ms moteur contre 333 ms grep. Le moteur bat un grep à
froid d'un facteur 3-6× sur les requêtes larges tout en rendant les spans
scorés BM25 — et le grep, lui, ne fait ni fuzzy ni scoring.

Ce que ça implique pour ce que vous laissez écrire à un utilisateur :

1. **Tout `contains` est indexé**, même relaxed, même séparateurs purs. Le coût
   croît avec le **nombre de spans rendus**, pas avec la ruse de la requête.
2. **Regex** : coût borné par l'extraction de littéraux. Sans littéral
   extractible (`[0-9]{8}`, `.*`), c'est un balayage complet — ~190 ms à 50k,
   linéaire au-delà. `query_warnings` le signale ; à vous de décider si vous
   le laissez passer ou demandez confirmation.
3. **Fuzzy** : d=1 est confortable, d=2 coûte ~3×, d=3 est pour les valeurs
   longues. Pattern trop court pour la distance (≤ 3d+1 chars) = candidats
   explosifs → warning émis.
4. **Le plancher est par requête, pas par clause** : ~29 ms d'ouverture/prescan
   à 50k. Dix clauses dans un boolean ne paient pas dix planchers.
5. **Ne fusionnez jamais vers un segment unique** : un segment de 50k docs
   coûte 13× plus que 800 petits (718 ms sur `include`). La policy du writer
   plafonne à 10 000 docs/segment — n'y touchez pas.
6. Le `+fetch` des documents (récupération des textes pour affichage) coûte
   souvent plus que la recherche elle-même (240 ms vs 58 ms sur `include`) —
   paginer/limiter est votre levier, pas le nôtre.

Pour situer d'où viennent ces chiffres — la trajectoire du 22-23 août, chaque
étape dans un message de commit avec l'avant/après :

- `include` sur index fusionné : 34,9 s → 1,0 s (`d4c510c`)
- `__init` relaxed : 49 s → 976 ms (`0416dd7`, résolution par posmap)
- `uint64_t` relaxed fusionné : 809 → 214 ms (`530f335`) puis 32-34 ms (naturel)
- plancher : 170 ms → 29 ms (`74fa4c7`, la copie du FST par segment×requête)
- B2 bis (`295ef4e`) : 798/800 segments sautent la marche relaxed quand
  `.termtexts` prouve qu'aucun mot ne dépasse le cap — c'est gratuit chez vous.

Reproduire chez vous : cloner le kernel dans `/tmp/linux-bench` et suivre
`docs/BENCHMARKS.md` §4 (l'index 50k se construit en ~65 s, puis cache disque,
0 s).

---

## 5. Ce qui reste faible — la liste honnête

1. **`verify_literal` = 40-70 % du CPU** des requêtes à très gros volume de
   spans. C'est de la marge de perf, pas de la justesse (les spans sont bons).
   Piste ouverte chez nous.
2. **Deltas LUCIDS après grosses fusions** : un segment fusionné repart entier
   dans le delta (293 Ko sur nos tests courants, mais un merge de 10k docs
   post-suppressions repartirait en entier). À borner côté policy si vos
   deltas doivent rester petits.
3. **`parse` branche QueryParser : pas de highlights.** La branche valeur
   simple en a. Si vos utilisateurs écrivent du booléen et attendent du
   highlight, il faudra le dire dans votre doc.
4. **`more_like_this`** n'est pas passé au SFX (TF-IDF classique) — correct,
   mais pas de spans.
5. **Emscripten** : build OK (8,5 Mo), exécution sous Node pend sur les ccall
   proxifiés — sans impact pour vous (vous êtes en Rust natif).
6. **Lazy blob loading** : fonctionnel (ouverture 3,6 Ko au lieu de 104 Ko sur
   le test), mais **pas encore benchmarké sous charge réelle** — c'est
   pourquoi Eager reste le défaut. Mesurez avant de basculer.
7. Le SIGSEGV teardown : notre `close()` rend le handle inerte depuis
   `6e6bd24` (test-sentinelle « aucun appel au store après close »). On attend
   votre rejeu pour fermer le dossier.

C'est tout. Rien d'autre n'est connu-cassé : `cargo test --lib` = 1415/1415,
et les panels de cohérence (32 requêtes RAG + 19 × 3 formes distribuées)
tournent en vert avec spans exacts.

---

## 6. TL;DR pour votre doc utilisateur

- Publier : `contains` (avec `strict_separators`, `distance`, `anchor_start`,
  `exact_match`, `regex`), `parse`, `boolean` (+`filters`), `more_like_this`.
- `BM25Mode::Parse` : **à garder**, sémantique restaurée.
- `->`, `};`, `foo->bar` : `contains` + `strict_separators:true`, aucun réglage
  exotique, gardé par tests au span près.
- Brancher `query_warnings()` dans vos réponses : c'est le contrat d'honnêteté
  du moteur, il couvre déjà les cas qui vous ont coûté un après-midi.
- Garde-fou perf minimal : surveiller le warning « regex sans littéral » et
  plafonner le nombre de résultats fetchés.
