# 05 - Corrections identifier refs & heritage cross-langage

## Contexte

Suite au doc 04, les 8 tests en echec ont ete corriges. Ce doc resume les bugs trouves, les corrections appliquees, et les pistes restantes.

## Bugs corriges (8 tests : 0/8 → 8/8)

### Bug 1 : `get_property_access_parts` utilisait `named_child()` au lieu de `child()`

**Fichier** : `base_scope_extraction_parser.rs:2022-2035`

En C#, `this` est un noeud **anonyme** (unnamed) dans `member_access_expression`. L'ancienne logique utilisait `node.named_child(0)` pour trouver l'objet — mais comme `this` n'est pas "named", `named_child(0)` retournait `identifier "Initialize"` (la propriete), pas l'objet. La propriete etait donc confondue avec l'objet et skippee.

**Fix** : remplace `named_child(0)` / `named_child(count-1)` par `child(0)` / `child(count-1)` (tous les enfants, pas seulement les "named").

**Langages touches** : C# (`this` anonyme). C++ et Rust ont `this`/`self` comme noeuds named, mais le fix est generique et n'a pas d'effets de bord.

---

### Bug 2 : Types de noeuds member_expression hardcodes

**Fichier** : `base_scope_extraction_parser.rs:2091, 2109, 2019`

Les checks de member access (skip objet, extraction qualifier, validation dans `get_property_access_parts`) hardcodaient `"member_expression"` et `"property_access_expression"` (TS/JS). Les types de noeuds specifiques aux autres langages etaient ignores :
- C++/Rust : `field_expression`
- C# : `member_access_expression`
- Go : `selector_expression`

**Fix** : remplace les checks hardcodes par `self.node_types.member_expression.iter().any(...)`, qui utilise la config specifique au langage.

**Langages touches** : C++, C#, Rust, Go, C — tous sauf TS/JS.

---

### Bug 3 : `field_identifier` non reconnu comme identifiant

**Fichier** : `base_scope_extraction_parser.rs:2075`

La condition d'entree dans le bloc d'extraction d'identifiants ne matchait que `"identifier"` et `"property_identifier"`. En C++/Rust/Go/C, les proprietes de member access sont des `field_identifier`, qui etaient ignores.

**Fix** : ajoute `|| kind == "field_identifier"` a la condition.

**Langages touches** : C++, Rust, Go, C.

---

### Bug 4 : `is_definition_identifier` classait les proprietes de member access comme definitions

**Fichier** : `base_scope_extraction_parser.rs:2157-2162`

L'heuristique "si ce noeud est le champ `name` de son parent, c'est une definition" etait trop large. En C#, `member_access_expression` a un champ `name` qui contient la propriete accedee (ex: `Initialize` dans `this.Initialize()`). Ce n'est PAS une definition, c'est un usage.

**Fix** : skip le check name-field quand le parent est un type member_expression (via `self.node_types.member_expression`).

**Langages touches** : C# principalement. Pourrait affecter Go (`selector_expression`) si son grammar definit un champ `name`.

---

### Bug 5 : `collect_local_symbols` collectait les proprietes de member access

**Fichier** : `base_scope_extraction_parser.rs:1992-1999`

Meme probleme que bug 4 : `collect_local_symbols_visit` parcourait les noeuds avec un champ `name` et les ajoutait aux exclusions. `member_access_expression.name = "Initialize"` etait donc exclu comme "symbole local", bloquant l'extraction de la reference.

**Fix** : skip les types member_expression dans le check name-field.

**Langages touches** : C#, potentiellement Go.

---

### Bug 6 : Qualifier `self` exclu comme parametre

**Fichier** : `base_scope_extraction_parser.rs:2139-2144`

En Rust, `self` est un parametre de methode (ex: `fn parse(&self)`). `build_reference_exclusions` ajoute tous les noms de parametres a l'ensemble d'exclusions, y compris `"self"`. Du coup, quand on trouve `tokenize` avec qualifier `"self"`, le check `if exclude.contains(q)` drop la reference.

**Fix** : ne pas exclure les qualifiers `this`/`self` meme quand ils sont dans l'exclusion set. Ce sont des keywords d'instance, pas des variables utilisateur.

```rust
let is_instance_kw = q == "this"
    || (q == "self" && matches!(self.language, SupportedLanguage::Python | SupportedLanguage::Rust));
if !is_instance_kw && exclude.contains(q) {
    return;
}
```

