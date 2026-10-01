# Banc de concurrence — rapport de session, nuit du 1er au 2 octobre 2026

Session « banc » (rag3db-19). Chantier confié par l'orchestration : la marche **A1**
du plan des écritures parallèles
(`docs/2-octobre-2026-00h17/01-ecritures-paralleles-vela-et-le-chemin.md`, §7 et
§11), c'est-à-dire **un banc de concurrence et un vérificateur d'intégrité, sans
toucher au moteur**. Le moteur n'a pas été modifié.

## Où le travail s'est arrêté

| | |
|---|---|
| Branche | `banc-de-concurrence`, poussée sur `origin` |
| Dernier commit | `00b2a0263` — étape 2 **en cours**, compile |
| Commits précédents | `71c27ae76` (spécification), `37a44e351` (étape 1) |
| Worktree | `../rag3db-banc` (depuis `origin/master` `aa7d64484`) |
| Build | `../rag3db-banc/build/release` (Ninja, Release, tests, sans extensions) |
| Cible | `concurrence_test` → `build/release/test/transaction/concurrence/concurrence_test` |
| Journaux des passes | `../rag3db-banc/build/banc-*.log`, non suivis |

Pour reprendre :

```bash
cd ../rag3db-banc
cmake --build build/release -j 8 --target concurrence_test   # pas « ninja » : alias -j32
./build/release/test/transaction/concurrence/concurrence_test > build/passe.log 2>&1
ctest --test-dir build/release/test -R 'concurrence|ConcurrencyBench'   # avec la comparaison known_red
```

**Rien n'est fusionné dans master.** La fusion de l'étape 1 attend la relecture
de la session cœur C++, puis un rebase et une avance rapide demandés par
l'orchestration.

## Ce qui a été fait

### La spécification (`71c27ae76`)

`docs/2-octobre-2026-00h36/01-specification-du-banc-de-concurrence.md` : la liste
des cas avec leur attendu, le vérificateur, le banc paramétré « fil ou processus »,
TSan, ce qui n'est pas su. Les réponses de l'orchestration sont en §7 : une
bibliothèque côté test et pas un `CALL check_integrity()`, des rouges qui tournent
dans la passe sous un label, le point de reprise automatique coupé hors de C8,
HNSW hors du banc.

### Étape 1 (`37a44e351`) : les trois corruptions déduites sont réelles

Lanceur Fil, vérificateur de niveau 1, cas C0 à C3. **Exécuté**, 20 passes sur 20
identiques, environ 100 ms :

| cas | observé |
|---|---|
| C0 témoin, clés disjointes | vert |
| C1 même clé primaire, 2 puis 3 écrivains | **rouge** : tous valident, la clé 7 existe en double ou en triple |
| C2 suppression d'un nœud contre relation vers lui, dans les deux ordres de commit | **rouge** : les deux valident, la relation pend |
| C3 nœuds et relations créés par deux écrivains dans une même transaction | **rouge** : les 7 relations du second pointent vers les nœuds du premier |

Aucune erreur, aucun refus : tout est silencieux.

**La relation pendante est invisible aux requêtes ordinaires.** Après C2,
`MATCH ()-[r:Link]->() RETURN count(r)` rend 1, mais toute requête qui lit une
propriété de l'extrémité supprimée (`MATCH (a)-[r]->(b) RETURN a.id`) ne la rend
plus : la jointure sur la table des nœuds efface la ligne. La première version du
vérificateur s'y est fait prendre et a rendu un faux vert ; elle est corrigée, et la
règle est écrite en tête de `integrity_checker.h`.

**La comparaison avec la liste des rouges connus** (`concurrence_test.known_red`)
échoue dans les deux sens. C'est vérifié sur une copie modifiée de la liste :
« new red » d'un côté, « known red turned green » de l'autre.

**Une fausse alerte retirée** : j'avais cru que le test amont
`ConcurrentRelationshipUpdatesWithMixedTransactions` attendait 2000 au lieu de 3000.
Mais 3 % 3 = 0 et le test est juste : je l'ai lancé, puis rejoué en lisant chaque
COMMIT et ROLLBACK.

### Étape 2 (`00b2a0263`, en cours)

Ajouts :
- la triple vérification de chaque cas : à chaud ; après `CHECKPOINT`, fermeture et
  réouverture ; après SIGKILL et rejeu du journal. Les deux dernières comparent
  aussi un vidage canonique de toutes les réponses ;
- le lanceur Processus ;
- les cas C4 à C9.

**Passe complète exécutée une fois** (316 s, presque tout dans C8) :

| cas | à chaud | après réouverture | après arrêt brutal |
|---|---|---|---|
| C0 témoin | vert | vert | vert |
| C1 même clé (2 et 3 écrivains) | rouge | rouge, identique | rouge, identique |
| C2 relation pendante (deux ordres) | rouge | rouge, identique | rouge, identique |
| C3 numérotation qui se chevauche | rouge | rouge, identique | rouge, identique |
| C4 virements | **rouge** | rouge | rouge |
| C5 double suppression | vert | vert | vert |
| C6 suppression contre mise à jour (deux ordres) | **rouge** | rouge | rouge |
| C7 mélange aléatoire | probabiliste | probabiliste | probabiliste |
| C8 points de reprise sous écrivains | vert, 94 à 189 s | vert | vert |
| C9 second processus écrivain (lanceur Processus) | vert | — | — |

