# Au réveil

**Nuit du 6 septembre 2026.** Ce qui a été fait pendant que tu dormais, ce qui
attend ta décision, et ce qu'il faut regarder en premier.

## En trois lignes

Neuf commits. Le plus important n'était pas sur ma liste : **le chemin par
défaut de la lecture n'indexait rien en plein texte**. Tout le reste est du même
genre — des choses qui ne cassaient pas, qui ne levaient aucune erreur, et qui
rendaient faux.

## À regarder en premier

1. [`01-les-mensonges-et-l-ordre-pour-les-regler.md`](01-les-mensonges-et-l-ordre-pour-les-regler.md)
   — l'inventaire complet, l'ordre, et les tableaux de ce qui est réglé.
2. Le commit `90fbea2b0` — `flush_insertions`, le plus grave.
3. La section « ce qui attend ta décision » ci-dessous.

## Le plus grave, en détail

`open_fts_handles_for` porte cette phrase depuis des semaines :

> À appeler depuis **chaque** point d'entrée d'ingestion : sans handle ouvert,
> `InsertRecordNode` saute l'indexation en silence, et la recherche rend 0 sans
> que rien ne le signale.

Trois points d'entrée l'appelaient. `flush_insertions` — **le quatrième** — non.
Et son registre de services « minimal » ne portait pas `fts_handles`.

Or `flush_insertions` est ce qu'appelle toute recherche en `Consistency::Eventual`,
c'est-à-dire **le défaut**, dès qu'il y a du travail en file. Les entités qui y
passaient étaient consommées — `mem::take`, elles ne repassent pas au drain — et
jamais indexées en plein texte.

Il a survécu parce qu'**aucun test n'empruntait la combinaison du produit** :
`create()` puis une recherche en `Eventual`. Tous nos e2e de recherche passent
par `ingest_entities` et `Consistency::Immediate`. Une suite peut être verte sur
tous les chemins sauf celui que les gens empruntent.

## Ce qui a été réglé

**Le contrat de lecture** (`558c06662`, `0b611562b`)
`meta.partial` mentait dans le mode par défaut ; `pending_count` était mesuré
avant la consigne ; le verdict de `attendre_les_ecritures` était calculé puis
jeté. Et `Consistency` n'avait **aucun appelant en production** — l'outil des
agents ne traversait aucune de ses trois branches. Il descend maintenant dans le
chemin composable, par un seul écrivain partagé, avec un paramètre à liste close
sur la fiche.

**Les comptes** (`42d40a8b1`)
`FlushResult` gagne `unchanged` — le compte existait et n'allait qu'à un
`eprintln!` — et `warnings`, qui fait remonter ce que les nœuds disaient déjà
dans le vide.

**Les silences d'écriture** (`42d40a8b1`, `b6c8afd71`, `8ff4ffbf2`, `2ca866013`)
L'indexation sautée ; la troncature d'embarquement (le MiniLM multilingue coupe
à **128 jetons** quand nos chunks font 1 500 caractères) ; trois erreurs avalées
dans `KBUpdateNode` et `DeleteRecordNode`, dont une qui laissait des chunks
périmés en base **en les comptant comme supprimés** ; trois `create_vector_index`
dont l'erreur avalée était forcément la seule qui compte, le DDL étant
idempotent ; et un vecteur sparse **déjà calculé** qui partait à la poubelle
sans un mot faute de handle.

**Les cadrans débranchés** (`d730a0973`)
`title_boost`, `content_boost`, `special_ops`, `boost` sont acceptés et jamais
appliqués. Ils le disent maintenant au montage. En écrivant le test, j'ai
découvert que **notre propre fiche de test pose un `boost`** : l'alarme ne vise
pas un cas de laboratoire.

## Ce qui attend ta décision

Je n'y ai pas touché — chacun force un choix qui t'appartient.

| | pourquoi c'est à toi |
|---|---|
| **Les quatre disponibilités** (`data`/`textsearch`/`sparse`/`dense`) à la place de `Consistency` | changement de surface d'API. Il n'y a plus qu'**un** endroit à changer, `appliquer_la_consigne` — le moment est bon |
| **Le régime « au tick »** et le niveau `data` synchrone | change ce que `create` promet |
| **`PendingWork` par ressource** | le couplage faux ; dépend du précédent, et c'est le gros morceau |
| **`FlushResult.failed`** | restera faux tant qu'il n'y aura pas de canal d'échec **par enregistrement** ; ça change une structure publique |
| **`fusion.rs`** | aucun appelant, supplanté par `search::fuse_by_strategy`. Supprimer, ou assumer comme surface publique ? Ses neuf tests verts donnent l'illusion d'une pièce vivante — j'ai mis l'avertissement en tête de module |
| **`crate::acces`** et **`KBSearchNode`** | toujours sans appelant de production ; les brancher est un choix de produit |
| **Les quatre registres de services d'ingestion** | trois partagent quarante-quatre lignes identiques, mais ils **divergent** (`ingest_entities` n'enregistre ni `kb_metadata` ni `event_bus`). Unifier sans savoir si ces absences sont voulues changerait le comportement en silence |

## Deux choses à surveiller

**Ma correction de `flush_insertions` a un coût.** Elle appelle maintenant
`open_fts_handles_for` à chaque recherche `Eventual` avec du travail en file, et
cette fonction balaie **toutes** les KB en posant du DDL — idempotent, mais pas
gratuit. Le balayage des KB est justifié (`create()` enfile aussi des lignes
`{KB}_Index`, qui ont besoin de leur handle), donc je ne l'ai pas réduit. Si le
test de charge s'est allongé, c'est là qu'il faut regarder ; une mémoïsation des
index déjà posés est possible, mais elle peut masquer un index tombé, donc elle
se décide.

**Le régime.** `confort` mettait vingt minutes de mur pour trois minutes de CPU
sur un seul test — c'est son travail, il existait pour la contrainte que tu as
levée. Tout a été relancé en `plein`. C'est noté dans le knowledge dump du 5.

## L'état des tests

- **924 tests de bibliothèque** verts, dont **huit neufs** cette nuit.
- La passe e2e complète a été relancée en `plein` avec l'ensemble des commits.
  Son verdict est en fin de ce document, ajouté quand elle a rendu.
- Un test e2e neuf, `une_recherche_eventual_indexe_ce_qu_elle_pose`, tient le
  chemin qui manquait : la ligne existe **et** on la retrouve.
