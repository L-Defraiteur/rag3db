# Appliquer la machine à états à l'écriture — ce qui reste vraiment

18 septembre 2026. Conception avant code, à la demande de l'orchestrateur.
Rien n'est modifié tant qu'il n'a pas annoncé son commit du repli des KB.

## 0. Correction d'entrée : la garde existe déjà

J'ai proposé ce chantier en disant « rien n'empêche encore une transition à
l'écriture ». **C'est faux**, et depuis le 27 août : la garde est dans
`UpdateRecordNode` (`src/dataflow/record_nodes.rs`, section « 1 bis. Une
transition non déclarée ne passe pas »), avec son test e2e
(`tests/e2e_simple_entity.rs`, `une_transition_non_declaree_ne_passe_pas`).

Elle est complète pour ce qu'elle couvre : elle lit l'état d'avant dans la
même requête que l'empreinte de contenu, laisse passer trois cas nommés (la
mise à jour ne touche pas au champ, l'état ne change pas, l'état d'avant est
inconnu — sinon ajouter un `Lifecycle` à une entité qui tourne bloquerait
toutes ses lignes), écarte la ligne fautive sans faire tomber le lot, et
**dit ce qui aurait été permis** au lieu de se contenter d'un refus.

**Pourquoi je l'ignorais**, et ça vaut d'être écrit : le doc-commentaire de
`Lifecycle` dans `src/config.rs` dit encore *« Ce que ce type ne fait pas
encore : l'empêcher au moment d'écrire »*. J'ai cru le commentaire. C'est
mot pour mot le piège n°4 de notre propre mémo — *croire un commentaire
plutôt que mesurer* — et je l'avais écrit moi-même.

## 1. Le trou qui compte : un chemin d'écriture sur deux

Il y a **deux** façons d'écrire dans une entité, et la garde n'en couvre
qu'une.

| Chemin | Par où | Gardé ? |
|---|---|---|
| `Catalog::update` | `pending.updates` → `UpdateRecordNode` | **oui** |
| `Catalog::ingest_entities` | `InsertRecordNode` en `Upsert` (`MERGE` sur l'uuid déterministe, puis `SET`) | **non** |

Vérifié plutôt que supposé : `lifecycle` n'apparaît que dans `config.rs` et
`record_nodes.rs`. Les trois autres occurrences du mot dans le dépôt
(`events.rs`, `catalog.rs`) sont des commentaires de section sans rapport.

Conséquence : l'uuid étant dérivé des champs d'identité (`hashsafe`), une
ré-ingestion de la **même** ligne avec un autre état la fait passer d'un état
à l'autre **sans aucune vérification**. Et c'est le chemin le plus emprunté :
toute ré-ingestion d'un dépôt, d'un lot, d'un import.

> **Une garde qu'un chemin sur deux ignore est pire que pas de garde :** elle
> inspire une confiance qu'elle ne mérite pas. Tant qu'il n'y en avait
> aucune, personne ne comptait dessus.

## 2. `initial` ne veut rien dire aujourd'hui

`Lifecycle::initial` n'est **lu nulle part** hors de `config.rs` (sa propre
validation et ses tests). L'état de départ vient donc entièrement de
l'appelant : le mot « initial » décrit une intention que le code n'applique
pas.

## 3. Ce que je propose

### 3.1 La vérification à l'ingestion va là où la ligne est déjà lue

C'est la même idée qu'en août pour `UpdateRecordNode` : **ne pas payer une
lecture pour vérifier**. Sur le chemin d'ingestion, `split_unchanged` relit
déjà les lignes existantes champ par champ (`dialect.batch_select`) pour
court-circuiter les enregistrements identiques. L'état d'avant y est donc
**déjà en main, gratuitement**.

**Correction après relecture de l'orchestrateur** : j'avais écrit que le cas
de la table vide « se referme tout seul » parce que `split_unchanged` y est
sauté. **C'est faux** — il n'y a pas de transition à vérifier, mais il y a
toujours un état à *écrire*, et rien ne dit qu'il est déclaré.

Il y a donc **trois** points, pas un :

| cas | où | ce qu'on vérifie |
|---|---|---|
| la ligne existe | contre `split_unchanged`, qui l'a déjà relue | la **transition** est déclarée |
| la ligne naît, table vide | dans la boucle qui construit le CSV, **avant l'écriture** | l'état est **déclaré** (§3.2) |
| la ligne naît, table non vide | même endroit que le premier cas, branche « pas d'état d'avant » | idem |

Le troisième est à moi : une ligne neuve dans une table qui tourne est une
naissance elle aussi, et `split_unchanged` ne lui trouvera pas d'état
d'avant. La règle des naissances a donc deux domiciles, pas un.

Autre chose qui simplifie, depuis le repli des KB : `split_unchanged` n'a
plus sa garde `has_kb_participation` — le pipeline qui « repartait de chaque
enregistrement » n'existe plus, donc il s'applique à **toute** entité. Une
exception de moins à porter dans le raisonnement.

