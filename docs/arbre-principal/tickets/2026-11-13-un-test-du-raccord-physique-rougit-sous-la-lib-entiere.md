# Un test du raccord physique d'index rougit sous la lib entière, vert seul

- **État** : ouvert (13 novembre 2026), relevé par archi pendant la passe du lot du README ;
  pour arbre principal (le chantier des couches), à la demande de c7.
- **Famille** : produit
- **Gravité** : passe rouge sans cause dans le lot qui la joue. Un rouge d'environnement qui
  bloque les pushes des autres sessions.
- **Atteignable en service** : à départager (voir les pistes).
- **Touche rag3weaver** : oui (`src/dataflow/migrations/physical_index/tests.rs`).

## Ce que c'est

`dataflow::migrations::physical_index::tests::native_normal_catalog_open_refuses_owned_profile_before_initialization`
est arrivé sur master avec `d55b5264c` (13 novembre 2026, « Reconstruire les stockages
physiques et ouvrir les blobs en lecture seule réelle »). Dans la lib entière, il rougit :

```
panicked at src/dataflow/migrations/physical_index/tests.rs:477:5:
normal Catalog initialization through an owned RO handle changed files
test result: FAILED. 2307 passed; 1 failed; 67 ignored
```

Joué seul (`cargo test --lib -- native_normal_catalog_open_refuses_owned_profile`), il est
**vert deux fois sur deux**, sur la même branche et dans le même target.

- **La lib** : `rag3db-build-29a7bc52a…` (9 novembre), avec `RAG3WEAVER_MOTEUR_ANCIEN=1`,
  puisqu'elle est plus vieille que les sources du moteur sur master.
- **Le poste** : le portable, sous `poste lourd`, avec `POSTE_MEM_MAX=32G`.
- **Le lot qui l'a vu** (`readme-refonte`) ne touche aucun code de produit.

## Les pistes

1. **La charge** : sous la lib entière, une écriture de fond (un autre test, un fil de la même
   base temporaire) toucherait les fichiers pendant l'ouverture en lecture seule.
2. **La lib** : le test attendrait un comportement du moteur que la lib du 9 novembre n'a pas.
   La lib neuve (sources `207701bb2`) arrive ; la rejouer dessus départagera.

## Ce qu'il faut pour le fermer

Le rejouer dans la lib entière sur la lib neuve. Vert : fermer, avec ce résultat. Rouge : la
cause (le fichier changé et qui l'écrit), puis le correctif ou la borne du test.
