# Les roadmaps et les visions en mémoire — un design

Demandé par Lucie le 25 octobre 2026 au soir, cadré par c7, tranché ici. Rien n'est codé :
cette page attend son mot. Elle s'appuie sur la page 04 du 18 octobre (le modèle des fiches),
sur la page 07 d'everything declarative (le mini-plan de l'équipier, branche equipier-1) et
sur ce qui est sur master ce soir (`cited_by`, les fiches en fichiers, le manifeste du dossier).

## 1. Le cadre de c7, repris

- **Pas de type de fiche nouveau.** Un plan a trois horizons : le **pas** (un élément du
  mini-plan, pas de fiche), le **lot** (une tâche, dont la fiche porte les L1 `plan` de
  l'équipier et les bilans du juge) et la **roadmap** (un plan dont les éléments sont des
  phases, abstraites sauf la première ou la seconde). Une phase devient un plan quand elle se
  concrétise.
- **Une vision est une fiche sujet ordinaire** : ce qu'on veut et pourquoi, sans phases.
- **« Quand on aura ça, on pourra enfin faire ce truc » est une arête**, jamais un drapeau.

Une correction au cadre, avec sa raison. Dans ce gabarit, une Ref est *une chose citée*
(genre, valeur), et le gabarit n'a aucune relation de fiche à fiche. Une arête entre deux
fiches s'écrit donc comme une Ref **dont la valeur est le nom d'une fiche**, posée sur la
fiche d'un côté. C'est la forme que `outil:<nom>` a déjà, et `cited_by` la lit sans rien de
plus : « qui cite cette chose » devient « qui pointe cette fiche ».

## 2. Quatre genres, aucun champ

Tous se créent par `create_ref_type`, en lignes de RefType : le schéma ne bouge pas, le
format des fichiers reste 1, le manifeste du dossier les liste dans ses genres.

| genre | valeur | posé sur | sens |
|---|---|---|---|
| `fiche` | le nom d'une fiche | un apport (`citer_source`) | ce texte parle de cette fiche : un plan liste ses phases, une phase ses tâches |
| `after` | le nom d'une fiche | une ancre de la fiche qui vient après | cette phase vient après celle-là (une dépendance d'ordre, au même horizon) |
| `enables` | le nom d'une fiche vision | une ancre de la phase | cette phase rend cette vision possible |
| `horizon` | `vision`, `roadmap` ou `phase` | une ancre | ce que la fiche est, lu par `cited_by horizon:vision` |

- Le harnais rhai d'un genre n'a pas d'entrée-sortie (`create_ref_type` le dit) : il met la
  valeur en forme normale et rien d'autre. **« Cette fiche existe-t-elle » se répond à la
  lecture**, par le Plan de rendu (§6), jamais dans l'état de la Ref.
- `after` et `enables` sont deux genres parce qu'ils ne disent pas la même chose : l'un
  ordonne des éléments entre eux, l'autre nomme un résultat. Une vision n'est pas une phase,
  et « la vision vient après la phase » dirait moins que « la phase la rend possible ».
- Une phrase dite au fil, « quand on aura X on pourra Y », se range **sur la fiche de la
  phase X**, pas sur la vision : un apport qui cite `enables:Y`. Si X n'a pas de fiche, le
  rangement en crée une à l'état `proposed` (un état que la Fiche a déjà). Mettre l'apport sur
  la vision Y avec une Ref `enables:Y` ferait une fiche se citer elle-même ; `cited_by
  enables:Y` rend de toute façon la phrase depuis la vision.

## 3. Le L0 d'un plan à phases

La même forme que le plan de l'équipier, à un horizon de plus. Une Source
`sources/<jour>-<nom>-plan-<n>.md`, dont le texte rend le plan lisible (la demande, la
proposition, les phases par statut, la raison du redécoupage), puis le bloc json entier :

```json
{ "horizon": "roadmap",
  "request": "…", "proposal": "…",
  "items": [
    { "id": "p1", "title": "La mémoire dans ses fichiers", "status": "done",
      "fiche": "La mémoire dans ses fichiers", "after": [], "enables": ["L'archiviste branché sur Claude Code"] },
    { "id": "p2", "title": "L'archiviste", "status": "doing", "fiche": "L'archiviste", "after": ["p1"] },
    { "id": "p3", "title": "Le partage par git", "status": "todo", "after": ["p2"] }
  ],
  "closed": null }
```

- **Une phase abstraite n'a pas de `fiche`** : un titre, un statut, ses `after`. Elle en
  reçoit une quand elle se concrétise (§4). `enables` nomme des fiches vision.
- **Un L1 `plan` par redécoupage**, pas par changement de statut : ajouter, retirer,
  réordonner ou concrétiser une phase fait un L1 neuf, dont le json dit tout l'état du
  moment. Une phase qui avance ne réécrit rien ici : son état est sur sa propre fiche (§5).
  C'est la règle de l'équipier (un L1 ne se réécrit pas) portée à un horizon où le plan dure
  des mois.
- **Chaque `items[].fiche` du json a sa Ref `fiche:<nom>` sur l'apport**, par `citer_source`,
  et chaque `enables` sa Ref `enables:<vision>` : ce que le json dit, la base le lit. Depuis
  une phase, `cited_by fiche:<phase>` rend les roadmaps qui la listent ; depuis une vision,
  `cited_by enables:<vision>` rend les phases qui y mènent et les phrases dites au fil.

## 4. Comment une phase pointe le plan qui la concrétise

