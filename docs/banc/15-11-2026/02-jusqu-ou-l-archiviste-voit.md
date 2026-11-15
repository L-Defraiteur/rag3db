# Jusqu'où l'archiviste voit : un vrai long contexte (15 novembre 2026)

Mot de Lucie, par l'orchestration (c7, 12 h 4x), « vraiment important ». C'est la première mesure
du banc de la compaction (page 01) : la comparaison contre le résumé de Claude Code vient après.
Cette page dit le corpus, puis ce qu'on mesure et comment, avant tout appel.

## 1. Le corpus

- **La source** : le transcript de l'orchestration, `99bc8451-…`, entre les frontières de compaction
  des lignes 27 496 et 35 037. Claude Code a compacté ce segment à 967 171 jetons.
- **Rien n'est coupé** : tours, appels d'outils et leurs arguments, résultats, pièces jointes,
  messages reçus des autres sessions, réflexions, dans l'ordre du transcript. En tête, le résumé
  de la compaction d'entrée : c'est ce que le modèle avait au début du segment.

| | |
|---|---|
| tours | 244 (un tour commence à un message de l'utilisateur ou à un message de session livré) |
| éléments | 3 235 : 540 appels d'outils, 540 résultats, 106 messages reçus, 1 026 pièces jointes, 456 réflexions, 284 messages de l'utilisateur, 278 réponses de l'agent, 4 lignes système, 1 résumé |
| caractères | 1 891 600 |
| jetons | **880 233** : le contexte réel passe de 86 182 au premier appel de l'agent à 966 415 au dernier |
| jetons par tour | médiane 2 935, p90 9 065, max 24 551 |
| secrets masqués | 2 |

- **Les jetons** viennent de l'usage que porte chaque réponse de l'agent (`input_tokens` +
  `cache_read_input_tokens` + `cache_creation_input_tokens`). C'est le compte de Claude lui-même,
  pas une estimation. Le compte d'un tour est l'écart entre son premier appel et celui du tour
  suivant. 28 tours sans appel de l'agent n'ont pas de compte propre, mais ils sont dans le total.
- **Les fichiers**, à luciepc et au portable, hors du dépôt, dans
  `~/.cache/rag3db-banc-notes/compaction/corpus/c7-99bc8451-f5/` :
  - `corpus.jsonl` : un enregistrement par élément (ligne, tour, genre, caractères, texte,
    contexte) ;
  - `corpus.md` : lisible, en sections `## turns n-n` ;
  - `tours.json` : par tour (jetons, caractères, appels, messages reçus).
- **Le script** : `x_corpus.py`, à côté. Il extrait n'importe quel segment de n'importe quel
  transcript.

## 2. La question

Jusqu'où l'archiviste range un vrai long contexte sans dérailler : où il perd des sujets, où il les
duplique, où il oublie. Et quels contournements tiennent.

**La forme de Lucie, à mesurer d'abord** (par c7, 12 h 4x) : « comme s'il y avait trois
conversations de 300 k au lieu d'une de 900 ». L'archiviste range la conversation fenêtre par
fenêtre. Chaque fenêtre lui arrive avec deux choses :
- **les résumés déjà extraits des fenêtres d'avant** : leurs L1 et le bilan courant, rendus par le
  composeur de la session mémoire dans la forme que la fenêtre suivante relit ;
- **un bout du texte qui précède** : le chevauchement, les derniers tours de la fenêtre d'avant.

Puis on recommence avec la fenêtre suivante.

L'outil `archive` décide section par section. Une fenêtre lui est donc donnée comme **une seule
section** de tous ses tours (`## turns a-b`). Les résumés et le chevauchement lui arrivent **en
contexte**, non comme contenu à ranger. C'est l'argument à demander à la session mémoire (§ 5).

## 3. Les variantes

Les tailles sont comptées en jetons du corpus, par tours entiers. Le corpus en fait 880 233.

| variante | fenêtre | chevauchement | résumés d'avant |
|---|---|---|---|
| **F, la forme de Lucie** | 300 k (trois fenêtres) | 10 % puis 25 % | oui |
| F, à d'autres tailles | 50 k, 100 k, 200 k | 10 % et 25 % | oui |
| **U, la fenêtre unique** | 880 k (le segment entier) | aucun | aucun |
| **S, sans chevauchement** | aux mêmes tailles que F | 0 | oui |
| **N, nues** | aux mêmes tailles que F | 0 | non |

- **La référence** : le rangement au fil de l'eau, une section par tour (244 sections closes). Ses
  sujets, c'est-à-dire les fiches qui reçoivent au moins un L1, sont la liste de référence des
  sujets du corpus. Cette liste est relue à la main avant de servir de juge (§ 5). Elle dit aussi
  le coût du rangement le plus fin.
- **Les copies** : chaque variante, à chaque taille, a sa propre copie neuve du gabarit memory,
  vide, sans la mémoire de Lucie. Une copie de son instantané rendrait les candidates réalistes,
  mais c'est une base durable : seulement sur son mot.
- **L'ordre** : F à 300 k d'abord, avec ses deux chevauchements, puis U, puis S et N à 300 k. Les
  autres tailles viennent ensuite, si F à 300 k tient ou pour trouver où il casse.

## 4. Les métriques, par variante et par taille

- **P1, le rappel des sujets** : la part des sujets de la référence qui reçoivent un L1. Gemini
  apparie les sujets, aveugle à la variante : il propose des couples, puis les confirme.
- **P2, les doublons** : les fiches créées qui en doublent une autre de la même variante, surtout
  d'une fenêtre à l'autre. C'est ce que les résumés d'avant doivent empêcher.
- **P3, l'oubli par position** : pour chaque fenêtre, quels tours sont représentés dans ses L1 (un
  tirage de tours, jugé par Gemini), selon leur position dans la fenêtre : début, milieu, fin.
  C'est le « perdu au milieu », et le bord que le chevauchement doit couvrir.
