# Rapport de la session « mémoire » (96) — état au 14 novembre 2026

Ce rapport s'adresse à un agent qui reprend ce périmètre sans souvenir de nos échanges. Il remplace
pour l'état celui du 11 novembre (`docs/memoire/11-11-2026/01-rapport-de-session.md`), qui reste
l'histoire de la semaine. Le vocabulaire est défini dans
`docs/memoire/11-11-2026/02-knowledge-dump.md` (§ 1). Ce que la semaine a appris de plus est dans le
second fichier de ce dossier, `04-knowledge-dump.md`.

- **Lucie** : la propriétaire du projet. Elle seule décide de ce qui touche sa mémoire, ses bases
  et ses postes.
- **c7** : la session d'orchestration, nommée « rag3db-5a ». Son adresse du 13 et du 14 novembre est
  `uds:/run/user/1000/cc-socks/3576675.sock`. Elle ordonne les pushes et relaie les décisions de
  Lucie.
- **G** : la session « optimiseur ». Elle pose les libs et les binaires sur les deux postes et
  redémarre les hôtes.
- **Un blanc** : un archivage à blanc, c'est-à-dire l'outil `archive` joué sur une copie de la
  mémoire de Lucie, jamais sur sa base.

## 1. Ce qui est sur master (13 et 14 novembre)

