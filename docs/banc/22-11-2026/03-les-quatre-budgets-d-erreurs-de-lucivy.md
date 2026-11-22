# Les quatre budgets d'erreurs de lucivy

22 novembre 2026, midi, suite de [02-le-chemin-des-mots-en-contains-split.md](02-le-chemin-des-mots-en-contains-split.md).
Lucivy propose quatre variantes du budget d'erreurs de `contains_split` (branche `fuzzy-error-budget`, tête `71b72e6`,
rien de publié), choisies par une clé `budget` dans la requête :

| variante | ce qu'elle fait |
|---|---|
| `flat` | la 4.4.2 de la page 02 |
| `idf` | un mot absent coûte selon sa rareté |
| `window` | le budget compté dans la meilleure fenêtre |
| `soft` | des paliers de 3 points de BM25 au lieu de 1000 |

Sans la clé, `contains_split` reste le booléen de 4.4.1.

## Le banc

Un seul bâti de la branche de mesure de recherche, `bm25-budget-measure` (tête `ef28203b37`, patch des cinq crates vers
lucivy `71b72e6`). `build_bm25_query` y lit deux variables de mesure :
- `RAG3WEAVER_BM25_BUDGET` (`flat`, `idf`, `window`, `soft`) donne le budget au **chemin des mots** ;
- `RAG3WEAVER_BM25_BUDGET_WHOLE=1` bascule **aussi la passe de la requête entière** en `contains_split`.

Six hôtes du même binaire tournent sur la même base, reprise (voir plus bas) :
- sans budget ;
- les quatre variantes sur le chemin des mots seul ;
- `flat` avec la passe entière basculée (`flat-whole`).

Les jeux sont ceux des pages 01 et 02, plus le signal BM25 seul.

**Une première passe confondait deux effets** : la passe entière y basculait avec le budget. Les quatre variantes y
perdaient alors les mêmes cinq questions françaises sur neuf. Recherche a séparé l'interrupteur, et c'est cette seconde
passe qui est rapportée ici.

## Les résultats

Comparés à « sans budget » :

| | or premier (questions FR) | rappel@10 (questions FR) | monte / recule | premiers changés G / jeu commun | top 10 commun | pages Markdown en tête du BM25 seul (sur 17) | temps médian |
|---|---|---|---|---|---|---|---|
| sans budget | 1/9 | 5/9 | — | — | — | 1 | 1 739 ms |
| **flat** | **1/9** | **5/9** | **3 / 2** | 1/10, 2/10 | 98 % | 6 | 1 665 ms |
| idf | 0/9 | 5/9 | 2 / 2 | 1/10, 2/10 | 98 % | 6 | 1 686 ms |
| window | 0/9 | 5/9 | 2 / 2 | 1/10, 2/10 | 98 % | 4 | 1 649 ms |
| soft | 0/9 | 5/9 | 2 / 2 | 1/10, 3/10 | 98 % | 3 | 1 652 ms |
| **flat-whole** | 0/9 | **0/9** | 0 / 5 | 4/10, 4/10 | 51 à 56 % | 6 | 1 676 ms |

**Les noms exacts ne bougent dans aucun cas** : or premier 13/20, rappel@10 18/20.

Le détail des questions françaises, avec le budget sur le chemin des mots seul :

| question | sans budget | `flat` | `idf`, `window`, `soft` |
|---|---|---|---|
| « construire la requête lucivy du BM25… » | 4 | 3 | 3 |
| « relire … cross-encoder » | 3 | 2 | 2 |
| « le nœud de recherche plein texte BM25 » | 2 | **1** | 2 |
| « reconstruire l'index vectoriel » | 6 | 7 | 7 |
| « la configuration du bac à sable… » | 1 | 2 | 2 |

**L'avertissement de lucivy** (« fuzzy `contains` in one boolean ») ne tombe à 0/17 que sur `flat-whole` : c'est la
passe entière qui le portait.

## Ce que j'en conclus

1. **La passe de la requête entière ne doit pas passer à `contains_split`.** Elle seule cause l'effondrement : la phrase
   exacte n'y passe plus devant, et le rappel@10 des questions françaises tombe de 5/9 à 0/9.
2. **Le budget sur le seul chemin des mots est neutre à légèrement positif dans la recherche complète** : rappel@10
   gardé, le top 10 identique à 98 %, et un temps de recherche même un peu meilleur, de 3 à 5 %.
3. **`flat` est la meilleure des quatre ici** : la seule qui garde l'or premier (1/9), avec trois questions qui montent
   pour deux qui reculent.
4. **Le biais de longueur reste visible au BM25 seul.** `soft` (3 pages Markdown en tête sur 17) et `window` (4) le
   réduisent mieux que `flat` et `idf` (6). La fusion le rattrape dans la recherche complète.
5. **Les écarts entre variantes portent sur un ou deux cas sur neuf.** C'est trop peu pour départager `flat` de `soft`
   sur la qualité. Un jeu de questions plus large, de trente à cinquante avec leur or, le dirait.
6. **Pour 4.4.2** : rien ne recule tant que la passe entière n'est pas basculée. La publication reste au mot de Lucie,
   et le choix de la variante par défaut, à lucivy et à elle.

## La base reprise, et un signal mal attribué

G a effacé par erreur, à 11 h 14, la base source (`mesures/code-principal-migree-repassee-20261120`) pendant le ménage
du disque. Mon hôte « sans budget » tournait sur une copie reflink faite à 11 h 13. J'ai arrêté la passe, puis repris
cette copie fermée comme source, `banc-parite/source-code-principal-20261120`. Les six hôtes lisent tous la même.

Un SIGTERM vu au journal de cet hôte à 11:14:51 m'a d'abord fait soupçonner un signal d'un tiers, et G et c7 l'ont
cherché. **C'était mon propre `kill -TERM`**, à l'arrêt de la passe, juste avant sa relance de 11:15:10.

## La procédure : un job long à luciepc

Un job long lancé par `ssh … setsid nohup …` vit dans le scope de la session ssh (`session-cNNN.scope`), même si cette
session est fermée. **Je lance désormais par `systemd-run --user --scope`**, qui lui donne son unité à lui :

```
ssh luciepc 'cd ~/.cache/rag3weaver-build && setsid nohup systemd-run --user --scope --quiet \
  --unit=banc-<nom>-$(date +%H%M) -- ./poste cpu -- timeout <durée> ./<script> > <journal> 2>&1 < /dev/null & disown'
```

- L'état se lit par `systemctl --user is-active banc-<nom>-<hhmm>.scope`.
- **Jamais par `pgrep -f`** avec le nom du script dans la commande ssh : il se trouve lui-même.

Les sorties sont à luciepc, sous `banc-parite/sorties/lucivy-budget-*` ; le banc est `lucivy-budget.sh`.
