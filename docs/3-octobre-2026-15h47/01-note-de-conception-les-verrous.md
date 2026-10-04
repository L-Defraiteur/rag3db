# Note de conception — les verrous des écritures parallèles

Session « cœur C++ », 3 octobre 2026. Suite du rapport
`docs/2-octobre-2026-00h17/01-ecritures-paralleles-vela-et-le-chemin.md`.

Décision de Lucie qui ouvre cette note : **le second écrivain attend au lieu
d'échouer, sur un verrou par clé**. Principe qui la cadre : **faire comme
PostgreSQL et Neo4j, sauf quand on trouve mieux** ; ne lui rendre que les
écarts.

Rien n'est codé. Rien ne se code sur A3 et A4 avant qu'elle ait lu la partie 1.

Étiquettes : **[doc]** vérifié dans la documentation citée en fin de note ;
**[lu]** lu dans notre code ; **[exécuté]** prouvé par un test ; **[déduit]**
raisonnement, à valider.

---

## 1. Ce qui attend la décision de Lucie : trois écarts

> **Tranché le 3 octobre 2026 au soir**, par l'orchestration sur délégation de Lucie —
> « tu sais pas trancher au mieux ? de toute façon tout ça peut être corrigé plus tard —
> toujours prendre le meilleur chemin possible et laisser la voie aux autres options » :
>
> 1. **Écart 1**, l'annonce en tête de transaction (`CALL acquire_locks`) : **oui**.
> 2. **Écart 2**, hors annonce : **option A** — on reste en instantané par transaction ;
>    après l'attente, si le premier a validé, une erreur nommée et transitoire. La voie
>    laissée : l'erreur porte un nom stable et un code distinct, pour qu'un passage à
>    l'instantané par instruction (option B) puisse un jour la faire disparaître sans
>    changer l'appelant ; rien dans V1 ni A4′ ne suppose qu'on ne rejouera jamais une
>    instruction seule.
> 3. **Écart 3**, créer une relation : **verrou partagé** sur les extrémités. La voie
>    laissée : le mode est un paramètre de la prise, pas une branche en dur.
>
> Le texte ci-dessous est celui qui a été soumis ; il reste pour ses raisons.

Tout le reste de la note suit le comportement établi et n'est pas une question.

### Écart 1 — Les verrous annoncés en tête de transaction (mieux que les deux)

PostgreSQL et Neo4j découvrent leurs verrous un par un, au fil des
instructions. C'est ce qui produit leurs deux désagréments : l'interblocage
(deux transactions prennent les mêmes verrous dans l'ordre inverse) et
l'échec après attente (celui qui a attendu découvre que ce qu'il avait lu est
périmé). Tous deux recommandent la même parade à leurs utilisateurs : prendre
ses verrous dans un ordre constant **[doc]**.

rag3weaver connaît ses clés avant d'écrire. Il peut donc faire ce que leurs
applications ne peuvent pas :

1. la transaction s'ouvre et **annonce** toutes ses clés ;
2. le moteur les trie et les prend dans cet ordre, en attendant s'il le faut ;
3. **l'instantané de la transaction est pris après**, quand elle tient tout.

Conséquences : pas d'interblocage possible entre deux transactions qui
annoncent (ordre unique) ; et celui qui a attendu **n'échoue jamais**, parce
qu'il lit l'état d'après l'attente. C'est exactement « attendre au lieu
d'échouer », sans réserve.

Forme proposée : une fonction appelée en première instruction de la
transaction, `CALL acquire_locks('Table', [clés])` (une par table, ou une
liste de couples). Les deux autres formes — un mot-clé dans `BEGIN`, une
instruction neuve — demandent de toucher la grammaire pour le même effet.

Coût : 1,5 jour. **Recommandation : oui.**

### Écart 2 — Ce que fait celui qui a attendu, hors annonce

Pour une transaction qui n'a rien annoncé (Cypher ordinaire), PostgreSQL a
**deux** comportements établis, selon le niveau d'isolation **[doc]** :

| | Read Committed (son défaut) | Repeatable Read |
|---|---|---|
| l'instantané | un par instruction | un par transaction |
| après l'attente, si le premier a validé | relit la ligne à jour, réévalue sa condition, continue | échoue : « could not serialize access due to concurrent update » |
| après l'attente, si le premier a annulé | continue | continue |

