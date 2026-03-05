# 08 — Contains : Optimisations & Benchmarks

## Etat actuel (7 fevrier 2026)

### Cascade 4 niveaux (code actuel)

Par token de la query, dans l'ordre :

| Niveau | Methode | Cout FST | Distance cascade |
|--------|---------|----------|-----------------|
| 1. Exact | Lookup direct term dict | O(1) | 0 |
| 2. Fuzzy | DFA Levenshtein sur FST | Walk partiel (pruning DFA, ~10-20% FST) | d |
| 3. Substring | Regex `.*token.*` sur FST | Walk complet (pas de pruning) | 0 |
| 4. FuzzySubstring | NFA simulation sur FST | Walk complet (pas de pruning) | d |

Early termination : des qu'un niveau trouve des term_infos, les suivants sont skipped.

### Probleme : walk FST redondant

Niveaux 3 (Substring) et 4 (FuzzySubstring) font chacun un walk **complet** du FST. FuzzySubstring subsume Substring (distance 0 du DFA Levenshtein = substring exact). Garder Substring separe signifie :
- Quand Substring trouve : OK, distance 0, pas de walk redondant.
- Quand Substring trouve rien : on a walk le FST pour rien, puis on re-walk avec FuzzySubstring.

Le cout par byte de FuzzySubstring est legerement superieur (maintenir Vec<u32> de ~40 etats DFA vs un DFA regex), mais les deux sont O(FST) et le walk domine.

---

## Optimisation A : Fusionner Substring dans FuzzySubstring

**Impact : elimine 1 walk FST complet dans le pire cas.**

Cascade 3 niveaux :

| Niveau | Methode | Cout FST | Distance cascade |
|--------|---------|----------|-----------------|
| 1. Exact | Lookup direct | O(1) | 0 |
| 2. Fuzzy | DFA Levenshtein | Walk partiel | d |
| 3. FuzzySubstring | NFA simulation | Walk complet | d |

Tradeoff : les matches qui etaient "Substring distance 0" deviennent "FuzzySubstring distance d". C'est plus conservateur pour le budget. Mais c'est coherent — si un token arrive au niveau 3, ni Exact ni Fuzzy ne l'ont trouve, donc le match est effectivement de moindre qualite.

### Pourquoi garder Fuzzy separe ?

Le DFA Fuzzy **prune** le FST : `can_match()` retourne false pour les branches impossibles. Sur un FST typique, ca visite 10-20% des noeuds. FuzzySubstring ne peut pas pruner (`can_match()` retourne toujours true a cause du `.*` prefix). Donc Fuzzy comme "fast path" reste pertinent.

### Pourquoi ne PAS garder Substring separe ?

Substring et FuzzySubstring font le meme travail (walk complet). Le cout par byte de FuzzySubstring est marginalement superieur (~2-5x par byte pour maintenir les etats actifs), mais :
- Le nombre d'etats actifs est borne par la taille du DFA (~40 pour d=1, ~100 pour d=2)
- Le walk FST (I/O + traversee trie) domine largement le cout per-byte
- En pratique la difference est negligeable

---

## Optimisation B : Classification post-walk

**Impact : distance cascade plus precise, meilleur budget.**

Apres le walk FuzzySubstring, pour chaque `term_info` trouve, verifier si le terme contient le token comme **substring exact** (distance 0) ou seulement comme **fuzzy substring** (distance > 0).

Methode : re-appliquer le regex `.*token.*` sur le terme (ou utiliser `str::contains`). Si match → distance 0, sinon → distance d.

Cela permettrait de reporter `CascadeLevel::FuzzySubstring(0)` pour les substrings exacts et `CascadeLevel::FuzzySubstring(d)` pour les fuzzy, preservant le budget pour les separateurs.

Cout : un `str::contains` par terme trouve. Negligeable compare au walk FST.

Prerequis : avoir acces au texte du terme pendant le walk FST. Le `TermStreamer` expose `stream.key()` (les bytes du terme). Donc c'est faisable.

---

## Optimisation C : ContainsScorer — acces store

**Impact : reduction I/O pour la validation des separateurs.**

Actuellement, pour chaque doc candidat, le ContainsScorer :
1. Charge le texte stocke via `StoreReader::get(doc_id)`
2. Extrait les separateurs entre tokens (via byte offsets des postings)
3. Compare avec les separateurs de la query

