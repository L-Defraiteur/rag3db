# Banc de concurrence — rapport de session

Session « banc » (`rag3db-76`, anciennement `rag3db-19`). Mis à jour le 4 octobre 2026 à
midi, après la revue des amonts (`a10ee1c51`).

## Où en est le banc

- **Sur master**, tout est fusionné. Derniers commits de la session : `67c910f0d` (cas
  longs), `a10ee1c51` (témoins des amonts), poussés en `6fe2e4fc0`.
  Chaque lot est entré en avance rapide, sans force, et la comparaison `known_red` était
  verte à chaque fois.
- **Code** : `test/transaction/concurrence/`, une seule cible, `concurrence_test`.
- **Spécification** : `docs/2-octobre-2026-00h36/01-specification-du-banc-de-concurrence.md`,
  §1 à §13 ; chaque lot y a sa section.
- **Liste des rouges** : `known_red.txt`, 87 rouges connus rangés par marche, chacun avec
  l'ensemble exact des étiquettes de son échec. `probabilistic.txt` : 6 cas. `long.txt` :
  12 cas hors de la passe par défaut.
- **Moteur** : jamais modifié par cette session.

## Ce qui a été fait, dans l'ordre

1. **Le banc et son vérificateur (étapes 1 à 3, 2 et 3 octobre).**
   - Le harnais : un scénario, des fils ou des processus, une barrière, un journal
     d'événements.
   - Le vérificateur : niveau 1 par Cypher, niveau 2 par les internes.
   - Trois corruptions déduites par le plan se sont révélées réelles : C1, la clé en
     double ; C2, la relation pendante ; C3, les relations rattachées aux mauvais nœuds.
   - C4 était un défaut du banc. C6 est un défaut du moteur, reproduit sans concurrence.
   - Première passe ThreadSanitizer.
2. **Les remarques de la relecture du cœur C++.**
   - Chaque rouge est épinglé à sa raison, par ses étiquettes `[check: …]`.
   - Un faux vert du niveau 1 est fermé : le balayage force son sens par `HINT`, vérifié
     par `EXPLAIN`.
   - Nouveaux cas : C1 avec un instantané antérieur au commit de l'autre, C2 avec la
     destination supprimée et avec `DETACH DELETE`, deux colonnes d'une même ligne.
   - La passe TSan par signatures, dans son propre build.
3. **Les cas sur une table indexée par HNSW.**
   - L'invariant de l'index et l'exécution isolée dans un processus fils.
   - H1 à H4, et l'index construit sur cent lignes qui perd le nœud 13.
4. **Les témoins des verrous, écrits avant les verrous** (§6 de la note du 3 octobre),
   avec les noms d'erreurs convenus avec le cœur C++, et le nœud-carrefour comme
   garde-fou vert.
5. **La correction de la variante Crash, et son audit.**
   - Le processus fils fermait sa base avant le SIGKILL : la variante ne rejouait aucun
     journal.
   - L'audit a nommé ce qui change, et les phrases à retirer sont corrigées par des notes
     datées.
   - Avec un vrai arrêt, une clé en double rend la base impossible à rouvrir (A3′).
6. **La famille « arrêt brutal, un seul écrivain »** : les correctifs de reprise livrés
   tiennent sous un vrai SIGKILL. Elle a fait sortir un défaut du mode en service : une
   base indexée qui fait planter le processus qui l'ouvre.
7. **La reprise avec un index d'extension**, sur la condition élargie par le cœur C++ :
   les attendus des gardes 1 et 2, et une seconde mort. La garde 1 est livrée.
8. **L'index vectoriel juste en service (4 octobre)**, témoins écrits avant le correctif du
   cœur C++, par une autre main (`vector_index_update_test.cpp`) :
   - Ses chiffres ont d'abord été reproduits hors du harnais : ils n'étaient pas
     reproductibles à l'unité, parce que le compte varie d'une passe à l'autre. Nous
     sommes convenus d'un seuil « toutes les lignes justes », sur des essais répétés.
   - Deux contrôles. Toute ligne est joignable par une recherche exhaustive, et chaque
     ligne sort première sur son propre vecteur. Le second voit des pertes que le premier
     ne voit pas.
   - Les cas : sa recette, plus 768 dimensions, dix mille lignes, une ligne mise à jour
     cinquante fois, mise à jour puis suppression, transaction annulée, ligne lointaine.
     S'y ajoute le chemin du produit, décrit par la session des embarquements.
   - Le témoin déterministe du cœur C++ sur les relations relues de travers
     (`uncommitted_relations_test.cpp`) : rouge avant `c8fdaf196`, vert après.
9. **Les cas longs hors de la passe par défaut (4 octobre)** : `long.txt`, label
   `concurrence-long`, comparés seulement avec `CONCURRENCE_LONG=1`. La comparaison par
   défaut repasse de 14 min 30 à 2 min 17 ; la longue dure 29 min.
10. **La revue des amonts (4 octobre)** : Ladybug et Vela lus en lecture seule, chaque
    correctif présenté comme manquant reproduit chez nous avant d'être rangé. Vingt défauts
    confirmés, 21 témoins rouges (`upstream_fixes_test.cpp`). Les deux défauts graves ont
    été signalés dès leur confirmation. Tableau : `03-revue-des-amonts.md`.

