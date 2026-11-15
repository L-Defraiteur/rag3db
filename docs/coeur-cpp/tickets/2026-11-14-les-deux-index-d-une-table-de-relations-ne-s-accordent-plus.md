# Les deux index d'une table de relations ne s'accordent plus

- **État** : ouvert (14 novembre 2026, hotfixs)
- **Famille** : moteur
- **Gravité** : donnée fausse. Une même relation rend d'autres bouts, ou une autre `line`, selon qu'on la lit
  depuis sa source ou depuis sa destination. Des cases à zéro (relation 0, nœud 0) se lisent comme de vraies
  relations.
- **Atteignable en service** : oui. code-lourd et code-principal à luciepc, dans l'état laissé par 0c41f9fe6.
- **Touche rag3weaver** : par le moteur. `usages` et `impact` lisent l'un ou l'autre sens, selon le plan.
- **Signalé par** : G (le contrôle des deux sens de la bascule vers 373b5477b), lu par hotfixs avec c7
- **Pour** : cœur c++. La réparation des deux bases est chez hotfixs (ticket de G
  `docs/optimiseur/tickets/2026-11-14-code-lourd-et-code-principal-divergent-entre-leurs-deux-rangements.md`).

## Ce qu'on voit

Une relation est rangée deux fois, dans le groupe CSR de sa source (sens avant) et dans celui de sa destination
(sens arrière). Sur les instantanés `avant-373b5477b` de code-lourd et de code-principal, trois tables par base
ne s'accordent plus. Le détail des comptes est dans le ticket de G. Ce que hotfixs a lu, relation par relation,
en comparant les deux sens par leur **contenu** (bouts en `_uuid`, propriétés) et non par l'identifiant :

- **Des cases à zéro.** Des lectures portent la relation 0 et partent du nœud 0, ou y arrivent. Le Scope
  d'offset 0 est un fichier `.md` qui ne porte aucune de ces relations : dans code-lourd `df461a4b…`
  (`09-highlighting-architecture.md`), dans code-principal `8e37fb11…`. Le sens arrière les lit comme
  source. Dans code-principal, le sens avant les lit aussi comme destination, sur IMPLEMENTS.
- **Des identifiants consécutifs décalés.** `29:357` à `29:360` (INHERITS_FROM, code-lourd) vont de `11:5359`,
  `11:5358`, `11:5357`, `11:5356` vers `11:5354` dans un sens, avec d'autres bouts dans l'autre. Dans LOCKS
  de code-lourd, la même source et la même `line` (`1689599c…`, ligne 56) vont vers `923f0281…` dans un sens et
  vers `005a5807…` dans l'autre. `005a5807…` est aussi la destination de la ligne 71 d'une autre source.
  Une colonne d'un des deux index est décalée d'un cran par rapport à ses voisines.
- **Le sens avant est le juste, à ses cases à zéro près.** On juge une lecture plausible si ses deux bouts
  sont hors de l'offset 0, si sa ligne tombe dans les lignes de son Scope source, et si le nom de sa cible
  paraît dans le texte de la source. Les lectures plausibles s'ajoutent alors aux relations en accord, à
  une arête par paire. On retrouve **exactement** le nombre de contenus distincts du sens avant, dans les six
  tables : code-lourd 650, 20 093 et 544 ; code-principal 20 210, 328 et 425. Les lectures du sens arrière en
  désaccord sont presque toutes fausses (LOCKS de code-principal : 36 sur 36).

## Les comptes ancrés (G, 14 novembre 23 h, outil a34fe130a)

