# Les liens à clé (pas 3 des occurrences)

21 novembre 2026, session codeparsers. Décision de Lucie, relayée par c7 : **la voie (a)**. Plusieurs liens par couple,
distingués par une clé déclarée par relation. Le critère retenu est l'efficacité et l'utilité à long terme : un lien
par site, plutôt qu'un nœud et deux liens. Les liens multiples à clé servent aussi les sites d'appel, les citations
multiples et les relations par version.

Mesures et causes : [`01-le-cout-des-occurrences.md`](01-le-cout-des-occurrences.md).

## Ce qui est fait

- **Pas 1** (`keyed-links-1`, e8535aaa1f) : le filtre `derniers` du chemin par tranches est retiré. Le moteur garde le
  dernier `SET` d'une paire répétée depuis f19977332a.
- **Pas 2** (même lot) : le moteur garde les liens parallèles. Le montrent un `MERGE` dont le motif porte la clé, et
  un COPY de plusieurs liens entre deux mêmes nœuds (`e2e_copy_liens`).

## Le contrat

`RelationDef.key` (`Vec<String>`, absent du JSON quand il est vide) donne les noms des propriétés qui distinguent
plusieurs liens entre deux mêmes nœuds. Chaque clé est une propriété déclarée de la relation.

- **Vide** : un lien est sa paire, comme avant. Une seconde écriture de la paire met la première à jour.
- **Non vide** : un lien est sa paire plus les valeurs de ses clés. Une seconde écriture de mêmes (paire, clés) met
  le lien à jour, et toute autre écriture en ajoute un.

## Où la clé se lit

| endroit | sans clé (aujourd'hui) | avec clé |
|---|---|---|
| `Write::Link` (IR) | `relation`, `ends`, `props` | plus `key` |
| rag3db, chemin par tranches | `MERGE (a)-[r:R]->(b) SET …` | `MERGE (a)-[r:R {k: item.k}]->(b) SET …` |
| Postgres | `PRIMARY KEY (from_uuid, to_uuid)`, `ON CONFLICT (from_uuid, to_uuid)` | la clé dans les deux |
| COPY (`copier_les_liens`) | dédoublonnage, `deja` et `a_merger` par paire | par (paire, clés) |
| `paires_posees` (avant et après une tranche) | paires | (paire, clés) |
| `crees` et annulation d'un lot | `{from, to}`, et la déliaison retire toutes les arêtes de la paire | `{from, to, key}`, et la déliaison ne retire que ce lien |
| journal `noter_liee` | une paire annoncée | inchangé : la paire est annoncée une fois par lien neuf |

## Une base existante

- **rag3db** : pas de contrainte. Le `MERGE` à motif-clé suffit, et il n'y a rien à migrer.
- **Postgres, une clé ajoutée** à une relation déjà créée : une migration à l'enregistrement, dans une transaction,
  `ALTER TABLE t DROP CONSTRAINT t_pkey, ADD PRIMARY KEY (from_uuid, to_uuid, <clés>)`. Les lignes uniques par paire
  le restent par (paire, clés). Une colonne-clé NULL sur une ligne existante est **refusée par son nom**, car une clé
  primaire refuse NULL.
- **Une clé retirée**, sur les deux moteurs : l'enregistrement refuse par son nom (« la relation R portait la clé k ;
  la retirer n'est pas pris en charge »), même sans aucune paire multiple. La vraie migration inverse a son ticket.

## Sans effet sur l'analyse

Le lot touche `extension/rag3weaver/src/code.rs` seulement pour faire passer la clé déclarée d'une relation :
`register_relation_with` devient `register_keyed_relation(…, relation.key)`. Aucune relation du schéma du code ne
déclare encore de clé (`schema_configs`), donc le texte, le découpage, la candidature et les arêtes d'un scope ne
changent pas. `REGLES_DE_L_ANALYSE` ne monte pas. Il montera avec le lot des occurrences, où `OCCURS` déclarera sa
clé.

## L'ordre des commits

1. `RelationDef.key`, sans comportement (72ecf5d595).
2. `Catalog::register_keyed_relation`, qui ne fait que noter la clé, avec les témoins rouges : une relation à clé
   garde 100 % des sites par tranches et par COPY, au second lot comme au premier, et le retrait d'une clé est refusé.
3. Le comportement : IR, dialectes, drain, annulation, refus.
4. Postgres : la migration de la clé primaire, et son témoin dans `e2e_postgres` (rouge d'abord).
