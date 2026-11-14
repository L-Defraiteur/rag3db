# La première passe de l'archiviste sur la mémoire de Lucie

*14 novembre 2026, session mémoire (96). C'est la voie 3, choisie par Lucie le 14 novembre :
l'archiviste crée un sujet quand aucune fiche ne le porte, avec le dédoublonnage par le nom. Elle s'y
ajoute ses deux garde-fous : une fiche créée reste `proposed`, et un journal donne une fiche par
chantier. La passe est écrite dans sa vraie mémoire. La liste des fiches créées, à survoler, est
l'annexe `01-annexe-les-fiches-creees.md`.*

**En une phrase : 95 fiches proposées et 374 sections rangées, sur 432, depuis les 52 documents
du 3 au 9 novembre. 22 sections rejoignent une fiche existante. Les journaux ne font plus de
fiche de moment.**

## 1. Comment la passe s'est jouée

- **La fenêtre.** G a fermé le seul backend `memoire-longue` de l'hôte du portable à 13 h 45. Il l'a
  rouvert à 14 h 51, avec la base de la passe. L'index de code est resté servi pendant tout ce temps.
- **L'instantané d'avant** (backend fermé, 13 h 46) :
  `~/.local/share/rag3weaver/instantanes/avant-archivage-14-nov/`, au portable. Il fait 113 Mo et
  3713 fichiers, avec 19 fiches. Il contient la base, `.extensions`, `.fts`, `.readers`,
  `.checkpoints` et `~/memoire-longue`. Ses sommes sha256 sont dans `avant-archivage-14-nov.sha256`.
  Il reste le filet de Lucie, au moins jusqu'à ce qu'elle ait survolé l'annexe.
- **La passe** a joué à luciepc, sur une copie de cet instantané aux sommes vérifiées, de 13 h 47 à
  14 h 49 (61,9 min). C'était sous `poste lourd`, au régime des passes (`RAG3WEAVER_GPU_DUTY=70`,
  budget 4096). Elle a tourné avec le binaire de master 91c9f43d0, qui contient les deux garde-fous
  (`7961fae7f`), la lib 0c41f9fe6 et `gemini-3.8-flash` par Vertex. Elle a vu 52 documents et 432
  sections, sans erreur.
- **Le retour.** Avant de copier, la base vivante et `~/memoire-longue` étaient identiques à
  l'instantané (`cmp`, `diff -r`) : rien ne s'est écrit pendant la fenêtre. La base de la passe a
  été rapatriée, sommes vérifiées, et posée backend fermé. Ce qu'elle remplace est gardé tel quel
  dans `~/.local/share/rag3weaver/remplacee-archivage-14-nov/`.
- **La vérification par l'hôte.** `recall_fiches` répond après la réouverture, avec 116 fiches.

## 2. Les nombres, face au troisième archivage à blanc

| | blanc-96-3 (copie, 13 nov.) | **la passe (sa mémoire, 14 nov.)** |
|---|---|---|
| sections rangées | 414 | **374** |
| sections jugées « none » | 18 | **58** |
| sous une fiche existante | 19 | **22** |
| **fiches créées** | **115** | **95** |
| dont à une seule section | 45 | 40 |
| dont plusieurs sections d'un même document | 32 | 24 |
| dont plusieurs documents | 38 | 31 |
| noms de moins de 25 caractères | 29 | 45 |
| paires de noms voisins | 0 | 4 |
| refus de nom daté | — | 0 |
| durée | 65,9 min | 61,9 min |

**Les « none » passent de 18 à 58, surtout dans les journaux.** Le rapport de session
d'orchestration du 7 novembre range 32 sections au lieu de 60. D'autres rapports de session perdent
deux ou trois sections chacun. C'est l'effet de l'instruction « le chantier, pas le moment » : une
section de pur déroulé (une heure, une relance) reste dans le L0 au lieu de fonder une fiche de
moment. Aucun nom daté n'a été refusé, parce que le modèle n'en a plus proposé.

