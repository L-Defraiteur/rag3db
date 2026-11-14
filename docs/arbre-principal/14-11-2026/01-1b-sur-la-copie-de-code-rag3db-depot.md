# 1b sur la copie de `code-rag3db-depot` : ce qui a migré, ce qui a été vérifié, ce qui bloque

14 novembre 2026, arbre principal. La migration de l'ancre vers les versions (lot 1b), jouée sur une
**copie** de `code-rag3db-depot`, jamais sur l'original. La copie vient de l'instantané `avant-835d5298f`
de G (13 nov., 20 h 07, hôte arrêté), prise par reflink. Lib `0c41f9fe6`, branche
`arbre-principal/migration-1b`, au portable, sous `poste lourd` et 32 Gio.

## Ce qui a migré

`rag3weaver-backend migration versions --base <chemin>`, de 16 h 41 à 16 h 56, en 861 s :

| | |
|---|---|
| occurrences `Scope` | 79 662 (uuid, fichier et lignes inchangés) |
| versions `ScopeContent` | 72 652 |
| morceaux `ScopeContent_Chunk` | 119 887 |
| vecteurs granite_278m repris | 119 887, **0 en dette** |
| appels au modèle | **0** (un embarqueur qui refuse tout appel) |
| estampille | `code_scope_form = code-scope-v1/content` |
| mémoire résidente | plafond à 18 Gio ; plus gros lot tenu : 2 000 |

La version la plus partagée est `namespace rag3db` (cpp), au contenu vide une fois ses enfants repliés,
avec 295 occurrences. 2 110 versions partagées couvrent 9 120 occurrences.

**Les trois essais d'avant**, et ce qu'ils ont appris :
1. Un seul lot de 68 600 occurrences, et le transfert de tous les vecteurs en une requête : tué par le
   noyau à 32,4 Gio au bout de 3 min 21 s. D'où les lots de 2 000 et la règle « rien ne tient en mémoire
   plus qu'un lot » (page 29).
2. Les lots : le tampon de 8 Gio du moteur est plein au lot de transfert 42 000 / 72 652.
3. Les lots avec un `CHECKPOINT` après chaque lot : même refus, au même lot. La cause n'était donc pas
   des pages validées non évincées, mais la requête de transfert. Elle joignait chaque morceau neuf aux
   morceaux de **toutes** les occurrences de sa version, vecteurs compris, puis triait. Le correctif lit
   un représentant par version, en trois lectures bornées par le lot, appariées en Rust.

   Le `CHECKPOINT` est retiré (c26f906b7), puisqu'il est mesuré sans effet. **Réserve** : la copie a
   migré avec lui encore présent. Les témoins sur fixtures sont verts sans lui, et la passe complète
   le couvre.

Le souhait d'un tri qui déborde sur le disque au lieu de refuser est le ticket
`2026-11-14-le-tri-d-une-jointure-trop-grande-refuse-au-lieu-de-deborder`.

## Ce qui a été vérifié (sur des copies reflink de la copie migrée)

- **`check_rel_directions`** : les deux sens sont égaux sur les 21 tables de relations, aucun écart.
- **La couverture** :
  - 0 occurrence sans version, 0 occurrence à deux versions ;
  - 0 version sans occurrence, 0 morceau orphelin ;
  - 0 doublon de morceau (même version, champ et rang) ; tous les uuid sont distincts.
- **6 versions sans morceau** : les 8 scopes `texte_brut` vides (`CMakeLists.txt:1-1`, des
  `schema.cypher` vides, `py.typed`…). Ils n'en avaient déjà aucun dans la copie d'avant.
- **Avant et après, identiques au compte près** :
  - les nœuds : Scope 79 662, File 7 803, Symbol 86 152, Library 448 ;
  - les relations : CONSUMES 116 215, DEFINES 93 425, HAS_PARENT 36 951, MENTIONS 453 348,
    DEFINED_IN 75 731, USES_LIBRARY 16 908, QUERIES 23 441, LOCKS 568 ;
  - les sommes des `start_line` et des `end_line`.