Le `StoreReader::get()` est I/O-bound (decompression LZ4/Zstd d'un block store).

### C1. Skip validation early

Si la somme des `cascade_distances` depasse deja le budget **avant** de valider les separateurs, on peut skip le doc sans acceder au store. Deja fait dans le code actuel (lignes 303-314 de contains_scorer.rs). Mais on pourrait le faire encore plus tot, avant meme de creer le scorer, en verifiant que `sum(cascade_distances) <= budget`.

### C2. Batch store reads

Plutot que lire doc par doc, accumuler les doc_ids candidats puis les lire en batch. Le `StoreReader` utilise des blocks comprimes — plusieurs docs par block. Lire sequentiellement permet de decompresser un block une seule fois pour plusieurs docs.

### C3. Stocker les separateurs dans l'index

Nouveau `SegmentComponent` qui stocke, pour chaque occurrence de token, les bytes entre ce token et le suivant. Eliminerait completement l'acces au store pour la validation. Cout : espace disque supplementaire. Probablement overkill pour le moment.

### C4. Eviter le store pour les single-token sans prefix/suffix

Si un single-token query n'a pas de prefix/suffix a valider (`needs_validation() == false`), on peut skip le store completement. Deja fait dans le code (via le path `ConstScorer`).

---

## Optimisation D : FuzzySubstring automaton

**Impact : micro-tuning, marginal.**

### D1. SmallVec pour les etats actifs

Remplacer `Vec<u32>` par `SmallVec<[u32; 64]>`. Evite l'allocation heap pour les cas typiques (DFA d=1 a ~40 etats, d=2 ~100 etats). Requiert la dependance `smallvec` (deja presente dans lucivy).

### D2. Bit set au lieu de Vec triee

Si le nombre d'etats DFA est borne et connu a la construction, utiliser un `BitSet` fixe au lieu de `Vec<u32>` triee + dedup. Avantages : insert O(1), dedup implicite, iteration O(n_states). Le DFA Levenshtein a un nombre fixe d'etats (`dfa.num_states()` si expose).

Probleme : `levenshtein_automata::DFA` n'expose pas directement le nombre d'etats. Faudrait calculer le bound theorique (pour d=1 c'est ~2*(2*len+1) etats max).

### D3. Pre-compute initial transitions

`transition(initial_state, byte)` est appele a chaque byte du FST walk. Pre-calculer les 256 resultats dans un tableau `[u32; 256]` au moment de la construction. Gain : 1 lookup table au lieu de 1 appel de methode par byte.

### D4. Early match propagation

Une fois `matched = true`, l'implementation actuelle fait `state.clone()` a chaque `accept()`. On pourrait utiliser un sentinel state qui evite le clone. Mais le clone d'un `FuzzySubstringState { active: vec![], matched: true }` est deja quasi-gratuit (Vec vide = pas d'alloc).

---

## Optimisation E : Multi-token short-circuit

**Impact : evite des cascades inutiles pour les queries multi-token.**

Actuellement, on cascade chaque token independamment. Si le premier token ne trouve rien a aucun niveau, on return `None` (pas de match possible). Mais on cascade quand meme tous les tokens precedents.

Optimisation : cascader les tokens dans l'ordre du plus rare au plus commun (terme le moins frequent d'abord). Si un token ne trouve rien, on court-circuite immediatement.

Prerequis : estimer la frequence d'un token sans faire le walk complet. Le terme Exact donne une indication (si present, sa doc_frequency est connue). Pour les autres niveaux, c'est plus complexe.

---

## Optimisation F : Index N-gram (elimine le walk FST)

**Impact : remplace le walk FST O(n) par des lookups O(1). Changement de complexite.**

### Le probleme fondamental

FuzzySubstring (et Substring avant lui) font `can_match() → true` partout a cause du `.*` prefix → walk 100% du FST. On ne peut pas pruner. C'est le bottleneck principal sur les gros index.

### Principe

Au lieu de parcourir le FST pour trouver quels termes matchent, on **pre-indexe** des sous-sequences (n-grams) a l'indexation. A la query, on lookup les n-grams de la query → on obtient directement les **docs candidats** sans toucher au FST du raw field.

### Architecture : 3e champ

```
{name}           — stemmed, stored (existant)
{name}._raw      — lowercase, WithFreqsAndPositionsAndOffsets (existant)
{name}._ngram    — trigrams, Basic (doc IDs seulement) (NOUVEAU)
```

A l'indexation, chaque token est decompose en trigrams :
```
"programming" → {pro, rog, ogr, gra, ram, amm, mmi, min, ing}
```

### Query flow : contains avec n-grams

Pour `"progam"` d=1, multi-token ou single-token :

1. **Trigrams de la query** : `{pro, rog, oga, gam}`
2. **Seuil** : pour un fuzzy substring d=1 avec trigrams, au moins `|Q| - n + 1 - d*n` = `6 - 3 + 1 - 3` = **1** trigram doit matcher
3. **Lookup ngram field** : OR(pro, rog, oga, gam) → posting lists → union → **candidats doc_ids**
4. **Verification** : sur les candidats, utiliser le raw field (postings avec positions + offsets) + ContainsScorer pour valider adjacence et separateurs

Etape 4 = exactement ce qu'on fait deja. Le n-gram field remplace juste le walk FST comme source de candidats.

### Multi-token ("std::collections")

1. Token "std" → trigram "std" → lookup ngram → candidats_std
2. Token "collections" → trigrams {col, oll, lle, lec, ect, cti, tio, ion, ons} → lookup ngram → candidats_collections
3. Intersection candidats_std ∩ candidats_collections → docs candidats pour la phrase entiere
4. ContainsScorer verifie positions adjacentes + separateurs "::" via byte offsets

Pas de re-walk du FST, pas de re-parcours de tokens. Juste des lookups de posting lists (O(1) par trigram) puis intersection.

### Selectivite

Le seuil de trigrams controle le tradeoff faux-positifs vs recall :
- **Seuil haut** (tous les trigrams) : tres selectif, peu de candidats, mais rate les fuzzy matches
- **Seuil bas** (1 trigram) : beaucoup de candidats mais aucun miss
- **Seuil adaptatif** : `max(1, |Q| - 2 - d*3)` — ajuste selon la longueur de la query et la distance

Pour d=0 (substring exact), seuil = |Q| - 2 trigrams. Tres selectif.
Pour d=1, seuil diminue de 3. Moins selectif mais reste pratique pour des tokens de longueur >= 6.

### Cout memoire

- Chaque terme de longueur L genere L-2 trigrams
- Terme moyen dans du code : ~8 chars → ~6 trigrams par terme
- Le term dict du ngram field est compact : au plus ~17K trigrams distincts (lowercase ASCII)
- Ce sont les **posting lists** qui grossissent : chaque doc apparait dans ~6 posting lists au lieu de 1
- Estimation : **3-5x** la taille du raw field pour le ngram field
- Acceptable pour un index embedded (pas de reseau, tout en local)

### Cascade revisee avec n-grams

| Niveau | Methode | Cout | Quand |
|--------|---------|------|-------|
| 1. Exact | Lookup raw term dict | O(1) | Toujours |
| 2. Fuzzy | DFA Levenshtein sur raw FST | O(FST partiel) | Si Exact echoue |
| 3. N-gram | Lookup ngram posting lists + verification | O(k lookups + candidats) | Si Fuzzy echoue |

Le walk FST complet disparait. Le niveau 3 est O(k) lookups (k = nombre de trigrams, typiquement 4-8) plus verification des candidats.

FuzzySubstring (NFA simulation) devient un **fallback de verification** sur les candidats au lieu d'un walk FST independant.

### Implementation dans lucivy_fts

**Cote indexation** (`handle.rs`) :
- Ajouter un champ `{name}._ngram` avec un tokenizer custom qui genere les trigrams de chaque token
- `IndexRecordOption::Basic` suffit (on veut juste les doc IDs)

**Cote query** (`query.rs` + `automaton_phrase_weight.rs`) :
- Nouveau niveau de cascade qui :
  1. Genere les trigrams du query token
  2. Cherche chaque trigram dans le ngram field term dict
  3. Fait l'union des posting lists (seuil adaptatif)
  4. Retourne les doc_ids candidats
- Le ContainsScorer prend ces candidats et verifie avec le raw field

**Tokenizer trigram** :
```rust
// Input: "programming"
// Output tokens: ["pro", "rog", "ogr", "gra", "ram", "amm", "mmi", "min", "ing"]
fn trigrams(token: &str) -> Vec<String> {
    let bytes = token.as_bytes();
    (0..bytes.len().saturating_sub(2))
        .map(|i| String::from_utf8_lossy(&bytes[i..i+3]).to_string())
        .collect()
}
```

### Fausse bonne idee : DFA Levenshtein sur les n-grams

Intuition : "et si on appliquait le fuzzy directement sur les n-grams pour tout unifier ?"

Trois variantes envisagees, aucune ne tient :

**a) DFA Levenshtein sur le FST du champ n-gram.** Le FST ngram contient ~17K trigrams (3 chars). Chercher "progam" (6 chars) avec un DFA Levenshtein dedans n'a pas de sens — les unites ne matchent pas.

**b) Generer les variantes fuzzy de chaque trigram.** "pro" d=1 → ~150 variantes (substitutions, insertions, deletions a chaque position). Pour 4 trigrams → ~600 lookups. Ca marche mais c'est overkill — le seuil adaptatif (`max(1, |Q| - 2 - d*3)`) gere deja le fuzzy : il suffit qu'un sous-ensemble de trigrams exacts matche pour capturer les candidats fuzzy.

