# Les lignes d'un COPY forcé validé juste après un CREATE_VECTOR_INDEX restent hors du graphe jusqu'à la reconstruction

- **État** : ouvert. Écrit le 18 octobre 2026 par la seconde session cœur C++, à la demande de
  l'orchestration ; mis sur master le 25 octobre seulement (il était resté en brouillon), toujours
  vrai après 6fe9ffe9b. Défaut connu, sans perte. La question qu'il pose est pour le produit : qui déclenche la
  reconstruction de l'index, et quand.
- **Gravité** : lenteur. La réponse est juste, mais chaque recherche balaie ces lignes une à une.
- **Atteignable en service** : pas aujourd'hui. rag3weaver lance ses CREATE_VECTOR_INDEX
  (`rebuild_vector_index`, `ensure_vector_index`) sous le `Mutex<Catalog>` de sa connexion
  d'écriture, que ses COPY prennent aussi : aucun COPY du processus ne court pendant un bâti. Le cas
  vient dès qu'un CREATE se fait sous écritures, avec (A), le CREATE concurrent (page 07), ou par un
  autre client. Le même état existe aussi, en petit, pour des lignes de queue à vecteur NULL.
- **Touche rag3weaver** : oui. C'est lui qui bâtit et reconstruit les index
  (`ajuster_l_index_pour_le_retard`, `rebuild_vector_index`).

## Ce que c'est

Le CREATE rattrape à sa validation ce que les autres ont validé pendant son bâti (ticket fermé
`closed/2026-10-12-les-lignes-validees-pendant-le-bati-d-un-create-manquent-a-l-index.md`, forme
δ). Un COPY forcé parti pendant le bâti valide après lui, dans la file du même point de reprise
(A7). Au moment du rattrapage, ses lignes sont déjà dans la table, mais pas encore validées.

Le rattrapage s'arrête donc à la première d'entre elles, et `numCheckpointedNodes` s'y pose. Ces
lignes, et toutes celles qui suivent, restent hors du graphe. La recherche les lit par
`searchFromUnCheckpointed`, un calcul de distance par ligne à chaque requête. Rien ne les fait
entrer dans le graphe ensuite : le point de reprise de l'index ne réécrit que ses tables de
relations. Il faut attendre la reconstruction de l'index.

Le même état existait déjà sur master pour toute ligne écrite au-delà du compte du graphe.

## Recette

Le témoin `VectorIndexUpdate.AForcedCopyCommittedRightAfterACreateIsReached`
(`vector_index_update_test.cpp`) :
- 20 000 lignes, puis le CREATE sur une connexion ;
- 300 ms après, sur une autre connexion, `CALL copy_journal_threshold=1;` puis un
  `COPY C FROM (UNWIND …)` de 1 000 lignes.

Les 21 000 lignes sont atteintes (vert), et les 1 000 du COPY le sont par le balayage.

## La mesure qui manque

Aucune mesure ne dit ce que coûte le balayage de N lignes hors du graphe. Les documents de session
le décrivent sans le chiffrer (`3-octobre-2026-23h31/coeur-cpp/02-knowledge-dump.md`). Il faudrait,
sous `poste mesure`, la latence d'une recherche à k = 10 pour N = 0, 1 000, 10 000 et 100 000
lignes hors du graphe, à la dimension du produit (768). Ce chiffre dira à partir de quel N la
reconstruction vaut son coût.

## La question pour le produit

La règle « un index ne se pense pas » demande que personne n'ait à y penser. Il faut donc dire qui
remet ces lignes dans le graphe, et quand :
- le moteur, au point de reprise, quand la part hors du graphe dépasse un seuil (comme
  `OnDiskHNSWIndex::finalize` le fait pour un COPY) ;
- rag3weaver, quand son état d'index voit un retard, par sa reconstruction ;
- ou les deux, avec le seuil tiré de la mesure ci-dessus.

## Pour le fermer

1. La mesure.
2. La décision : qui déclenche, à quel seuil.
3. Un témoin : après le cas de la recette et le déclencheur, `numCheckpointedNodes` égale le
   nombre de lignes, et les 21 000 lignes sont atteintes par le graphe seul.