- **Les sondes de hotfixs** : 0 relation orpheline dans les deux sens, 0 morceau sans parent.
  Elles relèvent aussi 1 291 morceaux `Scope_Chunk` à 2–7 arêtes vers le même Scope, **antérieurs à
  1b** (même compte dans la copie d'avant). C'est leur ticket
  `2026-11-14-des-morceaux-de-scope-portent-plusieurs-aretes-vers-le-meme-scope`.
- **Le jeu fixe de requêtes**, par le binaire du produit en mode local (sans montre), sur une copie
  d'avant et sur une copie migrée : 12 requêtes, chacune en bm25 puis en bm25+vector.
  - Le premier résultat est le même dans 21 cas sur 24, et 3 à 8 des 8 résultats sont communs. Les
    écarts d'ordre en BM25 viennent du corpus dédupliqué, une variation admise par le plan du 12 ;
    les trois premiers qui changent sont plausibles.
  - Il n'y a pas de rattrapage caché à la réouverture.

## Ce qui bloque la bascule des vraies bases

**La recherche est 10 à 25 fois plus lente sur la base à versions**, avec le même binaire : 7,5 s par
requête en bm25 et 25 s en bm25+vector, contre environ 1 s sur la copie d'avant. C'est un défaut du lot
1a, pas de 1b. Sans filtre, le masque des versions admet toutes les occurrences, puis passe
`_uuid IN [72 652 versions]` à chaque signal. ED l'a chiffré étage par étage : 5 s pour convertir ce
`IN` en offsets, et 0,9 s pour le masque lui-même. Il touche toute base neuve depuis e62151904.

Le correctif est un lot à part, **avant 1b dans la pile** (décision de c7) : sans filtre utilisateur et
sans vue, il n'y a rien à masquer.

## Après la bascule : le retrait de l'ancien index (un pas à part)

Après 1b, l'ancien index reste dans la base pour le retour, et le produit ne le lit plus. `scope_chunks`
rend `ScopeContent_Chunk` dès l'estampille ; `Scope` n'a plus ni morceaux ni signaux.

- **Ce qui reste** :
  - `Scope_Chunk` : 127 195 morceaux, avec leur texte et leurs vecteurs ;
  - `Scope_CHUNKED_FROM` : 129 400 arêtes, dont les 2 205 en double ;
  - `Lucivy_Scope`.
- **Ce que ça pèse** : environ 5 Go par base.
  - `Lucivy_Scope` fait **4,0 Gio**, contre **1,2 Gio** pour `Lucivy_ScopeContent` qui le remplace.
  - Dans la base, de l'ordre de 1 Go : la base est passée de 3,03 à 3,96 Go en ajoutant
    `ScopeContent`, ses morceaux et leurs vecteurs.
- **Le pas** (décision de c7) : après la bascule des vraies bases et une validation dont Lucie dit la
  durée, avec un instantané avant. On fait `DROP` de `Scope_Chunk` et `Scope_CHUNKED_FROM`, on efface
  `Lucivy_Scope`, et on pose une estampille « retour impossible ». Les doublons de hotfixs partent avec
  lui.
- **Les lecteurs en dur hors produit**, qui liraient un index mort : les sondes, bancs et mesures du
  ticket `2026-11-13-les-sondes-bancs-et-mesures-du-code-lisent-scope-chunk-en-dur`. Ils sont à faire
  passer par `code::scope_chunks` avant le retrait.

## Les copies

- `~/.cache/rag3weaver-build/1b-copie/2026-11-14-depuis-avant-835d5298f/` : la copie de travail,
  jamais ouverte.
- `~/.cache/rag3weaver-build/1b-copie/migree-20261114-1641/` : la copie migrée, la preuve. Elle est au
  portable et à luciepc, md5 identiques. On l'ouvre seulement par une copie reflink.
- `verif-20261114`, `avant-20261114` et `requetes/` : les copies des vérifications, à effacer après la
  passe complète.
