# (A) — Le CREATE_VECTOR_INDEX concurrent, comme CREATE INDEX CONCURRENTLY

Page de conception, avant le code. Écrite le 18 octobre 2026 au soir par la seconde session cœur
C++ (hotfixs), sur la décision de l'orchestration et l'avis du cœur C++. Elle est à relire par
le cœur C++ et l'orchestration avant toute ligne de code.

## 1. Le but, et ce qui l'impose

Le but : bâtir un index vectoriel sur une table que vingt écrivains modifient, sans perdre de
ligne et **sans retenir personne**. Trois règles de Lucie l'imposent :
- **faire comme PostgreSQL** ;
- **aucun bloqué** ;
- **« un index ne se pense pas »**. Personne n'écrit de CREATE INDEX à la main : c'est le
  produit (rag3weaver) qui bâtit et rebâtit ses index. Ce qui suit ne se voit donc que de lui.

## 2. Pourquoi (δ) ne suffit pas

(δ), le lot de trois commits du 18 octobre, est juste : le CREATE rattrape, à sa validation, ce
que les autres ont validé pendant son bâti. Mais il le fait une fois les autres partis, donc
pendant que les BEGIN attendent. Le scénario des vingt écrivains (`TwentyWriters`, 2 fils de
test) l'a mesuré :

| | durée du CREATE | retenue max des autres |
|---|---|---|
| master 8f4779bdc (perd des lignes du graphe) | 11,2 s | 7,8 s (la famine du bâti, pas un verrou) |
| (δ) | 126 s | 115,7 s |
| (δ′) : passes avant la validation | refusé au bout de 86 s (tampon plein) | 9,6 s |

(δ′) a montré la cause de fond. Un seul fil, celui du CREATE, ne rattrape jamais l'entretien HNSW
de ce que vingt écrivains changent en parallèle : les passes ne convergent pas. Et les arêtes
écrites par la transaction du CREATE s'accumulent dans son stockage local jusqu'à remplir le
tampon. Dans tous les moteurs, la seule forme qui tienne fait payer l'entretien à **chaque
écrivain**, à son coût et en parallèle, une fois l'index connu de lui.

## 3. La forme

Il y a trois transactions, et quatre états de l'index, plus INCOMPLET après un arrêt brutal
(§3 bis, point 7) :

```
            T_A (courte)      T_B (le bâti, à T1)          T_C (le rattrapage)
  absent ──────────────▶ NOTÉ ───────────────────▶ ENTRETENU ─────────────────▶ PRÊT
                         les écrivains notent        les écrivains entre-        les recherches
                         les lignes qu'ils           tiennent le graphe,         le lisent
                         touchent, sans toucher      T_C reprend les lignes
                         au graphe                   notées
```

**État NOTÉ (T_A).** Une transaction courte valide, à TS0 :
- l'entrée d'index au catalogue, avec son état ;
- les deux tables de relations, vides ;
- le holder au stockage, dans l'état NOTÉ.

Dès lors, tout écrivain qui voit l'entrée (startTS ≥ TS0) **note** l'offset de chaque ligne qu'il
insère, supprime ou dont il change le vecteur. Il le fait dans l'ensemble « lignes touchées » du
holder, sous le verrou d'index que l'écrivain d'une table indexée prend déjà (marche I1), et ne
touche pas au graphe. Le coût est d'une insertion dans un ensemble par ligne écrite.

**L'attente.** Le bâtisseur attend qu'aucune transaction active n'ait startTS < TS0 : celles-là ne
voient pas l'index, et ne noteraient rien. C'est l'attente de la plus longue transaction ouverte.
Ce n'est ni un verrou ni un point de reprise, et personne n'est retenu. Elle est bornée par un
délai, au-delà duquel le CREATE échoue en le disant.

**État ENTRETENU (T_B).** Le bâtisseur prend son instantané T1 après l'attente. Il bâtit le graphe
en mémoire à T1, comme aujourd'hui, puis l'écrit dans les tables de relations par l'insertion
par lots. Il n'y a pas de stockage local : les tables sont validées depuis T_A, et aucun
écrivain n'y écrit pendant ce temps, puisqu'ils ne font que noter.

