# L'archiviste en deux étapes, et les catégories qu'il établit

18 novembre 2026, session mémoire (96). C'est la correction de Lucie, par c7 :

> ce n'est pas ce que j'avais dit : il devrait découper par sujet déjà cette fenêtre, et ensuite
> ranger dans une seconde étape chaque sujet vers un vrai sujet dans la base, ou un nouveau ; la
> première étape peut être juste une extraction structurée.

C'est son mot du 15 novembre : « on déterminait les sujets de chaque bout de conversation puis on
créait des L1 dedans ». Le ticket est
`docs/memoire/tickets/2026-11-18-l-archiviste-range-en-deux-etapes.md`.

Rien n'est codé avant que Lucie ait vu cette page, ou demain matin si elle ne l'a pas regardée.

## (a) La forme d'aujourd'hui : une décision par section

`templates/backends/memory/graphs/archive.mmd`, sur master :

1. `SplitSections` découpe le document par ses titres : **une section par tour** dans les mesures
   du banc.
2. Une section plus longue que `segment_over` est d'abord découpée en sujets (`archive_segment`).
   C'est éteint par défaut.
3. Pour chaque section, `archive_decide` fait deux choses :
   - il rappelle les fiches candidates **par le texte de la section** (`query_chars`, 4 000
     caractères au plus) ;
   - il fait **un appel** au modèle (`Decide`), avec la section, les candidates et `context` (la
     place, les résumés d'avant, le bout de fenêtre). L'étiquette rendue est une candidate,
     `none` ou `create`.
4. `archive_subject` rapproche le nom proposé des fiches déjà là (`sujets_voisins`), puis le L1 est
   écrit avec `{fiche, source, section}`.

Le coût est **un appel par tour**. En S 50, chacun porte jusqu'à 50 k jetons de contexte : 5,33 M
jetons d'entrée sur le corpus du banc (244 tours).

Ce que cette forme mêle : la décision « de quoi parle ce tour » et la décision « sous quelle fiche
de la base » se prennent ensemble, dans un seul appel. Le sujet est vu à travers les candidates que
le texte brut du tour a rappelées.

## (b) La forme à deux étapes

### Étape 1 : l'extraction, sur la fenêtre seule, sans la base

**Entrée** : une fenêtre S 50, c'est-à-dire les tours de la fenêtre, numérotés, et sa place. Le
contexte de la fenêtre d'avant (les résumés par `compose_restart`) peut s'y ajouter, pour la
continuité des noms.

**Un appel par fenêtre**, par `LlmNode` sous `response_schema`. C'est le nœud livré par `segment`
(53a83d089) : une réponse non conforme y est refusée en nommant `response_schema`.

**Sortie** :

```json
{"topics": [
  {"title": "la reprise du journal",
   "turns": [{"from": 12, "to": 15}, {"from": 31, "to": 31}],
   "said": "ce qui en est dit, en quelques phrases : le fait, la décision, le reste à faire"}
]}
```

- Un sujet peut revenir plus loin dans la fenêtre : `turns` est une liste d'intervalles.
- Un tour peut porter deux sujets. Un tour sans sujet (un accusé de réception, une politesse)
  n'est dans aucun intervalle : c'est l'ancien « rien », décidé une fois par fenêtre.
- La longueur de `said` est bornée dans la consigne, pour que la réponse tienne sous
  `max_tokens`.

**Aucun rappel de la base** dans cette étape : elle dit de quoi parle la fenêtre, et seulement cela.

**Le même appel sort aussi un résumé de la tranche** : un champ de plus, `slice_summary`, qui dit
en quelques phrases ce qui s'est passé dans la fenêtre entière, au-delà des sujets un à un (Lucie,
18 novembre au soir).

### Étape 2 : le rangement, sujet par sujet, avec la base

Pour chaque sujet extrait :

- **le rappel** des fiches candidates par `title` et `said`, une requête courte et ciblée, et non
  plus le texte brut d'un tour ;
- **le Decide** d'aujourd'hui (une candidate, `none` ou `create`), sur ce sujet ;
- **le dédoublonnage**, qui vit ici : `archive_subject` (`sujets_voisins`) et les fiches proposées
  plus haut dans le même document (`carry: proposed`).

Le L1 a pour section `turns a-b · <title>`. Son identité reste unique, et le bilan de
`compose_restart` lit toujours le premier tour. Son texte est fait de `said`, suivi du texte des
tours cités.

### Entre deux fenêtres : le rapport de l'archiviste à lui-même (variante R)

Il n'y a pas de chevauchement. À la fin de chaque plage rangée, l'archiviste rend **un rapport
structuré**, que la fenêtre suivante reçoit à la place des résumés de S. C'est le « résumé des
fenêtres d'avant », mais fait par lui et structuré (Lucie).

**Sa forme** :