**Langages touches** : Rust (`self` parametre), potentiellement C++ (`this` implicite — mais `this` n'est pas un parametre en C++).

---

### Bug 7 : `extract_scopes` C++ delegait au base parser pour les noeuds inconnus

**Fichier** : `cpp_scope_extraction_parser.rs:374-376`

Quand un noeud ne matchait aucun pattern C++ (ex: `translation_unit`), le code appelait `self.base.extract_scopes(...)`. Le base parser recursait dans ses enfants avec sa propre methode `extract_scopes`, qui ne connait pas les types C++ (`class_specifier`, `function_definition`, etc.). Les classes C++ etaient extraites par le generique du base, produisant des signatures incorrectes (ex: `"class Circle()"` au lieu de `"class Circle : Shape"`).

**Fix** : remplace `self.base.extract_scopes(...)` par une recursion directe dans les enfants avec `self.extract_scopes(...)`.

**Langages touches** : C++ uniquement. Les autres parsers (C#, Rust, Go) avaient deja cette recursion directe.

> **NOTE** : le parser C (pure C, pas C++) utilise encore `self.base.parse_file()` dans son `parse_file` — meme probleme potentiel.

---

### Bug 8 : Signature de classe C++ sans heritage

**Fichier** : `cpp_scope_extraction_parser.rs:491-498`

La signature de classe C++ ne contenait pas les clauses d'heritage. `detect_relationship_type` utilise la signature pour determiner INHERITS_FROM via le pattern `: BaseClass`, donc l'info etait perdue.

**Fix** : inclut les heritage clauses dans la signature :

```rust
let base_sig = format!("{} {}", if is_struct { "struct" } else { "class" }, name);
if !heritage_clauses.is_empty() {
    let parents = heritage_clauses.iter().flat_map(|c| c.types.iter().cloned()).collect::<Vec<_>>();
    format!("{} : {}", base_sig, parents.join(", "))
} else {
    base_sig
}
```

**Langages touches** : C++ uniquement.

---

## Etat final des tests

```
cargo test --test relationships

running 27 tests
test result: ok. 27 passed; 0 failed; 0 ignored
```

## Resume des fichiers modifies

| Fichier | Modifications |
|---|---|
| `base_scope_extraction_parser.rs` | `get_property_access_parts` (child vs named_child), member_expression config dynamique (3 endroits), `field_identifier` reconnu, `is_definition_identifier` skip member_expr, `collect_local_symbols` skip member_expr, qualifier `this`/`self` preserve |
| `cpp_scope_extraction_parser.rs` | recursion `self.extract_scopes()`, heritage dans signature |

---

## Pistes restantes a propager aux autres langages

### 1. Heritage dans la signature de classe — C# et Go

**Etat actuel** :
- C++ : `"class Circle : Shape"` ✅ (fixe)
- C# : `"class Dog"` au lieu de `"class Dog : Animal"` ❌
- Go : `"type Server struct"` au lieu de `"type Server struct (embeds: Base)"` ❌
- Rust : N/A (utilise `impl Trait for Struct`, pas de heritage dans la struct elle-meme)
- Python : `"class Dog(Animal)"` ✅ (deja en place via la syntaxe Python)

**Impact** : `detect_relationship_type` utilise la signature pour identifier INHERITS_FROM/IMPLEMENTS. Sans l'info d'heritage dans la signature, la detection repose uniquement sur les `heritage_clauses` (ce qui fonctionne deja via `resolve_heritage_relations`). Donc ce n'est pas bloquant pour la detection, mais rend la signature moins informative.

**Recommandation** : ajouter l'heritage dans la signature C# :

```rust
// Dans extract_c_sharp_class, apres format!("{}class {}{}", ...)
if !heritage_clauses.is_empty() {
    let parents = heritage_clauses.iter().flat_map(|c| c.types.iter().cloned()).collect::<Vec<_>>();
    signature = format!("{} : {}", signature, parents.join(", "));
}
```

**Priorite** : basse (pas bloquant, `resolve_heritage_relations` fait deja le boulot).

### 2. Parser C — delegation au base

**Etat actuel** : `c_scope_extraction_parser.rs` utilise `self.base.parse_file()` dans son `parse_file`. Si des noeuds specifiques au C (comme `struct_specifier`) ne sont pas reconnus par le base, ils seront mal extraits.

**Recommandation** : verifier si le parser C a le meme probleme que le C++ avait (delegation au base pour les noeuds inconnus). Si oui, appliquer la meme correction (recursion `self.extract_scopes()`).

**Priorite** : basse (le C n'a pas de methodes dans les structs, donc l'impact est limite).

### 3. C++ methodes — `function_definition` uniquement

**Etat actuel** : dans `extract_scopes`, le C++ n'extrait que les enfants `function_definition` dans `field_declaration_list`. Cela peut manquer :
- Constructeurs/destructeurs si tree-sitter les categorise differemment
- Operators overloades
- Templates de methodes

**Recommandation** : tester avec du code C++ reel incluant constructeurs, destructeurs, et operator overloads. Verifier les node types produits par tree-sitter-cpp.

**Priorite** : moyenne.

### 4. Go receiver methods

**Etat actuel** : Go n'a pas de methodes dans les struct body. Les methodes sont definies separement avec un receiver :

```go
func (s *Server) Start() { ... }
```

Le parser Go ne cree pas de relation PARENT_OF entre `Server` et `Start`. Les methodes Go sont extraites comme des fonctions top-level.

**Recommandation** : lier les methodes Go a leur struct receiver via PARENT_OF.

**Priorite** : moyenne — utile pour la navigation du graphe.

### 5. C `field_expression` — pas teste

**Etat actuel** : le C utilise `field_expression` (meme que C++). Le fix `field_identifier` et le config dynamique de `member_expression` devraient beneficier au C aussi. Mais le parser C n'a pas de methodes, donc `obj.field` ne produit pas de CONSUMES vers un scope (les champs sont des donnees, pas des fonctions).

**Recommandation** : ecrire des tests pour le C (function pointers dans les structs, appels via `callback()`).

**Priorite** : basse.
