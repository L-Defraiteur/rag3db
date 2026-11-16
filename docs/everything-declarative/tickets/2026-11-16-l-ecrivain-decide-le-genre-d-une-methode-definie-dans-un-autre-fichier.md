# L'écrivain décide le genre d'une méthode définie dans un autre fichier que sa classe

- **État** : ouvert (16 novembre 2026)
- **Famille** : produit
- **Gravité** : basse. `void Foo::f() {}` dans un `.cpp`, quand `Foo` est déclarée dans un `.h`, reste `function` au lieu de
  `method`. L'arbre compact (`outline`) montre une fonction là où il y a une méthode.
- **Atteignable en service** : oui, sur tout dépôt C++ où les méthodes sont définies hors du fichier de leur classe.
- **Touche rag3weaver** : oui (`src/code.rs`, la détermination de `scope_type` à l'analyse ; `src/code_sync.rs`, l'écrivain des
  générations, `write_batch_by_generations`).
- **Pour** : everything declarative (ED) ; archi relit si la navigation change.
- **Signalé par** : codeparsers, `docs/codeparsers/tickets/2026-11-15-le-genre-d-une-methode-definie-dans-un-autre-fichier.md`
  (le constat, adressé « ED et archi ») ; ce ticket est la part d'ED, ouverte le 16 novembre sur la demande de c7.

## Ce que c'est

Depuis le lot de codeparsers sur l'appartenance des méthodes, `code.rs` fait de `Foo::f` une `method` quand le parent résolu
**dans le fichier** est une classe. Quand la classe est dans un autre fichier, le parent ne se résout qu'au rendez-vous des
symboles, après la publication de la ligne. Réécrire alors `scope_type` ferait naître une version hors de l'écrivain :
`scope_type` est un champ de version (`SCOPE_VERSION_FIELDS`, code.rs).

## Ce qu'il faut faire (choix d'ED)

L'orchestration a laissé deux voies : le genre décidé par l'écrivain à la publication, ou une navigation qui lit sans le genre.
**On prend la première**, la plus petite :
- avant d'écrire les lignes d'un fichier (`write_batch_by_generations`), pour chaque scope `function` dont le nom est qualifié
  (`Foo::f`, ou `parent_name` non vide) et dont le parent n'est pas dans le fichier, lire les `Scope` **visibles** de la même
  source nommées comme le parent, de genre `class` ou `struct` ;
- s'il y en a une, et une seule, le scope s'écrit `method` ; sinon il reste `function`, comme aujourd'hui ;
- une lecture par paquet et par source pour tous les noms de parents du paquet, comme le grain du paquet le fait (pas une
  requête par scope).

**La limite, dite dans le code et dans la nouveauté** : si le `.h` de la classe est indexé **après** le `.cpp`, dans le même
repassage, la méthode reste `function` jusqu'à la réécriture suivante du `.cpp`. Pas de réécriture rétroactive : elle ferait
naître une version hors de l'écrivain.

**Ce qui ne se touche pas** : le rendez-vous des symboles (`resoudre_les_symboles`), qui pose les arêtes et pas les champs de
version ; la clé d'un scope (le genre n'en fait pas partie, à vérifier dans `key` avant d'écrire).

## La preuve

- **Témoin, rouge d'abord**, dans `code::tests_de_l_ecrivain` (nom anglais) :
  `a_method_defined_in_another_file_than_its_class_is_a_method`. Une source `foo.h` (`class Foo { void f(); };`) et
  `foo.cpp` (`void Foo::f() {}`) : `foo.h` indexé d'abord, puis `foo.cpp` par `reingest_file`. La ligne visible de `f` a
  `scope_type = "method"`. Rouge aujourd'hui (`function`).
- Une seconde branche du même témoin : `foo.cpp` seul, sans classe visible, reste `function`.
- La passe épinglée, suites exigées par chemins (`e2e_code_sync` pour `code.rs` et `code_sync.rs`), plus `e2e_code`.

## Comment le fermer

Le témoin vert, la passe verte, une ligne de nouveautés qui dit la limite, et le ticket de codeparsers fermé avec un renvoi ici.