Notre moteur a **un instantané par transaction** [lu] : il est dans la
colonne de droite. Deux options :

- **Option A — rester où l'on est (Repeatable Read).** Le second attend ; si
  le premier valide, le second reçoit une erreur nommée, transitoire, et
  rejoue sa transaction ; si le premier annule, le second continue. Coût :
  celui des marches ci-dessous, rien de plus.
- **Option B — passer à un instantané par instruction (Read Committed).** Le
  second attend puis continue sur la ligne à jour. Il faut pour cela pouvoir
  annuler et rejouer **une instruction** à l'intérieur d'une transaction, ce
  que le moteur ne sait pas faire (son tampon d'annulation est par
  transaction). Coût : environ 5 jours de plus, et un changement de la
  sémantique de lecture de tout le moteur **[déduit]**.

**Recommandation : A.** Avec l'écart 1, rag3weaver n'échoue jamais après une
attente ; il ne reste donc à l'option A que le Cypher écrit à la main, où une
erreur transitoire à rejouer est le comportement que Neo4j lui-même impose à
ses pilotes. B se décidera si un besoin réel apparaît.

Ce choix n'est pas neutre pour ta phrase : sous A, hors annonce, « attendre »
peut encore finir par « échouer » — seulement quand le premier a réellement
validé un changement sur la même ligne.

### Écart 3 — Créer une relation : verrou partagé sur les extrémités

Les deux moteurs divergent, il faut donc choisir :

- Neo4j prend un verrou **d'écriture** sur la relation « and both its nodes »
  **[doc]** : deux créations de relations sur le même nœud s'attendent.
- PostgreSQL, pour une clé étrangère, prend sur la ligne référencée un verrou
  **partagé** (`FOR KEY SHARE`), qui ne bloque que la suppression de cette
  ligne et le changement de sa clé **[doc]**.

**Retenu : le partagé.** Un graphe de code a des nœuds-carrefours (un fichier,
un module) auxquels beaucoup de relations s'accrochent : le verrou exclusif de
Neo4j y mettrait tous les écrivains en file. Le partagé protège ce qu'il faut
— qu'on ne supprime pas le nœud sous la relation — et rien de plus. Je le
retiens sauf objection.

---

## 2. Ce qui suit le comportement établi (retenu, pas de question)

### Ce qui est verrouillé

| Opération | Verrou | Comme |
|---|---|---|
| insertion d'un nœud | exclusif sur la **clé primaire** (table, valeur) | PostgreSQL : l'insertion d'une clé en cours d'insertion ailleurs attend **[doc]** |
| mise à jour, suppression d'un nœud | exclusif sur la **ligne** | PostgreSQL (`FOR UPDATE` pris par tout `DELETE` et `UPDATE`), Neo4j (verrou d'écriture sur le nœud) **[doc]** |
| création, suppression d'une relation | exclusif sur la relation, **partagé** sur ses deux extrémités | écart 3 |
| mise à jour d'une relation | exclusif sur la relation | Neo4j **[doc]** |
| lecture | **rien** : un lecteur n'attend jamais | PostgreSQL : « Row-level locks do not affect data querying » **[doc]** |

**Grain : la ligne**, pas la colonne — voir §4.

**Durée : jusqu'au commit ou au rollback.** PostgreSQL : « released at
transaction end » ; Neo4j : « released when the transaction finishes »
**[doc]**.

### La clé en double après attente

Le second inséreur attend ; si le premier valide, il reçoit l'erreur de clé
en double ; si le premier annule, il insère. C'est PostgreSQL mot pour mot
**[doc]**.

Un point que le verrou seul ne règle pas : **le contrôle d'unicité doit
regarder le dernier état validé, pas l'instantané.** Aujourd'hui il regarde
l'instantané [lu : `node_table.cpp:399-400`, `isVisible(transaction, offset)`].
Une transaction dont l'instantané précède le commit d'une autre ne voit donc
pas sa clé, même sans aucun chevauchement des écritures. PostgreSQL, lui, va
lire l'état de validation de toute ligne qui porte la même clé (« reach into
the heap to check the commit status ») **[doc]** ; qu'il le fasse au-delà de
l'instantané de la transaction est ma lecture de cette page, elle ne le dit
pas en ces mots. Le cas est demandé au banc.

