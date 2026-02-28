# Clarification : ContainsQuery — scorer custom avec séparateurs et distance cumulative

> Suite de `04-contains-query-design.md` et `05-offsets-implementation-plan.md`.
> Session du 7 février 2026. Réflexions sur l'architecture du scorer après avoir implémenté une première version (AutomatonPhraseQuery) qui s'avère insuffisante.

---

## Ce qui a été fait cette session

### AutomatonPhraseQuery v1 (code existant dans izihawa-tantivy)

Fichiers créés :
- `src/query/phrase_query/automaton_phrase_query.rs` — Query struct
- `src/query/phrase_query/automaton_phrase_weight.rs` — Weight avec cascade + PhraseScorer

Principe : cascade par position (exact → fuzzy Levenshtein → substring regex), early termination, puis PhraseScorer pour vérifier les positions consécutives.

- 990/990 tests passent dans izihawa-tantivy (+6 tests spécifiques)
- tantivy_fts `build_contains_query()` mis à jour pour utiliser AutomatonPhraseQuery

### Problème découvert : "c++"

La query `contains: "c++"` échoue parce que :
1. Le tokenizer raw split "c++" → ["c"] (les "+" sont des séparateurs non-alnum)
2. La cascade trouve "c" comme substring dans "quick", "custom", etc.
3. Résultat : beaucoup trop de matches, le "++" est perdu

C'est le symptôme d'un problème plus profond : **l'absence de validation des séparateurs**.

### Décision : scorer custom au lieu de post-filtre

Plutôt qu'un post-filtre ajouté après coup, on veut un **scorer custom end-to-end** qui intègre :
1. Validation des séparateurs via distance (les séparateurs participent au budget fuzzy)
2. Distance cumulative fuzzy (budget global, tokens + séparateurs)
3. Scoring context-aware : un substring bien placé au bord vaut un exact
   - Premier token : substring-suffix (match la fin du terme doc) → coût 0
   - Dernier token : substring-prefix (match le début du terme doc) → coût 0
   - Token du milieu ou substring mal placé → coût selon distance


---

## Modèle proposé

### Séparateurs

**Définition** : le texte entre `offset_to[token_i]` et `offset_from[token_i+1]` dans le document stocké.

**Validation** : les séparateurs participent au **budget de distance cumulatif**. La distance d'édition (Levenshtein) entre le séparateur de la query et celui du document est ajoutée au budget global.

Exemples :
- Query `"-"` vs doc `"-"` → distance 0 (match exact)
- Query `"-"` vs doc `"_"` → distance 1 (substitution)
- Query `"++"` vs doc `""` (séparateur vide) → distance 2 (2 deletions)
- Query `"<("` vs doc `"<["` → distance 1 (substitution du 2e char)

Exemple : query `"option<result<(i32"`
```
Tokens query :    ["option",  "result",  "i32"]
Offsets query :   [(0,6),     (7,13),    (15,18)]
Séparateurs :     "<" (6..7), "<(" (13..15)
```

Dans le document `"vec<Option<Result<(i32,&str)>>"` :
```
Tokens doc :      ["vec", "option", "result", "i32", "str"]
Offsets doc :     [(0,3), (4,10),   (11,17),  (19,22), ...]
Séparateurs :     "<" (10..11), "<(" (17..19)  ← match exactement ✓
```

**Contraintes aux bords :**
- Premier token : pas de contrainte sur ce qui **précède** dans le doc (sauf si la query a des chars avant le premier token)
- Dernier token : pas de contrainte sur ce qui **suit** dans le doc (idem)
- Si la query est `"+c++"` : le "+" avant "c" EST contraint, le "++" après aussi

### Distance cumulative fuzzy

Budget global sur la query. Somme des distances Levenshtein de chaque token matché via le niveau fuzzy de la cascade.

```
"helo wrld" → "hello world"
  "helo"  → fuzzy d=1 de "hello"
  "wrld"  → fuzzy d=1 de "world"
  Total : 2 → OK si budget ≥ 2
```

**Coûts par token :**
- Exact match : coût 0
- Fuzzy match : coût = distance Levenshtein (capturée via DFA.distance(state))
- Substring match (regex) : coût 0 (match exact du contenu cherché, juste dans un terme plus long)
  - **Précision bords** : premier token → substring-suffix (fin du terme) = coût 0 ; dernier token → substring-prefix (début du terme) = coût 0

**Coûts par séparateur :**
- Séparateur identique : coût 0
- Séparateur différent : coût = distance d'édition entre séparateur query et séparateur doc
- Séparateur absent vs présent : coût = longueur du séparateur attendu

### Cas "jour-Bidule#machin"

Query tokens : `["jour", "bidule", "machin"]`, séparateurs `["-", "#"]`