```json
{"window": {"n": 3, "of": 18, "turns": {"from": 101, "to": 152}},
 "filed": [{"topic": "la reprise du journal", "fiche": "la reprise du journal",
            "l1": "turns 120-131 · la reprise du journal", "action": "joined"}],
 "open": [{"topic": "la compaction par l'archiviste", "last_turns": {"from": 148, "to": 152},
           "said": "ce qui reste en suspens à la fin de la plage"}],
 "proposed": {"categories": [{"name": "couche", "values": ["moteur C++", "rag3weaver"]}],
              "values": [{"category": "état", "value": "bloqué"}]},
 "not_filed": [{"turns": {"from": 140, "to": 141}, "reason": "accusé de réception"}]}
```

- **`filed`, `proposed` et `not_filed` se composent sans appel au modèle**, depuis la trace de la
  plage (le rapport d'`archive`, `trace: true`) : chaque sujet, son action, sa fiche et son L1.
- **`open` vient de l'extraction** : son schéma porte un drapeau `open` sur chaque sujet encore en
  cours au dernier tour de la fenêtre. C'est le seul champ que le modèle décide.
- **La borne** est de 4 000 jetons (~16 000 caractères), par le même compte que `compose_restart`.
  Au-delà, on garde dans cet ordre `open`, puis `filed` des plus récents aux plus anciens, puis
  `proposed`, puis `not_filed`, et le rapport dit `truncated` avec ce qui manque.

**Son usage** :

- **par l'étape 1** de la fenêtre suivante : les sujets `open` sont donnés en tête, avec leur
  titre. Un tour qui les prolonge reprend **le même titre**, donc la même fiche, au lieu d'un
  sujet neuf au nom voisin ;
- **par l'étape 2** : les fiches de `filed` (et celles des sujets `open`) passent **en tête des
  candidates** d'un sujet, avant celles du rappel. Une fiche déjà touchée dans la session se
  rejoint avant d'en créer une sœur.

Le banc le mesure comme la **variante R**, contre S 50.

### Ce que gagne la seconde étape

- **La base n'entre plus dans la fenêtre** : l'extraction ne voit pas de candidates, donc pas de
  noms de fiches qui tirent le découpage vers ce qui existe.
- **Le rappel se fait par sujet** : une requête nette (un titre et quelques phrases), et non un
  tour brut de 5 à 10 k caractères, coupé à 4 000.
- **Des L1 par sujet et non par tour** : un sujet traité sur dix tours fait un L1, et non dix
  apports sur la même fiche.
- **Moins de décisions** : autant que de sujets, au lieu d'autant que de tours.

### Le coût en appels

- **Aujourd'hui (S 50)** : un appel par tour, soit 244 sur le corpus, chacun avec jusqu'à 50 k
  de contexte : 5,33 M jetons d'entrée mesurés.
- **En deux étapes** :
  - une extraction par fenêtre : environ 18 sur 880 k jetons. Chacune lit sa fenêtre une fois,
    ce qui fait de l'ordre de 0,9 à 1,8 M jetons d'entrée avec les résumés d'avant ;
  - puis un Decide court par sujet extrait, soit quelques dizaines par fenêtre, à quelques
    centaines de jetons chacun.

C'est une estimation, **à mesurer** par le banc, et non une promesse. **Le risque** est une
fenêtre qui porte plus de sujets que `max_tokens` ne laisse en sortie : la réponse est tronquée,
donc refusée en le nommant, et la fenêtre se redécoupe en deux.

## (c) Ce que le banc remesure

S 50 reste la forme de fenêtre. Le banc compare, sur le même corpus et contre la N rejouée :

- **la forme d'aujourd'hui, S 50** : 0,84 de rappel, 5,33 M jetons ;
- **la forme à deux étapes, sur les mêmes fenêtres S 50** ;
- **la variante R** : les deux étapes, avec le rapport de l'archiviste à lui-même entre les
  fenêtres, à la place des résumés de S.

Les métriques sont celles du § 4 de `docs/banc/15-11-2026/02-jusqu-ou-l-archiviste-voit.md` :

- le rappel des sujets de N ;
- les doublons ;
- les jetons d'entrée et de sortie ;
- le p90 par fenêtre ;
- plus le nombre de L1 par sujet.

Le juge est le même. La trace (`trace: true`, `usage_total`) donne les jetons de chaque étape.

## (d) Les catégories, et l'archiviste qui les établit

Seconde graine de Lucie, le 18 novembre au soir :

> plutôt que leur laisser poser des valeurs arbitraires, leur laisser construire des enums nommés
> et les référencer, peut-être eux-mêmes avec description

> le chaînon manquant : à quel moment tu retrouves des fiches du même type ? ces références sont
> des relations parfaites entre fiches sans lien direct

> comment intégrer ça dans l'archiviste aussi ? si personne s'en occupe personne le fera, il faut
> un mécanisme qui les établit, ces catégories

ED regarde la part des schémas.

### Ce qui existe déjà dans le gabarit memory

| Pièce | Ce qu'elle porte | Ce qu'elle sert aujourd'hui |
| --- | --- | --- |
| `RefType` | `name`, `description`, `forme`, `script` (une validation Rhai), `etat` (`current`, `retiree`) | un genre de référence. `create_ref_type` le crée, et le hook de Claude Code en crée déjà pour ses genres (`handle`, `alias`, `identity`). |
| `Ref` | `genre`, `valeur`, `dit_comme`, `empreinte`, `etat` (`vue`, `a_revoir`, `introuvable`, `retiree`) | une référence, d'un genre, vers une valeur |
| `ANCRE` (Fiche → Ref) et `APPORT_CITE` (Apport → Ref) | les liens des fiches et des apports vers les Refs | `cited_by` rend les fiches qui citent une valeur : c'est déjà « retrouver des fiches sans lien direct par ce qu'elles citent » |
| `Subject` | `name`, `about`, `state` | le sujet d'une `Memory`, par `ANCHORED_TO` (le modèle d'avant les fiches) |