## 3. Ce qui a rejoint une fiche existante

22 sections, sur cinq fiches qui existaient déjà :

- **« hooks-essai »** (16 sections) : les pages de la session d'essai des hooks.
- **Quatre bilans du juge** (6 sections en tout : 3, 1, 1 et 1) : les rapports des tâches de
  l'équipier.

Le détail est en tête de l'annexe.

## 4. Les fiches créées

95 fiches, toutes `proposed` et `via: archiviste`. Les plus fournies :

- « L'équipier » (15 sections, 6 documents) ;
- « hooks Claude Code de rag3weaver » (15, 6) ;
- « client terminal interactif » (15, 1) ;
- « bascule au format 43 » (14, 5) ;
- « Hooks de Claude Code pour la mémoire » (14, 6) ;
- « hooks de rag3weaver vers la mémoire longue » (13, 4) ;
- « Classeur local du rappel » (11, 2) ;
- « Vues par worktree » (10, 2).

La liste entière est dans l'annexe. **Ce que Lucie y fait** : garder une fiche (la transition `accept`
la rend `current`), la retirer (`reject`), ou dire lesquelles fusionner.

## 5. Ce qui ne va pas encore

- **Des doublons restent**, sur deux points :
  - **Quatre paires de noms voisins.** La recherche par le nom ne les a pas trouvées parmi plus de
    cent fiches : « Client terminal rag3weaver-tty » et « client tty de rag3weaver » ; « Hooks de
    rag3weaver » et « hooks Claude Code de rag3weaver » ; « Hooks de rag3weaver » et « hooks de
    rag3weaver vers la mémoire longue » ; « Vues par worktree » et « modèle des vues par worktree ».
  - **Les doublons de sens**, que le voisinage des mots ne voit pas. Six fiches ont « hooks » dans leur nom. On trouve aussi « Ménage des worktrees et caches » face à « Nettoyage des worktrees
    et de l'espace disque ». La fusion par le sens reste la piste, non codée.
- **Des descriptions pauvres.** Une fiche créée prend pour `about` la première phrase de la section
  qui l'a créée : « 1. », « Session archi, 9 novembre 2026. », « Accepté par c7. ». C'est un défaut à
  part, pas encore ouvert.
- **Six fichiers de fiche étaient en retard à la fin de la passe.** Le binaire s'est arrêté
  (`shutdown`) sans réécrire les fichiers des fiches touchées par ses derniers documents : 2
  manquaient et 4 étaient périmés. La base, elle, les portait. L'hôte les a réécrits à la réouverture,
  à 14 h 51 min 28 s. Ticket :
  `docs/tickets/closed/2026-11-14-un-arret-du-backend-laisse-les-fichiers-de-ses-dernieres-ecritures-en-retard.md`.
  C'est corrigé depuis, selon une règle : **la base est la vérité, et les fichiers la suivent jusqu'à
  l'arrêt compris**. L'arrêt d'un backend régénère ses fichiers composés depuis la base avant de
  rendre.

## 6. Ce qui reste sur disque

- **Au portable :**
  - `instantanes/avant-archivage-14-nov/`, l'instantané d'avant, avec ses sommes ;
  - `remplacee-archivage-14-nov/`, ce qui était en place, identique à l'instantané.

  Les deux sont dans `~/.local/share/rag3weaver/`.
- **À luciepc :** `~/.cache/rag3weaver-build/passe-96-reelle/`, avec :
  - `base/`, la base rendue ;
  - `base-origine/`, l'avant ;
  - `templates/` et `src/` ;
  - `lancer.py`, `resultats.jsonl` et `apports.json`.

**Pour revenir en arrière** : faire fermer le backend `memoire-longue` par G, remettre les pièces de
`instantanes/avant-archivage-14-nov/` à leur place, puis faire rouvrir. C'est la même procédure que le
retour, dans l'autre sens.
