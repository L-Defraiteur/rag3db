# L'intra-scope : conception

Session codeparsers, 19 novembre 2026, à la demande de c7. Page de conception seule, pour relecture par Lucie : aucun
code n'est écrit. Les chiffres viennent d'une sonde jetable jouée à luciepc en release sur la tête codeparsers
b67ad75 (la pile des mots vides et des locales), sur l'arbre figé `rag3db-garde-mesure`.

## La vision, dans les mots de Lucie

Nuit du 18 au 19 novembre : « que codeparsers aille beaucoup plus loin avec l'intra-scope : les symboles à
l'intérieur, les expressions » ; « que les trous de couverture soient comblés par LLM ». Et la suite : « il y a un
état stable à réécrire : tu changes toutes les occurrences de ce symbole pour être une expression à la place, hop tu
appliques, hop ton expression est partout ; comme dans un éditeur quand on sélectionne plusieurs occurrences avec
Ctrl-D puis édition multiple. »

Trois choses, donc :
1. **voir l'intérieur d'un scope** (symboles locaux, occurrences, expressions, types) ;
2. **combler par un modèle** ce que les règles ne comprennent pas, sous une provenance à part ;
3. **réécrire par le graphe** : une édition déclarée, appliquée à chaque occurrence, vérifiée et annulable d'un bloc.

## Ce qui existe aujourd'hui

### Ce que codeparsers calcule à l'intérieur d'un scope

Un scope (`ScopeInfo`, `types.rs`) porte ses **paramètres** (nom, type, ligne, colonne), son **type de retour**, et ses
**références** (`identifier_references`), une par occurrence. Chaque référence porte son identifiant, sa ligne, sa
colonne, la ligne de source, son qualificatif (le texte du receveur), son usage (appel, type, import, héritage,
verrou…), et, quand on le sait, le type du receveur, lu sur place (`qualifier_type`) ou différé (`qualifier_deferred` :
le champ d'un type, le retour d'une fonction, avec la chaîne d'appels à éplucher).

C'est tout. Les **symboles locaux**, les **liaisons typées** et les **chaînes d'appels** sont calculés pendant
l'extraction, servent à écarter des références ou à typer un receveur, puis sont jetés. Il n'existe **aucune
représentation des expressions** : la seule forme approchante est la chaîne d'appels d'un type différé, lue comme du
texte (`receveur::split_chain`). Le champ `variables` existe, mais il est à peu près vide hors JavaScript (247
variables sur rag3weaver/src, 3 sur le moteur).

### Par langage

| Langage | Locales reconnues | Types des locales | Lambdas en scopes | `variables` |
|---|---|---|---|---|
| Rust | `let`, `for`, `match`, `if let`, paramètres de fermeture | les plus riches : annotation, constructeur, `let x = f()` différé, `Some`/`Ok` dépliés, conteneurs std, liaisons datées par position | oui | vide |
| C, C++ | déclarateurs (`auto x = …`, `int x;`, `for (auto& x : …)`), depuis la pile de ce soir | annotation, constructeur, `for` sur plage ; `auto x = f()` différé | lambdas C++ oui | vide |
| Python | affectations simples, paramètres, `self`/`cls` | annotation, constructeur, `x = f()` | oui | affectations simples, sans type |
| Go | `:=` | à peu près rien (pas de branche pour `:=` ni pour les paramètres ; à vérifier) | à vérifier (`func_literal`) | vide |
| C# | déclarateurs génériques | branches génériques seulement (à vérifier) | à vérifier | vide |
| TS, JS | `const`/`let`/`var`, destructurations | annotation, constructeur ; portée de bloc (`visible_until_line`) | oui (flèches, fonctions valeurs) | oui |

Les écarts sont nets : Rust est le langage le mieux typé, Go et C# les moins bien. La règle de Lucie « un lot prend
tous les langages » s'applique : chaque lot ci-dessous vise du code commun, avec une table par grammaire.

### Ce que le graphe garde

Le graphe de rag3weaver (`code.rs`) a quatre tables de nœuds (`File`, `Scope`, `Library`, `Symbol`) et des relations
de structure (`DEFINED_IN`, `PARENT_OF`), d'usage (`CONSUMES`, `INHERITS_FROM`, `IMPLEMENTS`, `DECORATES`,
`USES_LIBRARY`) et de rendez-vous (`DEFINES`, `MENTIONS`, `LOCKS`…).

