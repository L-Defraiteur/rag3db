# Un point de reprise fige toutes les transactions le temps de s'écrire, lecteurs compris, et il s'écrit toutes les trois secondes sous charge

- **État** : ouvert (mesuré par le banc le 18 octobre 2026 ; à placer dans la file du cœur C++
  par l'orchestration)
- **Gravité** : attente, pour tout le monde : aucun BEGIN n'entre et chaque validation attend
  que le point de reprise soit écrit ; sous vingt écrivains, 1,2 à 1,4 s par point de reprise,
  un toutes les 3 s au seuil. Ni perte ni réponse fausse.
- **Atteignable en service** : oui, c'est le régime ordinaire d'un hôte qui indexe (le journal
  atteint le seuil sans cesse).
- **Touche rag3weaver** : oui (chaque appel d'outil pendant une indexation peut attendre un
  point de reprise entier).
- **Ouvert le** : 18 octobre 2026, session cœur C++, sur demande de l'orchestration, d'après la
  mesure du banc.
- **Pour** : cœur C++.

## Ce qu'on voit (banc, `DdlEveryMinute`, luciepc, 6ff805fcb sur master 248bf03cc, 2 Gio,
`RAG3DB_PROFILE_CHECKPOINT=1`, vert)

- Les `ADD COLUMN` ne coûtent rien : `ADD note` 7,9 ms, `ADD rang DEFAULT 0` 9,1 ms ; pendant
  chacun, p99 des écrivains de `Doc` 39 et 32 ms, de `Note` 19 et 15 ms. L'attente de 1,3 s « sur
  toutes les tables » de la première passe était un point de reprise au seuil tombé au même
  moment (page `coeur-cpp/16`, §2, départagé).
- Sur la passe : **129 points de reprise « au seuil », environ un toutes les 3 s**, et un
  « forcé » (celui de `CREATE_VECTOR_INDEX`, 862 ms : `Doc` 764, `Link` 48, `Note` 10). Le
  `CHECKPOINT` explicite de la minute 5 dure 1,24 s, dont 1,07 s pour `Doc`.
- Journal : `~/.cache/rag3weaver-build/jeu-vingt/minute-profil-6ff805fcb.log` sur luciepc.

## Ce que fait le code

`TransactionManager::checkpointNoLock` : `stopNewTransactionsAndWaitForOthersNoLock` (aucun
BEGIN n'entre, lecteurs compris ; attente bornée du départ de tous), puis `writeCheckpoint`
(chaque table réécrit ses groupes modifiés en place, les colonnes sont renumérotées
`vacuumColumnIDs`, les tables retirées sont récupérées, catalogue et métadonnées sérialisés,
fsync), puis `allowNewTransactionsNoLock`. La validation qui dépasse le seuil l'écrit elle-même
si elle est seule, sinon pose la demande (A8) et la dernière à partir l'écrit ; une demande qui
attend plus de `AUTO_CHECKPOINT_WAIT` est abandonnée. Pendant l'écriture, tout attend.

Chez PostgreSQL, un checkpoint n'arrête aucune transaction : il écrit les pages sales en arrière-
plan, les lecteurs et les écrivains continuent sur les versions en place, et le journal borne la
reprise. Chez nous, l'arrêt tient à la réécriture en place sous les états de balayage (les
pointeurs de blocs et de groupes des lecteurs, les plans liés aux numéros de colonnes).

## Ce qu'il faudrait

Un point de reprise qui ne fige pas tout le monde. Trois pièces, à étudier avant toute écriture :
1. **ne plus arrêter les lecteurs** : les blocs qu'un point de reprise remplace restent vivants
   tant qu'un balayage les tient (la même garde que pour le rollback qui détruit un bloc sous un
   lecteur, ticket du 18 octobre) ; la renumérotation des colonnes ne se fait que sans lecteur, ou
   se fait sans renuméroter ;
2. **ne plus arrêter les écrivains** : la part du point de reprise qui ne dépend que des versions
   validées avant son départ s'écrit pendant que les écrivains continuent, comme un instantané ;
   ce qui arrive après ira au suivant ;
3. **moins souvent** : le seuil (taille du journal) déclenche un point de reprise toutes les 3 s
   sous vingt écrivains ; un seuil en temps ou un seuil plus haut quand le journal rejoue vite
   change déjà le régime (à mesurer avec le même scénario).

Le « forcé » du `CREATE_VECTOR_INDEX` est le sujet du ticket du CREATE concurrent (hotfixs).

## Témoin

Le scénario du banc avec `RAG3DB_PROFILE_CHECKPOINT=1` : le p99 des écrivains et des lecteurs
pendant les fenêtres des points de reprise au seuil, avant et après. Fermé quand un point de
reprise au seuil ne fait plus attendre les lecteurs, et que l'attente des écrivains tient dans
une borne dite.
