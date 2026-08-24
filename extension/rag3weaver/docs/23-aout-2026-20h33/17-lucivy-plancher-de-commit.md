# `commit()` : un plancher de ~0,6 à 1,2 s qui est de l'attente, pas du calcul

Mesure faite depuis rag3weaver en profilant nos suites E2E, qui mettaient
beaucoup plus longtemps que ce que vos chiffres laissent attendre. Ce n'est pas
une régression de votre part et rien n'est cassé : c'est un coût que personne
n'avait encore isolé, et qui se voit surtout sur les **petits lots**.

Reproductible : `tests/e2e_profile_overhead.rs` chez nous
(`cargo test --features rag3db-native --test e2e_profile_overhead -- --ignored --nocapture`).

## Le point de départ

Neuf documents minuscules, `MemBlobStore`, 2 shards, aucune base de données
dans la boucle :

```
── lucivy seul, 9 documents ──
  création index :   31.2 ms
  indexation     :    0.1 ms
  commit         :  678.9 ms   ←
  recherche      :    0.8 ms
```

L'indexation et la recherche sont exactement là où vous les annoncez. Le
`commit()` est trois ordres de grandeur au-dessus du reste.

## La forme du coût

```
   1 doc  : commit#1    95.9 ms · commit#2 (rien de sale)  10.7 ms
   9 docs : commit#1   640.1 ms · commit#2                 10.9 ms
  90 docs : commit#1  1159.5 ms · commit#2                 23.7 ms
 900 docs : commit#1  1183.9 ms · commit#2                 18.9 ms
```

Trois choses à en tirer :

1. **Ça sature.** 900 documents coûtent autant que 90. Le terme dominant n'est
   donc pas par-document.
2. **Le chemin à vide est bon marché** (~11-19 ms). Ce n'est pas la structure de
   l'appel qui coûte, c'est ce qu'il déclenche quand il y a du sale.
3. **Ce n'est pas sous-linéaire par hasard** : de 1 à 9 documents le coût est
   multiplié par 6,7 alors que le volume l'est par 9 ; de 90 à 900, il ne bouge
   plus.

## Ce n'est pas du CPU

Deux mesures indépendantes le disent.

**Release ne change rien** : commit 678,9 ms en debug contre 662,7 ms en
release. Sur de la construction de FST, on attendrait un facteur, pas 2 %.

**Le processus est inactif 88 % du temps** :

```
wall = 3,69 s | CPU (user+sys) = 0,46 s | ratio = 0,12
```

Sur une machine à 24 cœurs, un travail parallèle donnerait un ratio *supérieur*
à 1. À 0,12, le processus dort. Il y a un sleep, un intervalle de poll, ou un
rendez-vous avec un acteur en tâche de fond quelque part sur le chemin de
commit.

## Pourquoi ça nous concerne

Nous appelons `commit()` **une fois par drain**, dans `FlushNode`. Un agent qui
ingère au fil de l'eau — quelques documents à la fois, ce qui est le mode
normal d'un agent de code — paierait donc ~0,6 à 1,2 s d'attente pure à chaque
lot, quelle que soit sa taille. Sur nos suites E2E, ça représentait la moitié du
temps de chaque test.

Précision utile : nos tests mesurent avec 2 shards, la production en utilise 4
(`DEFAULT_SHARDS`). Si le coût est lié à un rendez-vous par shard, l'écart
compte.

## Ce qu'on ne demande pas

Pas de correctif dans l'urgence — rien n'est faux, les résultats sont bons, et
sur un gros corpus le coût s'amortit. Mais si c'est bien un délai fixe, le
rendre configurable (ou le supprimer quand le commit est explicite et
synchrone) transformerait le profil de l'ingestion incrémentale.

Et si vous savez déjà d'où ça vient, ça nous intéresse : on saura si c'est à
nous de committer moins souvent, ou à vous de committer plus vite.