Le manque est donc précis. Un genre de Ref d'aujourd'hui accepte **n'importe quelle valeur** qui
passe sa forme. Rien ne dit **la liste des valeurs** permises, ni ce que chacune veut dire, ni
qu'une valeur neuve devrait d'abord se définir.

### Ce que je propose : une catégorie est un RefType dont les valeurs sont définies dans une fiche

Je réutilise l'existant plutôt que d'ajouter une entité.

- **La catégorie** est un `RefType` dont la `forme` dit « valeur d'une catégorie » (par exemple
  `categorie:<nom>`) et dont la `description` dit ce qu'elle classe. Son `script` refuse toute
  valeur qui n'est pas définie (point suivant).
- **Les valeurs** sont définies dans une fiche de définitions, `catégorie : <nom>` : **un apport
  par valeur**. La section de l'apport est l'**identifiant stable** de la valeur (un slug posé une
  fois, qui ne change jamais). Son texte porte le **nom affiché** et la **description**. Renommer
  l'affichage ne touche pas à l'identifiant.
- **Une fiche porte une valeur** par une Ref `{genre: <nom de la catégorie>, valeur: <identifiant>}`,
  liée par `ANCRE`. Elle **porte la catégorie entière** (« cette fiche est classée selon… ») par une
  Ref `{genre: "categorie", valeur: <nom>}`.
- **Le rappel « les fiches qui portent cette valeur »** est `cited_by`, déjà là, sur
  `<nom>:<identifiant>`. C'est le chaînon que Lucie nomme : deux fiches sans lien direct se
  retrouvent par la valeur qu'elles portent.

**L'autre voie** serait d'ajouter deux entités, `Category` et `CategoryValue`, et une relation
`HAS_VALUE`. Le typage serait plus net, mais il faudrait refaire ce que Ref a déjà : les états, le
miroir des fichiers, `cited_by`, l'ancrage. Je ne la propose pas, sauf si ED y voit une raison de
schéma.

### Le juge : ce qui garde les valeurs propres

- **Une valeur inconnue** : `add_ref` (et `ancrer_fiche`) la refuse par le `script` du RefType, en
  nommant les valeurs définies et la voie pour en créer une.
- **Une valeur neuve** passe par un geste à part, `define_value(categorie, nom affiché,
  description)`. Avant de l'écrire, il compare le nom aux valeurs déjà définies par le voisinage
  des mots qui sert déjà aux fiches (`sujets_voisins`). Une valeur proche ou en doublon est
  rendue : on la réutilise, on n'en crée pas une sœur.
- **Une définition qui change** (la description d'une valeur réécrite, une valeur retirée) : les
  Refs qui la portent passent à `a_revoir`. C'est l'état qui existe déjà. Les fiches dépendantes se
  listent par `cited_by` et se signalent : rien n'est réécrit en silence.
- **Valide n'est pas vrai** : qu'une valeur passe la garde dit seulement qu'elle est **définie**.
  Que la fiche relève vraiment de cette valeur reste une affirmation, à sourcer comme le reste
  (un apport et sa Source). La garde ne juge pas la vérité, et ses messages le disent.

### Le mécanisme dans l'archiviste : la forme à discuter

1. **L'extraction (étape 1)** sort aussi, pour chaque sujet, des **attributs** en paires
   `(dimension, valeur)`, en mots libres, **sans décider** : par exemple `("état", "bloqué")` ou
   `("couche", "moteur C++")`. Le schéma devient `{topics: [{title, turns, said, attributes:
   [{dimension, value}]}]}`.
2. **Le rangement (étape 2)**, pour chaque paire, commence par une **pré-recherche** : un rappel
   des catégories et de leurs valeurs, lexical sur les noms et dense sur les descriptions. Il met
   les catégories et les valeurs candidates dans le contexte, exactement comme les fiches
   candidates d'un sujet, pour que l'appel n'aille pas au hasard (Lucie). L'appel choisit parmi
   elles, ou dit « aucune ». Il rend l'une de trois choses :
   - **réutiliser** une valeur définie : la Ref est posée ;
   - **proposer une valeur neuve** dans une catégorie existante, par `define_value` avec son juge ;
   - **laisser l'attribut libre** sur la fiche, comme un apport de section `attribut : <dimension>`.
3. **Une catégorie naît par consolidation.** Quand une même dimension libre revient sur au moins
   deux ou trois fiches, le rangement existant la consolide (`Rangement`, et la relation
   `CONSOLIDE` entre apports) :
   - la fiche de définitions `catégorie : <dimension>` est créée **proposée** ;
   - ses valeurs viennent des attributs libres, rapprochés par voisinage ;
   - les attributs libres deviennent des Refs vers ces valeurs, et l'apport libre d'origine est
     gardé comme provenance (`recopied`, comme les gestes scinder et rejoindre).
4. **Catégories et valeurs sont « proposées »** jusqu'à une relecture, ou jusqu'à ce que k fiches
   les portent. C'est la même règle que les fiches (`confirmer_fiche`).
5. **Le juge** : une valeur sémantiquement proche d'une valeur définie (voisinage des mots, puis le
   dense sur les descriptions) se **réutilise**, ou le choix d'une valeur neuve se **justifie**
   dans l'apport.