### L'interblocage

- **Détecté à la prise du verrou**, par le graphe d'attente : celui dont la
  demande fermerait un cycle reçoit tout de suite une erreur nommée et
  transitoire, et sa transaction est annulée. C'est Neo4j
  (`Neo.TransientError.Transaction.DeadlockDetected`) **[doc]**. PostgreSQL
  fait la même recherche, mais seulement après une seconde d'attente
  (`deadlock_timeout`), parce qu'elle lui coûte cher **[doc]** ; dans un seul
  processus, avec une table de verrous sous un seul mutex, elle ne nous coûte
  rien : on la fait tout de suite.
- **Un délai d'attente en filet**, réglable, avec sa propre erreur nommée
  (Neo4j : `db.lock.acquisition.timeout`) **[doc]**. Par défaut assez long
  pour ne jamais gêner (30 s).
- **L'attente regarde le drapeau d'interruption** de la connexion
  [lu : `client_context.h:97`]. Le banc a constaté qu'un `range()` géant ne le
  regarde pas ; une attente de verrou n'a pas le droit de faire pareil.

### La transaction en échec — un défaut d'aujourd'hui, à corriger d'abord

Trouvé par le banc, **[exécuté]** dans un seul fil : après une erreur dans
`BEGIN … COMMIT`, le moteur annule la transaction et **repasse en
auto-commit** ; les instructions suivantes sont validées une à une, et le
`ROLLBACK` final répond « No active transaction ».

PostgreSQL : la transaction passe dans un état d'échec où toute instruction
est refusée, et seul `ROLLBACK` (ou `COMMIT`, qui annule) en sort **[doc]**.

Chez nous cela se corrige dans `ClientContext::TransactionHelper::runFuncInTransaction`
(`src/main/client_context.cpp`) : le `catch (std::exception&)` appelle
`context.rollback()` quelle que soit la nature de la transaction [lu]. Pour
une transaction explicite, il faut annuler ses effets mais **la garder
ouverte, marquée en échec**, et refuser toute instruction par une erreur
nommée jusqu'au `ROLLBACK`.

**C'est un préalable aux verrous, pas un à-côté** : les verrous ajoutent des
erreurs en pleine transaction (interblocage, échec après attente). Un client
qui continuerait après l'une d'elles écrirait le reste de sa transaction en
auto-commit, par morceaux. Coût : 1 jour.

---

## 3. Où vit la table des verrous

Derrière une interface étroite, pour que la cible B (plusieurs processus)
change l'implémentation et pas les appelants :

```
LockManager
  acquire(transaction, clés triées, mode, échéance)  → ok | interblocage | délai | interrompu
  releaseAll(transaction)                            → au commit et au rollback
```

- **Aujourd'hui (cible A)** : une table en mémoire dans la `Database`, un
  mutex, une variable de condition par verrou attendu, le graphe d'attente
  dans la même table.
- **Pour B** [déduit, à valider] : la même interface, avec en plus un verrou
  entre processus par clé. Piste : des verrous `fcntl` par plage d'octets sur
  un fichier de verrous, la clé hachée donnant la plage — le noyau détecte
  lui-même les interblocages entre processus (`EDEADLK`) et libère tout à la
  mort d'un processus, ce qui règle d'un coup le cas du processus qui meurt en
  tenant un verrou. Ces verrous sont par processus, pas par fil : la table en
  mémoire reste nécessaire entre les fils d'un même processus.

**L'annonce en tête (écart 1) devient, pour B, la forme naturelle** : un
processus prend d'un coup, dans l'ordre, tout ce qu'il va écrire, puis
rattrape le journal des autres, puis écrit.

**La validation au commit ne reste que comme filet.** Avec des verrous tenus
jusqu'au commit et des contrôles faits contre le dernier état validé une fois
le verrou obtenu, il n'y a plus rien à valider au commit. Le filet est le
vérificateur du banc, pas du code de production. A2 (le remappage des offsets
au commit) n'est pas concerné : il reste tel quel.

### Ajout du 4 octobre 2026 — un cycle que le graphe d'attente ne voit pas, et pourquoi on ne le construit pas

