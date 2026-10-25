# Une relation LOCKS trouvée d'un seul côté fait échouer le drain de la montre

- **État** : ouvert, le 25 octobre 2026. Signalé par G, à la demande de l'orchestration ; confié
  à la seconde session cœur C++ (hotfixs). **Forme réelle relevée le 25 à 22 h 55** (section « Le
  relevé complet ») : des longueurs ou des débuts de liste CSR faux dans deux tables de liens, LOCKS
  et QUERIES, antérieurs à la bascule. Cause non trouvée, aucun correctif moteur écrit.
- **Gravité** : lectures fausses et écriture refusée. Les deux tables rendent des lignes qui
  n'existent pas (relation 0 vers le nœud 0), en perdent d'autres d'un côté, et en rattachent à un
  mauvais nœud selon le sens lu. La garde de `RelTableData::update` ne voit que les demi-arêtes
  qu'une mise à jour touche.
- **Atteignable en service** : oui. C'est l'hôte de Lucie, sur la base du dépôt de code, depuis
  le réveil de 17 h 43 sur la lib 04063ee06. Jamais avant.
- **Touche rag3weaver** : oui, par le drain de la montre (gabarit de code, table de liens LOCKS).
- **Pour** : hotfixs, avec le cœur C++. Les lots du stockage de ce jour sont les premiers
  suspects.

## Ce qu'on voit

Le drain de la montre échoue parfois, avec ce message :

```
drain FAILED: … The transaction was rolled back because a statement in it failed … l'instruction
qui l'a défaite : Query execution failed: Runtime exception: Relationship 851 of table LOCKS was
not found in the forward direction, among the relationships of node 16249. Its update is
refused: it would have reached one direction only.
```

- Il y a trois occurrences dans `~/.local/state/rag3weaver/daemon.log` (lignes 1861, 2074 et 2464).
  La liste, avec le journal depuis 21 h 12, est copiée dans
  `~/.cache/rag3weaver-build/temoins-rust-348fa758b/locks-trois-occurrences.txt`.
- La base est `~/.local/share/rag3weaver/code-rag3db-depot.rag3db`, passée au format 42 à
  18 h 32.
- Dans le même drain, juste après la troisième occurrence, des fichiers restent en erreur :
  `extension/rag3weaver/scripts/rag3db_libs.py`, `scripts/test_rag3db_libs.py`, `src/agent.rs`, …
- Il existe deux instantanés pour comparer : `bascule-04063ee06` (avant la réouverture de 17 h 43)
  et `bascule-66994dd43`.

**Une quatrième occurrence, la même arête** (G, 25 octobre au soir). Elle est survenue juste après
le réveil de 21 h 19 min 23 s (repli sur 04063ee06, Rust 348fa758b), à la ligne 2551 du journal.
C'est encore la relation 851 de LOCKS, nœud 16249 : **l'arête est donc toujours en base**, et la
montre bute dessus à chaque rattrapage qui la touche. Depuis ce réveil, 27 fichiers sont restés en
erreur, chacun une fois, surtout des fichiers du moteur (transaction_manager.cpp, rel_table,
node_table, node_group, csr_node_group, chunked_node_group, storage_manager.cpp,
single_writer_crash_test.cpp, et backend, catalog, daemon_host.rs côté crate).
L'hypothèse de G : ce sont des fichiers dont les symboles prennent des verrous (LOCKS est la
relation « prend le verrou » du gabarit de code). Chaque drain qui réécrit leurs arêtes LOCKS
retombe sur l'arête 851 et défait tout le paquet. Ces fichiers restent donc en retard dans l'index
tant que l'arête est là. Un défaut durable plutôt qu'une course passagère.

## La garde