**Par identité.** La phase *est* la fiche sur laquelle la tâche range : le juge nomme la fiche
de session par `nom_de_tache` (page 04 du proto), et l'équipier range ses plans clos sur la
même. Une tâche ouverte pour une phase prend le nom de la phase ; ses L1 `plan` (horizon
`lot`) et ses bilans arrivent sur la fiche de la phase. Aucun pointeur à tenir à jour.

Quand une phase demande plusieurs tâches, elle est elle-même un plan à l'horizon `lot` : un
L1 `plan` dont les `items` sont les tâches, chacune avec sa `fiche`, et la Ref `fiche:` qui
va avec. La récursion s'arrête au pas, qui n'a pas de fiche.

## 5. Qui clôt une phase

| horizon | qui | comment |
|---|---|---|
| le pas | l'agent | `step_done` (page 07) |
| le lot | l'agent clôt son plan ; le superviseur fusionne ou rejette la tâche | le L1 `plan` suivant porte `closed` |
| la phase | **un humain** : Lucie, ou le superviseur | `put_fiche` vers `retired`, et un L1 de section `closure` qui dit la raison, ce qui a atterri (le hash) et cite `doc:docs/nouveautes.md#<mois>` |
| la vision | personne | une vision ne se clôt pas ; abandonnée, elle passe `retired` avec son L1 `closure`. « Atteinte » se lit : toutes ses phases `enables` sont closes, ce que le Plan de rendu calcule et que personne n'écrit |

Le juge **ne change jamais un état**. Quand la tendance des bilans d'une phase le permet
(tous ses lots fusionnés, les bilans verts), il écrit un apport de section `trend` sur la
fiche de la phase : « prête à clore, voici pourquoi ». L'humain lit, puis clôt ou non.

Le statut d'un élément dans le json (`todo`, `doing`, `done`, `blocked`, `dropped`) et l'état
de la fiche de la phase (`proposed`, `current`, `retired`) ne sont pas deux vérités : le json
est la photo au jour du redécoupage, la fiche est l'état d'aujourd'hui. Le Plan de rendu
montre les deux côte à côte, et un désaccord se voit.

## 6. Où ça vit : le gabarit memory, pas un gabarit `plan`

Un gabarit à part rejouerait Fiche, Apport, Ref, Source et leurs relations, et couperait
l'espace des Refs en deux : `cited_by` sur un fichier ou un outil ne rendrait plus les plans
qui s'en sont servis, et une phase ne pourrait pas être la fiche de sa tâche. Ce qui s'ajoute
au gabarit memory tient en trois pièces, aucune ne touche le schéma :

1. les quatre genres (§2), créés dans la mémoire de Lucie par `create_ref_type` ;
2. la forme du L0 (§3), une convention de Source, comme le plan de l'équipier ;
3. **un Plan de rendu**, `plans/roadmap.json`, lu par un outil `roadmap { fiche }` : les
   phases du dernier L1 `plan` de la fiche, chacune jointe à sa fiche par le nom (son état, son
   dernier apport), ses `after` et ses `enables` résolus, et pour chaque nom cité sans fiche
   « aucune fiche de ce nom ». C'est là que « la fiche existe-t-elle » se répond.

Les visions existantes se relisent telles quelles : `extension/rag3weaver/visions/*.md` et
`docs/vision_roadmap_09_2026/` sont des L0, un document par vision, et `docs/nouveautes.md`
la liste des phases closes.

## 7. Ce que l'archiviste en fait (M4, M5)

- **M4 range les visions** comme n'importe quel document : une fiche vision par document
  (son titre pour nom, l'ancre `horizon:vision`), un L1 par section, la Ref `doc:` de
  chaque L1. Les phrases « quand on aura… » sont de sa classification : un apport sur la
  phase, créée `proposed` au besoin, qui cite `enables:<vision>` (§2).
- **M4 range `nouveautes.md`** ligne par ligne : chaque ligne est un L1 `closure` sur la
  phase qu'elle clôt, avec son hash. Une ligne qui ne retrouve aucune phase crée une fiche
  phase déjà `retired` : une étape atteinte que personne n'avait planifiée, ce qui est la
  définition même de ce fichier.
- **M5 ne change pas.** Le L2 d'une vision se calcule de ses L1 ; la question « où en est
  cette vision » n'est pas un L2, c'est une jointure, et c'est le Plan de rendu (§6) qui la
  fait. Un L2 qui lirait les fiches voisines dériverait d'autre chose que de ses L1, et la
  page 04 l'interdit.

## 8. Le lot, s'il est accepté

Petit. Les quatre genres et leurs descriptions ; le Plan `roadmap.json` et son outil ; un
témoin e2e : une roadmap de trois phases dont une concrétisée, `cited_by enables:` depuis la
vision rend la phase et la phrase dite au fil, `cited_by fiche:` depuis la phase rend la
roadmap, `roadmap` dit l'état des trois et nomme la phase sans fiche, et une mémoire
reconstruite de ses fichiers garde toutes les arêtes. Une ligne dans `nouveautes.md` quand
c'est sur master.

Deux questions laissées à Lucie : une phase dite au fil prend-elle sa fiche tout de suite
(`proposed`, ce que je propose) ou reste-t-elle du texte jusqu'au premier plan qui la liste ?
Et `nouveautes.md` reste-t-il écrit à la main (c7, aujourd'hui), ou se rend-il un jour depuis
les L1 `closure` ?
