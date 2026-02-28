# Design : ContainsQuery — recherche intelligente multi-strategie

> Suite de `03-next-steps.md`. Reflexions sur un type de query `contains` qui combine automatiquement exact, fuzzy et substring matching avec validation des separateurs.

---

## Le probleme

On veut un `contains` qui fonctionne comme un humain chercherait : on colle un bout de texte (code, identifiant, phrase), et ca retrouve le passage meme avec des typos, des variations de casse, ou des fragments partiels.

### Exemple cible

```
Query:   "this.I.My"
Texte:   "thys.Is.MyQueri"

Attendu : MATCH avec score BM25
  - "this" → "thys"     (fuzzy, distance 1)
  - "."    → "."         (separateur valide)
  - "I"    → "Is"        (substring, "i" dans "is")
  - "."    → "."         (separateur valide)
  - "My"   → "MyQueri"   (substring, "my" prefixe de "myqueri")
```

### Ce qui existe vs ce qu'on veut

| Feature | PhraseQuery | RegexPhraseQuery | FuzzyTermQuery | ContainsQuery (cible) |
|---------|-------------|------------------|----------------|----------------------|
| Tokens consecutifs (positions) | oui | oui | non | oui |
| Match exact | oui | oui (regex escaped) | non | oui |
| Fuzzy (Levenshtein) | non | non | oui | oui |
| Substring (regex) | non | oui | non | oui |
| Auto-cascade (essaie tout) | non | non | non | **oui** |
| Validation separateurs | non | non | non | **oui** |
| BM25 scoring | oui | oui | oui | oui |

---

## Architecture Tantivy : ce qu'on a pour construire ca

### Chaine Query → Weight → Scorer

```
Query (recette, pas de segment)
  └→ Weight (lie a un Searcher, stats BM25 globales)
       └→ Scorer (curseur sur docs matchants pour un segment)
              implements DocSet (iteration) + score() (BM25)
```

Fichiers cles dans `izihawa-tantivy/src/query/` :

| Composant | Fichier | Role |
|-----------|---------|------|
| **Query trait** | `query.rs:94-127` | Definit `weight()`, `query_terms()`, `count()` |
| **Weight trait** | `weight.rs` | Definit `scorer()`, `explain()` — seuls ces 2 sont requis |
| **Scorer trait** | `scorer.rs` | `DocSet + score()` — iteration + scoring |
| **PhraseQuery** | `phrase_query/phrase_query.rs` | Phrase exacte, `Vec<(usize, Term)>` + slop |
| **PhraseWeight** | `phrase_query/phrase_weight.rs:42` | `phrase_scorer()` : lookup terme → postings with positions |
| **PhraseScorer** | `phrase_query/phrase_scorer.rs` | Generique `T: Postings`, intersection de positions |
| **RegexPhraseQuery** | `phrase_query/regex_phrase_query.rs` | **Phrase ou chaque position est un regex** |
| **RegexPhraseWeight** | `phrase_query/regex_phrase_weight.rs` | Union de postings via automaton + PhraseScorer |
| **RegexQuery** | `regex_query.rs` | Regex sur termes individuels, utilise AutomatonWeight |
| **AutomatonWeight** | `automaton_weight.rs` | Generique `A: Automaton`, marche FST + automaton en lockstep |
| **FuzzyTermQuery** | `fuzzy_term_query.rs` | Levenshtein, utilise DfaWrapper avec AutomatonWeight |
| **Intersection** | `intersection.rs` | DocSet intersection pour BooleanQuery |

### Points cles

1. **`PhraseScorer<T: Postings>`** est generique — il accepte n'importe quelle source de postings. `PhraseQuery` lui donne des `SegmentPostings` (un terme), `RegexPhraseQuery` lui donne des `SimpleUnion` (union de plusieurs termes matchant le regex).

2. **`AutomatonWeight<A: Automaton>`** est generique — il marche avec `Regex` ET `DfaWrapper` (Levenshtein). La methode `get_match_term_infos()` retourne les `TermInfo` de tous les termes matchant l'automaton.

3. **`RegexPhraseWeight`** fait le pont : pour chaque position, il utilise `AutomatonWeight<Regex>` pour trouver les termes, construit une union de postings, puis les passe au `PhraseScorer`. Le bucketing (sparse/dense) optimise les unions larges.

4. **`wildcard_query_to_regex_str()`** dans `regex_phrase_query.rs` — helper pour convertir wildcards en regex.

5. **`max_expansions`** (defaut 16384) — limite le nombre de termes expands par regex pour eviter les explosions.

### Intersection de positions (coeur du PhraseScorer)

`phrase_scorer.rs:463` — `compute_phrase_match()` :