Depuis `68ff9d5e2`, la validation d'un `COPY` (dont la durabilité est son propre point de
reprise) **attend le départ des autres transactions avant de valider**. Si T1 tient un verrou
de ligne et valide un `COPY`, et que T2 attend ce verrou, T2 ne peut pas partir et T1
n'avancera pas : un interblocage que le graphe d'attente ne voit pas, tant que cette attente
se fait hors du gestionnaire.

La règle est celle de PostgreSQL : **toute attente qui peut entrer dans un cycle passe par le
gestionnaire de verrous**, sinon la détection est aveugle. La réponse envisagée était un
troisième genre de ressource, « la base ». **Il n'est pas construit** (décision de Lucie, le
4 octobre au soir, sur la recommandation de l'orchestration) : le `COPY` hors journal est la
cause commune de cette attente, du point de reprise forcé et des défauts de l'annulation
corrigés ce jour-là ; le chargement en masse journalisé passe donc **avant** le câblage des
verrous, et l'attente disparaît avec lui. La ressource du gestionnaire reste générique : un
genre s'ajoute sans rien changer d'autre, si le besoin revenait.

Autre conséquence du 4 octobre, qui conforte « la table des verrous vit dans la `Database` » :
un processus ne peut plus ouvrir deux exemplaires écrivains de la même base (`57c8389b4`),
il n'y a donc bien qu'une table de verrous par base.

---

## 4. La même ligne, deux colonnes différentes

**Ce que le moteur fait aujourd'hui** [lu, non exécuté] : l'information de
mise à jour est tenue par colonne (`ColumnChunk::update`,
`column_chunk.cpp:179-195`, une `UpdateInfo` par colonne) et le conflit n'est
cherché que dans la chaîne de cette colonne (`update_info.cpp:17-41`). Deux
transactions qui modifient la même ligne dans deux colonnes différentes ne se
voient donc pas : les deux valident, sans « Write-write conflict », et la
ligne porte les deux modifications. Le banc peut le montrer par un cas
déterministe de dix lignes ; il est demandé (§6).

**Ce que ça devient avec le verrou de ligne** : la seconde attend. Ensuite,
option A : elle échoue si la première a validé ; avec l'annonce en tête : elle
passe et la ligne porte les deux modifications, comme aujourd'hui, mais dans
un ordre défini.

**Un verrou plus fin que la ligne ?** Non. PostgreSQL et Neo4j verrouillent
la ligne (le nœud) **[doc]**. Le seul gain d'un verrou par colonne serait de
ne pas faire attendre deux écrivains sur deux colonnes de la même ligne ; son
prix serait une entrée de verrou par colonne touchée, et surtout il laisserait
passer précisément le cas dangereux — deux colonnes qui dépendent l'une de
l'autre.

**Les colonnes dépendantes** (un contenu et son empreinte) : le moteur ne
sait pas qu'elles sont liées, et n'a pas à le savoir ; aujourd'hui c'est
l'appelant qui doit les écrire dans la même instruction. Avec le verrou de
ligne le problème disparaît de lui-même : deux écrivains de la même ligne ne
s'entrelacent plus, quelles que soient les colonnes.

---

## 5. Ce que ça change à l'ordre et aux chiffres

Le plan du 2 octobre comptait A3 (2 j) et A4 (3 j) sur la forme « valider au
commit, le second échoue ». Avec « le second attend » :

| Marche | Contenu | Coût | Témoin dans le banc |
|---|---|---|---|
| **T0** | la transaction en échec refuse tout jusqu'au `ROLLBACK` | 1 j | `MinimalReproduction.FailedTransactionRefusesFurtherStatements` |
| **A2** | remappage des offsets locaux au commit (inchangé) | 1 j | C3 |
| **V1** | le gestionnaire de verrous : table, modes partagé et exclusif, attente, interblocage à la prise, délai, interruption, erreurs nommées, libération | 3 j | cas neufs du §6 |
| **A3′** | insertion : verrou de clé, puis unicité contre le dernier état validé | 2 j | C1, et le cas « instantané antérieur » |
| **A4′** | relation : partagé sur les extrémités ; suppression de nœud : exclusif ; mise à jour et suppression de ligne : exclusif, erreur nommée après attente (remplace le « Write-write conflict » immédiat) | 3 j | C2, C6, le cas des deux colonnes |
| **V2** | l'annonce en tête de transaction (écart 1) | 1,5 j | cas neufs du §6 |
| **A5** | le chemin de suppression rendu sûr entre fils | 2 j | C5 sous ThreadSanitizer |
| A6, A7 | inchangées | 0,5 j, 2 à 3 j | |

