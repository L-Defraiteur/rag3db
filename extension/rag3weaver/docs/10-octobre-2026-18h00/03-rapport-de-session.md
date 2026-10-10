# Mémoire longue — rapport du 10 octobre, soir

Écrit en hâte : le disque sature et les worktrees vont être effacés. Ce qui
suit est l'état exact, pour que la reprise ne reparte pas de zéro.

## Ce qui est dans master

| commit | |
|---|---|
| `bd25d7238` | le filet des gabarits — un gabarit qui ne charge plus rougit la lib |
| `773c5528a` | la section `"reactions"` du manifeste : lecture, validation, cinq refus nommés |
| `fa5f13e2a` | le septième piège au journal, rapport et relevé |
| `d98758f19` | la nuance des variables dans `run_e2e.sh` |
| `aeec6888f` | la réparation des deux bancs à corpus vivant + les deux derniers sites du piège 6 |
| `f7b1df13b` | la contre-épreuve de la garde 2 + le ticket des échecs de chargement |
| `cada06ac3` | la correction du septième piège (il accusait un garde-fou qui existe) |

## Ce qui est sur `memoire-longue-4` et **pas vert**

`6ba8596de` — le serveur MCP. **À lire avant de reprendre** : le message de ce
commit dit précisément ce qui est éprouvé et ce qui ne l'est pas.

- **vert** : les 8 tests de `src/mcp.rs`, le protocole contre un hôte de
  fantaisie (ni base, ni manifeste, ni moteur) ;
- **jamais compilé ni joué** : la sous-commande `servir_mcp` du binaire, et
  `tests/e2e_mcp_stdio.rs`. Le dernier bâti s'est arrêté sur un `mut` manquant,
  corrigé dans le commit mais **non recompilé** — l'arrêt est tombé pendant la
  reconstruction.

**Les deux premières commandes de la reprise :**

```sh
cargo build --features rag3db-native,code,daemon --bin rag3weaver-backend
./run_e2e.sh --test e2e_mcp_stdio
```

## Ce qui reste du chantier H

1. rendre `e2e_mcp_stdio` vert (il n'a jamais tourné : il peut être faux) ;
2. le `.mcp.json` pour Claude Code, et l'épreuve réelle — trois cas : chercher
   dans ce dépôt, se souvenir d'une session à l'autre, et deux projets sur une
   mémoire globale. Le troisième passe par `--demon` ;
3. le premier test du mode `--demon`, nommé dans la page : **N `initialize`
   concurrents sur la même base par le démon**. `initialize` repose des DDL ;
   `IF NOT EXISTS` partout, mais à éprouver et non à supposer. Si c'est faux, le
   pont redevient la bonne réponse — pour une raison mesurée.

## Ce qui attend quelqu'un d'autre

- **le montage de `Reactor::watch_bound`** : rag3db-97 a poussé l'étape 2
  (`da25d8e0f`), la signature est figée et convenue — liaisons +
  `NodeTypePolicy::only(reaction_nodes())`. Avec le témoin « une réaction hors
  liste est refusée à l'exécution, pas seulement au chargement », puis la
  déclaration de `review_anchored` dans le gabarit `memory`, qui donne enfin le
  **premier** témoin de la proposition du 5 octobre ;
- **l'exposition par clés** (vision §7) : `--keys` est lu et dit, mais rien
  n'est encore écarté. Trois points écrits dans la page MCP et acceptés : une
  expression illisible refuse **au chargement** et jamais en silence ; une
  liste vide se dit avec son remède ; l'exposition décide *si* une réaction
  vit, la liste blanche *ce qu'elle peut faire*, jamais mélangées ;
- **le défaut de réouverture intermittente** (ticket du 4 octobre) : dort,
  faute d'une base réelle dans cet état. Quatre hypothèses éliminées.

## Ce que la journée a appris, au-delà du code

Trois choses sont allées au journal (« Méthode »), et elles ont toutes la même
forme — une information exacte qui répond à une autre question que celle qu'on
croyait poser :

- **le piège 8** : `git diff HEAD..origin/master` montre ses **propres**
  fichiers comme supprimés. J'avais reconnu l'artefact deux heures plus tôt et
  j'y suis tombée la fois suivante, quand il nommait un fichier auquel je
  tenais. C'est la commande qu'il faut changer, pas la vigilance ;
- **le piège 1 complété** : la recette par `fetch` depuis l'arbre principal
  échoue quand l'arbre principal est lui-même en retard sur le pointeur. La
  sortie est l'URL déclarée dans `.gitmodules`, **sans** desserrer
  `protocol.file.allow` ;
- **le piège 7 corrigé** : j'avais écrit que personne ne surveille l'âge de la
  bibliothèque. Faux — `run_e2e.sh` la compare au dernier commit de ses
  **sources**, et il me l'a prouvé en me refusant une passe. Annoncer un
  garde-fou manquant qui existe coûte autant que d'en manquer un.

Et le motif de la journée, rencontré **six fois** : une information produite que
rien ne consulte. `Reactor::watch` sans appelant ; `getRecoveryLoadFailures()`
public et jamais lu ; `DataflowRecorder` dont les seuls appelants sont des
tests ; l'uuid d'exécution fabriqué puis jeté dans la même fonction ; la prose
d'un banc annonçant 66 aiguilles et 45 questions quand il en avait 63 et 43 ;
`SearchOptions.scopes`, qui lui **est** branché et que j'ai failli accuser.

Le dernier cas est le plus instructif : j'ai présumé le défaut parce que je
venais d'en voir cinq. Un motif qu'on reconnaît bien devient un motif qu'on voit
partout.
