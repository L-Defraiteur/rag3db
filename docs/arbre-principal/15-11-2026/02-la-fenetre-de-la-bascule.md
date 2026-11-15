# La fenêtre de la bascule des bases de Lucie

15 novembre 2026, arbre principal. Ce que la bascule fait sur chacune des trois bases, dans quel ordre, et
combien de temps chaque étape demande ; puis le retour arrière, et la question posée à Lucie.

## Les trois bases

| base | poste | fichier de base | `.fts` |
|---|---|---|---|
| `code-rag3db-depot` | portable | 3,55 Go | 1,4 Gio |
| `code-lourd` | luciepc | 1,37 Go | 730 Mio |
| `code-principal` | luciepc | 1,13 Go | 620 Mio |

Tailles relevées le 15 novembre à 14 h 50.

## L'ordre, base par base

1. **Annonce** aux sessions qui lisent la base : le backend s'arrête pendant la fenêtre.
2. **`maintain` de lucivy** par l'hôte (4.3.8, hôte rebâti sur e95a1b8c6 ou plus récent) :
   `fts-live-ids`, la lecture du graphe, puis `maintain` avec la purge des orphelins (recette de hotfixs)
   et `compact: true`. Il rend de la place avant la copie (4,93 → 1,35 Go mesurés par hotfixs sur
   `code-rag3db-depot`). Il tient le backend en écriture environ 20 s par grande table.
3. **L'instantané** : base, wal, extensions et `.fts` ensemble, avec le manifeste de copie. C'est le
   retour arrière.
4. **Le contrôle des deux sens** des tables de relations (check_rel_directions de G). Une base qui y rougit
   attend la réparation de hotfixs.
5. **1b**, la migration des versions (`rag3weaver-backend migration versions --base …`), par lots de 2 000.
6. **La montée aux générations** (`rag3weaver-backend migration generations --base …`). Son bilan dit :
   versions → generations, `backend:0` publiée, rang lu 1.
7. **La réouverture par l'hôte**, puis **le repassage de l'écrivain**, base ouverte : l'outil `index` avec
   `confirm=true`, attendu jusqu'à la dernière ligne de son journal (« indexation terminée »).
8. **La vérification** : les comptes visibles par table, avec CONSUMES = CONSUMED_BY, et le banc de recherche
   d'avant retrouvé dans celui d'après.

## Les durées, mesurées par la répétition du 15 novembre

La répétition s'est jouée à luciepc, sur des copies des instantanés de la nuit (`avant-fenetre-code-lourd-20261115-0027`,
`avant-fenetre-code-principal-20261115-0051`), les deux bases en parallèle, sous `poste lourd`. Elle a pris le binaire
4b28027c8 : l'écrivain d'ED c5ec47cc4, plus `migration generations --base`, sur la lib 373b5477b. Le script est
`~/.cache/rag3weaver-build/bascule/repetition-bascule.sh` à luciepc, avec un journal par base à côté.

| pas | code-lourd | code-principal |
|---|---|---|
| comptes et banc de recherche avant | 6 s | 4 s |
| réparation des deux index (copie seulement : l'instantané est d'avant celle de hotfixs) | 1 719 s | 1 402 s |
| orphelins du plein texte : relevé, purge, compact | 9 s (103 Scope) | 9 s (102 Scope) |
| 1b | **683 s** (82 855 occurrences, 76 920 versions) | **623 s** (65 914 occurrences, 61 224 versions) |
| montée aux générations | 2 s | 1 s |
| **repassage de l'écrivain**, avec les vecteurs | **3 661 s** (7 278 fichiers, 4 paquets) | **2 956 s** (6 314 fichiers) |
| comptes et banc après | 18 s, VERT | 17 s, VERT |

- **La part fermée de la vraie fenêtre** est l'instantané, la purge (~10 s), 1b (10 à 12 min) et la montée
  (2 s) : environ **15 min par base**, les trois bases en parallèle. Les bases vivantes ont eu leur réparation :
  un contrôle des deux sens (check_rel_directions) la garde avant 1b, et une base qui y rougit attend.
- **Le repassage se fait base ouverte** (ED) : 50 à 60 min par base, vecteurs compris, à 0,3 s par fichier
  ou plus sous charge. Les lectures voient le rang en attente, et seules les lignes File d'un paquet manquent
  un instant. L'outil `index` y demande `confirm=true`, parce qu'il estime plus de 5 min de vecteurs.
