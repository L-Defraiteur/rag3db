# Le signal creux sur du code : ce qui est publié

**4 octobre 2026, session « optimiseur ».** Lucie : « c'est à voir si on met
le signal sparse ou pas ; faut faire des expérimentations — à quel moment le
sparse devient utile sur un projet de code ; étudier sur internet si des gens
ont fait des benchmarks ». Ce document est une lecture, pas une mesure. Rien
n'a été lancé ni envoyé.

Notre fait de départ (session recherche, `91c44a96c`) : sur un dépôt de 7 600
morceaux, ajouter le creux de bge-m3 à granite-278m et au plein texte fait
passer le banc de 0,340 à 0,369 sur les phrases et de 0,850 à 0,900 sur les
identifiants, au prix d'un second embarquement de tous les morceaux.

**Ce que la lecture donne : personne n'a publié la mesure qui nous
intéresse.** Aucun banc ne compare, sur du code, plein texte + dense à plein
texte + dense + creux appris. Les sources voisines disent trois choses : le
creux de bge-m3 est d'un genre (un poids par jeton présent, sans expansion)
qui, sur du code, ne fait pas mieux que BM25 quand il est seul ; un troisième
signal aide le plus souvent mais pas toujours sur du texte ; et sur du code,
l'hybride à deux signaux ne bat pas toujours le meilleur des deux.

## Comment lire ce document

- **Relu** : le chiffre ou la phrase a été relu dans le texte brut de la
  page arXiv (sans outil qui résume).
- **Rapporté** : lu à travers un outil qui résume la page ; à relire avant de
  s'en servir.
- **Banc** : prépublication ou article avec protocole et tableaux.
  **Billet** : texte d'un éditeur d'outil ou de base de données, qui a un
  intérêt à défendre son choix.

Les quatre identifiants arXiv donnés comme pistes existent et portent les
titres annoncés.

## 1. Que gagne un creux appris par-dessus plein texte + dense ?

### Sur du code : aucune mesure trouvée

Aucun des bancs de code lus n'évalue un creux appris en fusion avec les deux
autres signaux. Ce qui s'en approche :

| source | nature | ce qui est mesuré | ce qu'elle dit | ce qu'elle ne mesure pas |
|---|---|---|---|---|
| **SPLADE-Code**, arXiv 2603.22008 (NAVER LABS Europe, les auteurs de SPLADE) | banc | nDCG@10 sur CoIR, MTEB-Code, CPRet, CodeRAG-Bench ; dense et creux entraînés avec la même chaîne | **relu** : BM25 45,8 / 47,9 / 57,7 / 18,6 ; un creux **sans expansion** (« SPLADE-lex.-8B ») 46,2 / 45,8 / 56,7 / 29,4 ; le creux avec expansion 72,6 à 76,7 sur CoIR, au niveau du dense de même taille (71,4 à 76,0) | aucune fusion ; aucun creux entraîné sur du texte ; rien par genre de requête |
| **« Not All RAGs Are Created Equal »**, arXiv 2605.14503 | banc | la **qualité du code généré** après recherche, pas la recherche ; corpus de 232 633 documents, Python | **relu** : BM25 38,00, **granite-278m** 35,63, hybride BM25 + meilleur dense 33,54 (APPS) ; « Hybrid Retrieval is a safe but conservative strategy that […] did not yield performance gains beyond the single best retriever » | pas de creux ; pas de recherche dans un dépôt |
| **Scientific Code Search at Scale**, arXiv 2607.05443 | banc | recherche de dépôts (5 264) et d'extraits (117 950) | rapporté : l'hybride BM25 + dense (nDCG@10 0,532) fait **moins bien** que le dense seul (0,572) | pas de creux |

La phrase de SPLADE-Code qui nous concerne, relue : « purely lexical
approaches such as SPLADE-lexical perform poorly. These models lack the
semantic expansion required […]. Similarly, BM25 struggles in this setting
for the same reason. » Et : « high performance critically relies on expansion
tokens (i.e., tokens not present in the input) ».

### Sur du texte : une mesure à trois voies, avec notre creux

