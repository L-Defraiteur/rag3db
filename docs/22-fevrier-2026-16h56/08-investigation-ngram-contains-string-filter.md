# 08 — Investigation : NgramContainsQuery sur STRING filter fields

## Symptome

Le test E2E `LucivyStringFilterFieldTest` (test #5) echoue : 0 resultats au lieu de 1.

```json
{"type":"contains","field":"body","value":"programming",
 "filters":[{"field":"tag","op":"contains","value":"ystem"}]}
```

Attendu : article 3 (body="C++ is a general-purpose programming language", tag="systems") — le tag "systems" contient "ystem".

Les tests 1-4 (eq, starts_with) du meme test PASSENT. Seul `contains` sur un STRING filter field echoue.

## Test Rust unitaire : PASSE

Un test Rust pur (`handle.rs:test_string_filter_field_contains`) qui reproduit exactement le meme scenario **passe** :
- Meme schema (body=text, tag=string)
- Memes 4 documents
- Meme query JSON
- Resultat : 1 doc trouve

Donc le code Rust (NgramContainsQuery + BooleanQuery intersection) est correct en isolation.

## Ce qu'on sait

### Schema et fields (E2E)

```
field_id 0 : _node_id (u64, auto)
field_id 1 : title (text, stemmed tokenizer)
field_id 2 : title._raw (text, default tokenizer)
field_id 3 : title._ngram (text, ngram tokenizer)
field_id 4 : body (text, stemmed tokenizer, STORED)
field_id 5 : body._raw (text, default tokenizer)
field_id 6 : body._ngram (text, ngram tokenizer)
field_id 7 : tag (string, raw tokenizer, STORED)
field_id 8 : tag._ngram (text, ngram tokenizer)
```

### Construction du query (verifiee par debug prints)

Le `build_filter_clause` pour `op="contains"` appelle `build_contains_query` qui appelle `build_contains_fuzzy` :

1. `resolve_field("tag", raw_pairs, use_raw=true)` → "tag" pas dans raw_pairs (STRING n'a pas de `._raw`) → retourne tag (field_id 7)
2. `resolve_field("tag", raw_pairs, use_raw=false)` → retourne tag (field_id 7) → `stored_field = Some(7)`
3. `tokenize_with_offsets(index, tag, "ystem")` → raw tokenizer, un seul token "ystem"
4. `resolve_ngram_field("tag", ngram_pairs)` → trouve "tag._ngram" (field_id 8)
5. NgramContainsQuery(raw_field=7, ngram_field=8, stored_field=Some(7), trigram_sources=["ystem"])

Le BooleanQuery final :
```
Must: NgramContainsQuery(body._raw=5, body._ngram=6, stored=body=4, "programming")
Must: NgramContainsQuery(tag=7, tag._ngram=8, stored=tag=7, "ystem")
```

### Debug output du scorer (verifie)

Dans le segment contenant l'article 3 :

```
# Body scorer — candidates=[0, 1] dans ce segment
[DEBUG NgramContainsWeight::scorer] raw_field=5 ngram_field=6 stored_field=Some(4)
  trigram_sources=["programming"] final_candidates=[0, 1]

# Tag scorer — candidates=[1]
[DEBUG NgramContainsWeight::scorer] raw_field=7 ngram_field=8 stored_field=Some(7)
  trigram_sources=["ystem"] final_candidates=[1]

# Tag verify doc_id=1 → tf=1 (constructeur)
[DEBUG XYZZY verify] doc_id=1 text_field=7 stored_text="systems"
[DEBUG count_single_token_fuzzy] doc_token="systems" query_token="ystem" → distance=0
[DEBUG verify] doc_id=1 → tf=1

# Tag verify doc_id=1 → tf=1 (seek depuis BooleanQuery)
[DEBUG XYZZY verify] doc_id=1 text_field=7 stored_text="systems"
[DEBUG verify] doc_id=1 → tf=1

# Body verify doc_id=1 → tf=1 (seek depuis BooleanQuery)
[DEBUG XYZZY verify] doc_id=1 text_field=4
  stored_text="C++ is a general-purpose programming language"
[DEBUG verify] doc_id=1 → tf=1
```

**Les deux scorers trouvent et verifient doc_id=1 dans le meme segment avec tf=1.**
Pourtant le resultat final est 0.

### Segments (multi-threaded writer)

4 documents, writer multi-thread → 3 segments :
- Segment A : 2 docs (body candidates=[0,1], tag candidates=[1]) — c'est ici que le match devrait se produire
- Segment B : 1 doc (body candidates=[0], tag candidates=[]) → EmptyScorer pour tag
- Segment C : 1 doc (body candidates=[0], tag candidates=[]) → EmptyScorer pour tag

## Pistes explorees

### 1. NgramContainsQuery — verify logic ✅ ELIMINEE
Le verify fonctionne : tf=1 pour les deux scorers sur le meme doc_id.

### 2. NgramContainsQuery — DocSet (seek/advance) ❓ NON CONCLUSIF
Le code semble correct a la lecture :
- `seek(target)` : avance le cursor, verifie, sinon advance
- `advance()` : incremente cursor, verifie, loop
- `doc()` : retourne candidates[cursor] ou TERMINATED

Mais on n'a **pas pu tracer** le DocSet en action a cause du probleme de build/link (voir doc 09).

### 3. BooleanQuery Intersection ❓ NON CONCLUSIF
Le code `intersection.rs` semble correct aussi :
- `go_to_first_doc()` : seek tous les scorers vers max(docs), loop jusqu'a alignement
- `intersect_scorers()` : trie par cout, aligne, retourne Intersection

Mais les debug prints ajoutees dans `intersection.rs` ne sont PAS apparues dans le binaire a cause du probleme de link statique (voir doc 09).

### 4. Difference entre test Rust et E2E ❓ PISTE PRINCIPALE

Le test Rust passe, le E2E echoue. Les differences :

| Aspect | Test Rust | E2E |
|---|---|---|
| Ajout docs | `writer.add_document(doc)` direct | `add_document_mixed` via cxx bridge |
| Duplication ngram | Manuelle dans le test | `auto_duplicate_field` dans bridge.rs |
| Commit | `writer.commit()` + `reader.reload()` | `commit()` + `reload_reader()` via bridge |
| Collecteur | `TopDocs::with_limit(10).order_by_score()` | `execute_top_docs()` (meme collecteur) |
| Writer threads | Multi-thread (defaut) | Multi-thread (defaut) |

Le path semble identique. Mais il y a un probleme de **linking** qui empeche de voir les debug prints de `ld-lucivy` dans le binaire final (voir doc 09), ce qui bloque l'investigation.

## Prochaines etapes

1. **Resoudre le probleme de link** (doc 09) pour que les debug prints de `intersection.rs` apparaissent
2. **Tracer le DocSet** : une fois les prints visibles, verifier :
   - Est-ce que `go_to_first_doc` aligne bien les deux scorers sur doc_id=1 ?
   - Est-ce que le collecteur TopDocs recoit bien le resultat ?
3. **Comparer le path exact** : ajouter un debug print dans `BooleanWeight::complex_scorer` pour voir combien de Must scorers sont crees et leur type
4. **Hypothese a tester** : le linker pourrait eliminer (dead-strip) les objets de `intersection.rs` et utiliser une version inlinee/optimisee sans les prints. Tester avec `#[inline(never)]` sur `intersect_scorers` et `go_to_first_doc`