Les références y sont **agrégées par couple (scope, nom)** : une arête `CONSUMES` porte la plus petite ligne de ses
sites, sans colonne. Sont jetés à l'entrée du graphe : les `variables`, les `parameters` (ils ne survivent que dans le
texte de la signature), les colonnes et les lignes de chaque occurrence. Les lambdas sont repliées dans leur scope.

Conséquence pour la réécriture : **le graphe ne sait pas où sont les occurrences**. Les outils d'édition actuels
(`edit`, `replace_text`, `replace_lines`) remplacent un texte unique ou une plage de lignes ; `usages` rend un fichier
et une ligne par arête. Aucun renommage, aucune édition multiple.

### Aucun modèle dans l'analyse

Ni codeparsers ni l'ingestion du code (`code.rs`, `code_sync.rs`, les dérivations) n'appellent de modèle. Les modèles
ne servent qu'au chat et aux outils de l'agent.

## Les volumes, mesurés

Comptes de la grammaire tree-sitter (nœuds nommés de tout le fichier) et de codeparsers (ce qu'il rend), sur les
quatre corpus de la sonde de marque.

| | rag3weaver/src (Rust) | moteur `src/` (C++) | `tools/python_api` (Python + C++) | UI (TS, TSX, JS) |
|---|---|---|---|---|
| fichiers | 288 (+2 JS) | 1 550 | 43 + 42 | 39 |
| octets | 10,3 Mo | 6,8 Mo | 0,50 Mo | 0,27 Mo |
| nœuds nommés | 1 530 821 | 1 014 336 | 89 083 | 48 488 |
| identifiants | 528 496 | 400 674 | 25 801 | 15 017 |
| appels | 113 225 | 51 883 | 5 180 | 1 967 |
| accès membres | 100 028 | 40 116 | 4 434 | 3 116 |
| liaisons locales | 28 741 | 31 703 | 2 586 | 1 361 |
| nœuds d'erreur | 0 | 1 306 | 0 | 0 |
| **codeparsers** : scopes | 25 399 | 19 462 | 857 | 452 |
| paramètres | 21 721 | 16 934 | 711 | 226 |
| références (une par occurrence) | 275 786 | 116 977 | 11 126 | 6 082 |
| dont receveur typé sur place | 70 045 | 35 787 | 1 114 | 547 |
| dont type différé | 26 933 | 15 269 | 370 | 308 |
| dont receveur non typé | 22 367 | 343 | 10 | 139 |
| temps d'analyse (release) | 2,17 s | 0,84 s | 0,04 s | 0,02 s |
| **graphe** : arêtes `CONSUMES` (sonde de marque) | 49 063 | 19 635 | 946 | 141 (7 fichiers admis) |

Lecture :
- les références sont déjà calculées une par occurrence : **275 786 sur rag3weaver/src, pour 49 063 arêtes**. Le
  graphe en garde un sur six environ, sans position ;
- les **liaisons locales** (28 741 en Rust, 31 703 en C++) sont du même ordre que les paramètres : ensemble, environ
  50 000 symboles locaux par grand corpus, deux fois le nombre de scopes ;
- les **identifiants** bornent le nombre d'occurrences possibles : 528 496 en Rust, 400 674 en C++, soit dix à vingt
  fois les scopes. Une partie sont des noms de déclaration, de champ ou de type, pas des usages ;
- les **appels** et **accès membres** bornent les expressions intéressantes pour la réécriture : environ 213 000 en
  Rust, 92 000 en C++ ;
- les **1 306 nœuds d'erreur** du moteur sont surtout des macros C++ que la grammaire ne sait pas lire : c'est le
  premier trou de couverture visible.

Sur l'UI, la sonde de marque n'admet que 7 fichiers ; la sonde de cette page en parcourt 39. Les comptes de la grammaire sont pris sur tout le fichier, pas seulement dans les corps : ils sont un **plafond**, pas
une prévision.

## Ce que le graphe porterait de plus

Trois entités, chacune optionnelle par langage et par profondeur (un réglage d'index, comme le reste du schéma
déclaratif) :

1. **`Local`** : un symbole déclaré dans un corps (liaison, paramètre, variable de boucle, capture de lambda). Il porte
   son nom, sa sorte (paramètre, `let`, boucle, motif, capture), son type déclaré ou lu, son type différé, sa plage
   d'octets, et sa provenance (`règle` ou `inféré`, voir plus bas). Relations : `DECLARES` (Scope → Local).
   **Volume** : environ 50 000 par grand corpus (paramètres + liaisons), 3 000 sur `tools/python_api`.
2. **`Occurrence`** : chaque site d'un nom dans un corps, avec sa plage d'octets exacte, son rôle (lecture, écriture,
   appel, type, déclaration) et sa cible résolue (un `Local`, un `Scope` ou un `Symbol`). Relations : `OCCURS_IN`
   (→ Scope), `REFERS_TO` (→ Local, Scope ou Symbol). C'est ce qui manque à la réécriture. **Volume** : au moins les
   références d'aujourd'hui (275 786 en Rust, 116 977 en C++), plus les occurrences des locales qu'on écarte
   aujourd'hui ; plafond, les identifiants (528 496, 400 674).
3. **`Expression`**, sélectif : seulement les formes qu'un outil sait utiliser, c'est-à-dire les **appels** et les
   **chaînes de membres** (`a.b().c`), avec leur plage, leur receveur et le type de chaque maillon quand on le sait.
   Pas l'arbre complet des expressions : il doublerait le volume sans usage nommé. Relations : `RECEIVER_OF`
   (Occurrence ou Expression → Expression), `CALLS` (Expression → Scope). **Volume** : plafond 213 000 en Rust,
   92 000 en C++.

Les arêtes agrégées d'aujourd'hui (`CONSUMES`) restent : elles servent la recherche, l'impact et `usages`, et se
dérivent des occurrences. Les nouvelles tables se lisent à la demande.

## Les trous de couverture, et où un modèle les comble

### Mesurer la couverture d'abord

La couverture d'un corps se définit nœud par nœud : un nœud nommé est **couvert** quand une règle de codeparsers le
classe (déclaration locale, occurrence résolue, expression rangée, ou nœud explicitement neutre comme un littéral),
**non compris** sinon. La sonde rend, par langage et par sorte de nœud, la part couverte et la liste des sortes non
comprises, triées par fréquence. C'est le tableau de bord de tout le chantier : chaque lot de règles le fait monter, et
on voit où il reste du travail sans deviner.

### Les trous visibles aujourd'hui

- **Receveurs non typés** : 22 367 appels en Rust dont le type du receveur ne se lit ni sur place ni par différé. Un
  modèle qui lit le corps de la fonction propose un type.
- **Types différés non résolus** : 26 933 en Rust, 15 269 en C++ ; une partie se résout par le résolveur, le reste est
  un trou.
- **Nœuds d'erreur** : 1 306 sur le moteur, surtout des macros (`RAG3DB_API`, macros d'assertion, macros de
  génération). Un modèle peut proposer ce que la macro déclare ou appelle.