- **Les comptes bruts doublent** pour File et ses arêtes : les lignes retirées restent. On juge sur les lignes
  visibles (`_jusqu_a` nul). Après le repassage, File visibles 7 275 et Scope visibles 82 768 à code-lourd,
  File 6 313 et Scope 65 814 à code-principal, comme le dit le journal de l'index.
- **Le dépôt au portable** : 1b a pris 861 s le 14 novembre et le repassage ~45 min (banc). Il n'a pas été
  répété.

## Ce que la répétition a trouvé, et la décision de c7

- **Un défaut de graphe de l'écrivain** : après le repassage, des CONSUMED_BY visibles n'ont plus leur
  CONSUMES, **2 241 à code-lourd et 1 947 à code-principal**. Il n'y en avait aucun avant, et aucun CONSUMES
  n'a perdu son miroir. Le consommateur est chaque fois réécrit, et sa résolution neuve ne consomme plus la
  cible. Mais la boucle générique de recopie des arêtes entrantes (`recopier_les_aretes_entrantes`) avait
  recopié le CONSUMED_BY sur lui. Effet : `impact` et les appelants sont faux sur ~2 % de ces arêtes.
- **ED l'a corrigé** en 5ed5d1fc9, avec un témoin 18 vu rouge d'abord : la boucle saute CONSUMED_BY.
- **La décision de c7, à 20 h 15 : la vraie bascule attend.** Pas ce soir avec un défaut connu du graphe,
  parce qu'un correctif après coup coûterait un second repassage par base, et parce que l'écrivain n'est pas
  encore sur master. L'ordre est donc :
  1. le correctif d'ED, jugé par le banc (CONSUMES = CONSUMED_BY au juge) ;
  2. l'atterrissage de l'écrivain, avec `migration generations --base` en pile ;
  3. une pose qui porte les deux ;
  4. une dernière répétition sur copie ;
  5. la bascule.

## La politique (Lucie dit oui, c7 la fixe, 15 novembre)

Lucie : « mes bases sont là pour ça » ; elle laisse la politique à c7.
- **Quand** : ce soir, dès que la file du jour est vidée et que l'écrivain d'ED a atterri (bff6c7409,
  c'est lui que le repassage demande). Le dépôt au portable et les deux bases de luciepc **en
  parallèle**, chacune par son script, sous `poste lourd timeout`, après le `maintain` de lucivy. Les
  hôtes des deux postes sont rebâtis au-delà de e95a1b8c6.
- **La répétition sur copie d'abord**, aujourd'hui, pour les vrais chiffres de luciepc. Si elle casse,
  la bascule attend le correctif, pas Lucie.
- **Chaque base a son journal de bascule** : les heures, les comptes avant et après par table, le relevé
  à blanc de lucivy avant et après. Son état d'index dit « basculée le … ». Chaque fermeture est
  annoncée aux sessions avant.
- **Le repassage** est la première synchronisation d'un hôte qui porte l'écrivain, sur une base passée
  aux générations (`migrer_vers_les_generations`, après 1b). Le journal de l'index dit « première
  synchronisation par l'écrivain : N fichiers à repasser ». Compter environ 0,35 s par fichier (ED,
  d'après banc), hors vecteurs, qui ne se recalculent pas.
- **Les orphelins de lucivy se relèvent au moment de la bascule** : les listes de hotfixs de la nuit ne
  servent plus. La recette est celle de hotfixs (`docs/coeur-cpp-hotfixs-tickets/13-11-2026/outils/`,
  `fenetre-purge.sh compter`) : les vivants du `.fts`, moins les offsets de la table du graphe, sur
  Scope et File. Base ouverte, l'ordre fait la sûreté : le jeton d'abord, le graphe ensuite, puis
  `maintain` avec `max_purge_ratio` à 0,01.

## Le retour arrière

- **Pendant la fenêtre**, si une étape échoue ou si la vérification diffère : on remet l'instantané de
  l'étape 3 (quelques minutes, le temps de la copie) et on rouvre.
- **Après** : l'instantané et l'ancien index sont gardés **72 heures**, puis retirés base par base au mot
  de c7, sans redemander à Lucie. Une base que Lucie a fait rouvrir sans plainte en 72 h est acquise. Le
  retrait rend environ 5 Go par base.

## Ce qui attend encore

- le correctif du miroir d'ED (5ed5d1fc9), vert au banc ;
- l'écrivain et `migration generations --base` sur master, en pile, puis une pose qui les porte ;
- la dernière répétition sur copie, puis la bascule ;
- le ticket « une recherche à versions prend des Gio » fermé.