**« Balancing the Blend »**, arXiv 2508.01405 (banc ; le code publié est celui
de la base de données Infinity, donc un éditeur) : plein texte BM25, creux de
**bge-m3**, dense, fusionnés par RRF, sur 11 jeux de texte. Tableau 2,
**relu** : en passant de plein texte + dense aux trois voies, le nDCG@10
monte sur 9 jeux sur 11 (de +0,3 à +5 points : par exemple 0,668 → 0,692,
0,604 → 0,654, 0,832 → 0,865) et baisse sur 2 (0,347 → 0,345, 0,748 →
0,739). Les auteurs : un hybride est « constrained by its least effective
component, sometimes underperforming its best constituent path ».

**BGE-M3**, arXiv 2402.03216 (banc, par les auteurs du modèle) : dense seul
69,2, dense + creux 70,4 sur MIRACL (**relu**) ; sur des documents longs
(MLDR), l'écart rapporté est d'une douzaine de points. Aucun code, et pas de
BM25 dans la fusion.

Notre gain (+3 points sur les phrases, +5 sur les identifiants) est donc du
même ordre que ce que le troisième signal apporte sur du texte.

## 2. Cela dépend-il de la taille, du langage, du genre de requête ?

Presque rien n'est mesuré pour un creux. Ce qui existe porte sur BM25 contre
dense.

