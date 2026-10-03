# Le backend de code — un moteur, deux politiques

Deux manifestes du même moteur, qui ne diffèrent que par leur `workspace`
et leurs politiques d'outils :

- **`backend.json`** — la politique de **poste** : l'arbre de travail réel
  (`working_tree`) sous `workspace/`, l'édition sur le disque, les
  commandes derrière la porte en mode `approbation`.
- **`snapshot.json`** — la politique **cloud** : un instantané chargé en
  mémoire à l'ouverture (un dépôt cloné s'y copie), l'édition en mémoire,
  pas de commandes.

Le schéma de code (`File`/`Scope`) **n'est pas déclaré ici** : la vérité
vit dans le moteur (`register_code_schema`), enregistrée à l'ouverture dès
qu'un `workspace` est déclaré (`index: false` pour refuser). Un schéma de
payload ne sait pas dire `Text`, une copie JSON dériverait ; les entités
**métier** d'un produit, elles, restent déclaratives.

Préparer : créer `workspace/` (ou pointer `root` ailleurs), poser
`RAG3WEAVER_EMBED_SERVICE` ou l'adresse du démon dans `embeddings.address`.
Les verbes d'amorçage `estimate`/`index` arrivent avec l'API d'indexation
en deux temps (doc du 3 octobre, « indexer ce dépôt ») ; en attendant, un
fichier édité est réindexé à l'édition.

La tuyauterie s'éprouve sans modèle : `scripts/test_backend_code.py`.
