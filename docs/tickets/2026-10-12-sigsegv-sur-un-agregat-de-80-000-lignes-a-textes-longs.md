# SIGSEGV sur un agrégat de 80 000 lignes qui rend des textes longs

- **État** : ouvert (12 octobre 2026, 2 h 10 ; vu une fois par la session embarquements, remonté au cœur C++)
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