- **CORE-Bench**, arXiv 2606.11864 (banc ; 632 dépôts au niveau 2). Ses
  annexes donnent BM25 comme référence creuse (**relu** ; la recherche
  documentaire déléguée avait conclu à tort qu'il n'y en avait pas). Niveau 2
  (retrouver dans un dépôt ce qu'un ticket demande de modifier), nDCG@10 :
  BM25 13,0 ; CodeRankEmbed, un dense de code de petite taille, 12,1 ; le
  meilleur dense non affiné (Qwen3-8B) 20,3.
  - **Par langage** : BM25 passe devant ce dense de 8 milliards en Go (16,0
    contre 14,6) et en PHP ; il est loin derrière en TypeScript (7,4 contre
    22,6) et en Rust (11,5 contre 22,1).
  - **Par intention du ticket** (bug, fonctionnalité, refonte, question,
    documentation) : BM25 est derrière partout, presque à égalité sur les
    questions (16,2 contre 17,8), très loin sur la documentation (7,0 contre
    21,2). Ce ne sont pas nos genres (phrase, identifiant, trace).
  - **Par difficulté du dépôt** : un tableau existe (dépôts à peu ou
    beaucoup de fichiers) ; BM25 y reste sous le grand dense dans toutes les
    cases lues.
  - La phrase qu'on lui prête, « BM25 […] and SPLADE-style sparse expansion
    remain useful for exact identifiers, APIs, file names, stack traces, and
    configuration keys », est dans son état de l'art, pas dans ses résultats.
- **CodeRAG-Bench**, arXiv 2406.14497 (banc, rapporté) : dans un dépôt
  (RepoEval), BM25 fait 93,2 et dépasse les denses généralistes ; vers de la
  documentation de bibliothèques, il s'effondre (5 à 7).
- **CoIR**, arXiv 2407.02883 (banc, **relu**) : BM25 29,79 de moyenne,
  bge-m3 en dense 39,31 ; BM25 passe devant bge-m3 sur trois jeux sur dix.
- **Cursor** (billet) : sa recherche sémantique ajoutée à grep améliore la
  rétention du code de 0,3 %, et de 2,6 % sur les dépôts de 1 000 fichiers ou
  plus. C'est la seule donnée de taille trouvée, et elle porte sur le dense.

**Aucune source ne sépare requêtes en phrases, identifiants et traces
d'erreur.**

## 3. Le creux de bge-m3 sur des identifiants

**Comment il est fait** (papier BGE-M3, **relu**) : « the term weight is
computed as Relu(W_lex^T H_q[i]) », avec W_lex « mapping the hidden state to
a float number » ; « If a term appears multiple times […] we only retain its
max weight ». Un poids par jeton présent dans le texte, donc. Il n'y a pas
d'expansion : un terme absent de l'entrée ne reçoit jamais de poids. C'est ce
qui le distingue de SPLADE, qui projette chaque position sur tout le
vocabulaire et active des termes que le texte ne contient pas.

**Ce que cela implique** : c'est la famille que SPLADE-Code appelle
« lexical (no expansion) », et qu'il mesure au niveau de BM25 sur du code.
Mais son modèle sans expansion était entraîné sur du code ; le creux de
bge-m3, entraîné sur du texte, n'a été mesuré sur du code par personne.

**La découpe en sous-mots**, vue sur notre copie du découpeur (vocabulaire de
250 002 pièces) :

| identifiant | pièces |
|---|---|
| `LocalVariablePool` | `▁Local` `Var` `i` `able` `Po` `ol` |
| `float_from_data` | `▁flo` `at` `_` `from` `_` `data` |
| `QUERY_VECTOR_INDEX` | `▁QUE` `RY` `_` `VE` `C` `TOR` `_` `IND` `EX` |
| `getOrCreateUser` | `▁get` `Or` `Cre` `ate` `U` `ser` |
| `HNSW` | `▁` `HN` `SW` |
| `std::collections::HashMap` | `▁st` `d` `:` `:` `collect` `ions` `:` `:` `Ha` `sh` `Map` |

Un identifiant devient six à onze pièces dont plusieurs ne veulent rien dire
(`i`, `at`, `_`, `ol`). Le creux rapproche deux textes par leurs pièces
communes : il peut donc relier `float_from_data` à une requête qui dit « from
data », ce qu'un plein texte qui garde l'identifiant entier ne fait pas. Une
lecture possible de notre gain sur les identifiants, **non vérifiée** : le
creux fait surtout office de plein texte avec une autre découpe.

**Un indice dans ce sens** : arXiv 2605.18561 (banc, rapporté) mesure qu'un
découpage de BM25 qui connaît les identifiants (l'identifiant entier et ses
parties) change fortement son score sur CodeSearchNet Go, au point d'effacer
le gain d'autres réglages.

**Des creux entraînés pour le code** : oui, un. SPLADE-Code (modèles
`naver/splade-code-*`, de 0,6 à 8 milliards de paramètres, bâtis sur des
LLM). Licence rapportée comme non commerciale (CC-BY-NC-SA), non relue. Aucun
creux bâti sur CodeBERT ou UniXcoder n'a été trouvé.

L'affirmation « SPLADE faiblit sur les jetons hors vocabulaire, donc sur les
identifiants » ne vient que d'un billet ; aucune mesure ne l'appuie.

## 4. Ce que font les outils d'agents de code

Tous des billets ou des propos d'éditeurs ; **aucun ne dit utiliser un creux
appris**, et presque aucun ne publie de mesure. Rapporté, non relu mot à mot.

| outil | choix déclaré | mesure publiée |
|---|---|---|
| Claude Code | recherche par grep et glob, sans index | aucune (« mostly vibes », dit son auteur) |
| Cline | pas d'index : lecture de fichiers, arbre syntaxique, ripgrep | aucune |
| Cursor | index d'embarquements denses, en plus de grep | banc interne : +12,5 % de réponses justes ; rétention +0,3 %, +2,6 % sur les gros dépôts |
| GitHub Copilot | embarquement dense propre au code | banc interne : 0,362 → 0,498 |
| Sourcegraph Cody | embarquements abandonnés pour une recherche à base de BM25 adapté | aucune ; raisons d'exploitation (code envoyé à un tiers, passage à l'échelle) |
| Aider | carte du dépôt par tree-sitter et classement sur le graphe | aucune |
| Windsurf / Codeium | embarquements jugés plafonnés ; plusieurs appels de modèle en parallèle | métrique décrite (rappel à 50 sur les fichiers d'une fusion), pas de chiffre |
| Augment | index avec ses propres modèles | aucune |
| Greptile | décrire le code en langue naturelle avant de l'embarquer | un exemple |
| SWE-bench (banc, pas un outil) | BM25 comme recherche de référence, les denses jugés mal adaptés aux très longs textes | rappel des fichiers modifiés : 29,6 % à 51,1 % selon le budget |

Trois familles : sans index, index dense, lexical ou graphe. Les sources sont
dans la section finale.

## 5. Ce que cela suggère comme expériences chez nous

Pour la session recherche. Dans l'ordre où elles répondent à la question de
Lucie.

1. **Le creux est-il un second plein texte ?** Jouer chaque voie seule,
   chaque paire, et le trio, avec la même fusion (le protocole de
   2508.01405). Si dense + creux fait à peu près dense + plein texte, les deux
   signaux lexicaux sont redondants.
2. **Un plein texte qui connaît les identifiants.** Découper `camelCase` et
   `snake_case` en gardant aussi l'identifiant entier, sans creux, et voir ce
   qu'il reste du gain de 0,850 à 0,900. C'est l'alternative qui ne coûte
   aucun embarquement.
3. **Par genre de requête.** Séparer le banc en phrases (français et
   anglais), identifiants exacts, identifiants partiels ou mal orthographiés,
   traces d'erreur. Personne ne l'a publié ; c'est là que se lit « à quel
   moment ».
4. **Par taille.** Le même banc sur des dépôts de tailles différentes, ou sur
   des sous-ensembles du même dépôt (1 000, 7 600, 50 000 morceaux).
5. **Le maillon faible.** Vérifier, par genre de requête, que le trio ne fait
   jamais moins bien que la meilleure paire, et faire varier le poids du
   creux dans la fusion.
6. **Identifiants brouillés.** Rejouer le banc avec des identifiants
   renommés (la variante de 2605.14503) : ce qui reste du gain ne vient pas
   de la correspondance lexicale.
7. **Le coût en face.** Le temps d'indexation ajouté par le second
   embarquement, et si le creux peut se limiter à certains genres de
   morceaux.
8. **L'effet sur l'agent**, pas seulement sur le classement : une recherche
   mieux notée change-t-elle ce que l'agent réussit ?

## Ce qui n'a pas été trouvé

- Une mesure à trois signaux (plein texte, dense, creux appris) sur du code.
- Le creux de bge-m3, ou SPLADE entraîné sur du texte, évalué sur du code.
- L'effet de la taille du dépôt ou du langage sur l'utilité d'un creux.
- Une ventilation par genre de requête (phrase, identifiant, trace).
- Le coût d'indexation d'un second embarquement rapporté à son gain.
- L'effet d'un signal creux sur la réussite d'un agent.
- Non lus : RepoBench ; la documentation de Continue ; Sourcegraph Amp ; Qodo.
- À relire avant de citer : les chiffres marqués « rapporté », et le tableau
  des outils.

## Sources

Bancs : CoIR, arxiv.org/abs/2407.02883 · CORE-Bench, arxiv.org/abs/2606.11864 ·
Not All RAGs Are Created Equal, arxiv.org/abs/2605.14503 · Scientific Code
Search at Scale, arxiv.org/abs/2607.05443 · SPLADE-Code,
arxiv.org/abs/2603.22008 · BGE-M3, arxiv.org/abs/2402.03216 · Balancing the
Blend, arxiv.org/abs/2508.01405 · CodeRAG-Bench, arxiv.org/abs/2406.14497 ·
SWE-bench, arxiv.org/abs/2310.06770 · découpe de BM25 sur du code,
arxiv.org/abs/2605.18561.

Billets : qdrant.tech/articles/minicoil (un creux hors domaine ferait pire
que BM25 : affirmé, sans tableau, et sans code) · cursor.com/blog/semsearch ·
github.blog/news-insights/product-news/copilot-new-embedding-model-vs-code ·
sourcegraph.com/blog/how-cody-understands-your-codebase ·
cline.bot/blog/why-cline-doesnt-index-your-codebase-and-why-thats-a-good-thing ·
aider.chat/docs/repomap.html · latent.space/p/claude-code ·
greptile.com/blog/semantic-codebase-search ·
augmentcode.com/blog/a-real-time-index-for-your-codebase-secure-personal-scalable.