### 3.2 La naissance n'est pas une transition

L'orchestrateur propose : *pas d'ancien état, donc seul l'état initial est
admis.* **Je propose autre chose**, et c'est la seule chose sur laquelle je
ne suis pas d'accord :

> `initial` s'applique quand la donnée **ne dit rien** ; si elle dit un état,
> il doit être un état **déclaré**.

Trois raisons :

1. **Importer des données qui ont une histoire est un cas réel.** « Seul
   l'initial » l'interdit : on ne pourrait pas charger un lot de tickets déjà
   fermés sans les rouvrir puis les refermer un par un.
2. **Ça rend `initial` vrai.** Aujourd'hui c'est un mot ; là c'est un défaut
   effectif, appliqué quand le champ est absent.
3. **La faute de frappe est toujours attrapée** — `promotedd` n'est pas un
   état déclaré, donc refusé. On ne perd pas la vérification, on perd
   seulement l'interdiction d'importer.

La règle tient en une phrase : **la machine gouverne les changements, pas les
naissances.** Si l'orchestrateur préfère la version stricte, elle est à un
champ près (`creation: Initial | ToutEtatDeclare`) — mais je ne l'ajoute pas
d'avance : un réglage qu'on met « au cas où » est un réglage que personne ne
saura régler.

### 3.3 Un refus fait ce que les refus font déjà ici

Rien à inventer : le précédent est en place et il est bon. `consigner_l_echec`
→ `EchecDeGroupe` avec `operations: 1` → la ligne entre dans `rejetes`, sort
du lot, compte dans `failed`, et la cause est nommée. `LinkRecordNode` fait
déjà ça pour un bout de relation non résolu.

Le refus d'ingestion reprend exactement cette forme, **y compris la phrase
qui dit ce qui aurait été permis** : un refus qui n'indique pas la sortie
oblige l'appelant à aller relire la déclaration.

### 3.4 Pas de nouvel événement sur le bus, et c'est délibéré

Aujourd'hui un refus vit dans `FlushResult.warnings` (texte libre) et dans un
`UpdateStatus::Failed` **sans cause attachée**. Il n'existe aucune variante
typée de refus sur le bus, seulement `Warning` et `Error`.

Je **n'ajoute pas** de `TransitionRefused`. Une variante que personne
n'écoute est décorative — c'est la critique que j'ai faite des renvois sans
`recall`, elle vaut contre moi ici. Ce qui manque vraiment est plus petit et
sert plus largement : **attacher sa cause à `UpdateStatus::Failed`**. Ça
profite à tous les refus par ligne, pas seulement aux transitions.

### 3.5 Le commentaire périmé

Corriger le doc-commentaire de `Lifecycle` dans `src/config.rs`. Il m'a coûté
une heure aujourd'hui ; il coûtera la même chose au suivant. À faire en
premier, c'est une ligne.

## 4. Le cas d'usage : ce qui existe, et ce qui n'existe pas

Je voulais proposer « un outil qui passe de brouillon à promu **sur preuve** »
(docs 49 et 05). **Ce n'est pas faisable** : `GraphTool` est une structure
Rust en mémoire (`src/dataflow/graph_tool.rs`), sans table, sans
`register_entity`, sans champ de statut ; `GraphVersion` et `Invocation`
n'existent qu'en documentation. La règle « on promeut sur preuve » n'a
aujourd'hui **aucun support en code**. Autant le dire.

Le candidat réel est ailleurs, et il est déjà dans les journaux de la suite
d'embarquements : *« modèle `modele-a` requalifié en stockage legacy »*. Un
modèle d'embarquement sur un index a bel et bien des états. Ce serait une
machine déclarée qui décrit quelque chose qui tourne — mais c'est le domaine
d'une autre session, donc **à proposer, pas à prendre**. *Proposé le 18 ;
l'orchestrateur le garde en note, pas maintenant.*

À défaut, le test existant (`open → in_progress → closed`) suffit à éprouver
le chemin d'ingestion ; il n'a juste pas la vertu de décrire notre propre
maison.

## 5. Les pas

| pas | contenu | durée |
|---|---|---|
| A | le doc-commentaire de `config.rs` | une ligne |
| B | la garde de **transition** contre `split_unchanged` | une demi-journée |
| B bis | la règle des **naissances** à ses deux domiciles : la boucle du CSV avant écriture, et la branche « pas d'état d'avant » ; `initial` appliqué quand le champ est absent ; tests | comprise dans B |
| C | la cause attachée à `UpdateStatus::Failed` | une heure |

## 6. Ce que je ne fais pas

- Pas de variante de bus (§3.4).
- Pas de réglage `creation:` d'avance (§3.2).
- Rien dans `catalog.rs` ni `record_nodes.rs` avant l'annonce du commit du
  repli des KB — `UpdateRecordNode` et `build_ingestion_graph` sont dans la
  zone de l'orchestrateur, et le fichier perdait mille lignes pendant que
  j'écrivais ceci.