- **Go et C#**, presque sans types de locales : un trou par langage, qui se comble d'abord par des règles (c'est
  moins cher et exact), pas par le modèle.

### La règle de provenance

Ce que propose un modèle porte la provenance **`inféré`**, avec le modèle, la date et l'empreinte de l'invite. Il n'est
jamais mélangé à ce qu'une règle établit : une requête choisit de le voir ou non, et la réécriture refuse de s'appuyer
sur une occurrence inférée sans confirmation.

### Mesurer ce que vaut le modèle

Sans juge, pas d'inférence en production. Le protocole :
1. **cas cachés** : prendre des sites où une règle connaît la réponse (70 045 receveurs typés sur place en Rust),
   masquer la réponse, demander au modèle ; la précision se compte site par site, par langage et par sorte de trou ;
2. **seuil** : le modèle n'est branché sur un trou que si sa précision sur les cas cachés dépasse un seuil fixé avant
   la mesure, et qu'on ne relâche pas sans preuve ;
3. **parité après coup** : quand une règle arrive pour un trou qu'un modèle comblait, on compare ses réponses à celles
   du modèle ; l'écart est consigné, et le trou passe à la règle.

Le modèle de production est Gemini, les modèles locaux servent à déboguer : la mesure se fait avec celui qui servira.

