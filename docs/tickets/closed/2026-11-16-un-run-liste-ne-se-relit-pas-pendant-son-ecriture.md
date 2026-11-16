# Un run listé par le journal ne se relit pas pendant son écriture

- **État** : fermé (16 novembre 2026, hotfixs) : le lot est sur master (e5e9e7f81, b896f47a0, 6305dec0d, dans 08643d923), et la CI `rag3weaver` de master est verte sur 457892de98 (run 43782802364), qui le contient.
- **Famille** : produit
- **Gravité** : la CI `rag3weaver` de master est rouge depuis 7ece05d8a (run 43773815569), et le défaut atteint
  aussi le service. Un lecteur du journal des runs (l'IDE, le chat, `runs_de` puis `lire`) peut tomber sur un run
  listé dont le fichier est encore vide ou tronqué.
- **Atteignable en service** : oui, dès qu'un run se lit pendant qu'il s'écrit (un ajout tardif réécrit aussi son
  fichier)
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/journal_des_runs.rs`)
- **Pour** : hotfixs

## Ce qu'on voit

Le job « check_features (cinq jeux) et tests unitaires » rougit sur un seul test, 1 échec pour 2480 :
`daemon_host::atelier_fichiers::tests::a_workshop_write_leaves_a_run_with_its_diff` panique à
`atelier_fichiers.rs:1350`, sur « le run se relit ». Ce test existe depuis c32c5b8d8 et était déjà dans le dernier vert,
e692fa3a58 : aucun des commits d'entre les deux n'en est la cause, c'est le minutage du runner qui l'a fait sortir.

## La cause

`ecrire_le_fichier` faisait un `std::fs::write(p, …)` direct : le fichier est créé vide, puis rempli. Pendant cette
fenêtre, `runs_de` le liste déjà (il suffit d'un nom en `.json.zst`), et `lire` rend `None`, faute de décoder un zstd
vide ou tronqué. Le test attend qu'un run neuf soit listé, puis le relit aussitôt. Les trois écritures du journal
passent par là : `deposer`, `ecrire` (fin d'un run racine) et `reecrire` (ajouts fondus).

## Le témoin

`journal_des_runs::tests::a_listed_run_always_reads_while_it_is_written` : un lecteur relit chaque run listé qu'il n'a
pas encore vu entier, pendant que 60 dépôts de ~4 Mio s'écrivent. Il est rouge sur master (24, puis 11 runs listés
qui ne se relisent pas) et vert sur le correctif (3 sur 3).

## Le correctif

Le journal s'écrit dans un temporaire du même dossier (`.<nom>.<pid>-<n>.tmp`), puis prend son nom par un `rename`.
Le temporaire ne finit ni en `.json` ni en `.json.zst` : ni `runs_de` ni l'inventaire ne le comptent.

## Pour fermer

Le lot sur master, et la CI `rag3weaver` de master verte.

## Fermeture (16 nov.)

- **Sur master** : e5e9e7f81 et b896f47a0 (les témoins), puis 6305dec0d (l'écriture atomique), atterris dans 08643d923
  après une passe entière verte à luciepc et un rejeu court vert sur la tête reposée (2550 passés).
- **La CI `rag3weaver` de master** est verte sur 457892de98 (run 43782802364), le premier run fini qui contient le
  correctif. Les runs de 08643d923 et 988651d39 avaient été annulés par les pushes suivants.
- **Ce vert confirme sans prouver** : le rouge était intermittent, et deux runs avaient été verts avant le correctif
  (c33574eb9, run 43777926518, et 9f0cbe7ce, run 43780138690). La preuve est le témoin
  `a_listed_run_always_reads_while_it_is_written`, rouge sur master (24, puis 11 runs listés qui ne se relisent pas)
  et vert avec le correctif.
