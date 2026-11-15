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
    mais son propre tour en partie. **Tranché par l'orchestration** (12 h 4x) : chaque tour est
    coupé à 4 000 caractères aussi dans le contexte, pour que toutes les variantes voient les tours
    de la même façon. On mesure l'archiviste tel qu'il est. Le plafond levé viendra ensuite comme
    variante, avec son coût. Le plafond lui-même est un défaut du produit, en ticket chez la
    session mémoire ; son remède naturel, l'étape `segment`, sera mesuré après ;
  - **le cache** : aucun cache explicite n'est demandé à l'appel (API compatible OpenAI). Seul le
    cache implicite de Vertex joue, sur un préfixe identique. L'instruction vient d'abord, puis le
    contexte : ce qui ne change pas dans la fenêtre d'abord, le bout en cours à la fin. La part
    servie par le cache se lit dans l'usage de chaque appel (`cached_prompt_tokens`), à mesurer sur
    la première fenêtre avant d'extrapoler le coût.
- **Pas de quota : un défaut de mon pilote** (15 nov., 13 h 51). La première référence N a attendu
  « des quotas » cinq fois d'affilée au tour 12, une demi-heure en tout, puis une fois au tour 13.
  Mais Vertex n'en a rendu aucun : le tour 12 est passé du premier coup (ok, sans erreur,
  `candidate_2`). Le pilote cherchait « 429 » dans toute la réponse, texte rangé compris, et le
  texte du tour 12 le contient dans un identifiant de message (`…9ef33429e104`), celui du tour 13
  dans une phrase sur la règle des quotas. Le pilote ne lit plus que l'erreur de l'appel et les
  erreurs du rangement. La référence est relancée à 13 h 51. Le relevé des autres appelants reste
  vrai : sur les deux postes, aucun autre processus n'appelait Vertex pendant ce temps, et l'hôte
  du portable a fait son dernier appel à gemini-3.8-flash à 12 h 46.
- **Rien n'est lancé** avant le composeur vu vert et l'entrée de contexte de l'archive. La conception
  est validée par l'orchestration (15 nov., 12 h 5x).

## 6. Les premiers chiffres (15 novembre, 15 h)

**Le juge est relu** (voir plus bas) : accord de 0,90 sur les appariements, 10 sur 10 sur les doublons.

| variante | décisions : créer, rejoindre, rien | fiches | P1, rappel des sujets de N | P2, doublons | s par décision (p50, p90) | passe | envoyé au modèle |
|---|---|---|---|---|---|---|---|
| **N**, sans contexte | 82, 124, 38 | 78 | (référence) | **9 (0,115)** | 6,2 ; 13,5 | 30 min | les sections seules |
| **F à 50 k**, chevauchement 10 %, résumés 10 k | 66, 106, **72** | 30 | **0,81** (63 sur 78) | 1 (0,033) | 7,9 ; 15,6 | 38 min | 14 788 793 car. de contexte et 722 399 de sections, soit un plafond de **3 877 798 jetons** à 4 caractères, sans cache, instruction et candidates en plus |

- **Sans contexte, l'archiviste casse lui-même au critère des doublons** : 11,5 % des fiches de N
  en doublent une autre, au-dessus du seuil de 10 %.
- **Le contexte ôte presque tous les doublons** : F à 50 k regroupe le corpus en 30 fiches, avec un
  seul doublon.
- **Mais il fait dire « rien » deux fois plus souvent** : 72 tours contre 38. C'est peut-être de
  là que vient la perte de rappel (0,81, à la limite de la casse). L'archiviste croirait déjà rangé
  un sujet qu'il n'a pas rangé. C'est à départager à la relecture : lire les tours « rien » de F que
  N a rangés.
- **Le coût** : 3,9 millions de jetons au plafond pour F à 50 k. Le réel, cache compris, se lit
  dans la console du projet.
