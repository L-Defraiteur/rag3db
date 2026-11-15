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

L'outil `archive` découpe son contenu par ses titres, puis décide **section par section**. « Voir
N tours » veut donc dire ranger une section qui porte N tours d'un bloc (`## turns a-(a+N-1)`).
**La variable, c'est la taille de la section en tours.**

## 3. Le protocole

- **Les niveaux** : N = 1, 2, 5, 10, 20, 50, 100, puis 244 (le segment entier). Chaque niveau range
  tout le corpus, en sections de N tours, en paquets d'environ 20 sections, avec une source par
  paquet.
- **Les copies** : chaque niveau a sa propre copie neuve du gabarit memory, vide, sans la mémoire
  de Lucie. Une copie de son instantané rendrait les candidates réalistes, mais c'est une base
  durable : seulement sur son mot.
- **La référence** : le niveau N = 1, le rangement au fil de l'eau par sections closes. Ses
  sujets, c'est-à-dire les fiches qui reçoivent au moins un L1, sont la liste de référence des
  sujets du corpus. Cette liste est relue à la main avant de servir de juge (§ 5).
- **La casse** se lit à trois signes :
  1. **une erreur** : la réponse tronquée au plafond de Decide, un refus, une section sans
     décision ;
  2. **la perte** : le rappel des sujets de la référence tombe sous 0,8 ;
  3. **la duplication** : une fiche neuve qui double une fiche du même niveau devient plus d'une
     sur dix.

## 4. Les métriques, par niveau N

- **P1, le rappel des sujets** : la part des sujets de la référence qui reçoivent un L1 au niveau
  N. Gemini apparie les sujets des deux niveaux, aveugle au niveau, avec une liste de couples
  proposés puis confirmés.
- **P2, les doublons** : les fiches créées au niveau N qui en doublent une autre du même niveau.
  Gemini juge les couples proches par le nom et le résumé.
- **P3, l'oubli par position** : pour chaque section de N tours, quels tours sont représentés dans
  ses L1 (un tirage de tours, jugé par Gemini). On en tire la courbe de la représentation selon la
  position dans la section : début, milieu, fin. C'est le « perdu au milieu ».
- **P4, les erreurs** : les sections sans décision, les troncatures et les refus, comptés par
  niveau.
- **P5, le coût** : le temps de passe, les appels au modèle et les jetons d'entrée envoyés, par
  niveau.
- **La casse** : le plus petit N où l'un des trois signes du § 3 apparaît.

## 5. Les contournements, joués au niveau de la casse et au-dessus

1. **Au fil de l'eau par sections closes** : c'est N = 1, la référence. Son prix, c'est P5.
2. **Des fenêtres chevauchées** : des sections de N tours qui avancent de N/2. Chaque tour est vu
   deux fois. On mesure si le chevauchement rend les sujets du bord et ce qu'il coûte en doublons.
3. **Un bilan courant relu avant chaque fenêtre** : avant chaque section, l'archiviste reçoit le
   bilan ordonné des fenêtres déjà rangées, en contexte et non comme contenu à ranger. Il faut
   pour ça un argument de l'outil archive qui ne crée rien, à demander à la session mémoire.

Chaque contournement est jugé par P1 à P5 contre la référence et contre le niveau nu.

## 6. La validation et les bornes

- **Le juge** : avant de publier un chiffre, je relis à la main 30 appariements de sujets, 30
  doublons et 30 représentations de tours. Sous 0,8 d'accord, rien n'est publié.
- **La passe** :
  - l'archiviste de la session mémoire sur Gemini, par l'hôte, sur des copies ;
  - les appels du juge en série, avec une attente doublée à partir de 60 s sur un quota ;
  - les secrets sont masqués avant tout envoi.
- **Le coût prévu** :
  - le niveau N = 1 fait 244 décisions ;
  - l'ensemble des niveaux en fait environ 460, sur à peu près 7 millions de jetons d'entrée ;
  - plus les contournements et le juge.
- **Rien n'est lancé** avant que l'orchestration ait validé, et avant les deux réponses de la
  session mémoire : l'entrée de l'archive qui reçoit un contexte sans le ranger, et sa limite de
  taille par section.