## Décisions et pourquoi

- **La raison du rouge est épinglée.** Sans cela, une marche qui échange une corruption
  contre une autre passe inaperçue (remarque de la relecture).
- **Un rouge probabiliste n'est jamais comparé.** Un test qui rougit une fois sur deux ne
  se lit jamais comme un vert.
- **La comparaison n'exécute pas les cas probabilistes**, parce qu'un plantage de C5
  emportait toute la passe.
- **Le second écrivain attend.** Les scripts y sont prêts (`writeInTurn`,
  `commitInOrderByEvents`) ; ceux que A4′ devra réécrire sont notés dans les cas : C5,
  C6_UpdateCommitsFirst, les trois C2_*RelationCommitsFirst.
- **Réouverture dans un processus neuf après chaque mort.** Un processus qui a déjà
  chargé l'extension vector la garde chargée, et cache le plantage d'un service qui
  redémarre.
- **Isoler le banc avant d'accuser le moteur, dans les deux sens.** Le banc a déjà
  accusé le moteur à tort (C4, les vecteurs mis à jour), et l'a aussi innocenté à tort
  (la fausse variante Crash).

## Ce qui attend quelqu'un

- **La session cœur C++** :
  - les marches V1, A3′, A4′ et V2 font passer les témoins des verrous au vert ; chaque
    marche retire ses lignes de `known_red.txt` dans son commit ;
  - la garde 2 retirera aussi les témoins de la garde 1 (`IndexToRebuildIsNamed`), qui
    deviendront rouges pour une autre raison ;
  - la question de ce que la reprise doit faire d'un journal qui porte un doublon (second
    témoin d'A3′) est chez l'orchestration ;
  - un crochet de test dans l'allocateur, si l'on veut la mort à chaque allocation pendant
    la phase de stockage d'un point de reprise. C'est non couvert ;
  - savoir si le graphe HNSW connaît la nouvelle position d'un vecteur mis à jour (le
    banc ne vérifie que ce que rend la recherche).
- **La session cœur C++, sur l'index vectoriel** :
  - la marche « mise à jour de l'index : contrôle en fin d'instruction » (quatre rouges
    dans `known_red.txt`, plus le ligne à ligne en probabiliste) ;
  - la ligne lointaine d'un nuage serré, sous la construction de l'index ;
  - **le ralentissement de la mise à jour depuis `c8fdaf196`** : dix mille lignes par
    lots de 512 passent de 25 s à 12–27 min. C'est signalé ; le cas est probabiliste (vert
    une fois sur sept essais).
- **La garde 2** est en cours chez le cœur C++. Elle retire elle-même ses cinq rouges, et
  ses `IndexToRebuildIsNamed` effacent la liste des extensions pour éprouver la garde 1.
  Il reste à faire vérifier par `IndexExactWithoutRebuild` que la liste existait avant la
  mort.
- **La session cœur C++, sur les amonts** : les 21 rouges de la marche « correctifs de
  l'amont à reprendre », dans l'ordre du tableau. D'abord la perte de relations au point de
  reprise, puis la lecture après le point de reprise d'une relation créée puis supprimée :
  rag3weaver passe par ces deux chemins.
- **Ce que la revue n'a pas couvert** :
  - deux recettes non reproduites (`254a7444d`, `a10cecbc7`) ;
  - dix défauts qui demandent une faute d'E/S, une coupure ou une mort placée au bon
    instant (§3 du tableau) ;
  - les défauts d'API sans témoin (§4).
- **TSan avec l'extension vector** : pas fait. L'extension se construit à un chemin fixe
  de l'arbre source, qu'un build TSan écraserait (§11).

## Comment reprendre

```bash
cd ../rag3db-banc                       # worktree du banc, branche locale banc-verrous
git fetch origin && git rebase origin/master
cmake -S . -B build/release -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTS=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_BENCHMARK=OFF -DBUILD_EXTENSIONS=vector
cmake --build build/release -j 8 --target rag3db_vector_extension concurrence_test   # jamais l'alias ninja
ctest --test-dir build/release/test -R known_red --output-on-failure   # la comparaison seule, 2 min 17
CONCURRENCE_LONG=1 ctest --test-dir build/release/test -R known_red   # avec les cas longs, 29 min
ctest --test-dir build/release/test -L concurrence-rouge-connu -N      # les rouges connus
```

- La comparaison : `test/transaction/concurrence/compare_known_red.cmake`. Elle lance le
  banc entier sans les cas probabilistes, lit le JSON de gtest et compare les étiquettes
  de chaque rouge à `known_red.txt`. Elle garde le JSON d'une passe en échec
  (`…result.json.failed-<horodatage>`).
- Les passes se lancent en série (`ctest -j 1`), jamais deux en parallèle.
- La passe TSan : un build à part, `build/tsan` (§9 et §10 de la spécification), puis
  `ctest --test-dir build/tsan/test -L concurrence-tsan`.
- Après un rebase qui touche `src/` ou `extension/vector/`, rebâtir et rejouer la
  comparaison avant de pousser master.
