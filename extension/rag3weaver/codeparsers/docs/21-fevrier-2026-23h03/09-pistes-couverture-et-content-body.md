# 09 - Pistes de couverture par langage + scope.content body-only

## Partie A : Gaps de couverture par langage

Analyse systematique des patterns courants non couverts par nos parsers tree-sitter, classes par langage et par impact.

---

### TypeScript / JavaScript (base parser)

| Pattern | Impact | Description |
|---|---|---|
| Higher-Order Functions & closures | HAUT | `const factory = (x) => (y) => x + y` — la fonction interne et sa relation au scope englobant ne sont pas tracees. Tres courant en React hooks, Redux, patterns fonctionnels. |
| Factory functions & object method returns | HAUT | `function createUser() { return { getName() {...} } }` — les methodes retournees ne sont pas liees a la factory. Pattern ubiquitaire en JS. |
| Conditional types & discriminated unions | MOYEN | `type Flatten<T> = T extends Array<infer U> ? Flatten<U> : T` — types conditionnels pas extraits comme relations. `type Shape = Circle \| Square` ne cree pas de lien union. |
| JSX components comme scopes | MOYEN | Les elements JSX sont ignores ("USAGE not DEFINITIONS"). Les composants fonctionnels retournant du JSX ne sont pas lies aux composants enfants. Props destructuring invisible. Central en React. |
| Decorator chains | MOYEN | `@validate @log class User {}` — la composition de decorateurs n'est pas modelisee comme relation. Courant en NestJS, Angular. |

---

### Python

| Pattern | Impact | Description |
|---|---|---|
| List/dict/set comprehensions | HAUT | `[x for x in items if x.valid]` — en Python 3, les comprehensions creent leur propre scope, non extrait. Meme chose pour les generator expressions. Extremement courant. |
| Metaclasses & class decorators | HAUT | `class User(metaclass=Meta)` — la relation metaclasse pas capturee. `@dataclass`, `@total_ordering` — transformations de classe invisibles. Courant en Django, SQLAlchemy, dataclasses stdlib. |
| Context managers (__enter__/__exit__) | MOYEN | `with context as var:` — le lifecycle du context manager et sa relation au code interne pas captures. Pattern standard pour la gestion de ressources. |
| Descriptor protocol | MOYEN | `__get__`, `__set__`, `__delete__` — methodes speciales pas reconnues comme relations. `@property` lie getter/setter/deleter mais pas au champ sous-jacent. |
| Pattern matching (3.10+) | MOYEN | `match`/`case` — les match statements creent des scopes implicites pas extraits. Adoption croissante. |
| Tuple unpacking dans signatures | MOYEN | `for x, y in items:` — noms destructures creent des scopes implicites. |
| ClassVar & TypeVar bounds | BAS-MOYEN | `T = TypeVar('T', bound=BaseClass)` — les bornes de type pas converties en relations d'heritage. Courant en code type. |

---

### Rust

| Pattern | Impact | Description |
|---|---|---|
| Macros & invocations de macros | HAUT | `#[derive(Clone)]` genere des implementations invisibles. `macro_rules!` definitions et invocations ne creent pas de relations. Procedural macros generent des modules/traits entiers. Les macros sont omnipresentes en Rust (derive, serde, sqlx, tokio, etc.). |
| Trait bounds & associated types | HAUT | `impl<T: Display + Clone> Trait for T {}` — les bornes pas montrees comme relations. `type Item = String;` — types associes pas lies aux requirements du trait. Higher-ranked trait bounds `for<'a>` invisibles. Central au systeme de types Rust. |
| Coherence impl block & orphan rule | MOYEN | `impl MyTrait for ForeignType` — relations de coherence pas capturees. Impl blocks generiques avec specialisation (`impl<T> for T` vs `impl for String`) pas differencies. |
| Closures avec capture lifetime | MOYEN | Captures de closures et bornes de lifetime pas tracees. `move` closures vs borrowing pas distingues dans les relations. Courant en async/await et chaines d'iterateurs. |
| Visibilite module & pub(crate) | BAS-MOYEN | `pub(crate)` et `pub(super)` — les restrictions de visibilite sont extraites mais pas utilisees pour filtrer les relations. Important pour la conception d'API. |

---

### Go

| Pattern | Impact | Description |
|---|---|---|
| Goroutines & channel operations | HAUT | `go func() {}()` — lance des scopes concurrents non traces. `chan <- value` — operations channel pas reconnues comme relations. Fondamental au modele de concurrence de Go. |
| Interface embedding & satisfaction implicite | HAUT | Pas de mot-cle `implements` en Go. La satisfaction d'interface est implicite et pas modelisee. Embedding d'interface (`type ReadWriter interface { Reader; Writer }`) cree des relations de composition. Core au design Go. |
| Defer statement scoping | MOYEN | `defer` — les statements defer creent des relations d'ordre d'execution pas capturees. Pattern standard de gestion d'erreurs. |
| Type assertions & type switches | MOYEN | `value.(Type)` et `switch v.(type)` — creent des relations de type runtime pas liees aux scopes statiques. Courant en code Go polymorphe. |
| Method value vs method expression | BAS | `obj.Method` (method value) vs `Type.Method` (method expression) — pas differencies. Edge case, moins courant. |