```
Pour chaque document contenant TOUS les termes :
  1. Charger les positions du premier terme
  2. Pour chaque terme suivant :
     a. Charger ses positions
     b. Intersect avec les positions accumulees (two-pointer merge)
     c. Si vide → pas de match, skip ce document
  3. Dernier terme : compter les matchs (phrase_count pour BM25)
```

Le `slop` permet un ecart de positions (0 = strict consecutif).

---

## Design propose : ContainsQuery

### Principe 1 : Auto-cascade (pas de mode a choisir)

Pour chaque token de la query, on construit un **automaton unifie** qui couvre les 3 strategies :

```
Token "this" →
  Union de :
    - Exact : terme "this" dans l'index
    - Fuzzy : tous termes a distance ≤ d de "this" (Levenshtein DFA)
    - Substring : tous termes matchant .*this.* (Regex automaton)
  → Union de postings → une position dans le PhraseScorer
```

Le `PhraseScorer` verifie ensuite que les positions sont consecutives (avec slop optionnel).

Le scoring BM25 est naturellement correct :
- Match exact → IDF eleve (terme rare = bon match)
- Match fuzzy → IDF du terme matche (peut etre different)
- Match substring → IDF du terme contenant la substring

### Principe 2 : Validation des separateurs

**Probleme :** Le SimpleTokenizer jette les separateurs. `"this.I.My"` et `"this::I::My"` produisent les memes tokens `["this", "i", "my"]`.

**Solution envisagee :** Modifier le tokenizer pour **emettre les separateurs comme tokens** avec un flag special :

```
"this.I.My"     → ["this", ".", "i", ".", "my"]
                    tok=0  sep=1 tok=2 sep=3 tok=4

"thys.Is.MyQueri" → ["thys", ".", "is", ".", "myqueri"]
                      tok=0  sep=1 tok=2 sep=3  tok=4
```

