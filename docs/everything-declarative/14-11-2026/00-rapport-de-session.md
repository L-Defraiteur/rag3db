# Everything declarative, 14 novembre 2026 : le rapport de session

## Ce qui est sur master

- **Le lot des octets d'un scope** (`500b1ede4`, rag3db ; `b9029dc`, codeparsers master en avance
  rapide) : le contenu d'un scope est borné à l'empan de son nœud (`node_start_byte`,
  `node_end_byte`), et non à ses lignes. La cause des 844 Mo mesurés par lucivy, avec la page
  `02-le-contenu-d-un-scope-borne-a-ses-octets.md`. Pas de `code-scope-v2` : l'adresse suit le texte.
- **Le plan de 2b-B** (`01-plan-du-chargeur-durable-des-couches-2b-b.md`), avec les décisions de Lucie
  du 14 novembre (§ 7 bis) et la relecture d'arbre principal (§ 7 ter, le pas 2 coupé en 2a′ et 2b′).
- **La page de 2a′** (`03-le-pas-2a-prime-stocker-les-generations-sans-fuite.md`) :
  - l'ancre a des générations ;
  - des lignes `Scope` et `File` immuables, à visibilité par `_couche`, `_depuis` et `_jusqu_a`, sur
    les nœuds et sur les arêtes ;
  - l'uuid par génération pour les lignes neuves seulement ;
  - le relevé des lecteurs, les témoins 0 à 6, la mesure du prédicat (§ 6).
- **La relecture de la page 30** et de la liaison (`0672d624f`) ; le défaut de la parente non suivie,
  corrigé par arbre principal (`912ea2084`).

- **2a′-1, la visibilité des générations** (78c8997de) :
  - l'estampille `code-scope-v1/content/generations`, sur une base neuve seulement, et sur rag3db
    seulement : postgres a son ticket ;
  - les trois colonnes sur `Scope` et `File`, et la table interne `_generation` ;
  - le prédicat appliqué aux plans, aux sauts, aux traductions, aux filtres de recherche, à `get` et à
    `find_by_field` ;
  - les témoins 0 et 4, et l'entité `Generation`, rouges puis verts.
- **2a′-2, la vue publiée** (e89cdb8e6, rang 30 de la pile, sur dense-sur-tenue-pile) :
  - la migration d'une base à versions vers les générations, un trou de 2a′-1 comblé ;
  - le masque des versions en prédicat et non en liste (une version est admise si une de ses
    occurrences, reliée par la relation canonique, est visible, de la cellule et passe le filtre) ;
  - `inline_params_borne` refuse une liste de plus de 1 000 éléments ;
  - le sans-filtre reste au court-circuit d'arbre principal ;
  - trois témoins rouges puis verts ; la parité de banc au bit près, 30 cas et une base à orphelines ;
  - la recherche à versions est de 2 à 6 fois plus rapide.
- **Les pages de la soirée** :
  - 04, la mesure par étage et ses chiffres (sur master par la pile) ;
  - 05, la vue publiée ;
  - 06, l'écrivain et le cache par rang, avec la décision des déplacés (§ 9) et du `Symbol` délié
    (§ 10) ;
  - 03, § 8 (l'identifiant stable de couche, avec arbre principal) et § 9 (la position sort de
    l'identité avec 2a′-3).
- **Les tickets de la soirée**, sur master :
  - fermé, par le cœur : la liaison d'une grande liste littérale (1d5580d74) ;
  - ouverts : la recherche à versions qui prend des Gio (1b n'atterrit sur aucune vraie base avant
    sa fermeture) ; l'`IN` sur la clé qui balaie la table, pour le cœur ; la mémoire résidente à
    versions (le filtré, à mesurer sur l'hôte réel par G) ; la visibilité dans le prédicat (l'état B,
    à fermer par le cache par rang) ; la recherche sans filtre qui paie le masque entier ;
  - le doublon du schéma postgres partagé, fermé dans celui de recherche.

## Ce qui est en cours

- **La mesure du cas filtré sur l'hôte réel** : G, demain matin. Avant et après vue-publiee
  (fac67a208 contre master ≥ e89cdb8e6), par chemin et par langage, avec le VmRSS par appel. Elle
  ferme le ticket de la mémoire résidente.
- **L'écrivain de 2a′-2** (page 06), dans l'ordre accepté par c7 :
  1. les témoins 1 à 6, rouges ;
  2. l'écrivain et la publication ;
  3. l'époque et le cache par (couche, rang) ;
  4. le post-filtre du dense, en dernier et mesuré, avec le rappel en chiffres.
  Le compte des recopies à chaque publication est dit dans l'état. L'arbre sale porte
  `arbre_propre: false` et le nombre de chemins sales.
- **Ensuite** : 2a′-3, l'écriture d'une suivie par les objets git, où la position sort de l'identité,
  puis 2b′.

## Reprendre

- **Le worktree du portable** : `~/.cache/rag3weaver-build/wt-generations`. Les branches poussées :
  `vue-publiee-pile` (atterrie), `mesure-deplaces` (la sonde des déplacés), `sonde-in-par-cle` (la
  sonde autonome de l'`IN`, pour recherche), `ed-mesure-court-circuit` (une fusion de mesure, jamais
  vers master).
- **À luciepc** :
  - le worktree `~/git_workspaces/rag3db-ed-generations-2` et le target `target-ed-navigateur` ;
  - les scripts dans `~/.cache/rag3weaver-build/ed-generations-2/` (`serie.sh` pour une série de
    mesure, `pile-passe.sh` comme modèle de passe sur une pile) ;
  - les copies de mesure dans `~/.cache/rag3weaver-build/ed-mesure-2a2/` (`base`, `ref`, `ref-source`,
    les manifestes et les offsets) ;
  - la lib des passes est 373b5477b ; les services d'embarquement sont 7878 et 7879 (7880 est arrêté).
- **Les sondes de mesure** (master) : `sonde_recherche_sans_filtre` (par étage, R contre A/B/C, le RSS
  par appel), `sonde_noeud_lexical`, `sonde_vecteur_filtre`, `sonde_clause_sans_filtre`.

## À effacer au mot de c7

- **Sur le portable** : les worktrees `wt-installeur`, `wt-equipier-fin`, `wt-premier-appel`,
  `wt-flux-partage`, `wt-octets`, `wt-codeparsers-empan`, et le target `target-codeparsers-empan`.
- **À luciepc** :
  - `rag3db-ed-installeur`, `rag3db-ed-equipier` et leurs targets ;
  - les copies de mesure `ed-mesure-2a/` (code-lourd), puis `ed-mesure-2a2/` une fois la mesure de G
    faite ;
  - les dossiers `ed-pile-journaux` et `ed-pile-vue-journaux` une fois leurs lots atterris.
- Le worktree `rag3db-ed-navigateur` de luciepc est déjà retiré (par la garde, 14 novembre).
