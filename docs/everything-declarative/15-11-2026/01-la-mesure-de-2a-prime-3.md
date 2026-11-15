# 2a′-3 : la mesure de la visibilité qui peut réapparaître

15 novembre 2026, 19 h 57, ED. La mesure demandée par c7 (rapport de c7 du 14 novembre, 00 h 2x) et posée
avec arbre principal (page 03 d'arbre principal, « L'ordre du rattrapage contre l'identité des lignes »).
Elle compare trois formes de visibilité, pour le cas où une génération g s'écrit après g+1 (la pointe
d'abord, le rattrapage, la citation qui passe devant) :

- **(2)**, les deux colonnes `_depuis` et `_jusqu_a` sur la ligne, la forme de 2a′-2. Une ligne écrite
  hors d'ordre devient un doublon au bord haut du trou, compté dans l'état. C'est le défaut tant que la
  mesure n'a pas parlé.
- **(3a)**, l'uuid par l'empreinte du contenu et une table d'intervalles, une rangée par intervalle. Le
  prédicat devient une jointure.
- **(3a′)**, les deux colonnes pour l'intervalle principal, et une annexe des seules réapparitions. Le
  prédicat devient « colonnes visibles OR EXISTS dans l'annexe ».

La sonde est `extension/rag3weaver/tests/sonde_visibilite_2a_prime_3.rs`, sur la branche
`mesure-2a-prime-3` (238681fb4), hors push. Elle a deux tests `#[ignore]`, qui mesurent sans échouer.

## 1. Combien de réapparitions : aucune

Une réapparition, c'est une clé de scope dont l'empreinte (sa rangée sans la position) revient à une
valeur déjà vue avant la précédente, A → B → A, ou une présence après une absence qui retrouve une
empreinte connue.

| Fenêtre (première parente de `origin/master` de rag3db) | Commits qui touchent du code | Fichiers de code | Clés suivies | Changements | Réapparitions |
|---|---:|---:|---:|---:|---:|
| 50 derniers commits, tous | 19 | 15 | 1 392 | 207 | **0** |
| 50 derniers commits sous `src/`, `extension/`, `tools/` (`RAG3WEAVER_SONDE_CODE_SEUL=1`) | 45 | 28 | 2 423 | 702 | **0** |

La limite est dite : 50 commits et 28 fichiers. Une réapparition naît d'un revert ou d'un aller-retour
sur une même fonction, et la fenêtre n'en contient pas. Le nombre ne prouve pas qu'il n'y en a jamais ;
il dit qu'elles sont rares devant les changements (0 sur 702).

## 2. Ce que coûte le prédicat

Sur une **copie** migrée d'une base de code réelle (79 662 `Scope`, 7 803 `File`, 116 215 `CONSUMES`), en
médiane de 10 tours, le jeu de lecteurs de `sonde_cout_du_predicat`. Le fichier le plus peuplé est
`templates/ide/app/app.js` (1 561 scopes) ; le nom le plus consommé est `json`.

| Lecteur | (2) colonnes | (3a) intervalles | (3a′) annexe vide | (3a′) annexe à 1 % |
|---|---:|---:|---:|---:|
| outline (un fichier) | 8,83 ms | 16,62 ms (×1,88) | 11,43 ms (×1,29) | 11,14 ms (×1,26) |
| usages (un saut) | 7,60 ms | 15,10 ms (×1,99) | 9,85 ms (×1,30) | 8,81 ms (×1,16) |
| impact (1 à 3 sauts) | 13,31 ms | 27,09 ms (×2,04) | 11,46 ms (×0,86) | 12,33 ms (×0,93) |
| racines d'une source | 18,10 ms | 45,72 ms (×2,53) | 24,64 ms (×1,36) | 38,42 ms (×2,12) |
| compte des scopes | 2,09 ms | 20,71 ms (×9,91) | 12,92 ms (×6,18) | 18,25 ms (×8,73) |

L'impact de (3a′) sous 1 est du bruit : la requête rend une ligne et varie d'un tour à l'autre plus que
l'écart.

**Les plans** (`EXPLAIN`, compte des opérateurs) :

- (2), usages : 3 `SCAN_NODE_TABLE`, 1 `HASH_JOIN`, aucun `TABLE_FUNCTION_CALL`.
- (3a), (3a′) vide et (3a′) à 1 %, usages : 9 `SCAN_NODE_TABLE`, 5 `HASH_JOIN`, 2 `TABLE_FUNCTION_CALL`
  (l'EXISTS), **les trois plans identiques**.

**L'élagage de l'annexe vide n'a pas lieu.** Le plan de (3a′) vide est celui de (3a′) à 1 % : le moteur
joint l'annexe même quand elle n'a aucune rangée. C'est ce qui coûte ×6 au compte des scopes.

## 3. Ce que coûte l'écriture

- (3a) : la pose des 79 662 intervalles, une rangée et une arête par ligne, prend 1 252 ms. C'est une
  rangée de plus par ligne écrite, à chaque génération, pour un bénéfice que la mesure du § 1 ne trouve
  pas (aucun second intervalle).
- (3a′) : 796 réapparitions (1 %) posées en 14 ms ; à 0 réapparition, l'annexe ne s'écrit pas.
- (2) : le doublon au bord haut d'un trou. Son coût en lignes n'est pas mesuré ici : il dépend de la taille
  des trous de rattrapage, que la montre de l'indexation borne (≤ 5 générations ou 30 s par passe).

## 4. Ce que j'en conclus

- **(3a) est écartée** : ×2 sur les lecteurs du quotidien, ×10 sur le compte, une rangée de plus par
  ligne, pour des réapparitions qui ne se voient pas sur 50 commits.
- **(3a′) n'est pas gratuite à vide** : le moteur n'élague pas l'EXISTS sur une annexe vide. Elle coûte
  ×1,3 sur outline et usages, ×1,4 sur les racines et ×6 sur le compte, pour une annexe qui reste vide.
- **(2) reste le défaut**, comme c7 l'a posé : la forme la moins chère à lire, et son défaut (le doublon)
  coûte des lignes, jamais une identité.

## 5. Une variante à trancher : l'uuid par le contenu, sans annexe

Proposition, rien n'est codé ; à trancher par arbre principal et c7.

La mesure sépare deux choses que (3a′) liait : **l'identité par le contenu** règle l'ordre, et **l'annexe**
ne sert qu'aux réapparitions, qui n'existent presque pas. D'où :

- l'uuid d'une ligne est `hash(clé, couche, empreinte)`, l'empreinte étant la rangée sans la position
  (la position sort de l'identité en 2a′-3, page 03, § 9) ;
- écrire g après g+1, pour une ligne au même contenu, retrouve la même ligne et **descend son `_depuis`** à
  g. L'uuid ne change pas, et les arêtes gardent leur cible. Ni doublon ni annexe ;
- une vraie réapparition (A → B → A) retombe sur le doublon de (2) : sa ligne prend
  `hash(clé, couche, empreinte, depuis)`, et l'état la compte comme aujourd'hui ;
- le prédicat reste celui de (2) : le coût de la colonne (2) du tableau du § 2.

Ce qui reste à dire avant de la prendre :

1. **Descendre `_depuis` sur une ligne publiée.** Les lignes de 2a′ sont immuables, sauf `_jusqu_a`
   à la retraite. Descendre `_depuis` n'ouvre la ligne qu'à un rang qui n'avait encore rien de publié, le
   rang rattrapé. Il faut quand même le prouver contre le cache par rang (page 06) : une lecture épinglée
   à g ne doit pas avoir vu la base avant la pose de g.
2. **L'ordre dans la transaction** : descendre `_depuis` et poser les naissances de g dans la même
   écriture, pour qu'aucun lecteur ne voie g à moitié.
3. **Le compte de l'état** : le nombre de lignes reprises par descente, et le nombre de doublons de
   réapparition.

Si elle est refusée, (2) reste tel quel, sans changement de code.
