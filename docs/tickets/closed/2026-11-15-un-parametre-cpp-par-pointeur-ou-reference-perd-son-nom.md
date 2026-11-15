# Un paramètre C++ par pointeur ou par référence perd son nom

- **État** : corrigé (15 novembre 2026) — codeparsers ff82d22 (`parametres-cpp`) : le nom par le déclarateur, même imbriqué ; le type écrit en entier ; témoin `tests/parametres_cpp.rs`, rouge sur le code d'avant
- **Famille** : produit
- **Gravité** : réponse fausse. La signature d'une fonction C++ qui a un paramètre `T* p`, `T& r` ou `T&& r`
  est stockée sans lui (`void JsonMutWrapper()` pour `explicit JsonMutWrapper(yyjson_mut_doc* ptr)`), et le
  nom du paramètre n'est pas écarté des références : `ptr` dans le corps passe pour une référence.
- **Atteignable en service** : oui, sur toute fonction C++ dont un paramètre est un pointeur ou une
  référence — l'essentiel du C++
- **Touche rag3weaver** : oui, par codeparsers (la signature stockée, les références des scopes C++)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, en lisant la cause des scopes sans `DEFINED_IN` (les quatre
  constructeurs de `JsonMutWrapper` avaient la même signature reconstruite)

## Ce que c'est

`CppScopeExtractionParser::extract_c_parameters` ne cherche le nom d'un paramètre que parmi les **enfants
directs** du `parameter_declaration` (`identifier`). Un paramètre par pointeur ou par référence porte son
nom dans un nœud imbriqué (`pointer_declarator`, `reference_declarator` → `identifier`) : il n'est pas
relevé, et le paramètre entier disparaît de la liste. Le type n'est cherché que parmi `primitive_type` et
`type_identifier` (un `qualified_identifier`, un `template_type` sont perdus de même). Et
`build_c_signature` écrit `void` quand le type de retour manque, ce qui arrive pour un constructeur.

## Pourquoi un lot à part

La liste des paramètres sert à la signature (stockée, et champ de version d'un scope) et aux exclusions de
références. La corriger change la signature et les références de l'essentiel des fonctions C++ : les
scopes changent de version (l'écrivain des générations les repasse), et des références tombent.

## Ce qu'il faudrait

Le nom par le déclarateur imbriqué, le type écrit en entier (`yyjson_mut_doc*`, `const std::string&`), pas
de `void` inventé pour un constructeur ou un destructeur ; un témoin par forme ; puis le banc : références
et signatures changées sur rag3weaver et sur une copie de code-lourd, ED prévenu du repassage.

Le défaut d'identité qu'il causait (des homonymes de même signature reconstruite, un seul uuid) est corrigé
à part : l'uuid d'un scope prend son rang d'apparition à partir du second homonyme.

## La correction

Le nom d'un paramètre se cherche par son déclarateur, même imbriqué (`pointer_declarator`,
`reference_declarator`) ; le type est le texte écrit avant le nom (`Foo*`, `const Foo&`).

**Le banc**, sur le C++ du moteur (`src/`, 1 550 fichiers), avant → après : scopes 19 462 → 19 462 ;
paramètres relevés 6 998 → 16 934 (×2,4) ; références 183 698 → 162 181 (−11,7 %) ; signatures changées :
7 164 scopes sur 19 462 (37 %). **Sur les 22 213 références perdues, 22 177 sont le nom d'un paramètre du même
scope** (la perte voulue) ; les 36 autres sont des accès à un membre à travers un paramètre de lambda ou une
variable de boucle désormais reconnue (`partition.hashTable`, `elem.first`), traités comme le membre de toute
variable locale. Aucune variable du corps, aucun membre homonyme d'un paramètre ne part. 696 références sont
gagnées : les types des paramètres par pointeur ou par référence (`ValueVector` 44, `LogicalType` 37…).

Les scopes dont la signature change changent de version une fois, au repassage.