## La réécriture par le graphe

Le geste : une **édition déclarée** (renommer un symbole ; remplacer chaque occurrence d'un symbole par une
expression ; extraire une expression dans une locale) est traduite en une liste d'éditions d'octets à partir des
`Occurrence`, appliquée dans le worktree en une transaction, puis **vérifiée** : on réanalyse les fichiers touchés et
on compare le graphe d'après au graphe attendu (le même, au nom ou à l'expression près). Un écart annule tout le bloc.

Ce que ça demande, dans l'ordre : les occurrences avec leur plage exacte (lot 2 ci-dessous), les locales comme
entités (lot 3), puis les expressions pour l'extraction (lot 5). Le renommage est possible dès le lot 3. C'est une
primitive du socle, utilisable par l'agent de code comme par l'IDE (l'édition multiple de Ctrl-D, à l'échelle du
dépôt).

## Les étapes en lots

Chaque lot prend tous les langages, se prouve par un témoin rouge puis vert, et se mesure en release sur les trois
corpus (rag3weaver/src, le moteur, `tools/python_api` et l'UI) avec la sonde de marque. Un lot qui change le graphe
change la version de l'analyse : on les groupe pour une repasse.

| Lot | Ce qu'il fait | Témoin | Version de l'analyse |
|---|---|---|---|
| 1. Couverture mesurée | la sonde de couverture par langage et par sorte de nœud ; un rapport, aucun changement du graphe | le rapport des quatre corpus ; un test qui fixe la couverture d'un petit corpus par langage | inchangée |
| 2. Occurrences | chaque référence entre au graphe avec sa plage d'octets et son rôle ; `usages` rend chaque site avec sa colonne | `usages` d'un symbole rend tous ses sites, colonne comprise, sur un corpus à six langages | change |
| 3. Locales | `Local` et `DECLARES` pour les huit langages ; les occurrences des locales visent leur `Local` ; Go et C# rattrapent leurs types de locales par règles ; puis le renommage transactionnel | une locale par sorte et par langage ; un renommage appliqué puis vérifié, et annulé sur un écart | change |
| 4. Trous comblés par modèle | provenance `inféré`, protocole des cas cachés, branché trou par trou au-dessus du seuil | précision mesurée sur les cas cachés ; une occurrence inférée refusée par la réécriture sans confirmation | change (la provenance) |
| 5. Expressions | appels et chaînes de membres comme `Expression`, avec le type de chaque maillon | une chaîne `a.b().c` rangée, maillons typés ; l'extraction d'une expression dans une locale, vérifiée | change |

Les lots 2 et 3 sont le cœur : ils rendent possible la réécriture. Le lot 1 se fait d'abord, parce qu'il dit où porter
l'effort.

## Le coût d'index, estimé

Ce qui est mesuré : l'analyse parcourt déjà chaque nœud de chaque corps, et calcule déjà les références une par
occurrence ; elle prend 2,17 s sur rag3weaver/src et 0,84 s sur le moteur en release. Ajouter la sortie des locales et
des occurrences ne refait pas le parcours : l'**analyse** devrait rester du même ordre (une estimation à confirmer par
la mesure du lot 2 ; la cible est moins de +30 %).

Ce qui ne l'est pas encore : l'**écriture**. Aujourd'hui, rag3weaver/src écrit environ 25 000 scopes et 49 000 arêtes
`CONSUMES`. Avec les occurrences et les locales, ce serait environ 275 000 occurrences et 50 000 locales en plus, donc
**cinq à six fois plus de rangées** que les arêtes actuelles ; avec les expressions, jusqu'à 210 000 de plus. À environ
100 octets par rangée, cela fait de l'ordre de 30 à 50 Mo par grand corpus. Le temps d'écriture, lui, dépend du débit
d'insertion du moteur pour ce schéma : il se mesure au lot 2, sur une copie de base, avant toute décision de garder
les occurrences par défaut.

