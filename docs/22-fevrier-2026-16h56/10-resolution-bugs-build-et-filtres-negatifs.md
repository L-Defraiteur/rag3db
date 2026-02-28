# 10 — Resolution des bugs build/link et filtres negatifs

## Contexte

Suite a l'investigation documentee dans les docs 08 et 09 :
- Le test `TantivyStringFilterFieldTest` echouait (0 resultats au lieu de 1)
- Les debug prints ajoutes dans `intersection.rs` (ld-tantivy) etaient strippes du binaire final
- Le build cmake/cargo avait des problemes de cache et de re-link

## Bug 1 : Linker strip les objets internes de ld-tantivy

### Probleme

`libtantivy_fts.a` contient tous les `.o` de Rust (ld-tantivy + tantivy-fts). Mais le linker n'inclut que les `.o` qui resolvent des symboles non-resolus depuis le C++. Les objets internes a Rust (ex: `intersection.rs` appele par `boolean_weight.rs`) peuvent etre strippes si le compilateur les a inlines en release.

### Fix : `--whole-archive` + suppression du bridge duplique

**Fichier :** `extension/tantivy_fts/CMakeLists.txt`

```cmake
# AVANT
add_library(tantivy_fts_lib INTERFACE)
target_link_libraries(tantivy_fts_lib INTERFACE ${TANTIVY_STATIC_LIB})

# cxx bridge compile separement
add_library(tantivy_fts_extension_bridge OBJECT ${CXX_BRIDGE_CC})
set(TANTIVY_FTS_EXTENSION_OBJECT_FILES $<TARGET_OBJECTS:tantivy_fts_extension_bridge>)

# APRES
add_library(tantivy_fts_lib INTERFACE)
target_link_libraries(tantivy_fts_lib INTERFACE
    -Wl,--whole-archive ${TANTIVY_STATIC_LIB} -Wl,--no-whole-archive)

# Plus de compilation separee de bridge.rs.cc
set(TANTIVY_FTS_EXTENSION_OBJECT_FILES "")
```

**Pourquoi supprimer le bridge OBJECT :**

`--whole-archive` force l'inclusion de TOUS les `.o` du `.a`, y compris `bridge.rs.o` (compile par le `cc` crate pendant `cargo build`). Si on compile aussi `bridge.rs.cc` cote cmake, on a des doublons → erreur "definitions multiples". Puisque le `.a` contient deja le bridge, on n'a plus besoin de le compiler separement.

**Resultat :** L'extension shared (`.rag3db_extension`) linke correctement avec tout le code Rust inclus.

## Bug 2 : Filtres negatifs (ne, not_in, must_not) retournent 0 resultats

### Probleme

Une fois le build corrige, les debug prints de `intersection.rs` sont apparus et ont montre que l'intersection trouvait bien le bon document. Le vrai bug etait dans le **test 6** (pas le test 5) :

```
Test 6 : tag != "programming" AND body contains "guide" → 0 au lieu de 1
```

Le filtre `ne` generait :

```rust
BooleanQuery::new(vec![
    (Occur::MustNot, Box::new(TermQuery("programming"))),
])
```

C'est un probleme classique de Tantivy/Lucene : **un BooleanQuery avec uniquement des clauses MustNot (sans clause positive Must ou Should) ne matche aucun document**. Il n'y a pas de candidats a exclure.

### Fix : AllQuery comme clause positive

**Fichier :** `tantivy_fts/rust/src/query.rs`

3 operateurs corriges :

```rust
// ne
"ne" => {
    let term = json_to_term(field, &field_type, value()?)?;
    let eq_query = TermQuery::new(term, IndexRecordOption::Basic);
    Ok(Box::new(BooleanQuery::new(vec![
        (Occur::Must, Box::new(AllQuery) as Box<dyn Query>),     // ← AJOUTE
        (Occur::MustNot, Box::new(eq_query) as Box<dyn Query>),
    ])))
}

// not_in — meme pattern
let in_query = BooleanQuery::new(inner_clauses);
Ok(Box::new(BooleanQuery::new(vec![
    (Occur::Must, Box::new(AllQuery) as Box<dyn Query>),         // ← AJOUTE
    (Occur::MustNot, Box::new(in_query) as Box<dyn Query>),
])))

// must_not composite — meme pattern
if filter.op == "must_not" {
    let mut full_clauses = vec![(Occur::Must, Box::new(AllQuery) as Box<dyn Query>)];
    full_clauses.extend(clauses);
    return Ok(Box::new(BooleanQuery::new(full_clauses)));
}
```

`AllQuery` matche tous les documents du segment, fournissant la base de candidats que `MustNot` peut ensuite filtrer.

## Nettoyage

Tous les `eprintln!("[DEBUG ...")` temporaires ont ete supprimes de :
- `intersection.rs` (8 lignes)
- `ngram_contains_query.rs` (10+ lignes)
- `query.rs` (3 lignes + 1 variable devenue inutile)

## Resultat final

- **15/15 tests E2E GTest** : PASSED
- **1062 tests Rust unitaires** : PASSED
- Build cmake → cargo → link fonctionne correctement en une seule commande

## Lecon retenue

Quand un BooleanQuery Tantivy/Lucene n'a que des clauses MustNot, il faut toujours ajouter `AllQuery` comme clause Must. C'est un piege classique qui s'applique a tout operateur de negation.