Puis dans le ContainsQuery :
- Positions paires (0, 2, 4) : tokens normaux → auto-cascade (exact/fuzzy/substring)
- Positions impaires (1, 3) : separateurs → match exact (ou regex si on veut `.` = n'importe quel separateur)

**Impact sur les autres queries :**
Les queries existantes (term, fuzzy, parse, phrase, regex) doivent **ignorer les tokens separateurs**. Options :
- Tokenizer custom qui tague les separateurs (metadata sur le token)
- Champ separe `._sep` qui inclut les separateurs (comme `._raw` pour le stemming)
- Filter qui les supprime sauf pour `contains`

### Principe 3 : Distance fuzzy cumulative + tolerance dernier token

La distance fuzzy est un **budget global** sur toute la query, pas par token. La somme des distances par token doit rester ≤ seuil.

**Exception dernier token :** si le token indexe est plus long que le token query, la distance est calculee sur le **prefixe de meme longueur** (les caracteres excedentaires sont gratuits — c'est un prefix match, pas une erreur).

```
Query:   "this.I.My"
Seuil:   distance max = 2

"thys.Is.MyQueri"  → VALIDE (distance totale = 1)
  - "this" → "thys"     distance 1  cumul = 1
  - "i"    → "is"       distance 0  cumul = 1  (dernier-non: "i" vs "is" = dist 1??)

  Hmm, en fait :
  - "this" → "thys"     distance 1  cumul = 1
  - "i"    → "is"       distance 1  cumul = 2  ← pile au seuil
  - "my"   → "myqueri"  distance 0  cumul = 2  (dernier token, prefix: "my" vs "my" = 0)
  → distance totale = 2 ≤ 2 → VALIDE ✓

"thyz.Is.MyQueri"  → INVALIDE (distance totale = 3)
  - "this" → "thyz"     distance 2  cumul = 2
  - "i"    → "is"       distance 1  cumul = 3  ← depasse le seuil
  → INVALIDE ✗ (on peut meme arreter ici, pas besoin de verifier "my")

"this.I.MxQueri"   → VALIDE (distance totale = 1)
  - "this" → "this"     distance 0  cumul = 0
  - "i"    → "i"        distance 0  cumul = 0
  - "my"   → "mxqueri"  dernier token, prefix: "my" vs "mx" = distance 1, cumul = 1
  → distance totale = 1 ≤ 2 → VALIDE ✓

"this.I.My"        → MATCH EXACT (distance totale = 0)
  - Tous tokens exacts, pas de suffix excedentaire
  → distance totale = 0 → VALIDE ✓
```

**Regle formelle :**
1. Tokeniser la query → tokens `[t0, t1, ..., tn]`
2. Pour chaque token `ti` matche avec le token indexe `di` :
   - Si `i < n` (pas le dernier) : `dist(ti, di)` = Levenshtein standard
   - Si `i == n` (dernier) : `dist(ti, prefix(di, len(ti)))` = Levenshtein sur le prefixe de meme longueur
3. `sum(dist) ≤ seuil` → match valide

**Consequence architecture :** On ne peut pas utiliser des automatons Levenshtein independants par position (ils ne partagent pas de budget). Il faut un scorer custom qui accumule la distance et coupe des que le budget est depasse. C'est un changement plus profond que juste combiner des automatons existants.

### Questions ouvertes

| Question | Options |
|----------|---------|
| Les separateurs sont-ils des tokens avec position ou un metadata ? | Token avec position (simple, fonctionne avec PhraseScorer) vs metadata (propre, mais necessite un nouveau Scorer) |
| Un `.` dans la query matche-t-il uniquement `.` ou tout separateur ? | Exact (`.` = `.`) vs classe (`.` = tout single-char non-alnum) vs regex configurable |
| Que faire des separateurs multi-caracteres (`::`, `->`, `<<=`) ? | Un seul token separateur par boundary vs un token par caractere |
| Impact perf de doubler le nombre de tokens dans l'index ? | Positions x2, taille index +~30-50% (separateurs sont courts et repetitifs) |
| Faut-il un champ dedie ou modifier le champ `._raw` ? | Champ `._sep` dedie (safe, pas d'impact sur l'existant) vs modifier raw (risque) |

---

## Stockage des offsets caracteres dans les postings

### Etat actuel

Le Token de Tantivy a deja `offset_from` et `offset_to` (remplis par le tokenizer), mais ils sont **jetes** a l'indexation. Les postings ne stockent que :

```
terme → [(doc_id, frequency, [positions])]
                                ↑ positions de tokens (0,1,2...), PAS offsets caracteres
```

`IndexRecordOption` actuel :

```rust
pub enum IndexRecordOption {
    Basic,                    // doc IDs seulement
    WithFreqs,                // + term frequencies
    WithFreqsAndPositions,    // + positions de tokens
}
```

### Ce qu'on veut : comme Lucene

Lucene a `IndexOptions.DOCS_AND_FREQS_AND_POSITIONS_AND_OFFSETS`. On veut la meme chose :

```rust
pub enum IndexRecordOption {
    Basic,
    WithFreqs,
    WithFreqsAndPositions,
    WithFreqsAndPositionsAndOffsets,  // NOUVEAU — positions + offsets caracteres
}
```

Avec ca, les postings stockent :

```
terme → [(doc_id, frequency, [(position, offset_from, offset_to)])]
```

### Pourquoi

Le post-filtre du ContainsQuery a besoin des offsets caracteres pour :
1. Extraire les separateurs entre tokens consecutifs dans le texte original
2. Valider que les separateurs matchent ceux de la query
3. Calculer la distance fuzzy sur les bonnes portions du texte

Sans les offsets dans les postings, il faut re-tokeniser le texte stocke (possible mais inelegant). Avec les offsets, le post-filtre a tout ce qu'il faut directement depuis l'index.

### Fichiers a modifier dans izihawa-tantivy

| Fichier | Modification |
|---------|-------------|
| `src/schema/index_record_option.rs` | Ajouter `WithFreqsAndPositionsAndOffsets` |
| `src/postings/serializer.rs` | Serialiser offset_from/offset_to dans les postings |
| `src/postings/segment_postings.rs` | Deserialiser et exposer les offsets |
| `src/postings/postings.rs` | Trait Postings : ajouter `offsets()` ou enrichir `positions()` |
| `src/indexer/segment_writer.rs` | Passer les offsets du Token aux postings |
| `src/query/phrase_query/phrase_scorer.rs` | Optionnel : rendre les offsets accessibles au scorer |

### Format de stockage des offsets

Deux options :

1. **Relatif au debut du document** : stocker offset_from et offset_to tels quels. Simple mais prend plus de place.
2. **Delta-encode** : stocker le delta par rapport a l'offset precedent. Compact (les deltas sont petits, bonne compression VInt).

Lucene utilise le delta-encoding. On ferait pareil.

---

## Approche d'implementation

### Etape A : Tokenizer avec separateurs

Creer un `SeparatorAwareTokenizer` qui emet :
- Tokens alphanumeriques (comme SimpleTokenizer)
- Tokens separateurs (les caracteres non-alnum entre deux tokens)
- Chaque token a une position incrementale

```rust
// Input: "this.Is.MyQueri"
// Output:
//   Token { text: "this",    position: 0, is_separator: false }
//   Token { text: ".",       position: 1, is_separator: true  }
//   Token { text: "is",      position: 2, is_separator: false }
//   Token { text: ".",       position: 3, is_separator: true  }
//   Token { text: "myqueri", position: 4, is_separator: false }
```

### Etape B : AutomatonPhraseQuery (generalise RegexPhraseQuery)

Modifier ou creer une variante de `RegexPhraseQuery` qui accepte differents types d'automatons par position :

```rust
enum PositionMatcher {
    // Token normal : union exact + fuzzy + substring
    Auto { term: String, fuzzy_distance: u8 },
    // Separateur : match exact ou classe
    Separator(String),
    // Fallback : regex libre
    Regex(String),
}

struct ContainsQuery {
    field: Field,
    positions: Vec<(usize, PositionMatcher)>,
    slop: u32,
}
```

Pour `PositionMatcher::Auto`, le Weight construit l'union de 3 automatons :
1. Exact : `TermQuery`-style (lookup direct)
2. Fuzzy : `DfaWrapper` (Levenshtein automaton)
3. Substring : `Regex(".*{escaped_term}.*")`

### Etape C : Integration dans tantivy_fts

Le `build_contains_query()` dans notre crate FFI :
1. Tokenize la valeur avec le `SeparatorAwareTokenizer`
2. Construit un `ContainsQuery` avec les positions alternees token/separateur
3. Le FFI reste identique : `{"type": "contains", "field": "body", "value": "this.I.My", "fuzzy_distance": 2}`

---

## Fichiers a modifier/creer

### Dans izihawa-tantivy (le fork)

| Fichier | Action |
|---------|--------|
| `src/tokenizer/separator_aware_tokenizer.rs` | **Nouveau** — tokenizer qui emet les separateurs |
| `src/tokenizer/mod.rs` | Ajouter export du nouveau tokenizer |
| `src/query/phrase_query/automaton_phrase_query.rs` | **Nouveau** — phrase query avec automaton generique par position |
| `src/query/phrase_query/automaton_phrase_weight.rs` | **Nouveau** — weight qui construit union d'automatons |
| `src/query/phrase_query/mod.rs` | Ajouter exports |
| `src/query/mod.rs` | Ajouter exports |

### Dans tantivy_fts (notre crate FFI)

| Fichier | Action |
|---------|--------|
| `rust/src/query.rs` | Modifier `build_contains_query()` pour utiliser le nouveau query |
| `rust/src/handle.rs` | Enregistrer le `SeparatorAwareTokenizer` |
| `test/test_ffi.c` | Tests contains avec separateurs et fuzzy |

---

## Complexite estimee

| Composant | Difficulte | Lignes estimees | Base sur |
|-----------|-----------|-----------------|----------|
| SeparatorAwareTokenizer | Faible | ~80 | SimpleTokenizer fait ~50 lignes |
| AutomatonPhraseQuery | Moyenne | ~150 | RegexPhraseQuery fait ~120 lignes |
| AutomatonPhraseWeight | Moyenne-haute | ~200 | RegexPhraseWeight fait ~180 lignes (bucketing union) |
| Integration FFI | Faible | ~50 | Modification build_contains_query |
| Tests | Moyen | ~100 | Existants comme reference |
| **Total** | | **~580** | |

Le plus gros morceau est le `AutomatonPhraseWeight` qui doit gerer l'union de 3 types d'automatons par position. Mais `RegexPhraseWeight` fait deja 90% du travail — il suffit de le generaliser.

---

## References code

### RegexPhraseQuery — le modele a suivre

```
izihawa-tantivy/src/query/phrase_query/regex_phrase_query.rs

pub struct RegexPhraseQuery {
    field: Field,
    phrase_terms: Vec<(usize, String)>,  // (offset, regex_pattern)
    slop: u32,
    max_expansions: u32,
}
```

### RegexPhraseWeight — construction union postings

```
izihawa-tantivy/src/query/phrase_query/regex_phrase_weight.rs:42

fn phrase_scorer():
  Pour chaque (offset, regex_pattern):
    1. Compile regex → Automaton
    2. AutomatonWeight::get_match_term_infos(reader) → Vec<TermInfo>
    3. get_union_from_term_infos() → SimpleUnion<Box<dyn Postings>>
    4. posting_lists.push((offset, union))
  PhraseScorer::new(posting_lists, ...)
```

### AutomatonWeight — generique sur le type d'automaton

```
izihawa-tantivy/src/query/automaton_weight.rs

pub struct AutomatonWeight<A: Automaton> { ... }

fn get_match_term_infos(&self, reader: &SegmentReader) -> Result<Vec<TermInfo>>
  → Marche le FST en lockstep avec l'automaton
  → Retourne tous les termes matchants avec leurs metadata
```

### PhraseScorer — generique sur Postings

```
izihawa-tantivy/src/query/phrase_query/phrase_scorer.rs

pub struct PhraseScorer<TPostings: Postings> { ... }

fn compute_phrase_match():
  → Intersection sorted positions (two-pointer merge)
  → Supporte slop (ecart de positions tolere)
```

### SimpleTokenizer — reference pour le nouveau tokenizer

```
izihawa-tantivy/src/tokenizer/simple_tokenizer.rs

Coupe sur !char::is_alphanumeric()
Emet un token par sequence alphanumerique
~50 lignes
```
