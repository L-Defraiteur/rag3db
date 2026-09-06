# Au réveil

**Nuit du 6 septembre 2026.** Ce qui a été fait pendant que tu dormais, ce qui
attend ta décision, et ce qu'il faut regarder en premier.

## En trois lignes

Treize commits. Le plus important n'était pas sur ma liste : **le chemin par
défaut de la lecture n'indexait rien en plein texte**. Tout le reste est du même
genre — des choses qui ne cassaient pas, qui ne levaient aucune erreur, et qui
rendaient faux.

Et trois fois je me suis corrigée moi-même dans la nuit. C'est noté avec le
mécanisme à chaque fois, pas seulement la conclusion : une correction sans son
mécanisme se refait.

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
`{KB}_Index`, qui ont besoin de leur handle), donc je ne l'ai pas réduit. Une
mémoïsation des index déjà posés est possible, mais elle peut masquer un index
tombé, donc elle se décide.

**Vérifié depuis :** le test de charge passe par `ingest_code` → `drain`, pas par
`flush_insertions` — ma correction ne le traverse donc pas, et ses 32 minutes ne
lui doivent rien. Le coût reste à mesurer là où il tombe vraiment : une recherche
`Eventual` répétée avec du travail en file.

**Le régime — et ma bévue dessus.** J'ai d'abord conclu que `confort` ne servait
qu'à ménager ton poste, donc que ta consigne levée autorisait `plein`. La passe
est morte par manque de mémoire ; j'ai corrigé la note en accusant le régime ;
puis la passe en `confort` est morte au même endroit. **Ce n'était pas le
régime.** Le vrai mécanisme est le cgroup à 16 Go, décrit plus bas.

Ce que ta consigne levée autorisait, c'était de **lancer** la passe sans
demander. Pas de la lancer autrement. Les deux se ressemblaient assez pour que je
confonde, et il a fallu deux morts pour les séparer.

## L'état des tests

- **924 tests de bibliothèque** verts, dont **huit neufs** cette nuit.
- **La suite e2e : 301 tests verts, zéro échec**, PostgreSQL compris (17), le
  test de charge compris (4 438 scopes, 46 698 relations, rien de perdu).
- Trois tests e2e neufs :
  `une_recherche_eventual_indexe_ce_qu_elle_pose` (la ligne existe **et** on la
  retrouve), `le_graphe_applique_la_consigne_de_coherence` (le chemin des agents
  applique enfin la consigne — 1 résultat depuis une file jamais vidée, contre
  zéro avant), et l'invariant de `partial`.

### La passe complète ne tient pas en une seule tâche de fond

Deux tentatives tuées par le veilleur mémoire du harnais, la première en
`plein`, la seconde en `confort` — donc **ce n'était pas le régime**, contrairement
à ce que j'ai d'abord écrit. Le script confine délibérément la passe dans un
cgroup à `MemoryHigh=16G`, pour que ce soit *son* cache qui soit récupéré plutôt
que les pages de tes applications ; la passe entière dépasse ce plafond, part en
récupération — 9,6 Go de swap mesurés — et ralentit jusqu'à se faire tuer.

**Suite par suite, tout passe et va vite.** C'est la façon de faire tant que ce
plafond vaut 16 Go ; `RAG3WEAVER_BUILD_MEMORY_HIGH` le change si tu préfères
l'autre compromis.

### Une mesure de référence, enfin posée

`e2e_charge_ingestion` n'en avait aucune d'écrite. Elle est dans le knowledge
dump du 5 (§3 bis). Le partage compte plus que le total : **99,1 % du temps est
dans `entities_ms`** — découpage et embarquement. Les relations coûtent 0,2 %.