`RelTableData::update` (`src/storage/table/rel_table_data.cpp`, autour de la ligne 145). Une
relation se met à jour sens par sens. Si `findMatchingRow` ne la retrouve pas dans un sens, la mise
à jour est refusée (`RelTable::REL_NOT_FOUND_IN_ONE_DIRECTION`). C'est la garde du 4 octobre :
avant elle, un seul sens était modifié, en silence.

## Pistes, non vérifiées

1. **Les numéros de colonnes qui ne changent plus** (V2-1a, 4ae060070, dans la lib 04063ee06 du
   réveil). Si la table LOCKS a perdu ou gagné une propriété (une migration du gabarit),
   l'identifiant de relation lu dans un sens peut ne pas venir de la bonne colonne, selon que les
   deux sens comptent les trous de la même façon.
2. **Le passage au format 42 d'une base de format 41** à son premier point de reprise (18 h 32) :
   la réécriture d'un groupe CSR d'un seul sens.
3. **Le segment de colonne rebâti au point de reprise** (V2-1b, 6b63e4922) et le bloc neuf bâti
   hors du verrou (3724accba), dans la lib actuelle de l'hôte. Mais le défaut précède cette lib :
   il est vu dès 04063ee06.

## La lecture du 25 octobre, 21 h 30 (copies, jamais la base vivante)

**En trois lignes** :
1. **La contradiction est dans le fichier, et déjà avant la bascule.** Sur la copie de
   bascule-04063ee06 (format d'avant 41, jamais ouverte par 04063ee06), le sens avant depuis Scope
   16249 ne porte pas la relation 851, alors que le sens arrière depuis sa destination, Symbol
   13803, la porte (avec une autre source : Scope 12067).
2. **Ce n'est pas une lecture de travers par la lib courante** : la lib d'avant la bascule
   (58b46a9bf) et 04063ee06 lisent exactement la même chose sur cette même copie. Ni la migration
   41/42 ni V2-1a ne l'ont créée.
3. **Elle dure dans la base vivante** (copie à chaud de 21 h 26, sous 04063ee06). Le sens avant
   depuis 16249 n'a toujours que la relation 76 ; le sens arrière depuis 13803 porte 851 (venue de
   16249) et 852 (de 16250). Le compte de LOCKS est passé de 506 à 514. C'est la relation que la
   garde ne trouve pas dans le sens avant.

Le détail. Le lecteur est `~/.cache/rag3db-locks/lire.cpp`, lié à chaque lib du magasin, ouvert en
lecture seule ; les copies sont avant-58b4, avant-0406 et m-0406. LOCKS va de Scope à Symbol et
porte quatre propriétés (line, usage, usages, via), sans trou. La requête qui cherche 851 par son
identifiant laisse le planificateur choisir le sens : seules la première (ancrée sur la source) et
la troisième (ancrée sur la destination) disent un sens à coup sûr. La suite : lire les deux CSR
par les internes, d'où vient l'écart dans le sens avant (une relation supprimée d'un seul côté, ou
un identifiant réutilisé), et chercher sous quel lot il a pu s'écrire avant le 25 (les COPY de
relations annulés, le rollback du CSR).

## Le relevé complet du 25 octobre, 22 h 30 – 22 h 55 (copies, jamais la base vivante)

**Méthode** : pour chaque table de liens, chaque sens est lu nœud par nœud, par une requête ancrée
sur un bout (`WHERE offset(id(a)) = …` pour le sens avant depuis chaque source, `offset(id(b))` pour
le sens arrière depuis chaque destination). Les deux ensembles (relation, source, destination) sont
comparés : relation d'un seul côté, relation vue deux fois, bouts différents. Un balayage sans
ancre ne suffit pas : les deux formes `(a)-[r]->(b)` et `(b)<-[r]-(a)` rendent ici les mêmes
ensembles (le planificateur prend le même sens). Programme : `~/.cache/rag3db-locks/ancre.cpp`, lié
à la lib 04063ee06, lecture seule.

**Copie de la base vivante à 21 h 26** (`maintenant-21h26`, recopiée en reflink) :

