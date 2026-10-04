# `analyze()` avec une racine relative rend zéro scope, en silence

- **État** : ouvert.
- **Gravité** : réponse fausse (un corpus vide qui passe pour un corpus analysé).
- **Atteignable en service** : oui — tout appelant de `rag3weaver::code::analyze`
  qui passe une racine relative (`.`, `../depot`), y compris par un backend
  dont le manifeste porterait un chemin relatif.
- **Touche rag3weaver** : oui, c'est son API d'analyse.

## Ce que c'est

`rag3weaver::code::analyze(root, sources)` avec une racine **relative** rend
une analyse à zéro scope — sans erreur, sans entrée dans `skipped`, le
fichier compté dans `files` : tout a l'air d'avoir marché sur un corpus vide.

## La recette minimale

Pas de Cypher — API Rust :

```rust
let a = rag3weaver::code::analyze(".", vec![("src/demo.rs".into(), "fn f() {}".into())]);
assert_eq!(a.files.len(), 1);      // le fichier est là
assert_eq!(a.scopes.len(), 0);     // mais rien n'en est sorti
assert!(a.skipped.is_empty());     // et rien ne le dit
// Avec "/depot-imaginaire" comme racine : 1 scope + file_scope, comme attendu.
```

## Le témoin

`tests/e2e_titre_indexe.rs` (4 octobre 2026) l'a rencontré : le dépôt en
mémoire du test utilisait la racine `"."` et la recherche rendait zéro —
corrigé dans le test par une racine absolue imaginaire (`/depot-en-memoire`,
le contenu vient de `content_map`, rien n'est lu sur disque). Pas de témoin
dédié au défaut lui-même : pour l'écrire, reprendre la recette ci-dessus en
test qui ÉCHOUE tant que le silence persiste.

## La cause

Non établie précisément. Le contenu est bien passé (`content_map`), le
verdict garde le fichier (`files.len() == 1`) ; c'est `parse_project`
(codeparsers) qui ne rend rien pour des chemins absolus fabriqués depuis une
racine relative (`./src/demo.rs`). À creuser côté codeparsers : détection de
langage ou résolution de chemin qui exige l'absolu.

## Ce qu'il faut pour le fermer

Au choix, et idéalement les deux :
- `analyze` canonicalise la racine (ou refuse une racine relative avec une
  erreur nommée) ;
- un fichier présent dans `files` mais dont le parse ne rend rien ET ne
  signale rien entre dans `skipped` avec une raison — le silence est le
  défaut autant que le zéro.
