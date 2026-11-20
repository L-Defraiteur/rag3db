# Le raisonnement de l'extraction de l'archiviste

20 novembre 2026, de 2 h 18 à 3 h 07, à luciepc, à la demande de c7. La parité de B3 (page du 19, `05-…`) avait trouvé 3
à 5 fenêtres sur 19 dont l'extraction était coupée : le raisonnement de Gemini comptait dans la borne de sortie
(`extract_max_tokens` = 8 192), et `archive_slice` ne fixait pas le raisonnement. Le lot de mémoire
`cx-memoire-extraction-cut` (`e94b15ca22`) ajoute `extract_reasoning` (défaut `low`), un refus explicite de la réponse
coupée, et l'usage réel de l'appel d'extraction (`extraction_usage`).

## Le dispositif

- Les mêmes 19 fenêtres de 50 000 jetons du corpus `c7-99bc8451-f5` que la parité de B3, rangées par `archive_slice`
  (S 50 nu), une base neuve par réglage, Gemini 3.8 Flash sur Vertex, en série.
- Un binaire bâti depuis `e94b15ca22`, les gabarits de la même tête.
- Le juge `j_sujets.py` contre la N rejouée. L'usage de l'extraction est celui que l'appel rend, plus une estimation.
- Les outils sont à luciepc, `~/.cache/rag3db-banc-notes/compaction/` : `raisonnement.sh` et `x_slice.py` (qui prend
  maintenant `EXTRA` et `DEADLINE`, et ne coupe plus une passe sur une extraction coupée). Les sorties sont dans
  `raisonnement/`.

## Les trois réglages

| | `low`, borne 8 192 | `minimal`, borne 8 192 | `low`, borne 16 384 |
|---|---|---|---|
| fenêtres coupées | **0** sur 19 | — (refusé par Vertex) | **0** sur 19 |
| fin de l'extraction | `Eos` ×19 | — | `Eos` ×19 |
| P1 contre N | **0,648** (35/54) | — | **0,722** (39/54) |
| fiches | 49 | — | 35 |
| P2, doublons | 2 (0,041) | — | 1 (0,029) |
| invite de l'extraction (rendue) | 580 798 jetons, 0 en cache | — | 580 798 jetons, 277 319 en cache (48 %) |
| sortie de l'extraction | 20 573 jetons | — | 19 909 jetons |
| raisonnement de l'extraction | **0** jeton rendu | — | **0** jeton rendu |
| temps d'une extraction : médiane, max | 10,8 s, 30,1 s | — | 10,2 s, 28,8 s |
| décisions | 96, invite 203 757, 0 en cache | — | 95, invite 194 860, 0 en cache |
| durée de la passe | 24 min | — | 22 min |

## Ce qu'on peut en dire

1. **`low` supprime la coupure.** Aucune fenêtre coupée sur 19 dans les deux passes, contre 3 et 5 sous le raisonnement
   par défaut du modèle (parité de B3). Vertex rend 0 jeton de raisonnement pour l'extraction sous `low`, et
   l'extraction prend 10 s de médiane, contre 24 à 51 s avant.
2. **La borne à 16 384 n'apporte rien de plus** sous `low` : aucune réponse n'approche 8 192 (au plus ~1 100 jetons de
   sortie par fenêtre).
3. **`minimal` est refusé par Vertex** : « HTTP 400 : Thinking level is unsupported: THINKING_LEVEL_MINIMAL » (requête
   gardée sous `minimal-8192-…/etat/requetes-refusees/`). Le paramètre offre `minimal` sans le traduire pour Gemini 3.8
   Flash, alors que le fournisseur Anthropic le ramène à `low`. C'est un défaut du lot, signalé à mémoire.
4. **P1 ne départage pas les deux passes `low`** : 0,648 contre 0,722, pour un même raisonnement et aucune coupure. C'est
   l'écart d'une passe à l'autre de l'extraction elle-même (49 fiches contre 35). Il est du même ordre que l'écart entre
   b2 et b3. Un P1 se lit à ± 0,07 sur un seul jeu.
5. **L'invite réelle de l'extraction est de 580 798 jetons par passe**, et non ~411 000 comme je l'estimais à 4
   caractères par jeton : le corpus compte plutôt 2,8 caractères par jeton.
6. **Les 48 % de cache de la troisième passe ne disent rien du réglage.** Elle rejoue mot pour mot les invites de la
   première, une demi-heure plus tard, et le cache implicite de Vertex les sert. En service, deux fenêtres d'une même
   session ne partagent que l'instruction et le schéma, sous le seuil de 6 144 jetons.

## La suite

- **Le défaut de `minimal`** : le traduire en `low` pour Vertex, ou le retirer des choix du paramètre (mémoire).
- **Le défaut** : `low` à 8 192 tient sur ce corpus ; rien ne demande une borne plus haute.
- **Mesurer P1 à plusieurs jeux** avant de départager deux réglages de l'archiviste : un seul jeu ne suffit pas.
