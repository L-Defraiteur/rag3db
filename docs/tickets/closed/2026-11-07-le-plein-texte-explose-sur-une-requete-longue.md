# Le plein texte explose sur une requête longue

- **État** : corrigé par le lot recherche-trois-lots (7 novembre 2026, recherche) : la requête entière sans flou au-delà de 100 caractères, la recherche par mots bornée à 64 termes, les deux coupes dites ; bornes tirées de la mesure ci-dessous ; témoin e2e_lectures::une_requete_longue_repond_vite_et_dit_ses_coupes (26 242 ms sur master, 82 ms après). L'arrêt d'un nœud après l'abandon du client reste à l'hôte ; le pendant lucivy reste ouvert
- **Gravité** : blocage. Un appel tient le nœud plein texte des dizaines de secondes, et le nœud
  tourne jusqu'au bout après l'abandon du client.
- **Atteignable en service** : oui, par tout appelant qui envoie une longue requête (un prompt
  entier, ce que faisait le hook).
- **Touche rag3weaver** : oui (`BM25SearchNode`, `build_bm25_query`, `src/search.rs`).
- **Famille** : produit

## Ce qu'on voit

| requête | nœud `bm25` |
|---|---|
| 1 135 caractères, 197 mots | 32 936 ms |
| 729 caractères, 102 mots | 13 158 ms |
| 426 caractères, 16 mots (chargés d'ids) | 1 944 ms |
| un mot | 2 ms |

L'embarquement et le dense restent sous 140 ms sur les mêmes requêtes.

## Cause

Sur le chemin lucivy, le mode par défaut lance deux requêtes (159087582) : la requête entière en
`contains` relâché à distance 1, et, pour plusieurs mots, la recherche par mots (`Parse` : lucivy
la traduit en OU d'un `contains` flou par mot et par champ). Rien ne borne ni le nombre de termes
ni la longueur de la requête entière : le coût croît avec les deux.

## La mesure (7 novembre 2026, sonde `tests/sonde_requete_longue.rs`)

lucivy seul sur la poignée de la cible, médiane de trois essais en ms (un seul au-delà de 5 s),
N termes distincts tirés d'une page de prose du dépôt. À luciepc sous `poste lourd`, pas sous
`poste mesure` (une mesure en attente gèle la file des lourds) : la charge est notée, l'effet
(trois ordres de grandeur) la domine.

**code-principal** (copie reflink, 65 946 documents `Scope` ; charge 31,1 avant, 24,4 après, 24 cœurs) :

| termes | caractères | mots (Parse, le nœud) | un par un d=1 | un par un d=0 | entière d=1 (le nœud) | entière d=0 |
|---|---|---|---|---|---|---|
| 1 | 4 | 6 | 114 | 6 | 123 | 9 |
| 4 | 17 | 124 | 1133 | 188 | 50 | 15 |
| 8 | 44 | 141 | 1416 | 90 | 18 | 4 |
| 16 | 104 | 276 | 2192 | 245 | 396 | 6 |
| 32 | 235 | 218 | 2594 | 495 | 2632 | 13 |
| 64 | 436 | 495 | 13902 | 1713 | 13376 | 36 |
| 128 | 933 | 2719 | 16711 | 969 | 50850 | 59 |
| 197 | 1397 | 1166 | 15068 | 1120 | 112630 | 132 |

**mémoire longue** (copie reflink de la base de Lucie, 2 documents `Fiche` ; charge 12,0 avant, 20,7 après) :

| termes | caractères | mots (Parse, le nœud) | un par un d=1 | un par un d=0 | entière d=1 (le nœud) | entière d=0 |
|---|---|---|---|---|---|---|
| 1 | 4 | 0 | 0 | 0 | 0 | 0 |
| 4 | 17 | 0 | 0 | 0 | 1 | 0 |
| 8 | 44 | 1 | 1 | 1 | 3 | 0 |
| 16 | 104 | 1 | 3 | 1 | 76 | 0 |
| 32 | 235 | 3 | 5 | 2 | 489 | 1 |
| 64 | 436 | 4 | 17 | 5 | 1753 | 2 |
| 128 | 933 | 9 | 65 | 9 | 10316 | 6 |
| 197 | 1397 | 19 | 89 | 12 | 29357 | 38 |

**Ce qui coûte** : la requête entière cherchée en flou (distance 1), le `contains` relâché de
159087582 — 112 s à 197 termes sur le code, 29 s sur deux fiches (les 33 s d'archi). La recherche
par mots du nœud reste sous 3 s ; la requête entière à distance 0 sous 140 ms partout.

**Les bornes, tirées de la mesure** : un budget de **500 ms** pour le nœud — le tiers de l'attente
bornée que la première recherche accorde déjà à la poignée (`ATTENTE_DE_LA_POIGNEE`, 1,5 s), sous
laquelle le nœud doit rendre, et l'ordre du reclassement jugé trop cher pour être par défaut
(436–529 ms, page 06 §3.1 du 12 octobre). **L = 100 caractères** : la requête entière garde son
flou jusque-là (396 ms à 104 caractères sur le code, 2,6 s à 235), au-delà elle se cherche à
distance 0. **K = 64 termes** : le plus grand palier de la recherche par mots sous le budget sur le
code (495 ms ; 2,7 s à 128), les plus longs d'abord, les moins de trois lettres tombent les
premiers (faute d'IDF). Les deux coupes se disent dans `meta.warnings`.

## Le témoin

À écrire : une requête de 200 mots sur un petit corpus, le nœud sous un budget (par exemple
500 ms) et un avertissement dans `meta.warnings` qui dit la coupe ; une requête courte inchangée.

## Ce qu'il faut pour le fermer

Une borne dans le nœud, pour tout appelant, dite dans la méta (options soumises à c7) : au-delà
de K termes, la recherche par mots ne garde que K termes ; au-delà de L caractères, la requête
entière n'est plus cherchée en flou. Séparément, pour l'hôte : un nœud dont le client a abandonné
devrait s'arrêter (ticket à ouvrir par l'hôte s'il n'existe pas).
