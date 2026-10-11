# B9 côté Rust — ce que l'index manque, par cas minimaux

Session recherche, 11 octobre 2026, pour la session codeparsers (B9, avec
rag3db-ab). Chaque cas : le fichier entier (5-15 lignes), la relation
attendue, la relation obtenue mot pour mot par l'outil `usages` sur un index
frais du backend de code (binaire du jour, moteur partagé de l'arbre
principal, service d'embarquement). La sonde est rejouable :
`sonde_b9.py` (scratchpad de la session recherche ; corpus de sept fichiers,
huit interrogations).

## Les trous, du plus coûteux au moins coûteux

### T1 — un bloc `impl` fabrique une seconde définition homonyme du type

`traits.rs` (le fichier entier) :

```rust
pub trait Outil { fn jouer(&self) -> i32; }
pub struct Marteau;
impl Outil for Marteau {
    fn jouer(&self) -> i32 { 1 }
}
pub fn frappe() -> i32 {
    let m = Marteau;
    m.jouer()
}
```

- **Attendu** : UNE définition `class Marteau` (la struct, ligne 2) ; le bloc
  `impl` (ligne 3) s'y rattache.
- **Obtenu** : DEUX définitions — `class Marteau — traits.rs:2` ET
  `class Marteau — traits.rs:3` ; « Nom ambigu : plusieurs définitions. Les
  usages trouvés par le nom seul ne sont pas attribués. »
- **Coût** : maximal. L'ambiguïté fabriquée contamine la résolution du type
  ET de toutes ses méthodes partout où un `impl` existe — c'est-à-dire sur
  tout type Rust réel. C'est elle qui casse T2.

### T2 — la méthode du trait et celle de l'impl sont deux homonymes non reliés

Même fichier. `usages(jouer)` :

- **Attendu** : `call: frappe -> jouer` (receveur `m` typé localement
  `Marteau`).
- **Obtenu** : deux définitions (`method jouer — traits.rs:1` et `:4`), et
  `frappe` rangé « non attribués (nom ambigu) ». L'appel n'entre donc pas
  dans l'impact.
- **La mécanique, précisée par la session codeparsers** : `choose_target`
  ne relie un receveur TYPÉ que par son type — un receveur au type connu ne
  crée jamais d'arête « nom » ; toutes les arêtes « nom » viennent du repli
  « un seul définisseur » sur un receveur SANS type. T1 est donc bien la
  cause de T2 : le type dédoublé n'est plus « un seul définisseur », et le
  repli lui-même se ferme.
- **Contraste qui situe le trou** : le receveur **dyn** résout —
  `dyn_receveur.rs` :

```rust
use crate::traits::Outil;
pub fn par_trait(o: &dyn Outil) -> i32 {
    o.jouer()
}
```

  donne `call: par_trait -> jouer → traits.rs:1`, ATTRIBUÉ. Le chemin
  « type du receveur → méthode du trait » existe donc ; c'est le couple
  struct-concrète/impl qui le casse (T1), pas la mécanique d'appel.

### T3 — le receveur de boucle ne produit AUCUNE arête (le B2d d'ab)

`boucle.rs` (le fichier entier) :

```rust
use crate::traits::{Marteau, Outil};
pub fn tous() -> i32 {
    let outils = vec![Marteau, Marteau];
    let mut total = 0;
    for o in &outils { total += o.jouer(); }
    total
}
```

- **Attendu** : `call: tous -> jouer` (receveur = variable de boucle sur
  `Vec<Marteau>`).
- **Obtenu** : RIEN — `tous` n'apparaît ni en call, ni en « non attribué »,
  ni « par le nom » dans `usages(jouer)`. L'appel au receveur de boucle ne
  laisse aucune trace, même pas une arête devinée. (Pour `usages(Marteau)`,
  `tous` apparaît en « non attribué », par le `use` — l'appel, lui, est
  perdu.)
- C'est le cas Rust du premier trou du classement C++ (méthode sur receveur
  non typé) : à la session codeparsers (B2d), comme convenu.

### T4 — le corps d'une macro s'évalue dans le scope du fichier

`macros.rs` (le fichier entier) :

```rust
macro_rules! disons {
    ($x:expr) => { crate::fermetures::aide($x) };
}
pub fn par_macro() -> i32 { disons!(41) }
```

- **Attendu** : `call: par_macro -> aide` (au travers de `disons!`) — ou à
  défaut une arête `par_macro -> disons`.
- **Obtenu** : `other: module file_scope_01 — macros.rs:2 (par le nom)` —
  l'appel écrit DANS le corps de la macro est rattaché au scope FICHIER et
  marqué « nom » (donc filtré de l'impact) ; le site d'expansion
  (`par_macro`) n'a aucune arête ; aucun scope pour la macro elle-même.

### T5 — pas de scope fermeture, et la fn passée en argument n'est pas un call

`fermetures.rs` (le fichier entier) :

```rust
pub fn aide(x: i32) -> i32 { x + 1 }
pub fn applique() -> i32 {
    let double = |x: i32| x * 2;
    let v: Vec<i32> = vec![1, 2].into_iter().map(aide).collect();
    v.into_iter().map(|x| double(x)).sum()
}
```

- **Attendu** : `call: applique -> aide` (fonction passée à `map`) ; et un
  scope pour `double`, ou au moins l'appel `double(x)` rattaché à
  `applique`.
- **Obtenu** : `other: applique -> aide` (la relation existe, mal classée —
  coût faible) ; la fermeture `double` n'existe nulle part (pas de scope,
  pas d'appel) — invisible aussi au relevé des non-résolues, comme ab le
  prévoyait : « le relevé ne voit pas un scope que l'analyse n'a jamais
  produit ».

## Ce qui marche (à ne pas casser en corrigeant)

- **Réexportations** : `pub use interne::secrete` traverse —
  `call: via_reexport -> secrete` ATTRIBUÉ, imports des deux côtés. RAS.
- **Inheritance** : `inheritance: class Marteau -> Outil` présente.
- **Receveur dyn** : voir T2.

## L'ordre que je propose (du point de vue des outils)

T1 d'abord : il est UNE cause pour deux symptômes (types ambigus, méthodes
non attribuées), et son correctif rend T2 mesurable proprement. T3 est chez
ab (B2d). T4 vaut par son silence (l'impact ne voit pas au travers des
macros — fréquent dans rag3weaver : simple_factory!, named_factory!).
T5 en dernier (reclasser other→call quand l'argument est un nom de fn).

Après correctifs : rejouer `banc/comparer.sh` de codeparsers sur
`banc/rag3weaver.txt` (la grille d'ab), même corpus — pas cette sonde, qui
ne mesure que la présence des relations, pas leur coût.
