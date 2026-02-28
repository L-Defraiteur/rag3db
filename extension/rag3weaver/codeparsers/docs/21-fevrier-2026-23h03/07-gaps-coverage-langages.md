# 07 - Gaps de couverture par langage

## Methode

Tests exploratoires ecrits pour chaque langage (dans `tests/relationships.rs`), testant des patterns courants. 49 tests au total, 41 OK, 8 FAIL.

## Bugs trouves (8 tests en echec)

### Bug 1 : C++ — fonctions top-level non extraites (CRITIQUE)

**Tests** : `cpp_toplevel_function_extracted`, `cpp_function_consumes_function`

**Probleme** : `extract_scopes` dans `cpp_scope_extraction_parser.rs` ne gere que :
- `namespace_definition`
- `template_declaration`
- `class_specifier` / `struct_specifier`

Les `function_definition` au top-level (hors classes) tombent dans le fallthrough (recursion enfants) et ne sont jamais extraites comme scopes. C'est le pattern le plus courant en C++ reel (fonctions libres, `main()`, etc.).

**Fix** : ajouter `function_definition` a `extract_scopes`, reutiliser `extract_cpp_method` (qui fait deja `find_function_declarator` + `extract_function_name`) mais avec `ScopeInfoType::Function` au lieu de `Method`.

**Impact** : eleve — tout C++ sans classes est invisible dans le graphe.

---

### Bug 2 : C++ — enums non extraits

**Test** : `cpp_enum_extracted`

**Probleme** : `extract_scopes` ne gere pas `enum_specifier`. Les enums (y compris `enum class`) tombent dans le fallthrough.

**Fix** : ajouter `enum_specifier` a `extract_scopes`. Reutiliser le pattern du C parser (`extract_c_enum_members`), ou ecrire un `extract_cpp_enum` specifique.

**Impact** : moyen — les enums sont des types courants.

---

### Bug 3 : C++ — methodes out-of-class non extraites

**Test** : `cpp_out_of_class_method`

**Probleme** : en C++ reel, les methodes sont souvent definies hors de la classe :
```cpp
void Engine::start() { /* impl */ }
```

Ces `function_definition` avec `qualified_identifier` (contenant `Engine::start`) ne sont pas extraites. Meme apres le fix du bug 1, le nom extrait sera `start` sans lien parent vers `Engine`.

**Fix en 2 etapes** :
1. Bug 1 — extraire les `function_definition` top-level
2. Detecter le `qualified_identifier` dans `function_declarator` → extraire le namespace/classe qualifier → mettre `parent = "Engine"`

**Impact** : eleve — c'est LE pattern C++ standard (.h declarations, .cpp definitions).

---

### Bug 4 : C# — classes imbriquees non extraites

**Test** : `csharp_nested_class_extracted`

**Probleme** : `extract_c_sharp_member_as_scope` (ligne 660) ne gere que `method_declaration` et `constructor_declaration`. Les `class_declaration`, `struct_declaration`, `record_declaration`, `interface_declaration` imbriquees sont ignorees.

**Fix** : ajouter ces node types a `extract_c_sharp_member_as_scope`, en appelant `self.extract_scopes(child, ...)` pour les types connus.

```rust
// Dans extract_c_sharp_member_as_scope, ajouter :
} else if node.kind() == "class_declaration"
    || node.kind() == "struct_declaration"
    || node.kind() == "record_declaration"
    || node.kind() == "interface_declaration"
    || node.kind() == "enum_declaration" {
    self.extract_scopes(node, scopes, content, depth, parent, file_imports, file_path);
}
```

**Impact** : moyen — les classes imbriquees sont courantes (builders, iterators, etc.).

---

### Bug 5 : Go — membres d'interface non extraits

**Test** : `go_interface_with_methods`

**Probleme** : `extract_go_interface_methods` retourne un Vec vide. L'interface `Shape` est extraite avec `members=None` alors qu'elle a `Area()` et `Perimeter()`.

**Cause probable** : `extract_go_interface_methods` itere sur `node.children()` en cherchant `method_spec`, mais le noeud passe est `interface_type` qui a probablement un noeud body intermediaire (comme `method_set` ou similaire dans tree-sitter-go). A verifier avec un dump AST.

**Impact** : moyen — les interfaces Go sont centrales, et sans leurs methodes le graphe manque la signature d'API.

---

### Bug 6 : Python — heritage multiple incomplet

**Test** : `python_multiple_inheritance`

**Probleme** : `class Service(Serializable, Loggable)` produit `INHERITSFROM: Service -> Serializable` mais PAS `INHERITSFROM: Service -> Loggable`.

**Cause probable** : soit le parser Python n'extrait qu'un seul parent dans `heritage_clauses`, soit `resolve_heritage_relations` ne boucle pas sur tous les parents, soit `detect_relationship_type` s'arrete au premier match.

**Impact** : moyen — Python utilise beaucoup les mixins et l'heritage multiple.

---

## Tests qui passent (confirmations positives)

| Test | Langage | Pattern | Statut |
|---|---|---|---|
| `csharp_implements_interface` | C# | `class : IService` → IMPLEMENTS | OK |
| `csharp_class_inherits_and_implements` | C# | `class Duck : Animal, ISwimmable` → INHERITS + IMPLEMENTS | OK |
| `go_method_parent_of_struct` | Go | `func (s *Server) Start()` → PARENTOF Server→Start | OK |
| `rust_derive_not_crash` | Rust | `#[derive(Debug, Clone)]` ne crash pas | OK |
| `rust_enum_with_variants` | Rust | `enum Shape { Circle(f64) }` extrait | OK |
| `python_multiple_inheritance` | Python | 1er parent OK, 2eme manque | PARTIEL |
| `python_decorator_relationship` | Python | `@my_decorator` dans decorators | OK |
| `c_enum_extracted` | C | `enum Status` + `typedef enum Color` | OK |

## Ordre de priorite pour les corrections

1. **C++ function_definition top-level** (bug 1) — bloque aussi bug 3 et 4 C++ tests
2. **C++ enum_specifier** (bug 2) — facile, ~10 lignes
3. **C++ out-of-class methods** (bug 3) — qualified_identifier → parent
4. **C# nested types** (bug 4) — ~5 lignes dans extract_c_sharp_member_as_scope
5. **Python heritage multiple** (bug 6) — investigation parser Python heritage_clauses
6. **Go interface members** (bug 5) — investigation AST tree-sitter-go

## Tests ecrits (dans tests/relationships.rs)

Les tests exploratoires sont deja en place et prets a valider les fixes :
- `cpp_toplevel_function_extracted`
- `cpp_enum_extracted`
- `cpp_out_of_class_method`
- `cpp_function_consumes_function`
- `csharp_nested_class_extracted`
- `csharp_implements_interface` (deja vert)
- `csharp_class_inherits_and_implements` (deja vert)
- `go_method_parent_of_struct` (deja vert)
- `go_interface_with_methods`
- `rust_derive_not_crash` (deja vert)
- `rust_enum_with_variants` (deja vert)
- `python_multiple_inheritance`
- `python_decorator_relationship` (deja vert)
- `c_enum_extracted` (deja vert)

**Etat** : 41 pass, 8 fail sur 49 tests total.