- **Une réserve sur les temps** : F à 50 k a tourné en partie pendant les parités de l'écrivain, à
  côté d'un index qui chargeait 7879.
- **Les 72 « rien » de F à 50 k, contre ce que N a fait des mêmes tours** (au mot de c7 : le contexte
  aide-t-il ou endort-il ?) :

  | cas | tours |
  |---|---|
  | **le contexte aide** : N a rangé le tour sous un sujet que F couvre ailleurs | **34** |
  | **le contexte endort** : N l'a rangé sous un sujet que F ne couvre nulle part | **22** |
  | accord : N dit « rien » aussi | 16 |

  - **Les tours endormis perdent des sujets entiers**, au moins neuf. Par exemple : « Bissection
    des hotfixs rag3db », « File d'atterrissage sur master », « Indexation des worktrees dans
    rag3weaver », « Intégration de la pile archi », « Licences des modèles locaux » ;
  - le contexte aide plus souvent qu'il n'endort, mais il endort près d'une fois sur trois ;
  - « aide » ne veut pas dire que rien n'est perdu : le sujet existe dans F, mais ce tour n'y est
    pas rangé.
- **La relecture du juge** (par moi, à la main) :
  - **30 appariements tirés au sort** (graine 7) : 27 justes, 1 faux (« Libération d'espace
    disque » apparié à « Charge système et ressources de luciepc »), 2 douteux (« Vision produit et
    positionnement commercial » pour « Présentation et synthèse du projet » ; « Surveillance et
    santé » pour « Utilisation des GPU »). Accord : 0,90 ;
  - **les 10 doublons relevés** (9 dans N, 1 dans F) sont tous de vrais doublons : deux fiches
    « Identité Git », deux « seat1 », deux « TRIM NVMe », etc. Les doublons manqués ne sont pas
    mesurés ;
  - **le juge est indulgent** : une fiche large de F (« Atelier rag3weaver ») couvre pour lui
    plusieurs fiches précises de N (les tests de la pile, l'intégration, le backend de l'atelier).
    P1 compte un sujet absorbé comme couvert. La précision perdue par cette absorption n'est pas
    mesurée.
- **La suite** :
  1. fait : le juge relu, et les tours « rien » de F départagés ;
  2. F à 50 k avec 25 % de chevauchement, et S à 50 k ;
  3. 300 k quand le lot `usage` de la session mémoire sera posé.
- **Les sorties**, à luciepc, dans `~/.cache/rag3db-banc-notes/compaction/long/` :
  `ref-N-20261115-1351/`, `F50-ov10-20261115-1422/`, `juge-F50-contre-N.json`,
  `juge-N-contre-N.json`.

## 7. La mesure arrêtée : chaque Source longue fait tomber le 7879 (15 novembre, 15 h 22)

- **La cause** (embarquement) : un tour rangé entier en Source, au-delà d'environ 6 000 jetons, demande
  à bge-m3 la matrice d'attention d'un seul texte. 4 102 146 304 octets font exactement 16 têtes ×
  4 octets × 8 006², au plafond de bge-m3. La réservation est refusée sur card2, le service panique et
  sort, puis systemd le relance en une vingtaine de secondes. Le tunnel du portable tombe avec lui :
  les hôtes de Lucie le perdent aussi.
  - Ce n'est pas un lot trop gros : `RAG3WEAVER_EMBED_CHAR_BUDGET` n'y peut rien, puisqu'un texte plus
    long que le budget part seul.
  - Le corpus a 21 tours de plus de 16 000 caractères, dont 2 de plus de 32 000 ; le plus long en
    fait 43 137.
  - Les trois passes rangent les mêmes tours dans le même ordre, d'où la même suite de douze tailles,
    rejouée à chaque passe.