T_B se valide avec son point de reprise forcé (la durabilité du graphe, comme maintenant), et le
holder passe à l'état ENTRETENU. Tout écrivain qui écrit ensuite entretient le graphe comme un
index ordinaire (commitInsert, update, delete_), chacun à son coût. L'état se lit au moment de
l'écriture, sous le verrou d'index, et non à l'instantané de l'écrivain.

Il n'y a pas de fenêtre perdue :
- toute ligne validée avant T1 est dans le bâti ;
- toute ligne écrite après TS0 et avant le passage à ENTRETENU est notée (l'attente garantit que
  ses écrivains voyaient l'index) ;
- toute ligne écrite après le passage à ENTRETENU est entretenue par son écrivain.

C'est le point du cœur : nos instantanés sont des horodatages stricts, donc la troisième passe de
PostgreSQL (`validate_index`) se réduit à reprendre les lignes notées.

**État PRÊT (T_C).** Le rattrapage reprend chaque ligne notée, par lots, chacun sous le verrou
d'index. Il relit la ligne au dernier état validé et la remet à sa juste place : retirée si elle
est absente ou à NULL, insérée si elle manque, reprise si son vecteur a changé. L'opération est
idempotente : une ligne notée deux fois, ou déjà entretenue par un écrivain après le passage à
ENTRETENU, ne coûte qu'une relecture.

Les lots rendent le verrou entre eux, et les écrivains passent. L'ensemble est borné par ce qui a
été touché entre TS0 et le passage à ENTRETENU : il ne grossit plus ensuite, puisque les écrivains
n'y notent plus rien. Les notes d'une instruction commencée à l'état NOTÉ et finie après le
passage sont reprises comme les autres. L'ensemble vidé, T_C passe l'index à PRÊT.

## 3 bis. Les précisions de la relecture (cœur C++, 19 octobre)

Elles priment sur le §3 là où elles le précisent.

1. **Un écrivain décide à sa validation, pas à son instruction.** Les décalages des lignes
   insérées ne sont connus qu'au versement du stockage local (étape 3 de `NodeTable::commit`) ou
   à la finalisation d'un COPY. C'est donc là, sous le verrou d'index, que l'écrivain lit l'état
   du holder et agit : NOTÉ → il note ; ENTRETENU ou PRÊT → il entretient. Le cas d'une
   instruction commencée en NOTÉ et finie après le passage à ENTRETENU disparaît.
2. **L'attente après T_A ne concerne que les transactions d'écriture** ouvertes avec
   startTS < TS0. Un lecteur long n'écrit rien, et ne retarde pas le bâtisseur.
3. **Le point de reprise forcé de T_B retient tout le monde le temps de s'écrire.** Comme toute
   validation forcée, il arrête les BEGIN et attend les autres. C'est la retenue qui reste avec
   (A), de l'ordre de l'écriture du graphe (quelques secondes ; la mesure est au témoin 2).
   Deux sorties : écrire le graphe par le chemin journalisé quand il tient sous
   `copy_journal_threshold`, et alors il n'y a pas de point de reprise forcé ; ou la voie 2 du
   point de reprise, qui ne retient plus personne.
4. **T_C avance au rythme des validations des écrivains**, tant qu'E11 tient : sous I1,
   l'écrivain d'une table indexée garde le verrou d'index en exclusif jusqu'à sa validation, et
   chaque lot de T_C attend la fin d'une transaction entière. Le rattrapage reste borné, mais
   il est lent sous vingt écrivains. Il se mesure dans `TwentyWriters`, et ce coût tombe avec
   I2.
5. **Le COPY note aussi.** Sa finalisation (`NodeBatchInsert::finalize`, qui filtre la
   visibilité depuis le correctif du 18 octobre) lit l'état du holder : elle note ses lignes en
   NOTÉ, et les entretient en ENTRETENU.
6. **Une suppression d'une ligne absente du graphe se note aussi en ENTRETENU.** T_C relit le
   dernier état validé : une ligne absente n'est pas réinsérée depuis l'instantané du bâti.
   C'est la règle du §3, à garder dans un témoin.
7. **Un état à part entière : INCOMPLET.** À la réouverture après un arrêt brutal, un holder
   ENTRETENU a perdu ses notes, qui étaient en mémoire. Il passe à INCOMPLET : refusé aux
   recherches par un nom, dit au journal, et rebâti par le produit. Un holder NOTÉ est retiré,
   comme le dit le §4.

## 4. Les pièces

- **Le catalogue.** L'entrée d'index porte son état (NOTÉ, ENTRETENU, PRÊT), versionné comme
  toute entrée : un changement d'état est une mise à jour validée. Une recherche refuse un index
  qui n'est pas PRÊT, par un nom (« index under construction »). Le produit sait alors répondre
  sans lui, comme aujourd'hui pendant un rebâti.
- **Le holder.** Il porte le même état, l'ensemble des lignes notées (sous un mutex court) et
  l'index lui-même dès ENTRETENU. Un holder NOTÉ n'a pas encore d'index chargé : `isWritableIndex`
  le traite à part (« note, n'écris pas »).
- **Ce que voit une lecture pendant la construction.** Les recherches refusent un index qui n'est
  pas PRÊT. Les balayages ordinaires de la table ne voient rien de changé.
- **delete_ et update sur un nœud absent du graphe.** À l'état ENTRETENU, la ligne d'un écrivain
  peut ne pas être encore dans le graphe : notée, et pas encore reprise. `deleteFromGraph` et
  `update` doivent tolérer un nœud sans arêtes. update l'insère, et le rattrapage le relira
  (sans dommage).
- **L'annulation.**
  - T_A annulée : rien n'existe.
  - T_B annulée, ou le bâti en échec : l'index reste NOTÉ, et le CREATE échoue en le disant. Un
    DROP le retire. Le produit refait le CREATE.
  - T_C interrompue : l'index reste ENTRETENU, avec ses lignes notées. Un second CREATE, ou la
    reprise après réouverture, termine le rattrapage.
- **La reprise après un arrêt brutal.** À la réouverture, un holder NOTÉ est retiré, et le journal
  le dit par un nom : son graphe n'existait pas. Un holder ENTRETENU garde son graphe, durable par
  le point de reprise de T_B. Son ensemble de notes, en mémoire, est perdu : il passe à
  INCOMPLET (§3 bis, point 7), et le produit le rebâtit (« un index ne se pense pas »). La garder
  persistante serait une autre étape, à mesurer avant.
- **DROP pendant la construction.** Il est permis à tout état. Le bâtisseur le voit à son
  prochain lot et s'arrête.
- **La famine du bâti** (ticket du 18 octobre, `IndexCreationThreads.LeavesAThreadToReaders`).
  Le bâti de T_B laisse au moins un fil aux autres connexions, et ses tâches cèdent entre deux
  morceaux. C'est ici qu'elle se règle, puisque le bâti tourne pendant que les autres écrivent.
- **Le reste à part.** E11 (les écrivains un par un sous le verrou INDEX) reste un lot à part, et
  n'empêche pas cette forme.

## 5. Les témoins

Ils sont à écrire avant le code, et à jouer rouges sur master :
1. Les témoins de (δ), gardés : `RowsCommittedDuringACreateAreIndexed`,
   `ChangesCommittedDuringACreateAreSeen` (sur disque et en mémoire), `AWriterOpenAcrossTheCreate…`,
   `AForcedCopyCommittedRightAfterACreateIsReached`.
2. Un neuf : sous vingt écrivains, aucune transaction n'attend plus d'une borne écrite pendant le
   CREATE. Il se joue à 8 fils (le réglage du scénario) et à 2. La retenue du point de reprise
   forcé de T_B se mesure à part, pour qu'on voie ce que (A) laisse et ce que la voie 2 du point
   de reprise enlèvera.
3. Un neuf : une recherche pendant la construction est refusée par le nom de l'état.
4. Des neufs pour l'arrêt brutal à chaque état (NOTÉ, ENTRETENU, pendant T_C), et pour le DROP à
   chaque état.
5. `TwentyWriters`, côte à côte avec master et (δ), avec les mêmes réglages de fils.

## 6. L'ordre

1. Relecture de cette page par le cœur C++ et l'orchestration.
2. Les témoins, rouges.
3. Le code, en lots :
   - l'état au catalogue et au holder, et le refus des recherches ;
   - les notes des écrivains, et la tolérance du nœud absent ;
   - T_B avec le passage à ENTRETENU ;
   - T_C ;
   - la reprise et le DROP ;
   - la famine du bâti.
4. (δ) passe sur master avant, juste, avec sa retenue écrite au ticket comme défaut connu.
