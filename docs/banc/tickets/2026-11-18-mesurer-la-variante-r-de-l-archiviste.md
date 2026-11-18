# Mesurer la variante R de l'archiviste

- **État** : ouvert ; attend la forme en deux étapes de l'archiviste (page de la session mémoire, 18 novembre au soir).
- **Famille** : produit
- **Gravité** : moyenne. C'est le choix de la forme de l'archiviste pour les sessions longues.
- **Atteignable en service** : oui. L'archiviste range les sessions longues.
- **Touche rag3weaver** : non. Une mesure.
- **Pour** : banc.
- **Signalé par** : c7, 18 novembre 2026.
- **Ouvert le** : 18 novembre 2026

## La variante

**R** : des fenêtres de 50 k sans chevauchement. Chaque fenêtre reçoit le **rapport structuré** que l'archiviste a rendu
à la fin de la précédente, à la place des résumés de S :
- les sujets et leur rangement ;
- les catégories proposées ;
- les sujets ouverts ;
- ce qui n'est pas rangé.

## La mesure

- Sur le même corpus que la mesure longue (`c7-99bc8451-f5`, 244 tours), avec la forme en deux étapes voulue par
  Lucie : d'abord l'extraction structurée des sujets de la fenêtre, sans la base, puis le rangement de chaque sujet.
- Contre S 50 (la forme de fenêtre retenue) et contre la N rejouée (`long/ref-N-0653-20261116-0305`).
- Avec le juge de la page du 15 (§ 4 et § 10) : P1, P2, les « rien » qui endorment, s par décision, jetons et part du
  cache.
- Le témoin d'abord : N contre N, P1 = 1.
- Le script s'arrête sur une erreur du modèle (`x_archive_long.py`, depuis le 18 novembre).

## Une boîte à outils, pas des variantes (c7, 18 novembre, au mot de Lucie)

Le contexte de l'archiviste se compose d'**ingrédients déclarés et débrayables**, chacun avec sa borne :
- le rapport de la tranche d'avant ;
- le résumé de la tranche d'avant, par l'extraction ;
- le résumé courant ;
- les résumés de S ;
- le chevauchement de F, en % ;
- le bilan ;
- les candidates.

F, S, R et leurs mélanges deviennent des **réglages** joués sur le même corpus, pas des variantes codées à part.

**La matrice à mesurer d'abord** (S 50 comme forme de fenêtre, la forme en deux étapes) :

| réglage | rapport de la tranche d'avant | résumé de la tranche d'avant |
|---|---|---|
| S 50 nu | non | non |
| S 50 + rapport (R) | oui | non |
| S 50 + résumé de tranche | non | oui |
| S 50 + les deux | oui | oui |

