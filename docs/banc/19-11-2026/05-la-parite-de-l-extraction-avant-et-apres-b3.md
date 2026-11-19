# La parité de l'extraction de l'archiviste, avant et après B3

19 novembre 2026, au soir, à la demande de c7. B3 (session mémoire, `cx-memoire-b3-extraction` 786f08a837) réduit l'invite
d'extraction d'`archive_slice` à une ligne de consigne plus la tranche. Toutes les règles passent dans les descriptions
des champs du schéma ; même chose pour le découpage. b2 est la tête d'avant (`cx-memoire-b2-presentation` f582d01e7a).

## Le dispositif

- **Le corpus** : celui de la mesure longue (`c7-99bc8451-f5`, 244 tours), coupé en 19 fenêtres de 50 000 jetons sans
  chevauchement (S 50 nu : ni rapport ni résumé de la tranche d'avant).
- **Le jeu** : chaque fenêtre est une tranche, rendue sous la forme de `render_turns`, rangée par `archive_slice` (avec
  `allow_create` et `trace`) sur une base neuve par passe. Gemini 3.8 Flash sur Vertex, à luciepc. Les deux passes sont
  jouées en série, b3 de 20 h 58 à 21 h 30, b2 de 21 h 45 à 22 h 09.
- **Un binaire par passe.** Les scripts de mémoire sont enregistrés dans le binaire (b3 a retiré
  `archive_slice_prompt.rhai`). Un premier jeu de b2 sur le binaire de b3 a échoué sur « unknown registered script » ;
  b2 a été rejoué sur son propre binaire.
- **Le juge** : `j_sujets.py` (page du 15, § 4) : P1 contre la N rejouée (`long/ref-N-0653-20261116-0305`), P2 les
  doublons. Puis chaque passe contre l'autre.
- **Les outils** sont à luciepc, sous `~/.cache/rag3db-banc-notes/compaction/` : `x_slice.py`, `b3-parite.sh`,
  `b2-rejeu.sh`, `b3-analyse.sh`. Les sorties sont dans `b3/`.

## Les deux colonnes

| | b2 (f582d01e7a) | b3 (786f08a837) |
|---|---|---|
| fenêtres en échec de l'extraction | **3** sur 19 (12, 15, 18) | **5** sur 19 (7, 8, 10, 17, 18) |
| temps de la passe | 23,3 min | 31,7 min |
| décisions (rangements) | 78 | 69 |
| fiches | 32 | 27 |
| **P1 contre N**, passe entière | **0,778** (42/54) | **0,704** (38/54) |
| **P1 contre N**, les 12 fenêtres rangées par les deux | **0,667** (28/42) | **0,641** (25/39) |
| P2, doublons (passe entière) | 1 (0,031) | 2 (0,074) |
| P2, doublons (12 fenêtres communes) | 1 (0,04) | 1 (0,042) |
| l'autre passe couverte (P1 contre l'autre) | 0,741 des fiches de b3 | 0,594 des fiches de b2 |
| jetons d'invite des décisions | 163 602, 0 % par le cache | 143 391, 0 % par le cache |
| jetons de sortie des décisions | 2 958 | 2 662 |
| invite de l'extraction (estimée) | ~411 000 jetons en 19 appels | ~411 000 jetons en 19 appels |
| temps d'une décision : médiane, p90, max | 5,7 s, 16,7 s, 81 s | 7,2 s, 34,5 s, 86 s |
| temps d'une extraction : médiane, max | 24 s, 80 s | 51 s, 101 s |

- **Le coût de l'extraction n'est pas rendu.** `usage_total` ne somme que les décisions : l'appel d'extraction ne rend
  pas son usage dans le rapport d'`archive_slice`. Son invite est estimée par la taille des fenêtres (1 645 056
  caractères, à 4 caractères par jeton), la même pour les deux passes puisque l'invite de b3 n'est plus courte que de
  ses règles. Son temps est celui de la fenêtre moins ses décisions (la recherche et les écritures comprises).
- **Aucun jeton servi par le cache**, ni en b2 ni en b3, sur les décisions.

## Ce qu'on peut en dire

1. **La coupure de l'extraction existait avant B3.** Les fenêtres en échec rendent toutes « la réponse n'est pas du JSON
   (EOF while parsing …) alors qu'un response_schema est déclaré » : la réponse est coupée à la borne
   `extract_max_tokens` (8 192), que le raisonnement de Gemini consomme aussi. b2 en a 3, b3 en a 5, et seule la
   fenêtre 18 échoue dans les deux. Sur 19 fenêtres, 3 contre 5 ne départage pas B3 : la coupure dépend du raisonnement
   tiré à chaque appel. Le lot de mémoire (le refus explicite, `extract_reasoning`) la traite ; sa mesure suit.
2. **Sur les fenêtres rangées par les deux, P1 ne départage pas b2 et b3** : 0,667 contre 0,641, soit 28 sujets de N
   retrouvés sur 42 contre 25 sur 39. La différence de P1 sur la passe entière (0,778 contre 0,704) vient d'abord des
   deux fenêtres en échec de plus de b3.
3. **Les deux passes ne rangent pas les mêmes sujets.** b2 couvre 74 % des fiches de b3, b3 en couvre 59 % de b2. À
   corpus égal, deux extractions rangent différemment, et l'écart d'une passe à l'autre est du même ordre que l'écart
   à N. Un seul jeu par tête ne suffit pas à trancher une différence de quelques sujets.
4. **La lenteur de b3 vient d'abord de Vertex.** Les décisions ne changent pas entre b2 et b3 (B3 ne touche que
   l'extraction et le découpage). Elles servent donc de témoin de la vitesse du service : en b3, leur p90 vaut 34,5 s
   contre 16,7 s en b2. L'extraction double aussi (51 s contre 24 s de médiane). On ne peut pas dire quelle part revient
   à l'invite de B3 sans rejouer les deux têtes dans la même fenêtre de temps.

## La suite

- **La mesure du raisonnement de l'extraction**, quand le lot de mémoire sera posé : b3 avec `extract_reasoning` à
  `low`, puis `minimal`, puis la borne à 16 384. Pour chaque réglage : les fenêtres coupées, P1, le coût et le temps
  par appel. Le défaut se fixera sur ce chiffre.
- **Pour départager b2 et b3**, il faudrait deux jeux de chaque tête, alternés dans la même heure.
