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

**Ce qu'est une fenêtre** (c7, validé le 15 nov. à 12 h 5x, avec une précision de Lucie) :
- **la fenêtre n'est pas une section : c'est le budget de ce que le modèle voit.** Dedans, les
  sections restent par tour, comme au fil de l'eau, avec une décision chacune. Chaque décision
  reçoit en contexte les résumés des fenêtres d'avant, le chevauchement, et le bout de la fenêtre
  en cours qui précède sa section. Sinon, le plafond de 4 096 jetons de sortie de Decide casserait
  la fenêtre unique pour une mauvaise raison : on mesurerait la sortie, pas la vue ;
- **l'archiviste sait où il est** : le contexte de chaque fenêtre dit sa place (« fenêtre 2 sur 3,
  tours a-b ; voici ce qui est déjà rangé »), et qu'une fenêtre suit. Il ne conclut pas la
  séance ;
- **ce qu'on mesure** : ce que le contexte large (les résumés et le chevauchement) change au
  rangement par section, contre le rangement sans contexte.
- **Ce qu'il faut de la session mémoire** : l'entrée de contexte de l'outil archive (§ 5), et le
  composeur des résumés. L'étape `segment` n'est pas nécessaire à cette mesure.

## 3. Les variantes

Les tailles sont comptées en jetons du corpus, par tours entiers. Le corpus en fait 880 233. Dans
chaque variante, les 244 tours sont rangés un par section, avec une décision chacun : seul change ce
que chaque décision voit en contexte.

| variante | ce que voit chaque décision, en plus de sa section | budget de la fenêtre |
|---|---|---|
| **N, au fil de l'eau** (la référence) | rien | — |
| **F, la forme de Lucie** | sa place ; les résumés des fenêtres d'avant ; le chevauchement (10 % puis 25 % de la fenêtre d'avant) ; le bout de la fenêtre en cours | 300 k (trois fenêtres) |
| F, à d'autres tailles | la même chose | 50 k, 100 k, 200 k |
| **S, sans chevauchement** | sa place ; les résumés d'avant ; le bout de la fenêtre en cours | aux mêmes tailles que F |
| **U, la fenêtre unique** | tout ce qui précède sa section dans le segment | 880 k |

- **La liste des sujets** : celle de N, relue à la main avant de servir de juge (§ 5). Chaque
  variante est jugée contre elle, et N contre la relecture.
- **Les copies** : chaque variante, à chaque taille, a sa propre copie neuve du gabarit memory,
  vide, sans la mémoire de Lucie. Une copie de son instantané rendrait les candidates réalistes,
  mais c'est une base durable : seulement sur son mot.
- **L'ordre** : N, puis F à 300 k avec ses deux chevauchements, puis S et U à 300 k et 880 k. Les
  autres tailles viennent ensuite, si F à 300 k tient ou pour trouver où il casse.
- **Le coût de la vue** : chaque décision reçoit jusqu'au budget en contexte. F à 300 k envoie de
  l'ordre de 244 × 150 k ≈ 37 millions de jetons d'entrée. U en envoie de l'ordre de
  244 × 440 k ≈ 107 millions, sauf cache de contexte chez Gemini. Le contexte d'une fenêtre ne
  change qu'au bout en cours : le cache de préfixe de Vertex le rend bon marché. C'est à vérifier
  sur les deux premières fenêtres avant le reste.

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

**La question de Lucie y répond ainsi** :
- **jusqu'où l'archiviste voit** : la plus grande fenêtre où F tient encore, et ce que devient U ;
- **avec quel chevauchement** : F à 10 % et à 25 %, contre S ;
- **ce que le contexte apporte** : F et S contre N, le rangement sans contexte.

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
  - **ce qui en sort pour cette mesure** (c7, 12 h 5x) : les sections restent par tour, donc l'étape
    `segment` n'est pas nécessaire. Il faut seulement l'entrée de contexte, et c'est un changement
    de l'archiviste, que l'orchestration a validé.
- **Deux faits de l'archiviste, pour lire la mesure** (96, 15 nov.) :
  - **la section est coupée à 4 000 caractères** dans ce que voit la décision
    (`archive_candidates.rhai`, un plafond ancien). Le L1 garde la section entière, et le contexte
    n'est pas coupé. Ici, un tour fait en médiane 4 132 caractères (p90 15 236), et 130 tours sur 245 dépassent
    4 000. Sans contexte (N), la décision ne voit que le début de ces tours ; avec contexte (F, S, U), elle voit en entier les tours d'avant,
    mais son propre tour en partie. Le biais n'est pas le même d'une variante à l'autre : à
    trancher avant de lancer. Soit le plafond est levé pour la mesure, soit chaque section est
    coupée à 4 000 caractères aussi dans le contexte ;
  - **le cache** : aucun cache explicite n'est demandé à l'appel (API compatible OpenAI). Seul le
    cache implicite de Vertex joue, sur un préfixe identique. L'instruction vient d'abord, puis le
    contexte : ce qui ne change pas dans la fenêtre d'abord, le bout en cours à la fin. La part
    servie par le cache se lit dans l'usage de chaque appel (`cached_prompt_tokens`), à mesurer sur
    la première fenêtre avant d'extrapoler le coût.
- **Rien n'est lancé** avant le composeur vu vert et l'entrée de contexte de l'archive. La conception
  est validée par l'orchestration (15 nov., 12 h 5x).
