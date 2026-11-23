# Après la suppression de toute une région sauf cinq lignes, un survivant peut être injoignable

- **État** : corrigé (23 novembre 2026, branche `region-delete-seed`) ; le cas sort de `probabilistic.txt` et revient au juge known_red
- **Famille** : moteur
- **Gravité** : réponse fausse, rare. Une ligne vivante n'est rendue par aucune recherche, et les plus proches d'une
  requête sont faux.
- **Atteignable en service** : à établir. Il faut supprimer presque toutes les lignes d'un index vectoriel (2 995 sur
  3 000, point d'entrée compris), en passant par `SET vec = NULL` avant le `DELETE`.
- **Touche rag3weaver** : à établir (le chemin « repasser par NULL » a été retiré de rag3weaver le 22 novembre)
- **Signalé par** : le juge known_red de la passe du lot `vector-index-special-names` (23 novembre, 03 h 39)
- **Pour** : le moteur, l'index vectoriel

## Le constat

`Cases/RegionDelete.SurvivorsStayReachableAndTheNearestAreRight/SetNullFirstAllButFive` (`concurrence_test`) :
`reached 4/5, nearest 1/5`. Le test n'avait jamais été relevé rouge, ni dans known_red ni dans probabilistic.

## La recette

Les données sont fixes : la graine mt19937 vaut 20261012, avec 3 000 vecteurs de dimension 16 autour de 30 centres,
en l2. L'index est créé avant les lignes, qui arrivent en 30 `UNWIND` de 100. Le test retire les 2 995 plus proches
du centre 0, triés par distance, en trois parts entrelacées (indices j ≡ 0, 1, 2 mod 3) : chaque part passe par
`SET n.vec = NULL`, puis par `DETACH DELETE`. Il cherche ensuite les 5 survivants à efs 5, et les 10 plus proches du
centre 0. Seuls le tirage des niveaux du graphe (pu) et l'ordre des fils varient d'un tour à l'autre.

## La fréquence, mesurée le 23 novembre (Release, sous poste cpu)

| moteur | rouges |
|---|---|
| master 03e649c42c + `vector-index-special-names` (avec le correctif des anciens voisins, 3196d6f66f) | 1 sur 121 (le juge, puis 0 sur 20 et 0 sur 100) |
| le même, le correctif des anciens voisins retiré (diff de cb3624f3ec en sens inverse) | 1 sur 100 |

La même signature des deux côtés. **Ce n'est pas une régression du correctif du 21 novembre** : le défaut est plus
ancien, et rare.

**Une réserve** (relecture du 23 novembre) : 1 sur 121 contre 1 sur 100 ne sépare pas les deux moteurs, mais ne prouve pas non
plus que le correctif du 21 novembre est sans effet sur ce cas. À cette fréquence, il faudrait des milliers de tours
pour départager. C'est le témoin déterministe (la graine du tirage des niveaux fixée sur un tour rouge) qui le dira.

## Ce qu'il faut chercher

Le passage par NULL fait de chaque ligne une mise à jour (retirée du graphe, réparée par `keepFormerNeighboursReachable`
avec ancre), puis une suppression. Avec presque toutes les lignes retirées, entrée comprise, les survivants dépendent de
la réparation du point d'entrée (`repairEntryPoint`) et des arêtes de rattrapage. Un témoin déterministe se cherchera
en fixant le tirage des niveaux (graine du `randomEngine` de l'index), pour retrouver le tour rouge à coup sûr.

## La cause, et le correctif (23 novembre, au soir)

**Un témoin déterministe.** `RAG3DB_HNSW_SEED` fixe, pour un test, le tirage de la couche haute (`HNSWIndex`, semé
sinon par l'entropie du système). Sur 300 graines, 4 rougissent (23, 93, 157, 217), toutes avec la même signature.
La graine 157, rejouée 10 fois, rougit 10 fois : l'ordre des fils ne joue pas. Le cas
`SetNullFirstAllButFiveSeed157` est ajouté à `RegionDelete`.

**Les quatre graines rougissent aussi sans le correctif du 21 novembre** (son diff retiré, puis rebâti). La réserve
est levée : le défaut est plus ancien.

**Les arêtes des cinq survivants à la graine 157** : quatre forment un bloc connexe dans la couche basse. Le
cinquième (1253) n'y a plus aucune arête, ni entrante ni sortante : il ne garde qu'une arête vers 1005 dans la
couche haute.

**La cause** : `keepNodeReachable` vérifie qu'un ancien voisin reste joignable en descendant la couche haute avec
son propre vecteur, puis en le cherchant dans la couche basse. Quand la descente aboutissait au nœud lui-même,
la fonction concluait « la requête part de lui » et ne cherchait rien. Pour un nœud de la couche haute dont toutes
les arêtes basses menaient à des lignes retirées, rien n'était rattaché. Aucune autre requête ne le joignait
(« reached 4/5 »), et une requête qui atterrissait sur lui ne rendait que lui (« nearest 1/5 »).

**Le correctif** : dans ce cas, la recherche part du point d'entrée bas, et le nœud est rattaché dans les deux
sens s'il n'est pas trouvé. Les quatre graines passent à 5/5 et 5/5, et les cinq cas de `RegionDelete` sont verts.

## Rejouer le balayage à graines

Pour un défaut de ce genre, rouge une fois sur cent dans un cas de l'index vectoriel : chaque graine fixe le tirage
de la couche haute, et une graine rouge rend le cas rejouable. Depuis le dossier du binaire
(`build/release/test/transaction/concurrence`), sous `poste cpu` (300 tours, une heure environ à luciepc) :

```bash
CAS='Cases/RegionDelete.SurvivorsStayReachableAndTheNearestAreRight/SetNullFirstAllButFive'
for s in $(seq 1 300); do
  RAG3DB_HNSW_SEED=$s ./concurrence_test --gtest_filter="$CAS" > /tmp/graine.log 2>&1 \
    || echo "graine $s : $(grep -h 'region delete' /tmp/graine.log)"
done
```

Une graine rouge se rejoue ensuite dix fois de suite. Si elle rougit à chaque fois, l'ordre des fils ne joue pas, et
la graine devient un cas du test (`RegionDeleteCase{true, true, <graine>}`).
