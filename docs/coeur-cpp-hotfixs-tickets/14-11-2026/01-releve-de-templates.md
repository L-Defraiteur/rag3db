# Le relevé de `templates/` pendant une lib entière (14 novembre 2026)

Session hotfixs, à la demande de c7 (l'orchestration), pour le ticket d'arbre principal
`docs/arbre-principal/tickets/2026-11-13-un-fichier-de-plus-apparait-sous-templates-pendant-la-lib-entiere.md`.
L'outil est confié à arbre principal, qui l'enveloppe autour de ses libs entières au portable
jusqu'à ce que le fichier revienne.

## Le problème

Une fois, au portable, le test `builtin_templates::tests::every_shipped_template_is_embedded_and_sorted`
a compté 224 fichiers sous `extension/rag3weaver/templates/` au lieu des 223 que git suit. Après coup,
le fichier de trop avait disparu. Un tri final des noms ne peut pas le voir : il faut écrire chaque nom
**au moment où il apparaît**.

## Les deux outils (`tools/poste/`)

- **`releve-templates.py <racine> <journal>`** : le relevé lui-même. `inotifywait` n'est pas installé au
  portable, donc le script appelle inotify du noyau par `ctypes`, sans dépendance. Il surveille la racine
  et tous ses sous-dossiers, y compris ceux qui apparaissent en route. Il écrit une ligne par événement
  (`HH:MM:SS.µs QUOI chemin`) et la vide sur disque aussitôt (`fsync`). QUOI vaut :
  - CRÉÉ, ÉCRIT, EFFACÉ, PARTI ou ARRIVÉ ;
  - VU-DÉJÀ-LÀ : un fichier déjà écrit dans un dossier neuf quand sa surveillance a été posée ;
  - DOSSIER-EFFACÉ, SURVEILLANCE-LEVÉE ;
  - DÉBORDEMENT : la file du noyau a perdu des événements, et le relevé ne vaut plus.

  Il s'arrête sur SIGTERM ou SIGINT, en écrivant FIN.
- **`lib-entiere-releve.sh <worktree> [dossier des relevés]`** : la lib entière de rag3weaver, enveloppée
  du relevé. Le script fait cinq choses :
  1. il note les fichiers de `templates/` que git suit ;
  2. il lance le relevé et attend sa ligne DÉBUT ;
  3. il joue la lib entière, `scripts/cargo_magasin.sh test --lib`, avec les features des tours (ou
     `FEATURES`) et la lib `RAG3DB_LIB`, par défaut `courant` ;
  4. il arrête le relevé ;
  5. il liste les noms apparus que git ne suit pas.

  Les relevés vont dans `~/.cache/rag3weaver-build/releves-templates/<date-heure>/` (`entete.txt`,
  `releve.log`, `lib.log`, `suivis.txt`, `inconnus.txt`). Code de sortie : celui de la lib, ou 3 si des
  noms non suivis sont apparus.

## L'usage

```bash
cd ~/.cache/rag3weaver-build && ./poste lourd timeout 4000 \
  <worktree>/tools/poste/lib-entiere-releve.sh <worktree>
```

Le fichier de trop, s'il revient, est la ligne `NOMS NON SUIVIS :` de la sortie, et son heure exacte
est dans `releve.log`. La ligne de `lib.log` la plus proche dans le temps désigne le test qui tournait.

## Ce qui a été éprouvé

- Sur un dossier jetable, le relevé voit :
  - un fichier créé, écrit puis effacé en moins de 3 millisecondes (`CRÉÉ a/x.lock`, `ÉCRIT`, `EFFACÉ`) ;
  - un fichier écrit dans un dossier neuf avant la pose de sa surveillance (`VU-DÉJÀ-LÀ n/y`).
- Pendant deux libs entières au portable, master et lib 835d5298f, il n'a vu **aucun** événement sous
  `templates/` (55 dossiers, 230 fichiers) : le fichier ne s'est pas montré. Un passage vert ne nomme pas
  le fichier ; le ticket reste ouvert.
- Le seul rouge de ces libs, `backend::tests::a_read_during_a_reindexed_batch_sees_the_batch_before_whole`,
  est le ticket connu de recherche (le lot vu à moitié), sans rapport avec `templates/`.

## Les limites

- inotify ne dit pas **quel processus** a écrit : le nom du fichier et l'heure, comparés au journal de la
  lib, désignent le test.
- Un fichier écrit hors de `templates/` puis déplacé dedans se lit `ARRIVÉ`, ce qui est bien vu.
- Une file du noyau débordée, sous une avalanche d'événements, se lit `DÉBORDEMENT` : le relevé de ce
  passage ne vaut alors plus.