| Table | Paire | Sens avant | Sens arrière | Écarts |
|---|---|---|---|---|
| LOCKS | Scope → Symbol | 429 | 514 | **729** |
| QUERIES | Scope → Symbol | 22 274 | 22 278 | **876** |
| MENTIONS, DEFINES, REGISTERS | Scope → Symbol | égaux | égaux | 0 |
| DEFINED_IN, CONSUMES, CONSUMED_BY, PARENT_OF, HAS_PARENT, IMPLEMENTS, INHERITS_FROM, USES_LIBRARY, les trois *_CHUNKED_FROM | | égaux | égaux | 0 |

La méthode est donc validée sur onze tables saines. Ce n'est pas la paire de nœuds : trois autres
tables Scope → Symbol sont saines.

**La forme des écarts** :

- **Des cases vides lues comme des lignes**, dans les deux sens : 79 sources de LOCKS lisent au sens
  avant « relation 0 → Symbol 0 », et des destinations lisent au sens arrière « relation 0 ← Scope
  0 » (jusqu'à cinq fois pour un même nœud). QUERIES en a 286 au sens arrière. Ce sont des
  longueurs de liste plus grandes que les données écrites.
- **De vraies relations absentes d'un sens** : 144 relations de LOCKS ne sont qu'au sens arrière,
  dont la 851 du drain et sa voisine 852.
- **Des relations rattachées au mauvais nœud** : 304 relations de QUERIES ont la même source dans
  les deux sens mais pas la même destination (la 10003 : Symbol 3344 au sens avant, 3389 au sens
  arrière). Ce sont des débuts de liste décalés entre nœuds voisins, sur des destinations très
  chargées.

**Antérieur à la bascule** : l'instantané `bascule-04063ee06` (copie, lu par la même méthode) donne
déjà 717 écarts sur LOCKS. Ni la migration 41 → 42 ni V2-1a. La bascule 43 n'est pas bloquée par ce
ticket (décision de l'orchestration).

**La réparation d'une demi-arête ne suffit pas.** L'outil `tools/repair_half_edge` (branche
`reparer-demi-arete`, 040daaecc) efface la 851 du seul sens qui la porte, sur une copie, puis
écrit un point de reprise. Relue : la 851 n'est plus dans aucun sens, et le compte passe de 514 à
513. Mais le drain retombe aussitôt sur la 852, et les lignes vides restent. **Décision de Lucie
(25 oct. au soir)** : pas de réparation partielle. G rebâtit la base du dépôt de code dans une base
neuve, et l'échange n'a lieu que si le relevé de la base neuve donne zéro écart. Sinon, c'est le
témoin du défaut moteur, et rien ne s'échange.

## Ce que la bascule 43 attend

**La bascule au format 43 (prévue le 26 octobre) attend la lecture du point 1 ci-dessous**
(décision de l'orchestration, 25 octobre au soir). Si la piste est la migration 41 → 42, ou les
trous de colonnes d'une table de liens migrée, on ne migre pas encore une base dont une migration a
peut-être déjà perdu un sens. La lecture se fait le 26 au matin, en premier.

## Pour avancer

1. Le relevé de la base neuve de G, par la même méthode : zéro écart, ou le défaut se reproduit
   sur une base fraîche (et on le tient).
2. Un témoin C++ rouge : réécrire en boucle les arêtes d'un nœud (effacer puis recréer, comme la
   montre à chaque drain) dans une table de liens clairsemée, avec des destinations chargées, sous
   lecteurs concurrents et points de reprise ; le relevé des deux sens doit rester égal.
3. La cause et un correctif générique : une liste qu'on réécrit garde ses bornes cohérentes des
   deux côtés, ou la réécriture est refusée.
4. Le CALL de réparation (la forme produit de `repair_half_edge`), et le relevé ancré versé à
   `tools/check_rel_directions`.