Chaque réglage est jugé contre la N rejouée, en **P1** (rappel des sujets), **P2** (doublons) et **P5** (coût : temps de
passe, appels, jetons d'entrée et part du cache), avec les définitions du § 4 de la page du 15. Les autres
ingrédients restent à leur réglage par défaut et sont écrits avec chaque passe. La matrice se joue après la page de
la session mémoire qui pose la forme.

## Le centre de gravité (c7, 18 novembre)

Lucie soupçonne un effet « centre de gravité » : une grosse fiche qui absorbe ce qui est distinct. Deux nombres sont
relevés sur chaque réglage, sans calcul de plus au modèle :
- **la concentration** : la part des L1 rangés dans la plus grosse fiche, et un coefficient de Gini des tailles de
  fiches ;
- **la cohésion intra-fiche** : la dispersion des vecteurs des L1 autour du centroïde de leur fiche (distance cosinus
  moyenne au centroïde, par fiche puis pondérée par la taille).

Une fiche à la fois grosse et dispersée est le signe de l'absorption. La forme des données (L1, fiches, vecteurs) est
décrite par la session mémoire.

## Le rappel des décisions rares (c7, 18 novembre, au mot de Lucie d'après un avis extérieur)

Une métrique de plus pour chaque réglage : **le rappel des décisions rares**. C'est P3 (« perdu au milieu ») rendu
ciblé. Les décisions rares sont :
- les contraintes négatives dites une fois (« ne jamais laisser un agent supprimer cette donnée sans confirmation ») ;
- les changements d'intention ;
- les décisions brèves noyées dans 50 k de débogage.

**La forme** :
- un jeu de phrases **plantées** dans une copie du corpus, à des positions connues : début, milieu et fin de
  fenêtre ;
- chaque phrase porte une décision au sens net, et son sens attendu est écrit à côté, dans le jeu ;
- pour chaque réglage, la mesure rend la part des phrases retrouvées dans un L1 **avec le bon sens**, par position.
  Une contrainte négative retrouvée à l'envers compte comme perdue ;
- le juge lit les L1 sans savoir quel réglage les a produits (il sait quelle phrase il cherche).

**Où la jouer** : sur chaque réglage de la matrice, et aussi sur le mode « tours bruts par segment » que la session
mémoire ajoute (`archivist.step2.input = raw_turns`).

## Le contexte d'après compaction, réglable aussi (c7, 18 novembre)

Le contexte qu'une session reçoit après une compaction se compose lui aussi d'ingrédients. Trois profils sont à juger
**contre la compaction de Claude Code**, sur des transcripts réels, dans la mesure prévue à la page
`docs/banc/15-11-2026/01-l-archiviste-contre-la-compaction.md` :
- les L1 seuls, dans l'ordre des tours ;
- le résumé global des tranches seul ;
- les deux.

**La vérité de terrain** est ce que les tours suivants ont eu besoin de savoir. Chaque profil est jugé sur la part de ce
besoin qu'il couvre, et sur son coût en jetons. Cela se fait après la page de la session mémoire.

**Par distance** (c7, 18 novembre, d'après un avis extérieur) : une contrainte perdue se révèle tard. Chaque profil est
donc jugé sur les besoins des tours suivants **par distance de la frontière** : les 10 premiers tours, jusqu'à 30, et
jusqu'à 100. Cela donne trois nombres au lieu d'un, et les erreurs que l'agent commet loin de la reprise comptent.

**Les combinaisons** : seulement des combinaisons informatives (un ingrédient de plus ou de moins à la fois autour de
S 50), pas toutes les permutations.

## Ce qui est préparé (18 novembre au soir, sans calcul)

Tout est à luciepc, sous `~/.cache/rag3db-banc-notes/compaction/`, avec une copie sur la tablette :
- **le jeu de phrases plantées** : `planted-set.json` (sha256 `6cc5406d…`), 24 phrases, dont 8 contraintes négatives, 8
  changements d'intention et 8 décisions brèves, chacune avec son sens attendu ;
- **l'implantation** : `plant_corpus.py`, avec son témoin `test_plant_corpus.py`, vu rouge puis vert.
  - Le corpus planté est `corpus/c7-99bc8451-f5-planted/`, avec `corpus.jsonl` (sha256 `9cfaaebc…`) et le manifeste
    `planted-positions.json`.
  - Les 24 phrases se répartissent sur les 19 fenêtres de 50 k, 8 en début, 8 au milieu et 8 en fin.
  - Les fenêtres sont identiques avant et après l'implantation ;
- **le juge des décisions rares** : `j_rare.py`, avec son témoin `test_j_rare.py` sur un modèle simulé (`JUDGE_STUB`),
  vu rouge puis vert.
  - Il prend les L1 d'une passe sous la forme neutre `[{fiche, text}]` ; l'adaptateur viendra avec la forme de la
    session mémoire.
  - Il rend le rappel, les phrases inversées, et le compte par position et par type.
  - Le juge sait quelle phrase il cherche. Il ne sait pas quel réglage a produit les L1 ;
- **les distances d'après compaction** : `x_frontieres.py` prend `DISTANCES=10,30,100`. Le défaut reste 60 et 20, et
  l'ancienne version est gardée en `x_frontieres.py.avant-distances`. Le bilan dit, pour chaque frontière, combien de
  tours chaque fenêtre contient vraiment. Essayé sur un transcript de 17 frontières : la 17ᵉ n'a que 18 tours après elle ;
- **reste** : le lanceur de la matrice. Il attend les noms des réglages que la session mémoire déclarera.

## Pour fermer

Les quatre réglages de la matrice joués dans la forme en deux étapes, jugés contre N en P1, P2 et P5 ; les trois profils d'après compaction jugés contre la compaction de Claude Code ; les deux résultats écrits dans leurs pages.
