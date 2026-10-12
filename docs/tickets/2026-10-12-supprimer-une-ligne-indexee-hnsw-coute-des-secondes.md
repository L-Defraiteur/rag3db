# Supprimer une ligne qui porte un vecteur indexé (HNSW) coûte plusieurs secondes

- **État** : ouvert. Le moteur n'est pas corrigé ; rag3weaver le contourne (le vecteur est
  remis à NULL avant la suppression, ci-dessous)
- **Gravité** : lenteur sévère (la réindexation d'un fichier par la montre tenait le catalogue
  plusieurs secondes ; retirer un fichier, des minutes)
- **Atteignable en service** : oui, à toute réindexation ou tout retrait de fichier d'un index
  à vecteurs, avant le contournement
- **Touche rag3weaver** : oui (`record_nodes.rs`, `dialect.rs` : le contournement)
- **Ouvert le** : 12 octobre 2026, session everything declarative (la mesure par phase des
  « 6 s » de Lucie)
- **Pour** : hotfixs (l'extension vector)

## Ce que c'est

Sur une table de morceaux dont la colonne `FLOAT[n]` porte un index HNSW,
`UNWIND $uuids AS uuid MATCH (c:Scope_Chunk {_parent_uuid: uuid}) DETACH DELETE c` coûte
environ **2,5 s par ligne supprimée**, et ce coût croît d'un essai à l'autre. La même
instruction, sur les mêmes lignes dont le vecteur vient d'être remis à NULL, prend quelques
millisecondes.

## Mesuré (12 octobre 2026, poste principal, lib `build/lecteurs-csv` de 2 h 34)

Le test `measure_one_file_reindexing` (`backend.rs`, `#[ignore]`) indexe `src/` (180 fichiers,
15 631 morceaux, embarqueur `hashembedder`), réindexe cinq fois un fichier dont un scope change,
puis retire un fichier de 32 scopes (45 morceaux). Une sonde jetable a chronométré chaque
instruction de `DeleteRecordNode` :

| instruction | avant | vecteur remis à NULL d'abord |
|---|---|---|
| lire les morceaux d'un scope (par `_parent_uuid` ou par l'arête) | 2 à 8 ms | — |
| remettre le vecteur à NULL (1 morceau / 45 morceaux) | — | 7 ms / 251 ms |
| `DETACH DELETE` des morceaux d'un scope (1 morceau) | 2 095 à 2 937 ms | 4 à 6 ms |
| `DETACH DELETE` des morceaux de 32 scopes (45 morceaux) | 112 351 ms | 8 ms |
| relire puis supprimer le parent, désindexer le plein texte | < 10 ms | < 10 ms |
| réindexer un fichier, de bout en bout | 2,5 à 3,9 s | 0,73 à 1,03 s |
| retirer un fichier de 32 scopes | 96 à 114 s | 0,3 s |

Ce n'est donc ni le balayage par `_parent_uuid` (sans index) ni la suppression du parent :
c'est l'effacement, dans l'index HNSW, d'une ligne qui y figure encore.

Autre chiffre de la même passe, à part : l'index initial a passé 309 s sur 329 à écrire
15 631 vecteurs (`write_ms` du nœud `embed`), soit environ 20 ms par vecteur.

## Le contournement (rag3weaver)

`SchemaDialect::vector_columns_to_clear` décrit la table (`CALL table_info`). Pour chaque
tableau de taille fixe, `clear_vector_by_parent` remet la colonne à NULL sur les morceaux des
parents cités, juste avant le `DETACH DELETE`. C'est fait dans `DeleteRecordNode` (retrait d'un
scope ou d'un fichier) et dans `RechunkDeleteNode` (redécoupage). PostgreSQL ne le fait pas.

Le `SET` à NULL par lot est l'opération que `record_nodes::write_vectors` évite pour une ligne
qui garde sa place (l'index en perd des lignes). Ici, les lignes partent juste après :
ce qu'il ferait perdre ne concerne qu'elles. `e2e_invariant_des_vecteurs` doit rester vert.

## Ce qu'il faut pour le fermer

Un `DETACH DELETE` d'une ligne indexée en quelques millisecondes sur un index de cette taille,
mesuré par le même test. Alors le contournement s'enlève : les deux méthodes du dialecte et
`vider_les_vecteurs_avant_suppression`.
