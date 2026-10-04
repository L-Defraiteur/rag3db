# Une arête ne dit pas comment elle a été résolue

- **État** : en cours — colonne sur master (986565f81) ; filtre et appels par chemin sur la branche `filtre-resolution` (c56ee578b), en attente du rejeu de l'arbre principal
- **Gravité** : réponse fausse (une arête devinée se montre comme une arête sûre)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (rendez-vous `code.rs`, `usages`, `impact`, liens)

## Ce que c'est

`reingest_file → clear (node_id_cache.rs) ← run` dans la section Liens : un
appel `x.clear()` sur une variable dont aucun type ne se lit, relié par le
rendez-vous à la seule méthode `clear` du projet. L'arête est posée par
`materialiser_les_symboles` (`code.rs`) dans la branche « un seul
définisseur » ; rien sur l'arête ne la distingue d'une arête dont le type
lu ou l'import a désigné la cible. Un consommateur ne peut ni la taire ni la
signaler.

## Recette minimale

```rust
// a.rs
pub struct Cache; impl Cache { pub fn clear(&self) {} }
// b.rs
pub fn run(v: &mut Vec<u32>) { v.clear(); }
```

Attendu : soit pas d'arête `run → clear`, soit une arête marquée « par le
nom seul ».

## Proposition

1. **La marque, posée là où la décision se prend.** Une propriété
   `resolution` sur `CONSUMES` / `CONSUMED_BY` (et `IMPLEMENTS`,
   `INHERITS_FROM`) : `fichier` (relation de l'analyseur, intra-fichier),
   `type` (le type lu a choisi), `import` (l'import a choisi), `nom` (seul
   définisseur, rien d'autre). `materialiser_les_symboles` connaît déjà la
   branche qu'il prend.
2. **Les consommateurs la lisent, déclarée au gabarit** : la section Liens et
   `impact` ne suivent que les arêtes dont la résolution n'est pas `nom`
   (un filtre sur l'arête dans le saut, générique : `trusted_field`,
   `untrusted_values`) ; `usages` garde tout et dit « par le nom » à côté
   d'un usage ainsi résolu, comme il le fait déjà pour un nom ambigu.
3. **Le coût** : une colonne de relation ; une base existante n'a pas la
   marque (NULL lu comme « inconnu », traité comme sûr jusqu'à
   réindexation).

## Pour le fermer

Le lot côté `code.rs` (arbre principal) : la colonne et sa valeur ; puis les
consommateurs (session codeparsers) ; mesure au banc des relations (la
précision de « dépend de » doit monter, le rappel ne pas baisser — les
arêtes `nom` justes existent aussi).

## Où on en est (4 octobre, soir)

- La marque est posée (`choose_target`, cef4c5ae5 ; colonne `resolution`,
  986565f81) : `fichier`, `type`, `import`, `nom`.
- Un appel par chemin sans `use` (`crate::a::f()`, `super::x::f()`) était
  marqué `nom` ; il prend maintenant son qualificatif comme module
  (c56ee578b).
- Le filtre est générique (`edge_field`, `edge_guessed`) et déclaré dans
  `links.mmd` ; `usages` dit « (par le nom) ».
- **Pas dans `impact`** : banc des relations, deux passes sur le même index —
  « dépend de » 1,00 / 0,85 → 1,00 / 1,00 (rappel / précision), « tests
  traversés » 0,89 / 0,92 → 0,89 / 0,99, mais « relie » 1,00 → 0,75, et deux
  e2e perdent un vrai test. Restent `nom` sans être faux :
  - un appel de méthode sur une variable sans type lu
    (`catalog.lock().unwrap().ingest_entities(…)`, `guard.probe_embedding_rate(…)`) ;
  - un appel par chemin dans les arguments d'une macro (`assert_eq!(crate::b::run(), 2)`) :
    ticket « le qualificatif d'un chemin se perd dans une macro ».

Pour allumer le filtre dans `impact` : que ces deux voies gagnent une
marque plus sûre que `nom`, puis la même mesure.
