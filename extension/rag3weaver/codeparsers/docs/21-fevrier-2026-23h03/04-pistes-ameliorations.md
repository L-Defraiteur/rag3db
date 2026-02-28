# 04 - Pistes d'ameliorations revelees par les tests

## Contexte

27 tests d'integration ecrits dans `tests/relationships.rs`, couvrant tous les langages supportes (TS, Python, Rust, C++, C#, Go). 19 passent, 8 echouent. Les echecs revelent des lacunes dans les scope extractors non-TS.

## Tests qui passent (19)

Les fix de cette session sont valides :
- `ts_this_method_call_produces_consumes` — `this.method()` en TS/JS → CONSUMES ✅
- `ts_this_method_call_cross_file_inheritance` — `this.helper()` resolu cross-fichier vers parent ✅
- `ts_class_extends_is_inherits_from` — `extends` → INHERITS_FROM ✅
- `ts_const_type_annotation_not_inherits` — regression: `const x: Type` n'est plus INHERITS_FROM ✅
- `ts_self_alias_not_treated_as_this` — `const self = obj; self.method()` ne resout pas vers la classe ✅
- `ts_import_cross_file_consumes` — import + appel → CONSUMES ✅
- `ts_spread_set_is_consumes_not_inherits` — `new Set([...BASE])` n'est plus INHERITS_FROM ✅
- `cpp_colon_inheritance_not_on_ts_files` — regex C++ ne se declenche plus sur .ts ✅
- `rust_impl_trait_not_detected_on_non_rs_files` — regex Rust ne se declenche plus sur .ts ✅
- `python_self_method_call_produces_consumes` — `self.method()` en Python → CONSUMES ✅
- `python_class_inheritance` — `class Dog(Animal)` → INHERITS_FROM ✅
- `python_self_not_treated_as_variable` — `self` toujours keyword en Python ✅
- `go_function_call_consumes` — appel de fonction Go → CONSUMES ✅
- `parent_of_and_has_parent_for_class_methods` — structure TS OK ✅
- `defined_in_for_every_scope` — chaque scope a DEFINED_IN ✅
- `empty_file_produces_no_crash` — pas de panic sur fichier vide ✅
- `no_self_reference` — recursion ne produit pas self-CONSUMES ✅
- `multiple_files_same_function_name` — pas de fausse relation cross-file ✅
- `python_class_parens_not_detected_on_non_py_files` — guard langage OK ✅

## Echecs et pistes d'amelioration (8)

### Piste 1 : Heritage clauses TS non peuplees

**Test** : `ts_class_implements_is_implements`

```typescript
interface Drawable { draw(): void; }
class Circle implements Drawable { draw() {} }
```

**Constat** : les scopes Circle et Drawable sont extraits, mais aucun IMPLEMENTS n'est produit. Le `detect_relationship_type` check les heritage_clauses (structurees) et le keyword `implements` dans la signature. Ni l'un ni l'autre ne semble etre present.

**Cause probable** : `BaseScopeExtractionParser` n'extrait pas les heritage clauses pour les classes TS. La signature generee ne contient pas `implements Drawable`.

**Fix** : verifier `extract_heritage_clauses` dans `base_scope_extraction_parser.rs` et s'assurer que la signature des classes inclut les clauses implements/extends.

**Priorite** : haute — affecte toutes les relations d'implementation TS.

---

### Piste 2 : Scope extraction Rust incomplete

**Tests** : `rust_self_method_call_produces_consumes`, `rust_impl_trait_for_is_implements`

```rust
impl Parser {
    fn parse(&self) { self.tokenize(); }
    fn tokenize(&self) -> Vec<String> { vec![] }
}
```

**Constat** : les scopes `parse`, `tokenize`, `validate` existent mais :
- Pas de PARENT_OF de `Parser` vers ses methodes
- Pas d'identifier_references extraites (donc pas de CONSUMES)
- `impl Drawable for Circle` ne produit pas IMPLEMENTS

**Cause probable** : `RustScopeExtractionParser` n'extrait pas les methodes `impl` comme enfants de la struct, et `extract_identifier_references` ne parse pas les appels `self.method()` dans du code Rust.