- **Les pannes, lues dans le journal du 7879** (`failed to reserve`, heure de Paris), passe par passe :

  | passe | réservations refusées |
  |---|---|
  | N (13 h 51 – 14 h 22) | 24 |
  | F à 50 k (14 h 22 – 15 h 05) | 24 |
  | F à 50 k, 25 % (15 h 05 – 15 h 21) | 14 |

  Il y en a 62 dans la journée, toutes pendant ces passes, et systemd compte 26 redémarrages : une
  panne écrit plusieurs refus. **C'est une réserve sur les temps par décision** de N et de F : à
  chaque panne, environ 20 s de service absent. Aucune décision n'est en erreur, mais des vecteurs ont
  pu manquer à l'écriture. C'est à compter sur les copies.
- **L'arrêt** (au mot de c7, les hôtes de Lucie passent devant la mesure) :
  - F à 50 k avec 25 % de chevauchement s'est arrêtée à 105 tours sur 244, juste avant 15 h 21, sans
    message. C'est peut-être le dépassement de mémoire de sa portée, ce n'est pas vérifié ;
  - S à 50 k est partie à 15 h 21, et je l'ai coupée à 15 h 22, avant sa première décision ;
  - aucun processus de la mesure ne reste. Le dernier redémarrage du 7879 date de 15 h 20 min 51 s.
- **Pas de contournement dans le banc** : tronquer la Source changerait la mesure sans le dire. La
  mesure reprend quand le lot de la session mémoire découpera la Source longue dans le gabarit
  memory (`query_chars`, en pile derrière `usage`), sur le même corpus et avec les mêmes variantes :
  F à 25 %, puis S, puis 300 k.

## 8. La reprise sur 0653a6aeff, et ses trois faux départs (15 et 16 novembre)

**La cause de l'arrêt est levée** (session mémoire et embarquement) : la Source n'est pas embarquée
(`signals: ["bm25"]`), et les L1 sont découpés à l'écriture (1 500 caractères, 200 de
chevauchement). Le texte de 8 006 jetons était la requête de la recherche des candidates, la section
entière. Trois plafonds sont maintenant posés :
- `query_chars`, 4 000 caractères, dans le gabarit memory ;
- `query_max_chars`, 6 000 caractères, sur toute requête embarquée, dans l'hôte ;
- dans le service lui-même : un texte de plus de 12 000 caractères y est refusé par un 413 nommé.

La reprise tourne sous un veilleur, qui arrête tout au premier refus de réservation ou au premier
redémarrage du 7879.

**Trois faux départs, tous de mon fait** :
1. **20 h 38** : dans la boucle qui somme l'usage, j'ai réutilisé le nom `k`, celui de l'indice de
   fenêtre. Chaque passe a planté dès sa première décision (`TypeError`), et le script a enchaîné les
   quatre passes sans rien faire en 26 secondes, puis il a écrit « FIN-REPRISE ».
2. **20 h 49** : la garde ajoutée pour l'éviter (une passe sans ses 244 décisions arrête la reprise)
   lisait le nom de la passe dans le mauvais argument. Je l'ai vu en relisant, et la reprise a été
   arrêtée et relancée avant toute décision.
3. **20 h 49 à 0 h 04** : `rsync` a recopié le script sans son droit d'exécution, puisqu'il venait
   d'un fichier en 644 du portable. `poste lourd` l'a refusé (« Failed to execute: Permission
   denied »). Rien n'a tourné pendant trois heures et quart, et je ne l'ai pas vu : je n'avais pas
   relu le journal après le lancement.

**Ce qui est changé** :
- le pilote ne réutilise plus de nom de la boucle principale ;
- la garde de la reprise compte les décisions de chaque passe ;
- **la règle** : un lancement se relit dans la minute, journal compris, avant d'annoncer qu'il
  tourne. Un `rsync` qui copie un script se fait suivre d'un `chmod +x`.

La reprise est partie à 0 h 04 min 54 s : F 25 % par défaut, puis avec la variante de consigne, puis
S 50, puis 300 k.