L'outil d'avant tirait ses bouts d'une seule lecture non ancrée, et ce que cette lecture perdait n'était relu par aucun
sens. Avec les bouts ancrés par tranches, LOCKS de code-principal (copie jetable de l'instantané `avant-373b5477b`)
donne : **avant 436, arrière 425, union 436, et 11 relations perdues d'un seul sens**, toutes présentes au sens avant
et absentes au sens arrière. L'outil d'avant n'en voyait aucune. Parmi elles, le cas d'hotfixs : `lockIndexForWrite`
→ `acquireLock`, ligne 1173 (`node_table.cpp:1173` à 284cc00c0), que le sens arrière attribue à `lockRowForWrite`.

## Un angle mort du relevé, et les comptes (hotfixs, 14 novembre, 23 h 20)

**Le contrôle des deux sens juge la cohérence des deux rangements, pas leur complétude.** Le premier relevé,
celui de `check_rel_directions` et le mien qui le reprenait, tirait ses sources et ses destinations d'une seule
lecture non ancrée, `MATCH (a)-[r:T]->(b) RETURN label(a), offset(id(a))…`. Le planificateur joue cette
lecture dans un seul sens. Une source perdue par ce sens n'était jamais ancrée, et sa relation n'était pas relue,
même par l'autre sens qui l'avait encore.

Le cas prouvé : dans code-principal, LOCKS, `lockIndexForWrite` (Scope des lignes 1169 à 1177) prend
`acquireLock` à la ligne 1173. C'est vrai dans le code (`src/storage/table/node_table.cpp:1173` à 284cc00c0),
et une requête ancrée sur ce Scope la rend. Pourtant, cette relation manquait aux deux relevés. Le sens arrière
attribue sa ligne 1173 à `lockRowForWrite`, et une première reconstruction l'avait perdue, alors même que le
contrôle disait « égales ».

**Le relevé corrigé** (`releve_sens2`) parcourt les bouts par tranches de 500 offsets, chacune ancrée sur son
étiquette (`CALL show_connection`). Le plan prouve le sens :
- `MATCH (a:Scope) WHERE offset(id(a)) in [o, o+500) MATCH (a)-[r]->(b)` donne `SCAN_NODE_TABLE Scope` puis
  `SCAN_REL_TABLE`, `Direction: (a)-[r]->(b)` ;
- la même tranche ancrée sur `Symbol` donne `Direction: (b)<-[r]-(a)`.

G porte ce correctif, avec les quatre comptes, dans `tools/check_rel_directions` (ticket 2329a532f).

**Les quatre comptes par table**, lus par le relevé ancré : lectures au sens avant / au sens arrière / union
des identifiants / identifiants d'un seul sens. Avant, sur l'instantané `avant-373b5477b`. Après, sur la copie
reconstruite, avec les arêtes en accord plus les lectures plausibles, une arête par paire :

| base | table | avant | après |
|---|---|---|---|
| code-principal | QUERIES | 20 210 / 20 210 / 20 210 / 7 | 20 210 / 20 210 / 20 210 / 0 |
| code-principal | IMPLEMENTS | 334 / 334 / 334 / 6 | 331 / 331 / 331 / 0 |
| code-principal | LOCKS | 436 / 436 / 436 / 11 | 436 / 436 / 436 / 0 |
| code-lourd | INHERITS_FROM | 658 / 658 / 658 / 9 | 658 / 658 / 658 / 0 |
| code-lourd | QUERIES | 20 093 / 20 093 / 20 093 / 5 | 20 093 / 20 093 / 20 093 / 0 |
| code-lourd | LOCKS | 546 / 546 / 546 / 2 | 546 / 546 / 546 / 0 |

Ancré, chaque sens lit autant de relations que l'autre : la lecture non ancrée de G en voyait 650/658, 331/334,
425/436. Les écarts sont des identifiants rattachés à des bouts différents, pas des relations en moins.
IMPLEMENTS perd 3 relations à la reconstruction : ce sont trois copies exactes d'arêtes déjà en accord, une par
paire suffit.

Sur les deux bases du portable (memoire-longue, code-rag3db-depot), le relevé ancré trouve toutes les tables
égales et complètes.

`usages` et `impact`, par le binaire installé, avant et après sur 8 noms touchés par base, ne montrent que des
corrections que le code confirme :
- les relations fantômes depuis les `.md` d'offset 0 disparaissent ;
- les lignes deviennent celles des `impl` (`WorkingTree` à 257, `Snapshot` à 347, `MockDecider` à 182,
  `LlamaServerDecider` à 293, à 284cc00c0) ;
- les héritages mal rattachés tombent (`LocalDuckDBConnector` → `RecordBase`, `Fixe` sur une ligne de
  commentaire) ;
- `acquireLock` garde `lockIndexForWrite`.

Les deux réparations sont prêtes pour la fenêtre. Les outils sont à luciepc, dans
`~/.cache/rag3db-locks/` : `chaine-complete.sh`, `reconstruire.sh`, `releve_sens2`, et les scripts de jugement.

## Les deux bases réparées (fenêtre de luciepc, 15 novembre 2026, 0 h 27 – 1 h 12)

hotfixs, avec G (binaire fac67a208 posé à 0 h 26, avec la fermeture qui tient). Une seule fermeture et un seul
instantané par base pour deux gestes : la réparation des deux index, puis la purge des orphelins du plein texte.
Pilote `fenetre-luciepc.sh` (outils de la page des sondes de hotfixs). Pour chaque base, dans l'ordre :
1. le banc de 10 requêtes par l'hôte ;
2. `POST /backend/close` avec raison et `until` (fermeture qui tient) ;
3. l'instantané `avant-fenetre-<base>-<date>` par `instantanes.sh` de G ;
4. la réparation : relevé ancré, jugement, `DROP` / `CREATE` / `COPY`, contrôle des deux sens, relevé ancré
   après ; s'il reste un identifiant d'un seul sens, tout s'arrête avant la purge ;
5. la purge et le compact ;
6. `POST /backend/open` ;
7. le banc après, normal et large, comparé au banc d'avant.

| base | fermée | table | arêtes reposées (accord + plausibles) | lectures fausses écartées (dont cases à zéro) | après : avant / arrière / d'un seul sens |
|---|---|---|---|---|---|
| code-lourd | 0 h 27 m 57 s → 0 h 51 m 35 s | INHERITS_FROM | 658 (636 + 22) | 22 (9) | 658 / 658 / 0 |
| | | QUERIES | 20 093 (20 080 + 13) | 13 (5) | 20 093 / 20 093 / 0 |
| | | LOCKS | 546 (541 + 5) | 5 (3) | 546 / 546 / 0 |
| code-principal | 0 h 51 m 40 s → 1 h 11 m 41 s | QUERIES | 20 210 (20 203 + 7) | 7 (7) | 20 210 / 20 210 / 0 |
| | | IMPLEMENTS | 331 (315 + 16) | 19 (6) | 331 / 331 / 0 |
| | | LOCKS | 436 (400 + 36) | 36 (11) | 436 / 436 / 0 |

Le contrôle des deux sens de G est égal sur les trois tables de chaque base. **Le banc par l'hôte** ne perd
aucun résultat :
- code-lourd : 10 requêtes sur 10 identiques ;
- code-principal : deux requêtes voient chacune un résultat sortir du top et réapparaître au 24ᵉ et au 54ᵉ rang
  du banc large. C'est le déplacement BM25 de la purge (`acquireLock` 47 → 47, `FileSource` 21 → 21).

Retour : `~/.local/share/rag3weaver/instantanes/avant-fenetre-code-lourd-20261115-0027/RETOUR.sh` et
`…/avant-fenetre-code-principal-20261115-0051/RETOUR.sh`, à luciepc.

Un faux départ à 0 h 27 : à luciepc, `instantanes.sh` n'a pas le droit d'exécution, et le pilote s'est arrêté
au refus de l'instantané. Rien n'avait été copié ni écrit. code-lourd a été rouverte aussitôt ; le pilote appelle
maintenant le script par `bash`.

## La piste (cœur, relayée par c7)

Sous 0c41f9fe6, l'annulation d'un COMMIT refusé tronquait le groupe CSR sens par sens
(`InMemoryVersionRecordHandler::rollbackInsert`). Quand un autre écrivain avait versé dans un sens et pas dans
l'autre, les deux index perdaient des lignes différentes. Cela donne exactement des cases à zéro et des
sources décalées d'un cran. Le correctif (a) de 373b5477b vise cette annulation.

Les deux bases ont été écrites en dernier le 14 novembre à 14 h 25, sous 0c41f9fe6.

**Ce qui reste à départager** : depuis quand l'écart existe-t-il ? Le contrôle d'un instantané plus ancien
(`avant-0c41f9fe6`, 13 nov. 20 h 36) le dira. Et le journal de l'hôte de luciepc a-t-il connu des refus du
pool ou des COMMIT refusés sous plusieurs écrivains ? `journalctl --user -u rag3weaver-daemon` depuis le
1ᵉʳ novembre ne rend rien sur « buffer pool », « pool is full » ou « rollback ».

## Ce qu'il faut pour le fermer

1. Un témoin rouge sous 0c41f9fe6 : deux écrivains de relations sur une même table, un COMMIT refusé qui
   s'annule, puis le contrôle des deux sens (`tools/check_rel_directions`) rend un écart.
2. Le même témoin, vert sous 373b5477b.
3. Les deux bases réparées dans la fenêtre d'hotfixs, puis le contrôle des deux sens égal sur les 22 tables.

## Le cœur : la fenêtre de l'annulation perd des relations validées (14 nov., 23 h 35)

**Le témoin** : `RefusedCommitRelDirections.BothDirectionsKeepTheRelationsOfAnotherWriter`
(`test/transaction/refused_commit_rel_directions_test.cpp`, branche `deux-sens-sous-annulation`,
`7c92562b1`, pas sur master).

- Le COMMIT de A (5 000 relations) est refusé : le journal ne se synchronise pas
  (`CutFileSystem`).
- B valide une relation pendant que A se déroule.
- Chaque relation de B est lue par sa source (sens avant) et par sa destination (sens arrière).
- Les bouts sont dans des groupes de nœuds différents : A va du groupe 0 au groupe 2, B du groupe
  0 au groupe 1.

| Base | Verdict |
|---|---|
| `f41c0ba5b` (avant le correctif de l'annulation, `373b5477b`) | rouge 2/2 : à chaque tour, la relation validée de B est perdue, par la source **et** par la destination (0 et 0) |
| `7c92562b1` (master, avec le correctif) | vert 3/3 |

Ce que cela établit : sous une lib d'avant `373b5477b`, la fenêtre de l'annulation (un COMMIT
refusé, annulé hors du verrou, qui tronquait les groupes) **perd des relations validées**.
code-lourd et code-principal ont tourné sous de telles libs.

**Ce que cela n'établit pas encore** : la forme d'**un seul côté**, ni les cases « vues deux
fois ». Le témoin perd la relation des deux côtés, alors que code-lourd la garde d'un côté. Il
faut une troncature qui ne touche qu'un sens : par exemple un refus entre le versement d'un sens
et celui de l'autre, ou une annulation dont un sens a déjà été réécrit par un point de reprise. Ce
sera le prochain témoin. *(Le 15 nov., ce second témoin a établi la cause : voir la section
suivante.)*

**Pour la réparation** : la garde de `373b5477b` empêche la suite, mais ne répare pas ce qui est
déjà écrit. La reconstruction des trois tables sur copie (hotfixs) reste le remède.

## La forme d'un seul côté, reproduite (15 nov., 0 h 30)

**Cause établie**, par le témoin
`RefusedCommitRelDirections.ACommitRefusedBetweenItsDirectionsLeavesBothDirectionsWhole` : rouge
2/2 sur la lib d'avant `373b5477b`, avec la forme exacte de code-lourd, vert 3/3 sur master. Les
cases « vues deux fois » relèvent de la même troncature, mais ne sont pas reproduites une à une.

**Le point de faute** : `rel-commit-between-directions` (branche `deux-sens-sous-annulation`,
`394f2a27d` et `7900e1b7b`). Il fait lever `RelTable::commit` une seule fois, après le versement
du premier sens et avant le second, comme un tampon plein au second sens.

- Il s'arme par l'environnement (`RAG3DB_TEST_HOOKS=1`, `RAG3DB_TEST_FAIL_AT=…`), comme les
  points de mort, ou par programme (`TestHooks::armFailPoint`).
- Il peut attendre un délai sous le verrou avant de lever, pour qu'un autre COMMIT soit en file.
- Il n'a aucun effet en service.

**Le témoin** : `RefusedCommitRelDirections.ACommitRefusedBetweenItsDirectionsLeavesBothDirectionsWhole`
(`6fddb1f5c`).

- A verse 5 000 relations du groupe de nœuds 0 vers le groupe 2. Son COMMIT lève entre les deux
  sens, après 200 ms sous le verrou.
- B, en file sur ce verrou, valide une relation du groupe 0 vers le groupe 1.

| Base | Verdict |
|---|---|
| `f41c0ba5b` + le crochet (avant `373b5477b`) | rouge 2/2 : à presque chaque tour, la relation de B est **perdue par sa source (0) et lisible par sa destination (1)**, et rien de A ne reste |
| `6fddb1f5c` (master + le crochet + les deux témoins) | vert 3/3, les deux cas |

**La cause, lue et prise au témoin.** Sous une lib d'avant `373b5477b` :

1. un COMMIT de relations lève entre ses deux sens, par exemple sous un tampon plein ;
2. son enregistrement d'annulation ne couvre que le sens avant, déjà versé ;
3. le verrou est rendu, et un autre écrivain valide ses relations à la suite, dans le sens avant
   comme dans le sens arrière ;
4. l'annulation, hors du verrou, tronque le sens avant à la première ligne refusée, et emporte
   la relation de l'autre écrivain dans ce sens seulement ;
5. le sens arrière n'a rien à tronquer, et garde cette relation : **une relation d'un seul côté**.

Les cases « vues deux fois » et les sources décalées sont la même troncature lue sur le CSR, où
des décalages de lignes réemployées pointent sur d'autres relations. Elles ne sont pas encore
reproduites une à une.

**Le correctif est sur master depuis `373b5477b`** : l'annulation d'un COMMIT refusé sous le même
verrou, et la garde dans les annulations d'insertion. Le témoin vert sur master le montre. Les
bases déjà écrites restent à reconstruire : c'est le travail de hotfixs sur copie.

**Pour G** : le même point de faute fabrique à coup sûr une base où une relation n'est lisible que
d'un sens. C'est le crochet d'injection que demande son ticket de rangements.

## La base témoin « un seul sens » (15 nov., 1 h 30)

Fabriquée pour le témoin `test_check_rel_directions.sh` de G : à luciepc,
`~/.cache/rag3weaver-build/bases-temoins/un-seul-sens/` (base.kz et ATTENDU.txt). Elle a été écrite
par `f41c0ba5b` avec le point de faute, puis fermée après un CHECKPOINT. L'outil de G, sous
`373b5477b`, et le cœur, sous la lib d'écriture par des requêtes ancrées, lisent la même chose :
avant 10, arrière 12, union 12, perdues d'un sens 2 (1:60 et 1:111).

Deux observations sous l'ancienne lib, à garder :

- **La troncature remonte au-delà de la fenêtre.** 1:60, validée au tour 0, était lisible des deux
  sens juste après. Le tour 1 l'a emportée du sens avant, alors qu'elle était versée **avant** les
  lignes de A de ce tour. Le correctif de `373b5477b` la couvre aussi : il n'y a plus de troncature
  hors du verrou.
- **Un balayage non ancré qui lit mal.** Sous `f41c0ba5b`, `MATCH ()-[r:R]->() RETURN r.line
  ORDER BY r.line` ne rend qu'une ligne (line 0) sur cette base. Ce n'est pas expliqué. À rejouer
  sous `373b5477b` ; si le défaut y reste, il faut lui ouvrir un ticket à part.

## Le miroir n'est pas obtenu : une perte des deux côtés reste à lire (15 nov., 1 h 30)

On a voulu fabriquer pour G la base miroir de « un seul sens » : une relation présente au sens
avant et absente du sens arrière. Sous `f41c0ba5b`, avec un refus en fin de COMMIT (le journal ne
se synchronise pas, `CutFileSystem`, 200 ms sous le verrou) :

- A verse ses relations du groupe de nœuds 2 vers le groupe 1 ;
- B valide une relation du groupe 0 vers le groupe 1.

**10 tours sur 10, la relation de B est perdue des deux côtés.** Au sens avant, elle était seule
dans son groupe CSR (le groupe 0), et elle y disparaît quand même. La règle « la troncature
emporte, dans chaque sens, le groupe que A partage avec B » est donc fausse pour un refus en fin de
COMMIT. Elle ne vaut que pour le refus entre les deux sens.

Le premier témoin (`BothDirectionsKeepTheRelationsOfAnotherWriter`) montre la même perte des deux
côtés. Le mécanisme reste à lire. Les suspects :

- `rollbackGroupCollectionInsert`, qui fait reculer le compte de lignes de toute la table ;
- le décalage des relations, `updateRelOffsets`, que le COMMIT de A a déjà avancé, et que B
  réemploie ou chevauche.

Rien n'a été déposé, et le dossier `un-seul-sens-avant` est effacé. Le journal de la fabrication
est à luciepc, dans `~/.cache/rag3db-moteur-notes/deux-sens/miroir-echec/`. Sur master, le
correctif de `373b5477b` couvre aussi ce cas : le témoin est vert 3/3.

## La perte des deux côtés, localisée (15 nov., 6 h 30)

Une sonde hors du dépôt (`sonde_perte_deux_cotes_test.cpp`, à luciepc dans
`~/.cache/rag3db-moteur-notes/deux-sens/`) rejoue, sous `f41c0ba5b` puis sous master `e8472e053`,
le refus de A en fin de COMMIT (le journal ne se synchronise pas, 200 ms sous le verrou), 4 tours
par cas. Les groupes de nœuds font 2 048 nœuds.

| Cas | Ce que B écrit pendant la fenêtre | `f41c0ba5b` | master |
|---|---|---|---|
| G1 | une relation de R, avec un groupe partagé avec A au sens arrière | perdue des deux côtés, 3/4 | lisible des deux côtés |
| G2 | une relation de R, sans **aucun** groupe partagé avec A | perdue des deux côtés, 4/4 | lisible |
| G3 | comme G1, mais B valide **après** l'annulation de A | lisible, 4/4 | lisible |
| P1 | une relation d'une **autre** table de relations (R2) | lisible, 4/4 | — |
| P2 | un nœud (M) | lisible, 4/4 | — |

Ce que la sonde établit :

- la perte des deux côtés tient à la **fenêtre** (G3) ;
- elle ne tient **pas aux groupes de nœuds** partagés (G2) ;
- elle est propre à **la table de relations que A et B partagent** (P1, P2).

Ce n'est donc pas la troncature d'un groupe CSR, mais un état commun à la table, que l'annulation
hors du verrou de A défait après le COMMIT de B. Les suspects, lus mais pas prouvés :

- `reserveRelOffsets` / `nextRelOffset`, les identifiants des relations ;
- `rollbackGroupCollectionInsert`, avec le compte de lignes de la collection et `removeTrailingGroups`.

**Cause localisée, pas expliquée.** Sur master, l'annulation sous le verrou couvre ce cas (vert
partout). Le témoin `BothDirectionsKeepTheRelationsOfAnotherWriter` le garde. Il reste à
instrumenter l'ancienne lib, si l'on veut le mécanisme exact.

**Statut de la perte des deux côtés** (c7, 15 nov.) : localisé, non expliqué, couvert par `373b5477b`, gardé par `BothDirectionsKeepTheRelationsOfAnotherWriter`. Le mécanisme exact, dans une lib qu'on ne livre plus, ne se cherche pas.