---

### C++

| Pattern | Impact | Description |
|---|---|---|
| Template specializations & partial | HAUT | `template<> class Vec<int> {}` — specialisation complete pas liee au template primaire. `template<class T> class Vec<T*> {}` — specialisation partielle invisible. Chaines de specialisation et SFINAE pas capturees. Core au C++ generique et aux bibliotheques (STL, Boost, etc.). |
| Macro expansion & variadic macros | MOYEN | Les invocations de macros ne creent pas de relations scope. `#define FACTORY(T) class T { ... }` — generation de type invisible. `__VA_ARGS__` — expansion variadic pas tracee. Prevalent en code systeme. |
| Virtual inheritance & diamond problem | MOYEN | `class D : virtual Base` — l'heritage virtuel pas distingue de l'heritage normal. Chemins d'heritage multiple pas analyses pour les ambiguites. Courant dans les grandes hierarchies C++. |
| Function pointers & member pointers | MOYEN | `void (*fp)()` — pas de relation vers la fonction cible. `void (Class::*mfp)()` — member function pointers pas lies. Callbacks par function pointers invisibles. Courant en C pour les callbacks et en C++ metaprogramming. |
| Lambdas C++ | MOYEN | `[=](int x) { ... }` — les lambdas C++ ne sont pas extraites comme scopes. Courant en C++ moderne (STL algorithms, async). |
| Const correctness & mutable | BAS | `const` member functions et `mutable` members — n'affectent pas les relations. |

---

### C#

| Pattern | Impact | Description |
|---|---|---|
| LINQ query expressions | HAUT | `from x in items select x.Transform()` — scope de la query et pipeline pas extraits. Method chaining LINQ (`.Where().Select().OrderBy()`) cree des scopes implicites. Central au data processing C# moderne. |
| Extension methods | MOYEN | `public static void Extend(this IFoo obj)` — cree du pseudo-heritage. L'ordre de resolution et le scope pas modelises. Courant pour etendre les types existants. |
| Generic constraints (where clauses) | MOYEN | `where T : IInterface, new()` — les contraintes creent des relations pas capturees. Courant en code generique de bibliotheque. |
| Async/await state machines | MOYEN | `async Task<T> Method()` — cree une state machine avec des scopes caches. Les points `await` creent des relations de control flow pas capturees. Ubiquitaire en code async .NET. |
| Nullable reference types & flow analysis | BAS-MOYEN | `T?` cree des relations de type variante pas capturees. Null-coalescing et flow-dependent type narrowing pas modelises. Croissant avec C# 8.0+. |
| Records & init-only properties | BAS-MOYEN | Patterns d'heritage de records et init-accessors creent des relations speciales. Primary constructor parameters en records (C# 12) pas completement analyses. |

---

### C

