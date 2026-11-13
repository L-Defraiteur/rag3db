# Un test du raccord physique d'index rougit sous la lib entière, vert seul

- **État** : corrigé par `d57273bba` (13 novembre 2026, arbre principal) : l'écrivain du raccord passe par le registre des écrivains, et les photos d'octets attendent la fin de la fermeture. Ouvert le 13 novembre par archi.
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

## Le départage du 13 novembre au soir (archi)

Sur master `4b4233853`, sous la **lib neuve 207701bb2, sans `MOTEUR_ANCIEN`**, et sous `poste mesure` (le
poste à lui seul, aucune autre passe), le jeu large (`rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm`) :

- tour 1 : 2307 passés, **1 échec** (ce test), 40,08 s ;
- tour 2 : 2307 passés, **1 échec** (ce test), 41,07 s.

La piste de la lib tombe : le rouge tient sous la lib neuve. Celle de la charge faiblit : le poste
n'avait aucune autre passe. Reste ce que la lib entière fait que le test seul ne fait pas, dans le
même processus : un autre test qui écrit dans les fichiers ouverts, un état global partagé, ou un
ordre des tests. À départager en jouant le module `physical_index` entier, puis par moitiés.

## La cause et le correctif (arbre principal, 13 novembre au soir)

**Ce qui fait rougir.** Le rouge vient de l'environnement de la passe, pas du poste.
`RAG3WEAVER_EMBED_SERVICE` vers trois services joignables et `LUCIVY_SCHEDULER_THREADS=8` :
rouge à chaque tour sur lucie-tablette. Sans eux : vert quatre tours sur quatre. Sous cet
environnement, `native_legacy_foreign_schema_and_values_refuse_owned_open_without_writes`
(sa photo d'octets, cas `change == 0`) rougit de la même façon. C'est le rouge de CI du ticket
`docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md`.

**Ce qui écrit.** Après le `drop` d'un `OwnedPhysicalIndex`, la dernière référence à sa
`rag3db::Database` tombe plus tard, sur un fil de fond. `~Database` fait alors son point de
reprise de fermeture : il réécrit `index.rag3db` et retire le `.wal`. Dans les fixtures rouges,
`index.rag3db` est daté 12 à 30 ms après le `.readers`, et le dossier change aussitôt après
(retrait du `.wal`). Si ce point de reprise passe après la photo du test, les octets diffèrent.
Une réouverture en écriture qui arrive avant est refusée (« already open for writing in this
process »).

**Pourquoi le raccord seul.** L'ouverture ordinaire en écriture (`open_for_writing`) s'inscrit au
registre `OPEN_FOR_WRITING` et attend jusqu'à `REOPEN_WAIT` la mort de l'instance précédente.
L'écrivain du raccord (`with_owned_physical_buffer_pool`) ouvrait par `open_explicit_config`, hors
du registre et sans attente.

**Le piège du `Weak<Database>`.** Le compte fort d'un `Arc` tombe à zéro *avant* que le
destructeur ne tourne. Une attente sur `Weak<Database>` voit donc la base morte pendant que
`~Database` écrit encore : le rouge revient à la photo même (vu, tour 2 du 13 novembre à
16 h 41). Chaque `Rag3dbConnection` tient un clone de la base puis un clone de son `ReopenState`,
et les relâche dans cet ordre. Le dernier `ReopenState` tombe donc après le retour du destructeur.

**Le correctif (`d57273bba`).**
- L'écrivain du raccord s'inscrit au registre et attend comme l'ouverture ordinaire, sans poser la
  marque `.ecrivain` : la racine possédée ne contient que ses propres fichiers.
- Le registre tient le `Weak<Database>` et le `Weak<ReopenState>`.
- `wait_writer_released(chemin, délai)` dit quand les fichiers sont au repos. Les tests du raccord
  l'appellent (`at_rest`) avant chaque photo qui suit un `drop` dans le processus.
- Témoin neuf : `owned_writer_reopen_waits_for_a_previous_instance_released_late`. Il garde
  l'instance vivante 300 ms après le `drop`, puis rouvre. Il est rouge sans le correctif.

**Les relevés.** Lib `207701bb2`, sans `MOTEUR_ANCIEN`, jeu large, 8 jobs, environnement d'archi :
- avant le correctif : 2308 passés et 1 échec, deux tours sur deux ;
- après : 2309 passés, 0 échec, 67 ignorés, trois tours sur trois.

**Ce qui reste.** Le porteur qui garde la base après le `drop` n'est pas nommé dans ce ticket
(poignée plein texte, fil de fusion lucivy ou `read_connection`). Le registre rend l'attente juste ;
il ne raccourcit pas la vie de l'instance.
