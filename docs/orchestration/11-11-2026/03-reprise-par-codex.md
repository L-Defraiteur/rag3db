# Reprise par un autre agent d'orchestration (Codex), 11 novembre 2026

Cette page est faite pour être donnée telle quelle à une session d'orchestration neuve, sans aucune
mémoire de nos échanges. Elle dit ce qu'elle doit lire, ce que sont les sessions qu'elle orchestre,
comment elle les démarre, et ce qui est en attente. Les amorces de la fin se collent dans chaque
session de travail.

## 1. Ce que l'orchestrateur lit d'abord, dans cet ordre

1. `docs/README.md` : l'organisation et les règles qui ne se discutent pas.
2. `docs/orchestration/11-11-2026/02-knowledge-dump.md` : les dépôts, les postes, la lib épinglée, la
   passe, le push, les verrous, les mesures, les décisions de Lucie.
3. `docs/orchestration/11-11-2026/01-rapport-de-session.md` : l'état au 11 novembre, session par session.
4. `docs/procedures/push-d-un-lot.md` : la procédure de passe et de push, à faire respecter mot pour mot.
5. `docs/nouveautes.md` (ce qui est atteint) et `docs/orchestration/visions/README.md` (où on va).

## 2. Le rôle de l'orchestrateur

- Il tient **la file des pushes sur master** : un lot atterrit à la fois ; il donne le tour par le mot
  « à toi » ; la session répond « poussé » avec le hash et les trois contrôles lus (status propre,
  aucun rebase en cours, `origin/master` = HEAD). Les docs seules se poussent sans tour.
- Il fait **relire master avant chaque push** : si du Rust est arrivé sous `extension/rag3weaver`
  depuis la passe, la session rejoue court ; du C++ du moteur ou des docs seuls : rebase sans rejeu.
- Il **nomme la lib** à toutes les sessions quand du C++ du moteur atterrit (une lib neuve au magasin,
  identifiant complet, posée aux deux postes par la session optimiseur).
- Il tient **le rapport d'orchestration** à jour après chaque atterrissage (hash, ce que ça change) dans
  `docs/orchestration/<date>/01-rapport-de-session.md`, et pousse ses docs lui-même.
- Il **ne code pas** et ne lance pas de passe : il lit, décide, relaie, et dit à Lucie quand regarder.
- Il **dit non** à ce que les règles interdisent (push en force, attribution, amont, tag lucivy, sudo,
  effacer sans le mot de Lucie) et remonte à Lucie tout refus de permission, sans le contourner par
  une autre session.

## 3. Les sessions qu'il orchestre

Chaque session porte le nom de sa barre chez Lucie, et son dossier `docs/<nom-en-kebab-case>/`. Elle
commence par lire `docs/README.md`, puis son `11-11-2026/01-rapport-de-session.md` (section
« Reprendre » : chaque worktree inachevé, son chemin, sa branche, son dernier commit, les commandes),
puis son `02-knowledge-dump.md`, puis ses tickets dans `docs/<nom>/tickets/`.

| session | dossier | premier travail (au 11 novembre) |
|---|---|---|
| everything declarative | `docs/everything-declarative/` | **le correctif des réclamations du rattrapage**, branche `reclamations-rendues` (9da8e4433), jamais compilé ni joué : rouge d'abord, passe, push — c'est lui qui rougit la CI par intermittence |
| recherche | `docs/recherche/` | arret-du-backend (branche `arret-du-backend-7`) : trancher le rouge des trois CSV de COPY d'un processus tué, rebase, rejeu court, push ; puis le journal des runs écrit après l'arrêt ; puis le Surveillant |
| rag3weaver archi | `docs/rag3weaver-archi/` | `ide-par-le-flux` (wt-flux, a9c5a3108) : rejouer la passe tuée (code 137), départager la lib entière lente, push ; puis le découpage au survol de la jauge ; relecture d'ED de l'atelier |
| coeur c++ | `docs/coeur-cpp/` | le témoin de l'annulation sous pool plein (`annulation-sous-pool-plein`, 6e7f11680, jamais joué) : le jouer rouge, puis le correctif ; fermer le ticket de l'index complet quand hotfixs a rejoué sa trace |
| coeur c++/hotfixs/tickets | `docs/coeur-cpp-hotfixs-tickets/` | rejouer la trace exacte des deux sens sous la lib 29a7bc52a (service 7882, hôte 7992 ; les outils dans `11-11-2026/outils/`) : c'est le vert qui ferme le ticket de l'index complet |
| mémoire | `docs/memoire/` | l'archivage à blanc des docs sur une copie de la base de Lucie (le 400 de Vertex se relit maintenant, lot 2d6a08a65) ; M8 après relecture d'ED ; le classeur local après le mot de Lucie |
| optimiseur | `docs/optimiseur/` | **poser le binaire de la tête de master** aux deux postes (lib 29a7bc52a inchangée), une coupure, préavis d'une minute ; tenir les libs, les services, les tunnels, le disque de luciepc |
| embarquement | `docs/embarquement/` | rien d'inachevé ; son ticket (granite-278m arrêté huit fois, cause illisible) |
| codeparsers | `docs/codeparsers/` | le lot require-js (pointeur 78888f5, branche `codeparsers/require-js`) : décider sur la mesure du 9 (lecture dans le rapport), passe, push ; le ticket import_binding |
| banc | `docs/banc/` | le banc Qwen3-4B en GGUF avec le llama.cpp déjà bâti de Lucie (`~/git_workspaces/llama.cpp/build-hip/bin/`, b9590), card0 seulement ; puis le décideur d'actions quand il y aura des accords journalisés |
| arbre principal | `docs/arbre-principal/` | wt-deplace (le garde-fou) et les mesures M3 à M5, décrits dans « Reprendre » |
| lucivy | `docs/lucivy/` | rien sans le mot de Lucie : PR #22, #23 et la branche des tickets attendent sa décision dans cette session |
| tty | `docs/tty/` | rien d'inachevé ; les thèmes suivants de l'interface terminal au premier fichier |
| hooks-essai (fermée) | `extension/rag3weaver/docs/07-novembre-2026-hooks-essai/` | sa page « la recherche de code par les hooks » attend le mot de Lucie ; à rouvrir seulement s'il le donne |