- **P4, les erreurs** : les fenêtres sans décision, les troncatures (le plafond de Decide) et les
  refus.
- **P5, le coût** : le temps de passe, les appels au modèle et les jetons d'entrée envoyés, résumés
  et chevauchement compris.
- **La casse** se lit à trois signes :
  1. une erreur ;
  2. P1 sous 0,8 ;
  3. plus d'une fiche neuve sur dix en double.

  On en tire **la plus grande fenêtre qui tient**, à chaque chevauchement.

**La question de Lucie y répond ainsi** : jusqu'où l'archiviste voit, c'est la casse de N et de U.
Avec quelle fenêtre et quel chevauchement il tient, c'est F contre S et N, à la même taille.

## 5. La validation et les bornes

- **Le juge** : avant de publier un chiffre, je relis à la main 30 appariements de sujets, 30
  doublons et 30 représentations de tours. Sous 0,8 d'accord, rien n'est publié.
- **La passe** :
  - l'archiviste de la session mémoire sur Gemini, par l'hôte, sur des copies ;
  - les appels du juge en série, avec une attente doublée à partir de 60 s sur un quota ;
  - les secrets sont masqués avant tout envoi.
- **Le coût prévu** :
  - la référence fait 244 décisions, sur environ 880 000 jetons d'entrée ;
  - chaque variante à chaque taille fait 880 000 jetons d'entrée, plus ses résumés et son
    chevauchement ;
  - les quatre variantes à 300 k font peu de décisions (trois ou quatre fenêtres chacune), mais
    environ 4 millions de jetons d'entrée ;
  - plus le juge.
- **Ce que dit la session mémoire** (96, 15 nov.) :
  - **le composeur** est `compose_restart` (sources = les fenêtres d'avant, un budget). Il rend le
    bilan, une ligne par L1 dans l'ordre des tours, puis les L1, jusqu'au budget. Il est sur la
    branche `cx-memoire-compaction-temoin` (bc93dc781), pas encore vu vert ;
  - **l'entrée de contexte n'existe pas**. 96 propose `context`, montré au modèle avant la section
    sous « Ce qui précède (déjà rangé, ne pas ranger) », jamais écrit en L0 ni en L1 ;
  - **une section ne va qu'à une fiche**, et c'est structurel : l'identité d'un L1 est {fiche,
    source, section}. Une fenêtre donnée en une section serait réduite à un sujet. La forme juste
    est **un découpage avant la décision** : une étape `segment`, éteinte par défaut, où le modèle
    lit la fenêtre (avec son contexte) et rend la liste de ses sujets en sous-sections
    (`[{titre, turns: "a-b"}]`, quelques centaines de jetons). Chaque sous-section passe ensuite par
    la décision d'aujourd'hui.
  - **Ces deux ajouts changent la conception de l'archiviste** : 96 ne les pose qu'au mot de
    l'orchestration, après le composeur.
- **Ce que la mesure juge donc** : la question « jusqu'où l'archiviste voit » est d'abord celle de
  l'étape `segment`. À quelle taille de fenêtre, avec quel chevauchement et quels résumés, le
  modèle rend-il encore les sujets de sa fenêtre sans en perdre, sans en doubler, sans oublier le
  milieu ? La décision par sujet, elle, ne change pas.
- **Rien n'est lancé** avant que l'orchestration ait validé, et avant ces trois réponses.
