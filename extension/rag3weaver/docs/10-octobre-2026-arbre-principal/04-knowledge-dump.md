# Arbre principal — ce qu'il faut savoir (11 octobre 2026, nuit)

## La durée de la lib complète

`cargo test --lib`, toutes les features, sur le même arbre (master `1bf29930d` + le retrait) et
la même lib (04:33) :

| | 1ʳᵉ passe | 2ᵉ passe |
|---|---|---|
| avec le filet (`force_checkpoint_on_copy=true` sur toute connexion) | 25,29 s | 25,36 s |
| sans le filet | 21,41 s | 21,46 s |

- **Le filet coûtait environ 4 s** : chaque COPY des tests unitaires payait un point de reprise.
- L'orchestration a vu la lib passer d'environ 10 s (à `0d8482f70`) à environ 24 s. **Environ
  11 s ne sont pas imputés.** Suspects, non mesurés : les tests ajoutés pendant la nuit (le
  réacteur et l'atelier d'everything declarative, la commande en fond, et le témoin du COPY
  journalisé, qui fait 6 COPY de 200 000 lignes). C'est un confort, pas une enquête de nuit.

## Le COPY journalisé et ses lectures

- Un COPY de plus d'un groupe (131 072 lignes) écrit ses groupes pleins dans le fichier avant
  sa validation. Le journal n'est écrit qu'à la validation. Une base qui grossit pendant que le
  journal reste à quelques Ko est donc un paquet **en cours**, pas un journal évité.
- Le seuil du journal d'un COPY est tiré du tampon : un huitième, au plus 256 Mio
  (`Transaction::copyJournalThreshold`). Sous un petit tampon, un gros COPY se replie sur le
  point de reprise forcé : une sonde à petit tampon ne voit pas le chemin journalisé.
- `auto_checkpoint`, `force_checkpoint_on_copy` et `copy_journal_threshold` sont des réglages
  **de connexion**. Le magasin de blobs de rag3weaver passe par la connexion du catalogue
  (`set_sync_connection` n'est jamais appelé).
- Ce qui force le point de reprise d'une transaction :
  - `force_checkpoint_on_copy` (`client_context.cpp`) ;
  - un COPY de relations avec IGNORE_ERRORS ;
  - un COPY de nœuds qui a écarté des lignes ;
  - le repli au seuil (nœuds et relations) ;
  - `CREATE_VECTOR_INDEX` (`create_hnsw_index.cpp:299`).
- La trace `RAG3DB_PROFILE_JOURNAL=1` dit, à chaque validation, le poids du journal et si la
  transaction a été forcée.

## Les sondes laissées

- `~/.cache/rag3weaver-notes-A/etendue/variantes.cpp` et `sonde.cpp` : la corruption du COPY.
  Elles sont vertes sur la lib corrigée.
- `e2e_tx_par_paquet_arret::sonde_le_premier_paquet_quitte_le_journal_par_l_index_vectoriel_releve`
  (branche `temoin-fuite-3`) : `TX_SONDE_TAILLE=FxN`, `TX_SONDE_AVEC_SEULEMENT=1`.