6. **Un outil `put_category`** permet à un agent en tâche de déclarer une catégorie et ses
   valeurs. Mais le mécanisme par défaut est l'archiviste : ce qu'aucun agent ne déclare
   s'établit quand même par la consolidation.
7. **Ce que le banc mesure**, sur sa session de 880 k jetons rangée en deux étapes :
   - les catégories qui y naissent ;
   - leur qualité, jugée comme les fiches (une dimension nette, des valeurs distinctes, peu de
     doublons) ;
   - le gain de « fiches du même type » : pour un échantillon de fiches, le nombre de fiches
     sans lien direct que `cited_by` d'une valeur fait retrouver, et combien sont justes.

### Ce qui reste à trancher

- **Avec ED** : les définitions dans la mémoire (une fiche par catégorie), comme ici, ou un enum
  déclaré au schéma du gabarit ? Je penche pour la mémoire : ce sont **les agents** et
  l'archiviste qui les établissent, à l'usage.
- **Avec Lucie** : le seuil de naissance (deux ou trois fiches), le k de confirmation, et si une
  catégorie proposée se montre déjà aux agents avant d'être confirmée.

## (e) La boîte à outils : un contexte composé d'ingrédients déclarés

Lucie, le 18 novembre au soir :

> et si on préparait tout ça quoi qu'il arrive, comme une boîte à outils pour nos
> expérimentations : on n'est pas obligé de tout utiliser mais on aurait tout sous la main.

Le contexte de chaque étape est **composé d'ingrédients déclarés**. Chacun est **débrayable** et
porte **sa borne en jetons**. Une variante du banc devient alors **une configuration, pas du
code** : F, S, R et leurs mélanges se jouent en changeant des réglages.

### Les ingrédients

Toutes les clés sont sous `archivist.context`. Chaque ingrédient vaut `{enabled, max_tokens}`,
sauf le chevauchement, qui est un pourcentage.

| Clé | Ce que c'est | Étape | Défaut (S 50) |
| --- | --- | --- | --- |
| `place` | « fenêtre n sur N, tours a-b » | 1 et 2 | toujours, quelques jetons |
| `previous_report` | le rapport de l'archiviste à lui-même, à la fin de la tranche d'avant (variante R) | 1 et 2 | éteint ; 4 000 |
| `previous_slice_summary` | le `slice_summary` de l'extraction de la tranche d'avant | 1 | éteint ; 1 500 |
| `current_slice_summary` | le `slice_summary` de la tranche courante, pour situer un sujet dans sa fenêtre | 2 | allumé ; 1 500 |
| `previous_summaries` | les résumés de toutes les tranches d'avant, composés par `compose_restart` (la forme S) | 1 | allumé ; 20 000 |
| `overlap_percent` | le chevauchement : le bout de la tranche d'avant, en % de la fenêtre (la forme F) | 1 | 0 |
| `ordered_bilan` | le bilan ordonné de la session (une ligne par pas, `fiche:<nom>`) | 1 et 2 | éteint ; 2 000 |
| `candidate_fiches` | les fiches rappelées pour un sujet, dont celles déjà touchées de `previous_report` | 2 | allumé ; 5 fiches |
| `candidate_categories` | les catégories et valeurs rappelées pour une paire (dimension, valeur) | 2 | allumé ; 5 valeurs |

Plus la taille de la fenêtre elle-même : `archivist.window_tokens`, 50 000 par défaut.

**L'ordre de composition**, quand la somme dépasse le budget de l'appel : `place`, puis le rapport
d'avant, puis le résumé de la tranche courante, puis le résumé de la tranche d'avant, puis le
bilan, puis les résumés de toutes les tranches, puis le chevauchement. Un ingrédient qui ne tient
pas est coupé à sa borne, et la trace le dit (`context_parts` : ce qui est entré, en jetons).

### Les variantes du banc, en réglages

