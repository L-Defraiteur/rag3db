# Rapport de la nuit du 15 au 16 novembre 2026 (hotfixs)

Session « coeur c++/hotfixs/tickets », sur l'orchestration de c7. Tout ce qui suit est sur master, poussé par la
garde. Les commits sont sous l'identité gmail, sans attribution.

## Ce qui a atterri

| Lot | Tête sur master | Ce qu'il change | Preuve |
| --- | --- | --- | --- |
| L'extension notée qui ne se charge pas se dit à l'ouverture | 7ece05d8a | `avertissement_d_extension`, lu dans `extension_error` de SHOW_INDEXES | témoin rouge sur master, passe entière verte |
| Le journal des runs s'écrit d'un coup | 08643d923 (avec le test shell) | temporaire puis `rename` dans `ecrire_le_fichier` | témoin rouge (24 puis 11 runs listés illisibles), vert ; CI `rag3weaver` verte sur 457892de9 |
| Le test shell instable | 08643d923 | historique isolé par `-p`, flèche haute à la place du troisième Ctrl-P | rouge 5 sur 5 sur un historique vide, puis 100 répétitions vertes |
| La suite du shell n'écrit plus dans l'historique de la personne | 1b2be095c | tout `ShellTest` reçoit son dossier d'historique | témoin rouge (`history.txt` créé dans l'arbre), vert ; premier passage du mode tests seuls |
| Le fichier de déversement repart de la page 0, et `clearFile` ne vide plus sous un groupe déversé | 5dc3247c8 | `FileHandle::restartAtPageZero`, et le compte des tampons sur disque | deux témoins rouges (un fichier creux ; 4 valeurs perdues en silence), passe C++ verte |
| Le nom d'un fichier créé survit à une coupure | 0818e6819 | `FileSystem::syncDirectory`, appelé par `VirtualFileSystem::openFile` à la création | témoin rouge (une base neuve disparue entière), coût +1,6 ms par cycle de point de reprise |
| La suppression du journal par un point de reprise | c6bf19dda | aucun correctif : sans dommage | témoin gardé, qui exige que le fichier soit revenu |
| Le shell ne perd plus de caractères d'une entrée par pipe | 843bda91a | `getCursorPosition` ne questionne qu'un terminal | témoin rouge 3 sur 3 (`CHECKPOIEATE`, à l'octet 4096), suite du shell 246 sur 246 |

## Ce qui est passé en confort, ou fermé faux

- **Le SIGSEGV d'`optimisticRead`** : en confort. Huit jours sans core ni garde déclenchée, et aucune base neuve
  creuse sur 340. Le premier événement reste inexpliqué ; la piste qui reste est l'écriture partielle du COPY borné.
- **« Une passe disparaît sous poste lourd »** : un faux constat de ma part. J'avais passé le dossier des journaux en
  chemin absolu, et le script l'emboîtait.

## Les tickets d'outillage posés pour G (tous fermés par lui dans la nuit, sauf le dernier)

Le chemin absolu de `passe_epinglee.sh` ; le défaut de 40G de `poste` ; le témoin de la lib par `run_e2e.sh` ; le
mode tests seuls de la garde ; et, ouvert, `tools/shell` dans la voie C++.

## Les pièges de la nuit, à ne pas refaire

- Un journal de passe se nomme, il ne se donne pas en chemin absolu.
- Un bâti C++ de `transaction_test` doit bâtir aussi `rag3db_vector_extension`, sinon dix tests tombent sur leur
  `LOAD EXTENSION`.
- Dans une pile, ne pas supprimer la copie ouverte d'un ticket que master a modifiée : le rebase de la garde bute.
  La garde retire elle-même les copies ouvertes.
- Après le rebase de la garde, `git submodule update` dans le worktree, sinon le premier contrôle montre
  `codeparsers` modifié.
- La lib d'un lot du moteur seul est le BUILD_ID qu'écrit le binaire joué (`CALL BUILD_ID()` pour le shell), jamais
  une ligne écrite à la main.
- Un témoin vert qui ne vérifie pas que son cas a été joué ne prouve rien : deux fois cette nuit, il a fallu
  resserrer ou compléter un témoin avant de croire son verdict.

## La suite

Reprendre avec le cœur la liste des défauts par gravité, après son passage sous ASan (vers 7 h 15).