| commit | ce qu'il fait |
|---|---|
| `c89e3f769` | les défauts de M8 (le plan de gabarits refuse ce qu'il ne sait pas comparer) |
| `cd51ab5cd` | l'archiviste voit les fiches proposées plus haut dans son document (`carry`) et nomme ses sujets (`name_as`) |
| `25c6b63d5` | le dédoublonnage par le nom de sujet : le voisinage des noms et l'étage `archive_subject` |
| `7961fae7f` | les garde-fous de la voie 3 : une fiche par chantier dans un journal, et la fiche courante rejointe reste courante |
| `c3864d6f5`, `f00210dc9`, `1c261850a` | les pages des trois blancs (02 et 03 de ce dossier, avec leurs annexes) |
| `08274c89d` | le « quatrième cas » de la page M8 (`recall_fiches` sans `via` chez Lucie) |

Les tickets fermés de ces lots sont sous `docs/tickets/closed/2026-11-13-*` : le titre de section
devenu nom de fiche, le même sujet sous deux formulations, le journal qui fait une fiche trop
large, et les quatre défauts de M8.

## 2. Ce qui n'est pas sur master

Rien. Aucune branche de lot de 96 n'attend, aucune remise git n'est à moi. La remise « recréée par
G » est entrée sous `08274c89d` puis a été supprimée.

## 3. Les décisions de Lucie (14 novembre), et ce qui en reste

**La voie 3** : l'archiviste crée un sujet quand aucune fiche ne le porte, avec le dédoublonnage.
« Il crée un nouveau sujet, c'est le but ; l'important c'est que ce soit bien rangé. » Elle l'a
accepté avec deux garde-fous :

1. **Une fiche créée reste `proposed`** jusqu'à sa relecture. C'est déjà le cas : seule la
   transition `accept` du lifecycle de Fiche mène à `current`. C'est fixé par le témoin
   `une_fiche_courante_rejointe_reste_courante`. La page de chaque passe liste les fiches créées.
2. **Un document journal donne une fiche par chantier, pas par moment.** Codé sous `7961fae7f`.

**Question ouverte, portée à Lucie par c7** : « une citation confirme une fiche ». Aucune promotion
par citation n'existe aujourd'hui, et elle n'est pas codée : c'est un changement de conception qui
attend son mot.

**Non codé, piste au ticket** : la fusion des fiches `proposed` voisines par le sens, par exemple
« Hooks Claude Code pour la mémoire » et « Hooks rag3weaver ».

## 4. Le chantier en cours : la première vraie passe sur la mémoire de Lucie

L'ordre donné par c7 le 14 novembre :

- (a) les deux garde-fous, rouges d'abord : **fait**, `7961fae7f` ;
- (b) un instantané complet de sa base avant la passe (base, wal, extensions, .fts), nommé et dit ;
- (c) la passe sur sa mémoire, les mêmes 432 sections, au régime GPU des passes, à luciepc ;
- (d) la page : les fiches créées, les ancrages, ce qui a rejoint une fiche existante ;
- (e) la fusion par le sens reste non codée.

**(b) et (c) attendent le mot de Lucie elle-même**, pas un relais. c7 lui a porté la demande : le
chemin de sa base, et un créneau où son hôte de mémoire longue peut s'arrêter pour l'instantané.

Sur le portable, la base que sert son hôte paraît être
`~/.local/share/rag3weaver/memoire-longue.rag3db`, avec ses fichiers sous `~/memoire-longue`. C'est à
confirmer avec elle avant tout geste ; rien n'y a été lu ni écrit. Les trois blancs ont joué sur
une copie de l'instantané pris par G, hôte arrêté, le 13 novembre à 15 h 54.

Il reste aussi à savoir **où la passe écrit**. Une passe « à luciepc » sur une base du portable
demande soit de jouer à luciepc sur une copie puis de la rendre, soit de jouer au portable. C'est à
trancher avec Lucie et G.

## 5. Ce qui reste sur disque

À luciepc :

- `~/.cache/rag3weaver-build/blanc-96`, `blanc-96-2`, `blanc-96-3` (environ 280 Mo chacun) : les trois
  blancs. Chacun a sa `copie/` (après la passe), sa `copie-origine/` (avant), ses `templates/`, ses
  `src/` (les 52 documents figés), son `lancer.py`, ses `resultats.jsonl` et `apports.json`.
- `~/.cache/rag3weaver-build/target-96-m8` (8,7 Go) : le target de mes tours et témoins, avec
  `debug/rag3weaver-backend` bâti sur `25c6b63d5`.
- `~/.cache/rag3weaver-build/fixtures-gardees/physical-crash-2026-11-14/` (1,6 Mo) : la fixture du
  test instable, pour arbre principal.
- Les copies d'avant mes retouches d'outillage : `tour.sh.avant-tour-lib`,
  `archfil-temoin.sh.avant-tour-lib`, `bin-blanc.sh.avant-tour-lib` et
  `instables.txt.avant-crash-physique`.
- `~/git_workspaces/rag3db-p1r-96` : l'arbre du tour, détaché sur `7961fae7f`, propre. Le tour le
  remet à zéro lui-même.

Sur le portable :

- `~/.cache/rag3weaver-build/wt-mf-96` : mon worktree, sur une branche de docs, propre une fois ce
  rapport poussé.
- `~/.cache/rag3weaver-build/target-96` (19 Go) : un ancien target, qui ne sert plus aux passes. Il
  peut être effacé sur le mot de G, qui tient le disque.

## 6. Mes tickets ouverts

- `docs/memoire/tickets/2026-11-14-l-enfant-du-crash-physique-ne-rend-pas-son-code-sous-la-charge.md`,
  pour arbre principal. c7 le lui a porté.
- `docs/memoire/tickets/2026-11-11-terminer-l-archivage-a-blanc-des-documents-sur-copie.md` : les
  trois blancs sont faits. Il reste à le clore, ou à le faire suivre par la vraie passe (§ 4).
- Les autres tickets ouverts sous `docs/memoire/tickets/` : M9 (l'archiviste automatique du chat),
  le classeur local, M8 et ses suites. Ils sont inchangés depuis le 11 novembre.

## 7. Reprendre

Aucun lot n'est entamé. Pour repartir :

**La vraie passe (§ 4), au mot de Lucie.**

1. L'instantané, son hôte arrêté, en copiant ensemble base, wal, extensions et .fts, plus le dossier
   des fichiers. Il prend un nom daté sous `~/.local/share/rag3weaver/instantanes/`, que l'on dit à
   c7 et à Lucie.
2. La passe se monte comme un blanc. Prendre `~/.cache/rag3weaver-build/blanc-96-3/lancer.py` et son
   manifeste (champs `database`, `files_root`, `vector_extension`, `models.llm`), les pointer sur la
   base que Lucie aura désignée, et lancer sous `poste lourd` au régime des passes :

       ssh lucied@luciepc 'systemd-run --user --unit=passe-96 --collect --working-directory=<dossier> \
         --setenv=RAG3WEAVER_GPU_DUTY=70 --setenv=RAG3WEAVER_EMBED_CHAR_BUDGET=4096 --setenv=POSTE_MEM_MAX=32G \
         bash -c "~/.cache/rag3weaver-build/poste lourd python3 lancer.py <dossier> <dossier>/src > lancer.log 2>&1"'

3. Les nombres se calculent par le script d'analyse des blancs : `select_apports` filtré sur
   `via = archiviste`, puis l'avant et l'après des fiches. Il est décrit au knowledge dump, § 4.

**Un lot de code.** Le tour se lance ainsi :

    ssh lucied@luciepc 'systemd-run --user --unit=<lot>-rejeu-96 --collect \
      --setenv=TOUR_TARGET=/home/lucied/.cache/rag3weaver-build/target-96-m8 \
      --setenv=TOUR_LIB=<lib du magasin> --setenv=POSTE_MEM_MAX=32G \
      bash ~/.cache/rag3weaver-build/tour.sh <lot> rejeu'

Puis `push` au lieu de `rejeu`, au « à toi » de c7. Les fichiers du lot sont expliqués au knowledge
dump, § 3.
