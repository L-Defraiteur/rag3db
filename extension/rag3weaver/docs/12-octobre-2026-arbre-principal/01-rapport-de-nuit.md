# Rapport de nuit — arbre principal

*12 octobre 2026, de 2 h à 6 h. La session « arbre principal » rebâtit la lib partagée, tient les
lots produit des morceaux sans texte et ferme les manques de la page 04 de codeparsers.*

## Le P1 : les morceaux de code sans texte — clos

**Ce qu'on croyait**, dans l'ordre :

1. Un texte perdu après l'embarquement.
2. Un binaire d'avant la conversion du Cypher brut.

**Ce qui l'a tranché** :

- La relecture de l'historique : `_text` passe par COPY CSV ou par des paramètres liés, jamais par
  du Cypher brut. Le marqueur prouvait que le texte était en base au moment de la passe de vecteurs.
- Le test des groupes de F : 26 092 vides, tous dans les 32 768 premières lignes de `Scope_Chunk`.

**La cause, trouvée par le cœur** : `StringColumn::scanFiltered` lit mal une fenêtre de 2 048
lignes à cheval sur deux segments, quand le groupe porte des versions. **Le disque est intact.**
Le correctif est `f6179ade0`, le témoin du banc `e6f626df4`. Avec la lib corrigée, la copie scellée
de F compte **0 vide sur 128 732**, sans rien réécrire.

**Côté produit** :

- `7b54c5f84` : la réécriture des morceaux vides depuis leur entité.
- `772d4e9d6` : la complétude de l'inchangé exige un texte.
- `1a31fea71` puis `2b0642b1e` : la réécriture suspendue le temps du correctif, puis remise en
  marche.
- Le ticket est dans `closed/`, avec ses suites (`961dd7b05`).

**Reste** : compter les vides de la base de Lucie après le redémarrage de son hôte au matin (0
attendu).

## La lib partagée, trois rebâtis

| heure | sur | identifiant (`courant`) | md5 de la lib |
|---|---|---|---|
| 2 h 34 | `7b54c5f84` | rag3db-build-6ffec5a82…-Release | 116acb25… |
| 3 h 49 | `e6f626df4` (scan filtré) | rag3db-build-f6179ade0…-Release | c2ddbb67… |
| 5 h 59 | `41d282843` (petit masque) | rag3db-build-41d282843…-Release | 8d6256cc… |

**Le magasin** : depuis 3 h 49, la recette publie au magasin et pose `courant`
(`rag3db_libs.py publier`, page 06). Les passes e2e n'ont plus à dire « fini » avant un rebâti.

**L'ABI** : rien n'a été retiré. Le dernier rebâti ajoute un symbole, `NodeTable::lookupOffsets`.
La surcharge d'avant B2 fait charger un binaire ancien, pas tourner : la disposition de
`SystemConfig` a changé. C'est noté au ticket du cœur.

## Le premier manque de la page 04 — fermé

« 0 tests qui la traversent » quand les tests passent par des requêtes : `94e21b066`, page
`af2dfd300`.

- **La déclaration** : `workspace.queries_via` au manifeste. Elle donne les fichiers de cas
  (marqués `suite`/`declared`, par accord avec codeparsers), les noms enregistrés (REGISTERS) et la
  table des mots-clés (QUERIES). impact suit `HAS_PARENT>?|DEFINES/REGISTERS>|<QUERIES`.
- **Sur le vrai moteur** : `impact lockIndexForWrite` passe de 1 test à **204**.
- **La table** : 24 mots ; 5 genres du moteur sont sans entrée.
- **Le manifeste installé de Lucie** n'a pas `queries_via` : il faut copier le bloc du gabarit
  livré.

## Relectures

- **Saut en deux temps (F, `6f5554868`)** : la fenêtre entre les deux instructions (une ligne
  écartée et comptée, puis un `CatalogEvent::Warning`) et les noms de colonnes.
- **Portage B1 (G)** : les dépendances `libc` et `windows-sys` manquaient, sans quoi macOS et
  Windows ne compilaient pas ; une doc était déplacée. B2 porte la citation du chemin du COPY.

## Les incidents de forme

- **wt-docs** : remis sur master par `checkout --detach`, il a laissé en plan un commit non poussé
  de codeparsers. Repris, rien de perdu. Règle : un worktree de docs par session, et lire
  `status` puis `log -1` avant tout checkout.
- **Une sonde « ancien binaire »** : impossible, faute d'une lib de son époque (ABI de
  `SystemConfig`). Elle a été remplacée par la lecture de l'historique.

## Suite

Les manques de la page 04, dans l'ordre :

1. la portée : borner usages, impact et search_code à un dossier ou un langage ;
2. les verrous du chemin ;
3. le lot du pointeur de codeparsers (`0081fa5` et au-delà), avec `test_role_name` et
   `test_certainty_name` remplacés par `.as_str()`.