| Variante | Réglages |
| --- | --- |
| **S 50** (le défaut) | `previous_summaries` allumé, `overlap_percent` 0 |
| **F 25 %** | `previous_summaries` allumé, `overlap_percent` 25 |
| **R** | `previous_report` allumé, `previous_summaries` éteint |
| **R + résumés** | `previous_report` et `previous_slice_summary` allumés |
| **N** (au fil de l'eau) | tout éteint sauf `place` et les candidates |

### Où vivent les réglages

Dans les `bindings` de l'outil `archive`, au manifeste du gabarit memory. Les outils y portent
déjà leurs défauts : `recall_apports` y tient son `default_filter`. Cela ne demande pas de Rust :
`BackendManifest` refuse une clé inconnue à sa racine, mais les bindings d'un outil sont libres.
Une copie de banc change ses bindings et rien d'autre.

### Ce que coûte de tout préparer

- **Le composeur de contexte**, un script Rhai : les ingrédients dans l'ordre, chacun coupé à sa
  borne par le même compte de jetons que `compose_restart`, et `context_parts` dans la trace.
  Environ 4 heures.
- **L'extraction**, avec `slice_summary` et le drapeau `open` dans son schéma. C'est inclus dans le
  lot des deux étapes.
- **Le rapport de tranche**, composé depuis la trace, et **sa mémoire d'une tranche à la
  suivante** : il est écrit comme un apport de section `rapport de tranche` sur la fiche de la
  session, et le `slice_summary` aussi, si bien que la tranche suivante les relit sans que
  l'appelant les porte. Environ 4 heures.
- **Les témoins**, un par ingrédient : allumé, il est dans le prompt ; éteint, il n'y est pas ;
  au-delà de sa borne, il est coupé et dit. Ils jouent sur le faux serveur, avec un cas par
  variante du tableau. Environ 4 heures.

**En tout, environ deux jours** au-dessus du lot des deux étapes. Tout s'éteint par défaut, sauf
S 50 : rien ne change pour qui ne touche pas aux réglages.

## (f) Ce que voit l'agent après la compaction, aussi en ingrédients

Lucie, le 18 novembre au soir :

> ce que voit l'archiviste est une config facilement modifiable ; et après compaction, ce que voit
> l'agent : un résumé global des résumés des tranches en plus de ses L1 ? seulement les L1 ?
> seulement le résumé global ? ça aussi dans la boîte à outils

### Ce qu'il y a sur master

`compose_restart` (39ce7c1ae) compose déjà dans un budget, par une **priorité fixe** :

1. l'identité ;
2. la fiche de tâche ;
3. le bilan ;
4. les L1 de la session dans l'ordre des tours ;
5. les L1 des autres.

Le budget vient de `models.llm.restart_budget_tokens` : 100 k pour un modèle cloud, et la moitié
de la fenêtre d'un modèle local.

### Ce qu'il devient : les mêmes pièces, plus trois, chacune déclarée

Toutes les clés sont sous `restart.context`. Chaque ingrédient vaut `{enabled, max_tokens}`, dans
le budget total.

| Clé | Ce que c'est | Défaut |
| --- | --- | --- |
| `identity` | le dernier apport `identity` de `agent:<clé>` | allumé ; 2 000 |
| `task` | la fiche de tâche, sa phrase et son dernier plan | allumé ; 3 000 |
| `ordered_bilan` | le bilan ordonné, une ligne par pas avec `fiche:<nom>` | allumé ; 4 000 |
| `session_l1` | les L1 de la session dans l'ordre des tours, la fiche en marge, les siens en entier | allumé ; le reste du budget |
| `global_summary` | **neuf** : le résumé global fait des `slice_summary` des tranches, dans l'ordre. Par défaut, c'est leur suite bornée, sans appel ; en option (`mode: llm`), un appel qui les résume en un | selon le profil ; 8 000 |
| `last_report` | **neuf** : le dernier rapport de l'archiviste à lui-même (§ b, variante R) | allumé ; 4 000 |
| `others_l1` | les L1 des autres sessions sur les mêmes fiches, les plus récents d'abord | allumé ; 10 000 |
| `raw_tail` | **neuf** : la queue brute des derniers tours, telle quelle. L'hôte la passe, puisqu'il tient le `ChatFile` (la part d'archi, au point de branchement à 80 % de la jauge) | allumé ; 8 000 |

**L'ordre de composition** quand le budget manque : `identity`, `task`, `ordered_bilan`,
`last_report`, `raw_tail`, puis le corps du profil (`session_l1` et/ou `global_summary`), puis
`others_l1`. La réponse garde `parts` (les jetons de chaque ingrédient) et `left_to_recall` (ce
qui n'a pas tenu), comme aujourd'hui.

### Les trois profils, pour le banc

| Profil (`restart.profile`) | Corps du contexte d'après |
| --- | --- |
| `l1_only` | les L1 de la session, sans le résumé global |
| `summary_only` | le résumé global, sans les L1 de la session (ils restent au rappel) |
| `l1_and_summary` | les deux |

Ce qui ne change pas d'un profil à l'autre : l'identité, la tâche, le bilan, le dernier rapport et
la queue brute, chacun débrayable à part.

**Le défaut proposé** est `l1_and_summary`, jusqu'à la mesure.

### Ce que le banc mesure

Les trois profils, jugés **contre la compaction de Claude Code**, à budget égal. La vérité de
terrain est ce que les tours qui suivent la frontière ont eu besoin de savoir (la forme du banc,
`docs/banc/15-11-2026/01`). Il les juge sur le même transcript et aux mêmes frontières.

### Ce que ça coûte

- **`compose_restart`** passe de sa priorité fixe à des ingrédients déclarés : le même script
  Rhai, son ordre lu dans les réglages, chaque ingrédient coupé à sa borne. On lui ajoute
  `global_summary` (lu dans les apports `slice_summary` des sources de la session), `last_report`
  (le dernier apport `rapport de tranche`) et `raw_tail` (un paramètre texte, que l'hôte passe).
  Environ une journée, avec un témoin par ingrédient et un par profil, sur le faux serveur pour
  `mode: llm`.
- **La forme rendue** (`text`, `tokens`, `bilan`, `parts`, `left_to_recall`, `over_budget`,
  `estimate`) ne change pas. Sans réglage, le profil `l1_and_summary` sans `global_summary`
  disponible rend ce que rend `compose_restart` aujourd'hui.

## (g) Des outils pour l'archiviste, débrayés par défaut

Lucie, le 18 novembre au soir :

> l'agent archiviste a-t-il accès à des outils pour relire du code, faire des recherches dans la
> mémoire, la session ou le code ? il faudrait qu'on puisse facilement lui brancher ça, au cas où
> c'est pertinent de le laisser relire un peu ce qui lui chante

### Aujourd'hui

L'archiviste est un **graphe** (`SplitSections`, `Decide`, `Route`) : un appel au modèle par
décision, **sans outil**. Aucun nœud du dataflow ne fait tourner un agent avec des outils à
l'intérieur d'un graphe :

- `SpawnNode` lance un nœud en fond, sans rendre son résultat au graphe ;
- le pas d'agent avec outils est celui de l'équipier, dans l'hôte (`src/daemon_host/teammate.rs`),
  avec ses bornes S1 du manifeste (`teammate.max_tool_calls_per_step`, 60 ; 40 appels au modèle).

### Ce qui est à prévoir

Chaque étape peut tourner **soit en appel simple**, comme aujourd'hui, **soit en pas d'agent**,
avec une liste d'outils déclarée et bornée.

- **L'étape 2 d'abord** : quand un sujet hésite entre deux fiches, relire la fiche candidate ou les
  tours voisins vaut mieux qu'une étiquette au jugé. Le pas d'agent se termine par un outil
  `decide(label, name, about)`, qui rend exactement ce que rend le `Decide` d'aujourd'hui. La suite
  du graphe (`archive_decided`, `archive_subject`, les écritures) ne change pas.
- **L'étape 1, peut-être** : chercher dans la session si un sujet de la fenêtre a déjà paru plus
  tôt (`search_turns`), ou relire un tour tronqué (la section est coupée à 4 000 caractères dans
  le prompt), avant d'en extraire les sujets.

**Les outils candidats**, la recherche d'abord et la lecture ensuite (Lucie : « pas seulement
relire, mais chercher ») :

| Famille | Chercher d'abord | Lire ensuite | D'où |
| --- | --- | --- | --- |
| la mémoire | `recall`, `recall_fiches`, `recall_subjects`, `recall_apports`, `cited_by` (lexical et dense) | `get_fiche` | le backend memory lui-même |
| la session | `search_turns(query)` : plein texte sur les tours du run, hors de sa tranche | `read_turns(from, to)` | le `ChatFile` et le journal des runs, donc **l'hôte** |
| le code | `search_code`, `usages` | `read_file` | le backend de code déclaré, donc **l'hôte** |

Dans la liste déclarée de chaque étape (`archivist.step1.tools`, `archivist.step2.tools`), les
outils de recherche viennent en premier, et la consigne du pas le dit : on cherche, puis on lit ce
qu'on a trouvé.

**Les réglages**, sous `archivist.step2` et `archivist.step1`, dans les bindings d'`archive` :
`tools` (une liste, vide par défaut), `max_tool_calls` (8 par défaut quand la liste n'est pas vide,
bien sous les 60 de l'équipier) et `max_model_calls` (4). Tout est **débrayé par défaut** : une
liste vide garde l'appel simple, et la mesure d'aujourd'hui ne bouge pas.

### Les vrais tours, indexés au fil de l'eau avec un TTL (les L0)

Lucie : « les L0, les vrais tours, doivent être indexés au fil de l'eau avec un TTL ». C'est ce que
`search_turns` cherche.

- **Où ils vivent** : une entité neuve du gabarit memory, `Turn`, avec une ligne par tour :
  `{source, section, run, text, created_at}`.
  - `source` est le chemin de la transcription du run, une Source comme aujourd'hui.
  - `section` vaut `turn <n>`. C'est déjà la forme de l'identité d'un Apport
    (`{fiche, source, section}`), si bien qu'un L1 cite une plage de tours sans dépendre des lignes
    `Turn`.
  - Les signaux sont `bm25` et `vector`, découpés par le mécanisme générique (1 500 caractères,
    200 de chevauchement), comme les apports.
  - La Source elle-même reste en plein texte seul, et immuable.
- **Quand elles s'écrivent** :
  - **le plein texte à chaque fin de tour** : l'hôte écrit la ligne `Turn` du tour fermé, et elle
    se cherche aussitôt (lucivy, local) ;
  - **le dense par lot, à la fermeture de la tranche** : les vecteurs des tours de la tranche
    partent en un lot au service. Le mécanisme de la dette des vecteurs existe déjà (« vecteurs en
    dette », rattrapés à l'index suivant). Reste **à vérifier** qu'il se déclenche à la fermeture
    d'une tranche, plutôt qu'à chaque écriture.
- **`search_turns(query, options)`** est le `recall` générique (`search_structured.mmd`) sur
  `Turn`, filtré par `run`, avec une option pour écarter la tranche courante.
- **Le rappel ordinaire ne les trouve jamais** (Lucie : « le filtre par défaut de la recherche
  mémoire ne les trouve pas ; il faut préciser qu'on veut chercher dedans »). C'est vrai par
  construction : `Turn` est une entité à part, et `recall_fiches`, `recall_apports`,
  `recall_subjects` et `recall` visent chacun leur entité (Fiche, Apport, Subject, Memory). Aucun
  rappel existant ne lit `Turn`. **La forme explicite est l'outil `search_turns` lui-même** : on le
  choisit pour chercher dans les tours. Il n'y a pas de drapeau `include_turns` sur les autres
  rappels, qui deviendrait un jour un défaut oublié. Le témoin le vérifie : des tours indexés dont
  le texte répond à la requête n'apparaissent ni dans `recall_fiches` ni dans `recall_apports`, et
  apparaissent dans `search_turns`.
- **Deux espèces de L0, à ne pas confondre** (relecture d'archi, 5eaeef3e7) :
  - **la Source du gabarit** reste « immuable, écrite une fois ». Chaque **tranche fermée**
    s'archive comme une Source neuve, `sources/<run>-turns-<a>-<b>.md`, écrite une fois à la
    fermeture, sans TTL : c'est elle que citent les L1 ;
  - **la trace en cours** est une espèce à part, en **ajout seul** : les lignes `Turn` d'origine
    `run_trace`, une par tour, qui ne se réécrivent jamais. Elles s'effacent au TTL, et rien
    d'autre ne les modifie. Leur `source` est le chemin de la Source de leur tranche, connu dès
    l'ouverture de la tranche.
- **Un seul écrivain** : l'écriture du tour (`Turn`) passe avant le rangement. Le rangement de la
  tranche avance par petites transactions (un sujet, ses écritures), si bien qu'il ne tient jamais
  la base longtemps devant l'écriture du tour suivant.
- **Ce qui n'entre jamais dans la trace** : la trace porte le code lu et les sorties de commandes.
  Les fichiers exclus du dépôt (ignorés par git, `.vault/`, les secrets) n'y entrent pas. L'hôte
  remplace leur contenu par « exclu du dépôt » avant l'écriture du tour. Le témoin vérifie qu'un
  `read_file` sur un fichier ignoré laisse dans `Turn` la mention, et jamais le contenu.
- **L'indexation des tours est un réglage**, `memory.index_turns`. Archi le propose **éteint par
  défaut**, puis allumé dépôt par dépôt par Lucie. **La question du défaut est posée à Lucie par
  c7.** Éteint, il n'y a ni ligne `Turn` ni `search_turns`, et l'archiviste range quand même ses
  tranches (Sources et L1).
- **Le TTL** : `memory.l0_ttl_days`, dans les bindings du gabarit memory (30 jours proposés). Une
  ligne `Turn` plus ancienne est effacée, avec ses morceaux, son index et ses vecteurs. **Ne se
  touchent pas** : le `ChatFile` (l'hôte le garde), la Source de la transcription (le L0 immuable)
  et les L1, qui citent des intervalles et non des lignes.
- **Qui purge** : un outil du gabarit, `purge_turns(older_than_days)`, qui efface par
  `EntityBatchNode` ou `EntityRecordNode`, nœuds de base d'un outil. C'est **l'hôte** qui l'appelle,
  à chaque fermeture de tranche (la part d'archi), puisqu'il a la cadence du run. Une réaction ne
  convient pas : elle répond à un événement, pas à une horloge. Une tâche propre au gabarit n'a pas
  de déclencheur sans l'hôte.
- **Le coût** :
  - le plein texte est une écriture lucivy locale par tour, de quelques millisecondes ;
  - le dense revient à une tranche de 50 k jetons (~200 k caractères), soit ~130 morceaux de
    1 500 caractères, en un lot au service bge-m3, de l'ordre de la seconde. Aucun morceau ne
    dépasse le plafond de requête ;
  - la place est bornée par le TTL.
- **La part d'archi** (l'hôte) : écrire la ligne `Turn` à chaque fin de tour, déclencher le lot
  dense et `purge_turns` à chaque fermeture de tranche, et exposer `search_turns` et `read_turns`
  à l'archiviste.
- **Ma part** (le gabarit) : l'entité `Turn`, l'outil `search_turns`, l'outil `purge_turns`, et
  leurs témoins (le TTL efface les tours anciens sans toucher aux L1 ni à la Source).

### Ce que ça change au graphe

- **Un nœud neuf, `AgentStepNode`** : un pas d'agent dans un graphe, son prompt, ses outils
  nommés, ses bornes, et un outil de fin qui rend la sortie au port. Pour les outils du backend
  lui-même (la mémoire), il appelle le registre de l'outil, comme un `GraphNode`. Il est admis sous
  `call_models`, et seulement avec sa liste d'outils déclarée, dans l'esprit de la garde de
  `LlmNode` (53a83d089).
- **Dans `archive_decide.mmd`**, un `Route` sur la longueur de `tools` : vide, le `Decide`
  d'aujourd'hui ; sinon, l'`AgentStepNode`, avec les mêmes entrées et la même sortie.
- **Les outils de la session et du code** passent par l'hôte, puisqu'un backend ne voit pas les
  autres. Ils ne sont disponibles que quand l'archiviste tourne dans un run de l'hôte : c'est la
  part d'archi, au même point de branchement que les tranches (§ h). Hors de l'hôte, ils sont
  refusés en le disant, pas ignorés.
- **La trace** dit pour chaque décision les outils appelés, leurs arguments et le coût (`usage`).

### Ce que le banc mesure

Le gain contre le coût, à l'étape 2. On compare la liste vide et la mémoire seule, puis la mémoire
avec la session, puis le tout. Le banc lit le rappel et les doublons contre N, les jetons et les
appels, et le p90 par décision.

### Ce que ça coûte

- `AgentStepNode` avec les outils du backend : environ 1 jour avec ses témoins, sur un faux modèle
  qui appelle un outil puis `decide`.
- Le branchement des outils de la session et du code par l'hôte : la part d'archi, à chiffrer
  avec lui.

## (h) L'ordre et les lots : qui fait quoi

Le mot de Lucie qui règle l'ordre :

> il faut le faire en même temps qu'on débogue le reste ; l'archiviste à chaque tranche de jetons,
> bien avant les 80 %, et à la compaction tout est déjà rangé sauf la dernière tranche

L'archiviste range donc **au fil du run**, tranche par tranche (une tranche S 50 de jetons de la
conversation). À la compaction, seule la dernière tranche, pas encore rangée, passe par lui : la
compaction est courte, et le contexte d'après se compose de ce qui est déjà en mémoire.

Le code démarre demain, après la lecture de Lucie, **en parallèle de ma file de défauts, un lot à
la fois**.

| Qui | Lot | Ce qu'il livre |
| --- | --- | --- |
| **mémoire (96)** | 1. les deux étapes | l'extraction (`topics`, `slice_summary`, `open`, `attributes`), puis le rangement par sujet, avec le témoin rouge d'abord (le ticket, phase 1) |
| **mémoire (96)** | 2. l'outil sur une tranche | `archive` appelable sur un intervalle de tours de la trace d'un run (`turns: {from, to}`, et la source de la transcription), qui rend le rapport de tranche (§ b) et l'écrit avec `slice_summary` sur la fiche de la session |
| **mémoire (96)** | 3. la boîte à outils de l'archiviste | les ingrédients `archivist.context.*` (§ e), dans les bindings d'`archive` |
| **mémoire (96)** | 4. `compose_restart` en ingrédients | `restart.context.*` et les trois profils (§ f) |
| **archi** | l'hôte | ferme les tranches au fil du run, appelle l'archiviste en fond sur chaque tranche fermée, note « rangé jusqu'au tour T » dans le `ChatFile` (seul rédacteur), et à la compaction ne traite que la dernière tranche, avant d'appeler `compose_restart` et de reprendre le run. Le seuil de 80 % de la jauge reste le déclencheur de la compaction, pas du rangement. |
| **banc** | les réglages | joue F, S, R et leurs mélanges, et les trois profils d'après compaction, par les bindings seuls ; juge contre N et contre la compaction de Claude Code |

**Le contrat entre mes lots et celui d'archi** : `archive` sur un intervalle de tours rend
`{report, archived_through_turn}`. L'hôte écrit lui-même `archived_through_turn` dans le
`ChatFile`, et l'archiviste ne l'écrit pas. Le témoin de la compaction
(`cx-memoire-compaction-temoin-archi`, 3d30f16ee, chez archi) se relit à cette lumière : sa forme
« une section par échange » devient « une tranche par fenêtre de jetons ». Je le dirai à archi
avant son lot.

## (i) Le ticket

`docs/memoire/tickets/2026-11-18-l-archiviste-range-en-deux-etapes.md` porte le détail du lot des
deux étapes et de son témoin, rouge d'abord. Les catégories (d) n'ont pas encore de ticket : elles
attendent la lecture de Lucie et la part des schémas d'ED. Le lot des deux étapes vient d'abord,
et laisse dans son schéma d'extraction la place des `attributes`.