Si le doc contient `"bonjour-Bidule#machin"` :
- "jour" → substring de "bonjour" → distance 0, pas de contrainte prefix (premier token)
- "-" entre "bonjour" (offset_to) et "bidule" (offset_from) → match ✓
- "bidule" → exact → distance 0
- "#" → match ✓
- "machin" → exact → distance 0, pas de contrainte suffix (dernier token)
- **Résultat : MATCH, distance totale = 0**

"bonjour" tout seul ne serait PAS trouvé par cette query (il manque "bidule" et "machin" aux positions suivantes avec les bons séparateurs).

---

## Deux options d'implémentation

### Option A : Cascade simple (exact → fuzzy → substring séparés)

C'est le modèle implémenté dans la v1, étendu avec validation séparateurs.

**Cascade par position :**
1. Exact : lookup direct dans le term dict → distance 0
2. Fuzzy : Levenshtein DFA → distance = d (capturée) → early exit si trouvé
3. Substring : regex `.*{escaped}.*` → distance 0 → early exit si trouvé

**Limitation connue** : ne gère pas le "fuzzy substring". Exemple :
- Query "progam" (typo pour "program")
- Terme indexé : "programming"
- Exact "progam" → pas trouvé
- Fuzzy "progam" d=1 → cherche termes à distance ≤1 de "progam" → "programming" est à distance 5 → pas trouvé
- Substring `.*progam.*` → aucun terme ne contient "progam" littéralement → pas trouvé
- **Résultat : MISS** (alors qu'on voudrait trouver "programming" via "program" qui est substring)

**Avantage** : relativement simple, réutilise `AutomatonWeight`, `DfaWrapper`, `RegexPhraseWeight::get_union_from_term_infos`.

**Difficulté** : scorer custom pour séparateurs (charger texte stocké, corréler offsets des postings avec positions matchées).

### Option B : Fuzzy substring (automate combiné)

Résout la limitation de l'option A : un seul automate qui accepte les matches fuzzy ET substring en même temps.

**Principe** : pour chaque token de la query, construire un automate qui :
- Accepte tout préfixe (excess chars au début = gratuits)
- Match le token avec tolérance Levenshtein d
- Accepte tout suffixe (excess chars à la fin = gratuits)

C'est un automate de type "fuzzy infix" : `.*{levenshtein(token, d)}.*`

**Comment calculer la distance** : L'automate Levenshtein standard a des états qui trackent la distance courante. On peut le modifier pour :
1. Phase préfixe : état start, consomme n'importe quel byte sans coût (comme `.*`)
2. Phase match : état Levenshtein classique (distance cumulée)
3. Phase suffixe : une fois le match Levenshtein terminé (état accepting), consomme n'importe quel byte sans coût

La distance du match = la distance Levenshtein à la sortie de la phase 2 (avant la phase suffixe).

**Implémentation possible** :
- Construire un NFA combiné (prefix-free + Levenshtein + suffix-free)
- Le convertir en DFA (ou simuler le NFA à la volée)
- Implémenter `tantivy_fst::Automaton` pour pouvoir walk le FST

**Avantage** : modèle unifié, "progam" trouverait "programming" (fuzzy d=1 de "program" qui est substring de "programming").

**Difficulté** : complexité de l'automate. L'espace d'états explose (prefix NFA × Levenshtein NFA × suffix NFA). Probablement besoin d'un NFA simulation plutôt qu'un DFA complet.

**Alternative plus simple** : utiliser le regex `.*` + un pattern Levenshtein-like. Mais `tantivy_fst::Regex` ne supporte pas le fuzzy. On pourrait :
1. Générer toutes les variantes à distance ≤ d du token
2. Pour chacune, faire un regex `.*{variante}.*`
3. Combiner en un seul automate union
- Problème : pour un token de 6 chars et d=1, ça fait ~150 variantes. Pour d=2, ~10000+. Explosif.

**Alternative réaliste** : pour le walk FST, simuler le NFA directement. Le FST est un automate aussi (DFA). L'intersection de deux automates est calculable. On peut faire NFA(fuzzy_infix) × DFA(FST) et itérer les matches. C'est ce que fait `AutomatonWeight` en interne mais avec un DFA.

---

## Architecture du scorer custom (commune aux deux options)

### Données nécessaires

**Par position (du Weight)** :
```
struct PositionMatch {
    offset: usize,                    // position dans la phrase
    term_infos: Vec<TermInfo>,        // termes matchés
    distances: Vec<u32>,              // distance fuzzy de chaque terme (0 si exact/substring)
    postings: UnionType,              // posting list union
}
```

**Globales (de la Query)** :
```
struct ContainsQueryInfo {
    query_text: String,               // texte original de la query
    token_offsets: Vec<(usize, usize)>, // offsets des tokens dans query_text
    separators: Vec<String>,          // séparateurs entre tokens (len = num_tokens - 1)
    prefix: String,                   // chars avant le premier token
    suffix: String,                   // chars après le dernier token
    distance_budget: u32,             // budget distance cumulative
}
```

**Par document (pendant le scoring)** :
- Texte stocké (chargé via `SegmentReader`)
- Offsets des tokens matchés (via `postings.offsets()` — notre implémentation WithFreqsAndPositionsAndOffsets)

### Flow du scorer

```
Pour chaque document candidat :
  1. Vérifier positions consécutives (comme PhraseScorer)
  2. Pour chaque occurrence de la phrase :
     a. Calculer la distance cumulative fuzzy
        → Si > budget → skip cette occurrence
     b. Extraire les offsets des tokens matchés (via postings)
     c. Charger le texte stocké du document
     d. Extraire les séparateurs réels (texte entre offsets)
     e. Comparer séparateurs réels vs attendus → ajouter distance d'édition au budget
        → prefix : si query a un prefix, distance(prefix_query, prefix_doc) ajoutée
        → séparateurs internes : distance(sep_query, sep_doc) ajoutée
        → suffix : si query a un suffix, distance(suffix_query, suffix_doc) ajoutée
     f. Vérifier bords substring : premier token substring-suffix → coût 0, dernier token substring-prefix → coût 0
     g. Si distance totale ≤ budget → match confirmé
  3. Si aucune occurrence ne passe → skip document
```

### Fichiers à créer/modifier dans izihawa-tantivy

| Fichier | Action | Rôle |
|---------|--------|------|
| `src/query/phrase_query/contains_scorer.rs` | NOUVEAU | Scorer custom (positions + offsets + séparateurs) |
| `src/query/phrase_query/automaton_phrase_weight.rs` | MODIFIER | Utiliser ContainsScorer au lieu de PhraseScorer |
| `src/query/phrase_query/automaton_phrase_query.rs` | MODIFIER | Ajouter query_text, token_offsets, separators |
| `src/query/phrase_query/mod.rs` | MODIFIER | Déclarer contains_scorer |

Pour l'option B, ajouter aussi :
| `src/query/fuzzy_infix_automaton.rs` | NOUVEAU | Automate fuzzy substring |

### Fichiers à modifier dans tantivy_fts

| Fichier | Action | Rôle |
|---------|--------|------|
| `rust/src/query.rs` | MODIFIER | Passer query_text, extraire séparateurs, passer distance_budget |

---

## Accès au texte stocké depuis le scorer

Point critique : le scorer a accès au `SegmentReader` (via `Weight::scorer(&self, reader, boost)`). Le `SegmentReader` permet de charger les champs stockés d'un document via `reader.doc(doc_id)`.

Mais `Postings::offsets()` renvoie les offsets des tokens d'un terme dans le document. Pour corréler offsets et positions :
- `positions()` retourne les positions [p0, p1, p2, ...]
- `offsets()` retourne les offsets [(from0, to0), (from1, to1), ...]
- Les deux listes sont dans le même ordre : `positions[i]` ↔ `offsets[i]`

Pour le scorer, il faut savoir QUEL index dans ces listes correspond à la position qu'on a matchée. C'est faisable : on itère positions() jusqu'à trouver la position cible, et on prend l'offset au même index.

**Important** : les offsets sont par terme dans le posting, pas par union. Dans le `UnionType` (union de posting lists de plusieurs termes), chaque sous-posting a ses propres offsets. Il faudra identifier QUEL terme dans l'union a matché pour le document courant, puis lire ses offsets.

C'est un changement significatif par rapport à PhraseScorer qui ne se soucie pas de quel terme exact a matché — il vérifie juste que les positions sont consécutives.

---

## Résumé des décisions

- ✅ Séparateurs : participent au budget de distance (pas match exact — distance d'édition)
- ✅ Distance cumulative : budget global unifié (tokens fuzzy + séparateurs + prefix/suffix)
- ✅ Bords tokens : premier token substring-suffix = coût 0, dernier token substring-prefix = coût 0
- ✅ Bords query : premier/dernier token sans contrainte sauf si query a des chars avant/après
- ✅ Scorer custom : intégré dans advance(), pas post-filtre
- ⏳ Option A (cascade simple) vs Option B (fuzzy substring) : à décider
  - Option A : plus simple, limitation connue (pas de "fuzzy substring")
  - Option B : plus puissant, complexité automate significative

## Prochaines étapes

1. Choisir Option A ou B (ou A d'abord, B plus tard)
2. Implémenter ContainsScorer avec séparateurs + distance cumulative
3. Résoudre le problème d'accès aux offsets par terme dans l'union
4. Tester avec les cas "c++", "jour-Bidule#machin", "this.I.My" → "thys.Is.MyQueri"
