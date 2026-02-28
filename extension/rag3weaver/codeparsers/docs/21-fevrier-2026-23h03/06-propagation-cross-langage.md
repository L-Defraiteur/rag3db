# 06 - Propagation cross-langage des corrections

## Contexte

Suite au doc 05, les 5 pistes d'amelioration ont ete analysees et 4 implementees. Ce doc resume les changements appliques.

## Corrections appliquees (4 pistes sur 5)

### Piste 1a : Heritage dans la signature C# (classes + interfaces)

**Fichier** : `c_sharp_scope_extraction_parser.rs`

**Probleme** : la signature de classe C# etait `"class Dog"` au lieu de `"class Dog : Animal"`. Meme probleme pour les interfaces (`"interface IDerived"` au lieu de `"interface IDerived : IBase"`).

**Fix** : dans `extract_c_sharp_class` et `extract_c_sharp_interface`, inclure les heritage_clauses dans la signature :

```rust
let signature = {
    let base_sig = format!("{}class {}{}", mod_str, name, generic_str);
    if !heritage_clauses.is_empty() {
        let parents: Vec<String> = heritage_clauses.iter()
            .flat_map(|c| c.types.iter().cloned()).collect();
        format!("{} : {}", base_sig, parents.join(", "))
    } else {
        base_sig
    }
};
```

**Note** : `extract_c_sharp_struct` delegue a `extract_c_sharp_class` puis remplace "class" par "struct" — le fix se propage automatiquement.

**Tests ajoutes** :
- `csharp_class_signature_includes_heritage` — verifie `": Animal"` dans la signature
- `csharp_interface_signature_includes_heritage` — verifie `": IBase"` dans la signature

---

### Piste 1b : Heritage dans la signature Go (structs avec embeds)

**Fichier** : `go_scope_extraction_parser.rs`

**Probleme** : la signature de struct Go etait `"type Server struct"` au lieu de `"type Server struct (embeds: Base)"`.

**Fix** : apres le calcul des heritage_clauses (champs embedded), ajouter les embeds a la signature :

```rust
if let Some(ref clauses) = heritage_clauses {
    let parents: Vec<String> = clauses.iter()
        .flat_map(|c| c.types.iter().cloned()).collect();
    if !parents.is_empty() {
        signature = format!("{} (embeds: {})", signature, parents.join(", "));
    }
}
```

**Tests ajoutes** :
- `go_struct_signature_includes_embeds` — verifie `"embeds: Base"` dans la signature

---

### Piste 2 : Parser C — delegation au base corrigee

**Fichier** : `c_scope_extraction_parser.rs`

**Probleme** : `parse_file()` delegait entierement a `self.base.parse_file()`. Les methodes d'extraction specifiques au C (`extract_function`, `extract_class`, `extract_enum`, `extract_type_alias`) n'etaient **jamais appelees**. Le base parser utilisait ses propres methodes generiques qui ne savent pas extraire correctement les constructs C (ex: `function_declarator` nested dans `pointer_declarator`).

**Fix** : remplacement de la delegation par une implementation propre :
- `parse_file()` — cree le tree-sitter parser pour C, appelle `self.extract_scopes()`, puis les etapes habituelles (classify, attach_signature_references)
- `extract_scopes()` — nouveau, route les noeuds vers les methodes C-specifiques :
  - `function_definition` → `self.extract_function()`
  - `struct_specifier` → `self.extract_class()`
  - `enum_specifier` → `self.extract_enum()`
  - `type_definition` → `self.extract_type_alias()`
  - Autres → recursion dans les enfants

**Tests ajoutes** :
- `c_struct_extracted_with_correct_signature` — verifie `"struct Point"` et `"move_point"` dans les signatures
- `c_function_call_consumes` — verifie que `compute` → `add` produit CONSUMES

---

### Piste 3 : C++ destructeurs et operator overloads — `extract_function_name` corrige

**Fichier** : `cpp_scope_extraction_parser.rs`

**Probleme** : `extract_function_name` ne reconnaissait que `identifier`, `qualified_identifier`, et `field_identifier` dans un `function_declarator`. En C++, tree-sitter produit :
- Destructeurs : `function_declarator` → `destructor_name` → `identifier "Widget"` (pas un `identifier` direct)
- Operators : `function_declarator` → `operator_name "operator=="` (pas un `identifier`)

**Resultat** : destructeurs et operators etaient nommes `"AnonymousMethod"`.

**Analyse AST** : tous les membres de classe (constructeurs, destructeurs, operators, methodes) sont des `function_definition` dans tree-sitter-cpp. Le code existant les extrait deja correctement via `if child.kind() == "function_definition"`. Seul le nommage etait casse.

**Fix** : ajouter la reconnaissance de `destructor_name` et `operator_name` dans `extract_function_name` :

```rust
// Handle destructor_name: ~Widget → "~Widget"
let mut cursor = declarator.walk();
for child in declarator.children(&mut cursor) {
    if child.kind() == "destructor_name" {
        return Some(self.base.get_node_text(Some(child), content));
    }
}

// Handle operator_name: operator== → "operator=="
let mut cursor = declarator.walk();
for child in declarator.children(&mut cursor) {
    if child.kind() == "operator_name" {
        return Some(self.base.get_node_text(Some(child), content));
    }
}
```

**Tests ajoutes** :
- `cpp_constructor_destructor_operator_extracted` — verifie que constructeur (name="Widget"), destructeur (name="~Widget"), operator (name="operator=="), et methode (name="draw") sont tous extraits avec parent="Widget"

---

### Piste 5 : C function pointers — teste et fonctionnel

**Analyse** : le C parser (apres la correction de la piste 2) extrait correctement les typedef struct contenant des function pointers. `vt->draw(obj)` dans une fonction C produit une reference via `field_expression` (gere par le base parser depuis les fixes du doc 05).

CONSUMES ne se resout pas vers le champ function pointer (car c'est un champ de struct, pas un scope), mais les references d'identifiant et l'extraction des scopes fonctionnent correctement.

**Tests ajoutes** :
- `c_function_pointer_struct_call` — verifie que `render()` et `Vtable` typedef sont extraits

---

## Etat final des tests

```
cargo test --test relationships

running 34 tests
test result: ok. 34 passed; 0 failed; 0 ignored
```

## Resume des fichiers modifies

| Fichier | Modifications |
|---|---|
| `c_sharp_scope_extraction_parser.rs` | Heritage dans signature classes + interfaces |
| `go_scope_extraction_parser.rs` | Heritage (embeds) dans signature structs |
| `c_scope_extraction_parser.rs` | `parse_file()` + `extract_scopes()` — delegation au base eliminee |
| `cpp_scope_extraction_parser.rs` | `extract_function_name` reconnait `destructor_name` et `operator_name` |
| `tests/relationships.rs` | 7 nouveaux tests (34 total) |

## Resume des signatures avant/apres

| Langage | Avant | Apres |
|---|---|---|
| C# class | `"class Dog"` | `"class Dog : Animal"` |
| C# interface | `"interface IDerived"` | `"interface IDerived : IBase"` |
| Go struct | `"type Server struct"` | `"type Server struct (embeds: Base)"` |
| C++ destructeur | `"void AnonymousMethod()"` | `"void ~Widget()"` |
| C++ operator | `"bool AnonymousMethod()"` | `"bool operator==()"` |
