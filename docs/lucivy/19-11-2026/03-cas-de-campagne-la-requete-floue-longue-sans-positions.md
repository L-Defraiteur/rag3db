# Cas de campagne : sans positions, une requête floue longue relit chaque valeur entière

Session lucivy, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie). C'est un cas de
performance en Rust, dans le moteur de lucivy. Il se joue sans la lib rag3db, en quelques minutes de `cargo test`.

## Le point de départ

- **Dépôt** : lucivy (`github.com/L-Defraiteur/lucivy`), pas rag3db.
- **Commit parent** : `fa65315ce8fc` (19 novembre 2026, 10 h 16). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : le workspace lucivy entier (le moteur `ld-lucivy` à la racine, `lucivy_core` pour le témoin).

## Le signalement, tel qu'il était

C'est le ticket du 14 novembre (`docs/lucivy/tickets/2026-11-14-le-chemin-sans-positions-relit-tout-pour-une-requete-longue.md`,
dans rag3db). Il est repris sans sa partie « Ce qu'on pourrait faire », qui oriente la résolution. Sa partie « Comment
le rejouer » est remplacée par le témoin, car la branche de mesure qu'elle nommait n'est pas dans l'arbre.

> Un index créé avec `positions: false` garde, dans ses postings, les documents et les fréquences, mais pas les
> positions. Chaque correspondance est retrouvée comme documents candidats, puis vérifiée sur le texte stocké. La
> doc de l'option l'annonce : mêmes réponses, et une relecture des candidats pour les spans et les correspondances
> à travers plusieurs tokens.
>
> Mesuré le 14 novembre 2026 sur une réindexation de 89 894 documents de code (932 Mo de texte), avec des requêtes
> `contains` sur trois champs, spans toujours demandés : 4 mots en distance 1, 35 ms avec positions et 4,4 à 5,5 s
> sans ; une requête longue de 300 caractères en distance 1, 176 ms avec positions et **60 s** sans.
>
> La requête longue en distance 1 ne trouve **aucun** document, et prend pourtant une minute. La vérification floue
> à travers les tokens se fait sur le texte stocké de tous les candidats.

## Le témoin

- **Filtre** : `a_long_fuzzy_query_without_positions_reads_windows_and_finds_the_same`, dans
  `lucivy_core/tests/test_no_positions_long_fuzzy.rs`. Le fichier est identique sur main (vérifié le 19 nov. sur
  792f514).
- **Commande** : `cargo test -p lucivy-core --test test_no_positions_long_fuzzy`.
- **Rouge au parent** : la dernière assertion échoue. La distance d'édition a couru sur 238 818 octets des valeurs
  stockées, alors qu'elle ne doit pas dépasser le dixième du total. Les deux premières assertions passent : les
  mêmes documents et les mêmes spans qu'avec positions.
- **Suite** : `cargo test` du workspace lucivy, celle de sa CI. Le juge ajoute
  `cargo test -p lucivy-core --test test_no_positions_queries -- --ignored` si un corpus est posé : c'est une mesure,
  pas un témoin.

## Les indices présents dans l'arbre

- `src/suffix_fst/briques/stored.rs` : `fuzzy_prescan`, la vérification sur le texte stocké, et le générateur par
  morceaux (les morceaux d'une requête longue).
- `src/suffix_fst/briques/profile.rs` : le compteur `n_stored_dp_bytes` et `stored_dp_bytes()`, que lit le témoin.

## Notre correctif (caché à Algareth, pour le juge)

`8a707ac72cb5`, « fix(ld-lucivy): sans positions, une requête floue coupée en morceaux ne passe à la distance
d'édition que des fenêtres autour des occurrences de ses morceaux (Levenshtein) » : `src/suffix_fst/briques/stored.rs`,
+99 −1. Le témoin passe de 238 818 à 26 729 octets relus ; la requête longue du banc passe de 6,8 s à 188 ms.
