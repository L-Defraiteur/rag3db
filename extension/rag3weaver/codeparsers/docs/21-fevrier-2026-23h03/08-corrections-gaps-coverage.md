# 08 - Corrections des gaps de couverture par langage

## Contexte

Suite au doc 07, 6 bugs avaient ete identifies par les tests exploratoires (8 tests en echec sur 49). Ce doc resume les 6 corrections appliquees.

## Corrections appliquees (6 bugs, 8 tests fixes)

### Bug 1 : C++ — fonctions top-level + out-of-class methods

**Fichier** : `cpp_scope_extraction_parser.rs`

**Probleme** : `extract_scopes` ne gerait que `namespace_definition`, `template_declaration`, et `class_specifier`/`struct_specifier`. Les `function_definition` au top-level (hors classes) tombaient dans le fallthrough (recursion enfants) et n'etaient jamais extraites.

**Fix** : ajout d'un handler `function_definition` avant le fallthrough dans `extract_scopes` :
- Extrait le nom via `find_function_declarator` + `extract_function_name`
- Detecte les methodes out-of-class via `qualified_identifier` dans le `function_declarator` (ex: `Engine::start` → parent = `"Engine"`)
- Reutilise `extract_cpp_method` puis override `type = Function` et `name`

**Tests corriges** : `cpp_toplevel_function_extracted`, `cpp_out_of_class_method`, `cpp_function_consumes_function`

---

### Bug 2 : C++ — enums non extraits

**Fichier** : `cpp_scope_extraction_parser.rs`

**Probleme** : `extract_scopes` ne gerait pas `enum_specifier`. Les enums (y compris `enum class`) etaient invisibles.

**Fix** : ajout d'un handler `enum_specifier` + nouvelle methode `extract_cpp_enum` qui :
- Detecte `enum` vs `enum class`
- Extrait les membres depuis `enumerator_list` → `enumerator` → `identifier`
- Construit la signature (`"enum Color"` ou `"enum class Direction"`)
- Produit un `ScopeInfo` avec `type = Enum` et `enum_members = Vec<EnumMemberInfo>`

**Import ajoute** : `EnumMemberInfo`

**Test corrige** : `cpp_enum_extracted`

---

### Bug 3 : C++ — out-of-class methods (qualified_identifier → parent)

**Fichier** : `cpp_scope_extraction_parser.rs` (meme handler que bug 1)

**Probleme** : `void Engine::start() {}` au top-level etait extrait mais sans parent. Le `qualified_identifier` (contenant `Engine::start`) n'etait pas analyse pour en extraire le qualifier.

**Fix** : dans le handler `function_definition`, detection du `qualified_identifier` dans `function_declarator`. Si >= 2 identifiers, le premier est le qualifier (nom de la classe), passe comme `parent` au scope.

**Test corrige** : `cpp_out_of_class_method` (verifie `parent = Some("Engine")`)

---

### Bug 4 : C# — classes imbriquees non extraites

**Fichier** : `c_sharp_scope_extraction_parser.rs`

**Probleme** : `extract_c_sharp_member_as_scope` ne gerait que `method_declaration` et `constructor_declaration`. Les types imbriques (classes, structs, interfaces, enums) dans le body d'une classe etaient ignores.

**Fix** : ajout d'une branche pour `class_declaration`, `struct_declaration`, `record_declaration`, `interface_declaration`, `enum_declaration` qui delegue a `self.extract_scopes(...)` pour une extraction recursive complete.

**Test corrige** : `csharp_nested_class_extracted`

---

### Bug 5 : Go — membres d'interface non extraits

**Fichier** : `go_scope_extraction_parser.rs`

**Probleme** : `extract_go_interface_methods` cherchait des noeuds `method_spec`, mais tree-sitter-go utilise `method_elem`.

**Cause** : dump AST confirme que `interface_type` contient `method_elem` (pas `method_spec`) avec `field_identifier`, `parameter_list`, et `type_identifier`.

**Fix** : ajout de `|| child.kind() == "method_elem"` dans la condition de filtrage.

**Test corrige** : `go_interface_with_methods`

---

### Bug 6 : Python — heritage multiple incomplet

**Fichier** : `python_scope_extraction_parser.rs`

**Probleme** : `class Service(Serializable, Loggable)` ne produisait INHERITSFROM que pour `Serializable` (le premier parent). La cause etait double :
1. `heritage_clauses` n'etait **jamais peuple** (`heritage_clauses: None` dans `extract_class`)
2. `detect_relationship_type` utilisait `sig.contains("(Loggable")` qui echouait pour les parents apres la virgule

**Fix** : peuplement de `heritage_clauses` dans `extract_class` :
- Extraction des `identifier` et `attribute` enfants du noeud `superclasses` (= `argument_list` en tree-sitter Python)
- Creation d'un `HeritageClause { clause: Extends, types: [...] }` avec tous les parents
- `resolve_heritage_relations` (existant) cree maintenant les relations pour chaque parent

**Imports ajoutes** : `HeritageClause`, `HeritageClauseClause`

**Test corrige** : `python_multiple_inheritance`

---

## Etat final des tests

```
cargo test --tests

running 49 tests
test result: ok. 49 passed; 0 failed; 0 ignored
```

Progression : 41 pass / 8 fail → **49 pass / 0 fail**

## Resume des fichiers modifies

| Fichier | Modifications |
|---|---|
| `cpp_scope_extraction_parser.rs` | `extract_scopes` : handlers `function_definition` + `enum_specifier` ; `extract_cpp_enum` ; import `EnumMemberInfo` |
| `c_sharp_scope_extraction_parser.rs` | `extract_c_sharp_member_as_scope` : delegation nested types → `extract_scopes` |
| `go_scope_extraction_parser.rs` | `extract_go_interface_methods` : ajout `method_elem` |
| `python_scope_extraction_parser.rs` | `extract_class` : peuplement `heritage_clauses` ; imports `HeritageClause`, `HeritageClauseClause` |
