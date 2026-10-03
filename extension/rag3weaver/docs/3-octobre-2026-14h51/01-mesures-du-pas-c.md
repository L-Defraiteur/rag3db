# Les mesures du pas C — la pondération par genre, et les poids de fusion

3 octobre 2026. Banc `e2e_banc_etage`, granite-278m (poids d'origine), corpus
vivant **5 278 scopes** (référence : addendum du 3 octobre au doc du
2 octobre 01h01 — tel quel 0,333, G 0,409), 43 questions. **Toutes les
passes comparées portent le même régime** : `RAG3WEAVER_EMBED_CHAR_BUDGET=4096`
et `RAG3WEAVER_GPU_DUTY=70` — vérifié sans effet sur les mesures (tel quel
0,333/0,334 et G 0,409 identiques au sans-régime), et les passes y sont même
plus courtes (193 s contre 250). Trois passes : deux identiques (variance
nulle hors M3, qui ne compte pas), une pour la courbe des valeurs douces.
Rien n'est posé dans `Scope`, rien n'est changé au gabarit : ce doc rend les
chiffres et les choix.

## 1. Le tableau

| ligne | MRR | R@1 | R@5 |
|---|---:|---:|---:|
| tel quel (vecteur seul) | 0,333 | 9 | 24 |
| G — le filtre : `scope_type` ∈ function \| method | 0,409 | 13 | 27 |
| P demande — file et namespace → 0,8 (identique à 0,6 et 0,4) | 0,355 | 10 | 24 |
| **P ombre de G** — function/method à 1, tout le reste à `default` : | | | |
| · default 0,95 | 0,399 | 13 | 24 |
| · default 0,90 | 0,403 | 13 | 26 |
| · default 0,85 | 0,407 | 13 | 27 |
| · default 0,80 | **0,413** | 13 | 27 |
| · default 0,60 / 0,40 (plateau) | 0,413 | 13 | 27 |
| H hybride — bm25 0,6 / vector 0,4 (le gabarit) | 0,264 | 7 | 16 |
| H hybride — bm25 0,3 / vector 0,7 (l'ancien moteur) | 0,332 | 9 | 21 |
| sparse | **non mesuré** — granite-278m n'a pas de sortie sparse |

## 2. La pondération par genre — lecture et recommandation

**L'ombre pondérée rejoint le filtre, puis le dépasse.** À `default` 0,8, la
pondération fait 0,413 là où le filtre G fait 0,409 : les genres dévalués
mais **gardés** récupèrent ce que le filtre perdait. R@1 13 (celui de G) est
atteint dès 0,95, la valeur la plus douce mesurée ; le MRR monte par paliers
(0,399 → 0,403 → 0,407 → 0,413) et atteint son plateau à 0,8 — en dessous,
plus rien ne bouge : les scores RRF sont serrés (1/(k+rang)), et une fois le
bloc des genres pollueurs déclassé, la multiplication, monotone, ne change
plus l'ordre.

**La forme « demande »** (file et namespace seuls) ne rend que 0,355 : les
`class`, `module` et `struct` polluent aussi — c'est l'ombre complète (tout
ce qui n'est pas function/method) qui vaut le filtre.

**Ce que le banc ne voit pas, à dire avec la valeur** : aucune de ses 43
questions n'a un fichier entier, un module ou un espace de noms pour bonne
réponse. Le banc mesure ce que la dévaluation **rend** (les fonctions
remontent), pas ce qu'elle **coûte** à « quel fichier gère X » — et c'est
précisément pour ces questions-là que Lucie a choisi un poids plutôt qu'un
filtre. À ×0,85, un fichier de rang 1 (1/61) retombe vers le rang 8-10 ;
à ×0,95, vers le rang 3-4.

**Recommandation** (le choix reste à Lucie) : déclarer dans `Scope`
`{ field: scope_type, weights: { function: 1.0, method: 1.0 }, default: 0.85 }`
— à 0,002 de G (la variance du banc), R@1 et R@5 de G atteints, et la plus
douce des valeurs qui y arrive ; 0,9 si elle préfère ménager davantage les
réponses-fichiers (R@1 13 conservé, −0,006 de MRR), 0,8 si elle veut le
plateau (0,413, au-dessus de G). La déclaration va dans la config d'entité
de `Scope` (`register_code_schema`) ; l'échelle du pas C fait le reste, et
un graphe ou un appelant peut toujours la surcharger.

## 3. Les poids de fusion — trois options, sans code

Sur les questions en langue naturelle du banc, l'hybride **perd** contre le
vecteur seul : 0,6/0,4 coûte 0,069 de MRR (0,264), 0,3/0,7 est au mieux
neutre (0,332 ≈ 0,333). Mais l'autre versant existe et il est prouvé
ailleurs : sur un identifiant exact, le plein texte doit gagner —
`e2e_code::read_and_grep_as_graph_tools` garde `merge_port_values` au rang 1
par bm25, et le 18 septembre a montré ce qui arrive avec 0,3/0,7 (la
correspondance exacte coulait sous le vecteur, hors du top 5). `Scope` est
interrogé **des deux façons** par le même agent : une déclaration par entité
ne le départage pas.

- **(a) Statu quo** — le gabarit garde 0,6/0,4. Coût : −0,069 de MRR sur
  les phrases (mesuré). Bénéfice : les identifiants protégés (prouvé).
  Correctif disponible sans code : une entité à requêtes naturelles déclare
  0,3/0,7 chez elle, l'appelant surcharge quand il sait. Ne règle pas
  `Scope`.
- **(b) 0,3/0,7 par défaut** — les phrases remontent à ≈ vecteur seul
  (mesuré). Ce qu'`e2e_code` perd : le rang 1 des correspondances exactes
  (démontré le 18 septembre) — il faudrait que `Scope` déclare 0,6/0,4 chez
  lui, ce qui inverse (a) sans régler `Scope` non plus : ses phrases y
  reperdent.
- **(c) Des poids selon la forme de la requête, déclarés dans le graphe** —
  ce que font les moteurs établis. Un nœud générique en amont de la fusion
  (`QueryShapeNode` : il lit la requête du payload et pose `options.fusion`
  à l'étage appelant), câblé et **paramétré dans le `.mmd`** — deux jeux de
  poids déclarés par le graphe, une heuristique de forme nommée et testable
  (un jeton unique, sans espace, ou snake/camelCase → identifiant ; des
  mots multiples → phrase), aucune règle en dur dans le moteur. Coût : un
  nœud (~80 lignes), l'heuristique à nommer, et les cas mixtes
  (« la fonction merge_port_values ») à trancher — l'identifiant détecté
  dans la phrase devrait l'emporter. Preuves : le banc pour le versant
  phrases, les cas d'identifiants d'`e2e_code` pour l'autre, une poignée de
  cas mixtes à écrire.
- **(c-light), le premier pas gratuit** : le paramètre `rerank` de l'outil
  des agents demande déjà à l'appelant de dire si sa requête est « une
  vraie question en langue naturelle » (sa fiche le dit mot pour mot, et ne
  le conseille que là). `rerank > 0` est donc une déclaration de forme qui
  existe déjà : le graphe peut peser vecteur-lourd quand `rerank` est
  demandé — zéro heuristique, et le cross-encoder rejuge le pool de toute
  façon. Limite : une question posée sans `rerank` garde les poids
  identifiants ; (c-light) se mesure au banc en une passe (les questions du
  banc avec `rerank`) avant de décider (c) complet.

Ma pente : (a) aujourd'hui — rien ne bouge sans mesure du versant
identifiants —, et (c-light) comme prototype mesurable de (c). Le choix est
à Lucie.


## 4. Ajout du 3 octobre au soir — le réglage global, cherché par la mesure

Lucie a écarté la détection de forme (« pas de détection auto ; un réglage
global qui favorise légèrement identifiants ») — et relevé que la coupe
« identifiant vs sémantique » était fausse : les mots entre les séparateurs
ont un sens, le vecteur les sert aussi. La mesure lui donne raison.

Six couples, deux versants, **pondération par genre de `Scope` comprise**
(déclarée entre-temps — ces chiffres ne se comparent donc pas aux 0,264 /
0,332 du §1, mesurés sans elle) ; la ligne I : dix identifiants exacts du
corpus (`merge_port_values` — le cas du 27 août — en tête), requête = le
nom, bonne réponse = son scope.

| bm25/vector | H phrases (MRR·R@1·R@5 /43) | I identifiants (MRR·R@1·R@5 /10) |
|---|---|---|
| 0,6/0,4 (gabarit) | 0,322 · 11 · 16 | 0,883 · 8 · 10 |
| 0,55/0,45 | 0,327 · 11 · 16 | 0,883 · 8 · 10 |
| 0,5/0,5 | 0,371 · 12 · 20 | 0,883 · 8 · 10 |
| **0,45/0,55** | **0,385 · 13 · 23** | **0,883 · 8 · 10** |
| 0,4/0,6 | 0,384 · 13 · 23 | 0,883 · 8 · 10 |
| 0,3/0,7 (ancien moteur) | 0,389 · 13 · 23 | 0,900 · 8 · 10 |
| témoin : vecteur seul | 0,406 · 13 · 27 (tel quel) | 0,775 · 6 · 10 |

**La lecture.** Le versant identifiants est **plat** : l'identifiant exact
est rang 1 du plein texte quel que soit son poids, et le vecteur granite le
classe bien aussi (témoin 10/10 en R@5) — la fusion RRF le garde donc en
tête à tous les couples. La crainte du 18 septembre (« 0,3/0,7 perd la
correspondance exacte ») ne se matérialise pas sur ce corpus : elle venait
d'un petit corpus et du bug de la fusion aplatie. Le versant phrases, lui,
veut le moins de bm25 possible : le plateau (0,385, R@1 13, R@5 23)
commence à **0,45/0,55** et ne gagne plus que 0,004 jusqu'à 0,3/0,7 — et
même à 0,3/0,7, l'hybride reste sous le vecteur seul (0,389 contre 0,406) :
sur des questions en langue naturelle, le bm25 coûte toujours un peu.

**Recommandation** (le choix est à Lucie, le gabarit n'a pas bougé) :
**0,45/0,55** — le premier point du plateau des phrases, donc le plus de
plein texte qu'on puisse garder sans les payer : la marge de sécurité
maximale pour les identifiants que ce banc ne couvre pas (10 noms, un seul
corpus). « Favorise légèrement les identifiants » au sens utile : non pas
plus de poids (la mesure dit qu'ils n'en ont pas besoin), mais plus de
marge. Si Lucie choisit un couple, la validation devra rejouer `e2e_code`
avec le gabarit modifié — son canari du 27 août vit sur un petit corpus où
le vecteur noyait, le monde où la crainte était vraie.