**c) Remplacer le niveau Fuzzy (DFA walk) par des n-grams.** Ca fonctionne ("helo" → trigrams {hel, elo} → candidats incluant "hello") mais c'est **plus lent** que le DFA walk pour le cas fuzzy pur :

| | DFA Fuzzy (raw FST) | N-grams |
|---|---|---|
| Pruning | Oui (~10-20% du FST visite) | Non (union large de posting lists) |
| Faux positifs | Zero | Beaucoup (verification necessaire) |
| Verification | Aucune | Levenshtein par candidat |

Le DFA Fuzzy et les n-grams sont **complementaires**, pas interchangeables. Le DFA excelle quand il peut pruner (fuzzy pur). Les n-grams excellent quand le pruning est impossible (contains/substring). D'ou la cascade a 3 niveaux : Exact → Fuzzy DFA → N-gram.

### Alternatives envisagees

| Approche | Pour | Contre |
|----------|------|--------|
| **Trigram field** (retenu) | Reutilise l'infra lucivy existante, simple | 3-5x taille index |
| Suffix array sur term dict | O(log n) substring exact | Pas de fuzzy, complexe a persister |
| BK-tree | Bon pour fuzzy pur | Pas pour substring, complexe |
| Bigrams au lieu de trigrams | Plus robuste au fuzzy | Moins selectif (plus de candidats) |
| Trigram → TermOrd map | Plus precis (niveau terme) | Hors infra lucivy, maintenance complexe |