De A2 à V2 : **11,5 jours** au lieu des 6 que comptaient A2, A3 et A4 —
**5,5 jours de plus**, qui sont le prix de « attendre » : le gestionnaire de
verrous, l'annonce en tête, et T0.

**A5 reste nécessaire.** Le verrou exclusif de ligne empêche deux
suppressions de la **même** ligne de se croiser (C5), mais les informations
de version sont partagées par toutes les lignes d'un même vecteur : deux
suppressions de lignes **voisines** écrivent encore la même structure sans
verrou [lu : `version_info.cpp:97-121` ; ThreadSanitizer du banc].

**C6** (suppression contre mise à jour de la même ligne) entre dans A4′ :
c'est le même verrou de ligne. Sous l'option A son attendu est celui que le
banc a écrit — jamais les deux ; sous l'option B ce serait « les deux
valident, la mise à jour touche zéro ligne ».

**L'index vectoriel n'est pas couvert par cette note** : nos chemins de
suppression et de mise à jour du HNSW modifient ses points d'entrée pendant
la transaction, sans verrou. C'est la section suivante de l'étude.

---

## 6. Ce que le banc doit ajouter

1. **Prouver l'attente par l'ordre des événements** : l'écriture du second se
   termine après le commit du premier. Le harnais a `mark`, `waitFor`,
   `runMarked`. Les scripts à barrière de C1, C2, C5, C6 sont à réécrire ainsi
   par chaque marche : avec l'attente, l'écrivain bloqué n'atteint plus la
   barrière.
2. **L'unicité quand l'instantané précède le commit de l'autre**, sans
   chevauchement des écritures.
3. **La même ligne, deux colonnes** : aujourd'hui les deux valident ; ensuite,
   la seconde attend.
4. **L'interblocage** : deux transactions prennent A puis B et B puis A ;
   exactement une reçoit l'erreur d'interblocage, l'autre valide.
5. **L'annonce en tête** : les deux mêmes transactions annoncent {A, B} et
   {B, A} ; aucune erreur, les deux valident, l'une après l'autre.
6. **Le rollback et l'erreur libèrent les verrous** : celui qui attendait
   derrière passe.
7. **L'interruption et le délai** pendant une attente : chacun avec son erreur
   nommée, et la transaction annulée proprement.
8. **Le nœud-carrefour** : huit écrivains créent chacun des relations vers le
   même nœud ; aucun n'attend l'autre (verrou partagé) — mesuré par l'ordre des
   événements.

---

## Sources vérifiées (3 octobre 2026)

- PostgreSQL, *Transaction Isolation* (§13.2.1 Read Committed, §13.2.2
  Repeatable Read) — `postgresql.org/docs/current/transaction-iso.html`
- PostgreSQL, *Explicit Locking* (§13.3.2 Row-Level Locks, §13.3.4 Deadlocks,
  §13.3.5 Advisory Locks) — `postgresql.org/docs/current/explicit-locking.html`
- PostgreSQL, *Index Uniqueness Checks* —
  `postgresql.org/docs/current/index-unique-checks.html`
- PostgreSQL, *Lock Management* (`deadlock_timeout`) —
  `postgresql.org/docs/current/runtime-config-locks.html`
- PostgreSQL, tutoriel *Transactions* et page `ROLLBACK` (état d'échec d'un
  bloc de transaction)
- Neo4j, *Operations Manual — Locks and deadlocks* —
  `neo4j.com/docs/operations-manual/current/database-internals/locks-deadlocks/`

**Non vérifié dans leur documentation** : ce que fait exactement Neo4j d'une
transaction après une instruction en erreur (le banc écrit « marquée en
échec » ; je n'ai pas trouvé la page) ; et ce que reçoit chez Neo4j une pose
de propriété sur un nœud qu'une autre transaction vient de supprimer.