## 4. Ce qui attend Lucie (à lui rappeler, pas à faire à sa place)

- Les fusions lucivy (PR #22, #23, branche des tickets), dans la session lucivy.
- Un index complet de ses bases de code pour réparer l'écart des deux sens (le correctif empêche, il ne
  répare pas) ; sa carte, son heure.
- Lire : la page des vues par worktree, celle de l'atelier, celle des hooks, celle du classeur local.
- Le ménage des gros dossiers laissés sur disque (listés dans chaque rapport) et des 476 `livre.json` de
  témoins (ticket de mémoire) ; la suppression des branches distantes fusionnées.

## 5. Les postes au 11 novembre

- Lib aux deux postes : `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
- Binaire posé : 0b0ff221a ; master a avancé depuis (a6637eeee, be319fa75, 3e8c87aa5, ab8272deb, 2d6a08a65 et les docs) : une pose est due.
- Services à luciepc (card2) : 7878 granite-278m, 7879 bge-m3, 7880 granite-107m, 7882 granite privé
  d'hotfixs (laissé en route pour le rejeu). Tunnels du portable : 7979 → 7878, 7980 → 7879, 7981 → 7880.
  Jamais un cinquième service sur card2 ; card0 au mot de Lucie.
- `lourds-max` à luciepc : 4. fstrim hebdomadaire avec `--minimum 16M`, le dimanche à 4 h (posé le 11).

## 6. Amorces à coller dans chaque session de travail

Remplacer `<nom>` par le dossier de la session. Le texte tient en quelques lignes, exprès.

> Tu es la session « <nom de la barre> » du projet rag3db (`~/git_workspaces/rag3db`). Lis d'abord
> `docs/README.md`, puis `docs/<nom>/11-11-2026/01-rapport-de-session.md` (sa section « Reprendre » dit
> où reprendre, et avec quelles commandes), puis `docs/<nom>/11-11-2026/02-knowledge-dump.md`, puis
> tes tickets dans `docs/<nom>/tickets/`. Règles qui ne se discutent pas : identité git
> `Lucie Defraiteur <luciedefraiteur@gmail.com>`, aucune ligne d'attribution ; jamais l'arbre
> `~/git_workspaces/rag3db` lui-même (un worktree par lot) ; chaque commit poussé sur sa branche dans la
> minute ; master seulement par la procédure `docs/procedures/push-d-un-lot.md`, au mot « à toi » de
> l'orchestration, dans `if grep -qx VERT`, après les trois contrôles ; lib épinglée par son identifiant
> complet ; rouge d'abord ; un défaut connu non corrigé est un ticket. Dis à l'orchestration « rouge
> posé », « vert », « poussé <hash> », et tout blocage dans la minute. Premier travail : <la ligne de la
> table ci-dessus>.

Pour la session optimiseur, ajouter : « tu poses les libs et les binaires aux deux postes (luciepc puis
le portable), avec instantané et retour, annonce une minute avant ; tu nommes la lib à l'orchestration ».
Pour lucivy, ajouter : « aucune fusion dans main ni aucun tag sans le mot de Lucie dans cette session ».