---

## Benchmarks a faire

### Micro-benchmarks (criterion)

1. **FST walk speed** : comparer Substring regex vs FuzzySubstring NFA sur le meme FST
   - Corpus : 100K termes typiques (code source)
   - Queries : tokens de longueur 3-10
   - Mesurer : throughput (terms/sec), latence p50/p99

2. **Cascade latence** : mesurer le temps par niveau de cascade
   - Exact : temps lookup
   - Fuzzy : temps walk avec pruning
   - FuzzySubstring : temps walk complet
   - Comparer 3 niveaux vs 4 niveaux

3. **NFA state set size** : mesurer empiriquement |active| pendant les walks
   - Pour d=1 et d=2
   - Sur des FSTs de taille variee

### Integration benchmarks

4. **ContainsScorer end-to-end** : temps total query → resultats
   - Avec/sans store access
   - Avec/sans offsets (fallback tokenize_raw vs postings offsets)
   - Comparer strict_separators true vs false

5. **Precision/recall** : qualite des resultats
   - FuzzySubstring d=1 vs Substring seul : quels matches supplementaires ?
   - Taux de faux positifs (matches FuzzySubstring rejetes par ContainsScorer)

### Corpus de test

- **Petit** : lucivy codebase elle-meme (~500 fichiers Rust)
- **Moyen** : linux kernel headers (~10K fichiers)
- **Grand** : npm top 100 packages (~100K fichiers JS/TS)

---

## Priorite suggeree

### Court terme (maintenant)
1. **A** (merge Substring/FuzzySubstring) — gain immediat, ~10 lignes de diff
2. **B** (classification post-walk) — ameliore la precision du budget, ~20 lignes

### Moyen terme (quand on a des benchmarks)
3. **F** (index n-gram) — change la complexite de O(FST) a O(k lookups), le plus gros gain possible. Effort significatif (~200 lignes : tokenizer + champ + query path) mais reutilise l'infra lucivy existante.
4. **D1** (SmallVec) — micro-opti triviale, 2 lignes
5. **C1** (early skip) — deja partiellement fait, verifier completude

### Si les benchmarks montrent un bottleneck store
6. **C2** (batch store reads) ou **C3** (separateurs dans l'index)
