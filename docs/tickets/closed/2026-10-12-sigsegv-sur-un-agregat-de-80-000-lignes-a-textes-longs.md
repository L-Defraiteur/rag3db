# SIGSEGV sur un agrégat de 80 000 lignes qui rend des textes longs

- **État** : corrigé (le commit qui déplace ce fichier dans `closed/`, 12 octobre 2026) — la lecture, pas le stockage : `StringColumn::scanFiltered` (chemin pris quand un groupe porte des versions — lignes supprimées ou écrites sous transaction) bornait la fin d'une fenêtre de 2 048 lignes par `startOffsetInChunk + pos < numValues` au lieu de `pos - offsetInResult < numValuesToScan` : quand la fenêtre chevauche deux segments de la colonne et que le second est plus court que la position, ces lignes n'étaient pas écrites dans le vecteur, qui gardait la chaîne de la fenêtre d'avant (ou rien). Venu de l'amont (« Segmentation », 18 septembre 2025). Même borne dans `ListColumn::scanFiltered`. Les données sur disque sont intactes : la relecture par clé (lookup) était juste. Sur la copie de F : deux lignes rendaient le `content` d'une ligne 2 048 plus tôt (1 944 551 octets pour une ligne de 271) ; le SIGSEGV était ce bloc (> 256 Ko, à part) rendu au système par `resetOutVectors` au début de la fenêtre suivante. Après : la requête d'origine passe, parité octet à octet balayage contre clé à 0 écart sur content, docstring, name (81 407 lignes)
- **Gravité** : plantage (le processus meurt par signal 11, sans message du moteur)
- **Atteignable en service** : oui en principe (une lecture d'outil qui agrège sur toute une table de chunks et rend `content`) ; vu une fois seulement
- **Touche rag3weaver** : oui (le test `e2e_mesure_index_entier::ce_que_porte_le_texte_des_scopes_vides`, depuis réécrit par paquets)

## Ce que c'est

Sur la copie fermée de `code-rag3db-depot` (7 310 fichiers), lib e71c85b551cf, base ouverte en
lecture-écriture par `Rag3dbConnection::new`, dans un test seul :

```cypher
MATCH (n:`Scope_Chunk`)-[:`Scope_CHUNKED_FROM`]->(s:`Scope`)
WITH s, count(CASE WHEN n.`_text` IS NULL OR n.`_text` = '' THEN 1 END) AS v, count(n) AS t
WHERE v = 0 OR v = t
RETURN v > 0 AS vide, s.`content`, s.`docstring`
```

Environ 80 000 lignes, dont des textes longs (`content` d'un scope : 700 caractères en moyenne,
certains bien plus). Le processus meurt par SIGSEGV. La même requête qui ne rend que `s._uuid`
passe ; lire `content` ensuite par paquets de 500 par clé passe aussi.

## Pistes

- L'agrégat par clé `s` (un nœud) porte dans sa table factorisée les colonnes `content` et
  `docstring` de `s` : des chaînes longues, débordant la page (overflow). À vérifier : la table de
  hachage de l'agrégat et le `RESULT_COLLECTOR` sur des chaînes débordantes à cette taille.
- Reproduire : la copie est sous `~/.cache/rag3weaver-build/embarquements/copie-depot/` (elle ne
  sert qu'à F ; en prendre un reflink avant toute ouverture), la requête ci-dessus par un petit
  programme C++ lié à la lib, sous `gdb`, pour la pile.
- Un plantage sans message est toujours à nous : le moteur doit refuser nommément ou servir.
