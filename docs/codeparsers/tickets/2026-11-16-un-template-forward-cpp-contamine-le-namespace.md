# Un template forward C++ contamine les paramètres du namespace

- **État** : ouvert ; anomalie observée dans la baseline, aucun correctif ni reparse lancé.
- **Pour** : Codeparsers, après fin de baseline ; Banc conserve le jugement indépendant.
- **Famille** : produit ; attributs de scope incorrects malgré AST valide.
- **Parent** : docs/codeparsers/tickets/2026-11-16-verifier-la-couverture-sur-des-corpus-publics-multilangages.md.

Source réelle du gel02 : grpc e67d793e4c19c3715fa3370d760280cdf3943903,
`include/grpc/event_engine/internal/slice_cast.h`, SHA256
`8cb2d8dfd38e13c33d233c11da64b88c8952be8aa440465dd67b03fe7e338408`.
Le namespace internal est déclaré ligne20, puis le template forward
`template <typename Result, typename T> struct SliceCastable;` aux lignes29–30.
Un namespace n'est pas un template et ne porte pas ces paramètres.

Baseline réelle, ELF bff1cafb72e8c6662d2d595457b0ca6da4718686ae64b91ae1bde7a4e95565e2,
`language-corpus-02b/campaign/results.jsonl` : completed, AST valid,
compact_inventory_omitted=0. Le scope Namespace internal, parent experimental,
span lignes20–75/octets743–3317, porte generic_parameters=[Result,T]. C'est un
attribut erroné effectivement conservé, pas une absence due au format compact.
Les trois fonctions SliceCast surchargées sont bien séparées et correctement
parentées ; ne pas les renommer pour corriger ce défaut.

Source relue de l'extracteur C++ : la branche template_declaration appelle
extract_scopes sur inner_decl puis modifie scopes.last_mut sans vérifier qu'un
scope a été ajouté. Une déclaration forward est intentionnellement sans scope
de définition (body requis), laissant le namespace comme dernier élément.
L'absence d'un scope pour le forward lui-même n'est donc pas qualifiée comme
régression par ce ticket ; c'est la contamination du namespace qui est confirmée.

Témoin minimal à écrire et voir rouge, **pas encore joué** :

```cpp
namespace N {
  template<class T> struct F;
}
```

Attendu : Namespace N n'a pas de generic_parameters. Ajouter aussi le cas où un
autre scope précède le forward pour garantir que le défaut n'est pas seulement
un nom de namespace. Puis un template réellement défini garde ses paramètres.
Aucun correctif de parseur avant END de la baseline. Le parent et le gel02 ne
sont pas modifiés ; parsergreen n'est pas une preuve de couverture sémantique.

Preuve figée Banc :
`~/.cache/rag3db-banc-notes/public-corpus-20261116/oracle-review-01/sample-results.jsonl`,
SHA256 b701cd4ebf9c8b2444328dacfcbedf90fa6176129dbee354a712fe5bd8e661f5,
aux deux postes. Capture d'un préfixe en cours, pas verdict global de campagne.

## Relu le 18 novembre (session codeparsers)

Confirmé par lecture de codeparsers bd13385 : la branche `template_declaration` (`cpp_scope_extraction_parser.rs:436-447`) écrit sur `scopes.last_mut()` sans vérifier qu'un scope vient d'être ajouté. Atteint rag3weaver par `template_params` (`code.rs:1665`). Témoin non joué.

Le parent `2026-11-16-verifier-la-couverture-sur-des-corpus-publics-multilangages.md`, le gel et les jugements restent sur la branche `cx-banc-corpus-public-20261116` ; le banc décide s'il les pose.