En mode Processus, les cas de conflit sont **sautés** avec une raison écrite :
« B non construite ». Ils comptent comme des sauts, jamais comme des verts.

**Ce que la triple vérification dit.** Les trois corruptions de l'étape 1 passent le
point de reprise et le rejeu **à l'identique** : rien n'est réparé, rien n'est
aggravé, les réponses sont les mêmes avant et après. Le rejeu du journal réinsère
la clé 7 en double sans rien refuser. Aucun fichier `.ecarte-` n'apparaît après un
arrêt brutal qui suit des commits acquittés.

**C4, la trouvaille inattendue.** Huit comptes à 100, quatre écrivains, des
virements de deux mises à jour par transaction. La somme finit à 1279 au lieu de
800 : de l'argent est créé, alors que 256 conflits d'écriture ont été correctement
refusés. Les sondes, non commitées et décrites ici pour qu'on puisse les refaire,
ont établi ceci :
- une mise à jour par transaction : somme exacte. Pas de mise à jour perdue au sens
  classique ;
- deux mises à jour par transaction : environ 60 à 85 de trop par passe. Les
  compteurs montrent que le COMMIT n'échoue jamais : ce sont les transactions qui
  échouent **sur leur seconde mise à jour** qui gardent la première, dans 60 à
  80 % des cas ;
- comptes écartés de 2048 lignes, chacun dans son vecteur : la fuite persiste. Ce
  n'est donc pas la chaîne de versions d'un vecteur partagé ;
- tout ce que j'ai écrit de déterministe est **correct**. Les cas essayés : conflit
  contre une transaction ouverte, conflit contre une version déjà validée,
  mise à jour refusée puis annulation ou validation du détenteur. Le cas à deux
  écrivains aussi (l'un incrémente en boucle, l'autre échoue sur sa seconde
  mise à jour) ;
- donc : il faut au moins trois écrivains, ou un ordonnancement libre.
  **Mécanisme non identifié.** Mon hypothèse, non vérifiée : une course entre
  l'annulation et un autre écrivain, du ressort de TSan (étape 3) et de la session
  cœur C++.

**C6.** La suppression et la mise à jour de la même ligne valident toutes les deux,
dans les deux ordres. La ligne finit supprimée, et la mise à jour est perdue sans
erreur. C'est une violation de l'isolation par instantané (le premier qui valide
devrait gagner), pas une corruption de structure : le vérificateur ne voit rien.
C'est l'invariant propre au cas qui tombe.

**C8.** L'intégrité tient, mais avec le délai du moteur (5 s), 17 points de reprise
sur 24 expirent sous trois écrivains courts, et chaque attente gèle les écrivains.
Le cas dure de 94 à 189 s par variante. C'est le problème de la marche A7, ici
mesuré. Le délai se règle par
`TransactionManager::setCheckPointWaitTimeoutForTransactionsToLeaveInMicros`, qui
est **privé**. Je ne l'ai pas rendu accessible, ce serait toucher au moteur.

**C7** a été vu vert puis rouge sur la même graine : l'ordonnancement décide. Il
est hors de `known_red.txt`, et la comparaison peut donc échouer à cause de lui.

## Ce qui n'est pas fait, dans l'ordre où je le reprendrais

1. **Une passe complète sur `00b2a0263`.** Le dernier binaire lancé en entier
   précède deux changements : la correction de `rollback()` dans le banc, qui
   rendait C7 faussement rouge par des « No active transaction for ROLLBACK »
   comptés comme inattendus, et le retrait du délai de C8. Ensuite, vérifier que
   `known_red.txt` correspond à la passe.
2. **Décider de C8** : le garder à 5 s (plusieurs minutes dans chaque passe), le
   raccourcir (moins d'écrivains, ou un nombre borné de CHECKPOINT), ou demander un
   accès au réglage. À trancher avec l'orchestration.
3. **Décider de C7** : un rouge probabiliste ne peut pas entrer tel quel dans une
   liste comparée à l'identique. Une piste : plusieurs essais par passe, « rouge si
   un essai l'est ». À proposer.
4. **Le niveau 2 du vérificateur et son témoin rouge**, demandé par l'orchestration :
   un test qui fabrique à coup sûr une relation pendante par `NodeTable::delete_`
   sans l'exécuteur, et qui exige que le vérificateur la voie. Les API sont
   cartographiées dans le knowledge dump.
5. **Les trois cas d'attente sur verrou par clé** (décision de Lucie, transmise par
   l'orchestration) : le premier annule, le second qui attendait réussit ; le
   premier valide, le second reçoit la clé en double ; un interblocage se termine
   par une erreur nommée en un délai borné. À écrire **après** la note de conception
   de la session cœur C++. Il faut préparer la mécanique : deux barrières par cas,
   un délai de garde côté banc, la lecture de l'ordre réel des événements.
6. **Étape 3, ThreadSanitizer** : un build séparé (`make relwithdebinfo TSAN=1` dans
   son propre répertoire, `-j8`), le lanceur Fil seulement. C4 en est le premier
   candidat.

## Ce qui attend quelqu'un d'autre

- **La session cœur C++** : relire le banc par git (`banc-de-concurrence`) ; la
  fusion de l'étape 1 en dépend. Lui transmettre C4 et C6, qu'elle n'avait pas dans
  son plan.
- **L'orchestration** : les choix pour C7 et C8 (points 2 et 3 ci-dessus).