| Pattern | Impact | Description |
|---|---|---|
| Macro expansion | MOYEN | Comme C++, les macros C ne creent pas de relations scope. `#define LIST_FOREACH(...)` — patterns d'iteration par macro invisibles. |
| Function pointer callbacks | MOYEN | Les callbacks via function pointers sont partiellement couverts (refs d'identifiants OK) mais sans relation explicite vers la fonction cible. |
| Opaque types & forward declarations | BAS-MOYEN | `typedef struct Foo Foo;` — declarations forward pas liees a la definition. Pattern courant pour l'encapsulation en C. |

---

### Langages supportes mais non audites en detail

Ces langages ont des parsers dans `src/scope_extraction/` mais n'ont pas ete testes par les tests exploratoires du doc 07. Chacun a ses patterns specifiques :

| Langage | Patterns specifiques probablement manquants |
|---|---|
| **Java** | Anonymous inner classes, lambdas & method references, inner/nested classes et leurs scopes speciaux, generics avec bounded wildcards (`? extends Base`) |
| **Kotlin** | Extension functions (pseudo-heritage), scope functions (`let`, `run`, `with`, `apply`, `also`) avec receivers implicites, delegation par `by`, operator overloading |
| **Swift** | Protocol extensions avec contraintes, generic constraints dans extensions (`where`), property observers (`willSet`, `didSet`), operator functions |
| **PHP** | Traits et resolution de conflits (`insteadof`, `as`), traits avec methodes statiques, anonymous classes, callable types et variable functions |
| **Ruby** | Blocks, procs, lambdas et leurs differences de capture de scope, `yield` et block parameters, `define_method` et `method_missing`, metaclass/singleton methods |
| **Scala** | Implicit parameters et conversions, pattern matching et case classes, higher-kinded types, for-comprehensions avec structure monadique |
| **Dart** | Mixins et leur composition, extension functions (similaire Kotlin), async/await et Future/Stream, null-coalescing et late initialization |

---

### Gaps cross-langage

| Pattern | Impact | Description |
|---|---|---|
| Anonymous functions & unnamed scopes | MOYEN | Toutes les langues supportent des fonctions anonymes. Le tracking de relations pour les callbacks sans nom est incomplet. TypeScript extrait "Lambda" mais les relations sont partielles. |
| Generic/template specialization instances | MOYEN | Les instanciations concretes de generiques pas liees aux declarations. La substitution de parametres de type pas modelisee. Affecte : Java generics, C++ templates, Rust generics, TS generics. |
| Type alias chains | BAS-MOYEN | `type A = B` → `type B = C` — les chaines pas deroulees. Les relations de typing structurel (Go, TypeScript) pas capturees. |
| Build-time vs runtime scopes | BAS | Evaluation d'expressions constantes a la compilation pas tracee. CTFE (C++ constexpr, Rust const) pas extrait. |

---

## Partie B : scope.content inclut la signature (duplicat)

### Probleme

Actuellement, `scope.content` contient le texte complet du noeud AST, signature incluse. Exemple pour une fonction :

```typescript
// scope.signature = "function process(x: number): string"
// scope.content =
function process(x: number): string {  // ← signature dupliquee
  const result = x.toString();
  return result;
}
```

En usage RAG, la signature est deja dans `scope.signature`. L'avoir aussi dans `scope.content` est un duplicat qui :
- Gonfle les embeddings inutilement
- Pollue le contexte lors de la generation
- Force le consommateur a deduire lui-meme le body

### Etat actuel par parser

| Parser | Scope type | Comment content est defini | Body dispo ? |
|---|---|---|---|
| **Base (TS/JS)** | Class | 1ere ligne seulement (definition) | Deja OK |
| **Base (TS/JS)** | Function, Method, Interface, Enum, TypeAlias | `get_node_text(Some(node))` = noeud complet | `child_by_field_name("body")` dispo |
| **Python** | Class | 1ere ligne seulement | Deja OK |
| **Python** | Function, Method, Lambda, Variable | `get_node_text(Some(node))` = noeud complet | `child_by_field_name("body")` dispo |
| **Go** | Struct, Interface, Function, Method | `get_node_text(Some(node))` = noeud complet | `child_by_field_name("body")` dispo |
| **C** | Struct, Function | `get_node_text(Some(node))` = noeud complet | body = `compound_statement` enfant |
| **C++** | Class, Struct, Function, Method, Enum | `get_node_text(Some(node))` = noeud complet | body = `compound_statement` ou `field_declaration_list` |
| **C#** | Class, Struct, Method, Interface, Enum | `get_node_text(Some(node))` = noeud complet | body = `declaration_list` ou `block` |
| **Rust** | Struct, Impl, Function, Method, Trait, Enum | `get_node_text(Some(node))` = noeud complet | body = `block` ou `declaration_list` |

### Strategie de fix

Le noeud body est accessible via tree-sitter dans tous les langages. La correction consiste a remplacer :

```rust
let node_content = self.base.get_node_text(Some(node), content);
```

par :

```rust
let node_content = node.child_by_field_name("body")
    .map(|body| self.base.get_node_text(Some(body), content))
    .unwrap_or_else(|| self.base.get_node_text(Some(node), content));
```

Avec fallback sur le noeud complet si pas de body (type alias, forward declarations, etc.).

`content_dedented` est derive de `content` (`self.dedent_content(&node_content)`), donc se corrigera automatiquement.

### Points d'attention

- **Classes TS/Python** : deja OK (1ere ligne seulement) — ne pas toucher
- **Enums** : le body contient les membres, c'est pertinent de le garder comme content
- **Interfaces sans body** : certaines interfaces n'ont que des signatures — le fallback gere ce cas
- **Tests** : aucun test actuel n'asserte sur `scope.content`, donc pas de regression a craindre
- **Impact downstream** : le changement affecte le contenu indexe dans le graphe RAG — necessite potentiellement un re-index

### Fichiers a modifier

1. `base_scope_extraction_parser.rs` — extract_function, extract_method, extract_interface, extract_enum, extract_type_alias
2. `python_scope_extraction_parser.rs` — extract_function, extract_method, extract_lambda, extract_global_variable
3. `go_scope_extraction_parser.rs` — extract_go_function, extract_go_method, extract_go_type
4. `c_scope_extraction_parser.rs` — extract_function, extract_class
5. `cpp_scope_extraction_parser.rs` — extract_cpp_class, extract_cpp_method, extract_cpp_enum, extract_namespace
6. `c_sharp_scope_extraction_parser.rs` — extract_c_sharp_class, extract_c_sharp_method, extract_c_sharp_constructor, extract_c_sharp_interface, extract_c_sharp_enum
7. `rust_scope_extraction_parser.rs` — extract_rust_struct, extract_rust_impl, extract_rust_function, extract_rust_method, extract_rust_trait, extract_rust_enum
