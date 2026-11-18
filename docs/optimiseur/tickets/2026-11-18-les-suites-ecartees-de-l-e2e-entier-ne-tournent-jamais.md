# Les suites écartées de l'e2e entier ne tournent jamais

- **État** : ouvert (18 novembre 2026) — à G, au mot de c7
- **Famille** : produit (outillage des passes : `extension/rag3weaver/run_e2e.sh` et une passe de nuit)
- **Gravité** : un rouge peut dormir dans 18 suites qu'aucune passe ne joue.
- **Atteignable en service** : oui pour ce qu'elles couvrent (les embarqueurs burn sur la carte, le service d'embarquement en démon, les suites à vrai modèle)
- **Touche rag3weaver** : oui
- **Pour** : G (optimiseur)
- **Signalé par** : G, à la relecture du 18 novembre 2026 demandée par c7, après le rouge caché d'`e2e_tx_par_paquet_arret`

## Ce qu'on voit

`run_e2e.sh` écarte de l'e2e entier trois listes : `SUITES_CARTE_LOCALE` (la carte d'ici), `SUITES_A_MODELE` (un vrai
modèle) et `BANCS_A_SERVICES`. Ces suites ne tournent que nommées par `--test`. Le 18 novembre, aucune passe ne les
avait jouées depuis une semaine, aux deux postes, d'après les journaux de `~/.cache/rag3weaver-build` :
- carte locale, aucun journal en 30 jours : `e2e_burn_attention`, `e2e_burn_embedder`, `e2e_burn_fusion`,
  `e2e_burn_granite`, `e2e_burn_minilm`, `e2e_burn_multilingual_minilm`, `e2e_burn_ocr`, `e2e_burn_reranker`,
  `e2e_burn_xlmr_reranker`, `e2e_banc_bge_m3`, `e2e_demon_embeddings` ;
- vrai modèle, dernier journal il y a 10,8 jours : `e2e_avis_du_modele`, `e2e_banc_vertex_embedding`,
  `e2e_cloud_code_agent`, `e2e_cloud_schema_probe`, `e2e_conversation_a_plusieurs`, `e2e_lecture_mermaid` ;
- banc à services, il y a 9,0 jours : `e2e_banc_fiches_classees`.

## Ce qu'il faudrait

Une passe de nuit à part, `docs/optimiseur/scripts/nuit-ecartees.sh` (nom anglais si c'est un script neuf du dépôt :
`tools/poste/excluded_suites_night.sh`), qui joue chacune de ces suites seule, par `run_e2e.sh --test <suite>`, sous
`poste lourd` et `POSTE_MEM_MAX=32G`, avec ce qu'elle exige :
- la carte locale (`RAG3WEAVER_SANS_CARTE_LOCALE=0`), sur la carte libre que dit la VRAM, jamais à côté des services ;
- un vrai modèle (`RAG3WEAVER_SUITES_A_MODELE=1`) et ses identifiants, lus dans `.vault` ;
- la voie de chaque banc.

La passe écrit un journal daté par suite, et un verdict en une ligne par suite (vert, rouge, ou non joué avec sa raison).
Le matin, on lit ce verdict et on le dit à c7. Une suite non jouée est rouge. La liste des suites se lit dans les trois
tableaux de `run_e2e.sh`, et non dans une copie : une suite ajoutée à un tableau entre dans la passe d'elle-même.