**Fix** :
1. Extraire les methodes d'un bloc `impl Struct { fn ... }` comme enfants du scope Struct
2. Extraire les identifier_references dans le corps des methodes Rust
3. Reconnaitre `impl Trait for Struct` pour les heritage clauses (IMPLEMENTS)

**Priorite** : moyenne — necessaire si on veut parser des projets Rust.

---

### Piste 3 : C++ scope extraction — methodes = AnonymousFunction

**Tests** : `cpp_class_colon_inheritance`, `cpp_this_arrow_method_call`

```cpp
class Circle : public Shape {
public:
    void draw() override { ... }
};
```

**Constat** :
- Les methodes C++ sont extraites comme `AnonymousFunction` au lieu de methodes nommees
- `class Circle : public Shape` → CONSUMES au lieu de INHERITS_FROM (le guard de langage marche, mais la signature ne contient pas le pattern `: public`)
- `this->initialize()` ne produit pas de CONSUMES (pas d'identifier_references)

**Cause probable** : `CppScopeExtractionParser` ne parse pas correctement les declarations de methodes dans les classes (function_definition dans class_specifier). La signature de la classe n'inclut pas la clause d'heritage.

**Fix** :
1. Extraire les methodes de classe C++ avec leur vrai nom (pas AnonymousFunction)
2. Inclure `: public Shape` dans la signature de la classe
3. Extraire les identifier_references pour `this->method()` (arrow member expression)

**Priorite** : moyenne.

---

### Piste 4 : C# scope extraction — pas d'identifier_references

**Tests** : `csharp_class_colon_inheritance`, `csharp_this_method_call`

```csharp
class Dog : Animal { public void Bark() {} }
class Service { public void Start() { this.Initialize(); } }
```

**Constat** :
- Les scopes et la hierarchie PARENT_OF sont corrects
- `class Dog : Animal` → CONSUMES au lieu de INHERITS_FROM (signature sans `: Animal`)
- `this.Initialize()` ne produit pas de CONSUMES (pas d'identifier_references)

**Cause probable** : meme probleme que C++ — la signature ne contient pas la clause d'heritage, et les identifier_references ne sont pas extraites.

**Fix** :
1. Inclure `: Animal` dans la signature de la classe C#
2. Extraire les identifier_references dans les corps de methodes C#

**Priorite** : moyenne.

---

### Piste 5 : Go scope extraction — structs = AnonymousClass

**Test** : `go_struct_embedding_is_inherits`

```go
type Base struct { Name string }
type Server struct { Base; Port int }
```

**Constat** : les structs Go sont extraites comme `AnonymousClass` sans nom. L'embedding `Base` dans `Server` n'est pas detecte.

**Cause probable** : `GoScopeExtractionParser` ne reconnait pas `type X struct { ... }` comme une classe nommee X. Les type declarations Go ne sont pas mappees sur les node types corrects.

**Fix** :
1. Reconnaitre `type_declaration` → `type_spec` → `struct_type` comme une classe nommee
2. Detecter l'embedding (champ sans nom explicite) pour produire heritage_clauses

**Priorite** : moyenne.

## Resume par langage

| Langage | Scopes | Identifier refs | Heritage/Inheritance | Etat |
|---|---|---|---|---|
| TypeScript/JS | ✅ | ✅ | ⚠️ heritage_clauses | Quasi-complet |
| Python | ✅ | ✅ | ✅ class(Parent) | Complet |
| Rust | ⚠️ pas de parent | ❌ | ❌ impl Trait for | A completer |
| C++ | ⚠️ AnonymousFunction | ❌ | ❌ signature sans : | A completer |
| C# | ✅ | ❌ | ❌ signature sans : | A completer |
| Go | ❌ AnonymousClass | ❌ | ❌ embedding | A completer |

## Ordre recommande

1. **TS heritage_clauses** — impact le plus large, probablement un fix simple (signature ou extraction)
2. **C# identifier_references + signature** — le plus proche de fonctionner (scopes OK)
3. **C++ methodes nommees + signature** — gros impact vu la popularite du langage
4. **Go nommage structs + embedding** — specifique Go
5. **Rust scope hierarchy + identifier_references** — utile mais Python/TS sont les cibles principales

## Fichier de tests

`tests/relationships.rs` — 27 tests, a executer avec `cargo test --test relationships`. Les 8 echecs sont des TODOs documentes ici.
