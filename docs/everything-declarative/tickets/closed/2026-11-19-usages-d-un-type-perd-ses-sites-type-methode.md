# `usages` d'un type perd ses sites `Type::méthode(…)` depuis un changement de l'analyse

- **État** : fermé le 20 novembre 2026 par la pile de codeparsers (`stop-words-locals`, atterrie à 01 h 06, baccb2ea6e). Voir « Fermeture ».
- **Famille** : produit (analyse du code)
- **Gravité** : `usages Catalog` perd 41 sites réels sur 774, tous dans `extension/rag3weaver/tests/*.rs`. Le critère de
  la répétition de la bascule est manqué par cet écart.
- **Atteignable en service** : oui. Toute réindexation d'un fichier par l'analyse d'aujourd'hui perd ces usages, avec
  ou sans versions. La bascule le révèle, parce que son premier passage réanalyse tout.
- **Touche rag3weaver** : oui (`src/code.rs`, l'analyse des rendez-vous) ; codeparsers peut-être.
- **Pour** : everything declarative
- **Signalé par** : G, à la répétition complète de la bascule (`mesures/repetition-versions-code-rag3db-depot-20261119-194739`
  au portable) ; cause lue par ED.

## Ce qu'on voit

Comparée à la production, au même arbre (b79878da9b), la copie migrée puis repassée a les mêmes usages pour
`acquireLock`, `FileSource` et `lockRowForWrite`. Pour `Catalog` : 774 sites, 733 sur la copie. Les 41 manquants
sont tous des `MENTIONS` devinées (`guessed`, sans définition, `usage: other`), venues de fonctions de
`tests/*.rs`. Par exemple `e2e_arret_brutal.rs:476` : `let mut catalog = Catalog::new(boxed, …)`.

## La cause, lue sur un reflink de la copie

Sonde `sonde_parite_search_code::releve_les_aretes_d_un_scope` (branche `rewrite-bounded-memory`).
- **Sur la copie** : `jouer_le_relecteur` a 64 arêtes, dont des `MENTIONS` vers `CatalogConfig` et `DbConnection`,
  **aucune vers `Catalog`**.
- **L'analyse d'aujourd'hui du fichier** : 44 rendez-vous en attente (`analysis.pending`) pour la fonction, **aucun
  vers `Catalog`**. `Catalog::new(…)` est retenu comme l'appel `new` qualifié par le type `Catalog`
  (`pending_qualifier_types`), comme `initialize`, `register_entity` et `execute_raw`.
- **La production** avait été indexée par une analyse plus ancienne, qui posait aussi une mention devinée du type
  `Catalog`.
- **Pourquoi `tests/` seulement** : `Catalog` y est importé du crate comme d'une bibliothèque
  (`use rag3weaver::Catalog`, `USES_LIBRARY → rag3weaver`). Le seul lien du site au type était cette mention.

## Les suspects

- `0e1074ca58` (16 nov.), « qualifier le chemin à la position exacte de sa mention » ;
- `05a0ebdfe2` (18 nov.), les mots vides de Rust et `REGLES_DE_L_ANALYSE` à 8 ;
- `bd9a9ed069` et `1055be55e9` (16 nov.), le membre d'un qualificatif de bibliothèque sûre.

## Ce qu'il faut pour le fermer

1. La bisection, avec un témoin d'analyse : `use x::Catalog; fn f() { let c = Catalog::new(); }`, puis vérifier que
   `f` a un rendez-vous vers `Catalog`. Il est rouge aujourd'hui. Le premier commit qui le fait rougir désigne la
   règle.
2. Décider avec c7 : l'analyse garde une mention du type qualifiant, ou `usages` lit `qualifier_types`. Je recommande
   la première : un `Type::méthode(…)` est un usage du type, quel que soit l'outil qui le lit.
3. Le correctif, le témoin vert, puis G rejoue la répétition (usages de `Catalog` égaux à la production).

## Fermeture (20 novembre 2026, session everything declarative)

**La cause.** Un appel `Catalog::new(…)` était écrit comme une mention de `new`, qualifiée par le type, sans aucun
rendez-vous vers `Catalog` lui-même. Un scope qui ne nommait le type que par cet appel ne comptait donc pas parmi
les `usages` du type. Les fonctions de `tests/*.rs` construisent le catalogue par `Catalog::new` sans le nommer
autrement : d'où les 41 sites perdus, tous des mentions devinées.

**Le correctif**, par codeparsers, atterri sur master :
- le témoin rouge : f3689875dc. Le site `Type::méthode()` / `Type.méthode()` d'un type importé compte pour le type ;
- le correctif : 7c0f13f7e2. La tête de type de l'appel qualifié pose une référence au type (`tetes_de_type`), en
  Rust, C++, Python, TypeScript et C# ;
- le témoin rouge suivant : d1073a98b4, puis le correctif cea011f57c. La tête de type ne rejoint qu'un scope qui ne
  nomme pas déjà le type, et ne déplace aucune arête que le fichier résout déjà ;
- la montée des règles de l'analyse à 9 : baccb2ea6e. Les bases repassent une fois.

**La preuve.**
- Les témoins de codeparsers ont été vus rouges puis verts (`usages_d_un_type_compte_ses_sites_type_methode`,
  suite e2e_usages).
- Sa sonde de marque, sur `rag3weaver/src`, donne 3 845 arêtes ajoutées et aucune retirée ; sur le moteur, 3 664 de
  plus.
- Le critère de la répétition de la bascule (774 sites de `Catalog`, comme la production) se rejoue par G à la
  prochaine répétition, qui suit cet atterrissage.