Si l'écriture coûte trop, deux replis, dans cet ordre : n'écrire les occurrences que des fichiers ouverts ou touchés
(calcul à la demande, la réécriture réanalyse de toute façon), puis ne garder par défaut que les occurrences des
symboles exportés.

## Lot 1 : la couverture mesurée (20 novembre 2026)

Le module `codeparsers::coverage` range chaque nœud nommé d'un fichier : **référence** (relevée par codeparsers à sa
ligne et sa colonne), **déclaration** (le nom d'un scope), **écarté** (un autre identifiant : locale, paramètre, champ,
type non relevé, mot vide, nom intégré), **neutre** (littéral, commentaire), **structure** (tout autre nœud ; un chemin
`a::b` compte en structure, ses noms un à un). La sonde `tests/probe_coverage.rs` le rend par langue et par sorte de
nœud, sans rien changer au graphe. Premier relevé, sur l'arbre de travail du 20 novembre :

| corpus | langue | nœuds nommés | référence | déclaration | écarté | neutre | structure |
|---|---|---|---|---|---|---|---|
| rag3weaver/src | Rust | 1 822 381 | 12,0 % | 2,0 % | 17,5 % | 15,7 % | 52,8 % |
| moteur `src/` | C++ (`.cpp`) | 684 473 | 10,3 % | 3,3 % | 21,0 % | 4,9 % | 60,5 % |
| moteur `src/` | C++ (`.h`) | 336 698 | 9,7 % | 7,1 % | 17,4 % | 4,9 % | 61,0 % |
| `tools/python_api` | Python | 62 971 | 10,8 % | 1,5 % | 11,8 % | 32,7 % | 43,2 % |
| UI | TSX | 43 132 | 12,2 % | 5,7 % | 13,3 % | 13,7 % | 55,2 % |

Les identifiants écartés sont d'abord des `identifier` (les locales et les paramètres que les lots 2 et 3 feront
entrer au graphe), puis des `field_identifier` et des `type_identifier` (des champs et des types lus dans un corps,
que la résolution par le receveur ne relève pas encore comme références). C'est là que porteront les lots suivants.

## Les réponses de Lucie (20 novembre 2026, 17 h 47)

Oui aux trois questions :

1. **Le découpage `Local`, `Occurrence`, `Expression` sélective**, « jusqu'à ce qu'on estime qu'il en manque » : pas
   d'arbre complet des expressions ; une forme s'ajoute quand un usage la demande.
2. **Le lot 1 d'abord** : la couverture mesurée, sans changement du graphe.
3. **La provenance `inféré`** jamais mêlée à l'analyse, et le seuil de précision mesuré sur des cas cachés avant tout
   branchement du modèle.

**Une précision de forme** : « faut pas que ce soit spécialement cherchable en vecteur, juste des relations et des
entités simples ». Les trois entités servent au graphe (`usages`, `impact`, la réécriture) : elles ne sont **ni
candidates au plein texte, ni découpées en morceaux, ni embarquées**. La recherche par le sens continue de passer par
les scopes.

Cela borne le coût d'index estimé plus haut : seules les **rangées** et les **arêtes** s'ajoutent (de l'ordre de 275 000
occurrences et 50 000 locales en plus par grand corpus, jusqu'à 210 000 expressions), sans document de plein texte,
sans morceau et sans vecteur. Le coût à mesurer au lot 2 est donc l'écriture des rangées et des arêtes seule ;
l'embarquement, qui domine le temps d'un index, n'en reçoit rien.

## Questions pour Lucie

1. Les trois entités (`Local`, `Occurrence`, `Expression` sélective) : le bon découpage, ou veux-tu l'arbre des
   expressions complet, pour l'horizon des traductions ?
2. Le lot 1 (la couverture mesurée, sans changement du graphe) en premier : d'accord ?
3. La provenance `inféré` jamais mêlée à l'analyse, et un seuil de précision avant tout branchement du modèle :
   d'accord ?
