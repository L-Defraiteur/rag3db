# Les appels typés par un champ ou un type de retour

3 octobre 2026, codeparsers `56d039f` (fusionné dans codeparsers, sans
pointeur : la pile 4 → 7 attend chez la session de l'arbre principal, et le
prochain pointeur viendra quand elle sera vidée). Suite de
`03-appels-sur-variable-typee.md` : deux sources de types **déclarés** de
plus, toujours sans inférence.

## Ce qui a été décidé, et pourquoi

| Décision | Pourquoi |
|---|---|
| Un champ : `self.f`, `this.f`, `this->f`, `v.f` avec `v` typé, et le champ implicite de C++ — un niveau, pas plus | Cadrage de l'orchestration ; `v.store.inner.get()` ne donne rien. |
| Le type du champ vient de ses membres déclarés : struct Rust, classes TS et C++ ; pour Python, `x: T` dans la classe et `self.x: T` / `self.x = T()` dans `__init__` | Python ne relevait aucun membre ; ces trois formes sont des déclarations, pas des inférences. |
| Un retour : `let s = g()`, `g().m()`, `g()?` quand `g` se résout **sans ambiguïté** (un seul scope de ce nom) et déclare son retour ; `?` déballe `Result` et `Option` ; `Self` vaut le type de l'impl | Un maillon, pas de chaîne ; deux fonctions `g` ne disent rien. |
| `Arc`, `Box`, `Rc`, `&` se traversent ; `Option<T>`, `Mutex<T>` gardent leur nom | On appelle à travers les premiers les méthodes du type enveloppé ; atteindre le `T` d'un `Mutex` demande `.lock()`, un maillon de plus. Testé, les quatre. |
| Un type qui devait se lire ailleurs et ne s'y lit pas ne se rabat jamais sur le nom — sauf qualificatif qui nomme un type ou un espace de noms (`Foo::bar`) | Sans cette règle, `n = make()` en Python (sans annotation) se reliait au premier `run` du fichier. |
| Le type différé est porté par la référence (`IdentifierReference.qualifier_deferred`) et lu par le résolveur | Le champ ou la fonction est souvent dans un autre fichier. |

**Un défaut corrigé en route.** L'extracteur Rust cherchait un enfant de
*genre* `return_type`, alors que c'est un *champ* : le retour d'une fonction
n'était jamais lu, et sa signature perdait son `-> T`. Les signatures Rust
le regagnent : le texte rendu change, pas les clés de rag3weaver (qui ne
dépendent pas de la signature) ; les uuid de codeparsers, qui en dépendent,
changent — d'où une comparaison par noms ci-dessous.

## Mesure, sur `src/dataflow` (contre `a569417`, par noms)

| | Nombre |
|---|---|
| `CONSUMES` | 6 397 → 6 445 |
| Ajoutées | 51 : 30 par un champ de `self`, 14 par une variable typée par un retour, 7 en chaîne sur plusieurs lignes |
| Retirées | 3 : `.clone()` et un champ qu'un homonyme du projet captait |
| Taux de faux | 25 tirées au hasard, relues ; trois types de champ vérifiés à la source (`services: Arc<ServiceRegistry>`, `taps: TapRegistry`, `spiller: Spiller`) : aucune fausse |
| Qualificatifs typés | 34 % sur place et 10 % à lire ailleurs (borne haute : une partie ne se résout pas), contre 33 % |

## Limites

Pas d'inférence à travers les appels ni les génériques (`ident<T>(x: T)`
ne dit rien) ; deux niveaux de champ ne donnent rien ; une méthode de trait
appelée sur un type qui l'implémente ailleurs se résout seulement si son
`impl` porte le nom du type.
