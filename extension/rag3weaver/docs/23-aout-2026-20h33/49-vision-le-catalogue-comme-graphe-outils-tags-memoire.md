# Doc 49 — Vision (premier jet) : le catalogue comme graphe — outils, tags, mémoire

**Statut : premier jet, écrit pour être critiqué.** Rien ici n'est implémenté.
Le but est d'avoir un texte à attaquer plutôt qu'une intuition à se rappeler.
Suite du [doc 36](36-vision-agents-comme-graphe-et-workflow.md) et de la
décision de Lucie : *un outil = un graphe entier + un spécificateur*.

## 1. Le principe

> **Tout ce que le système sait faire est une donnée dans le système.**

Les nœuds, les graphes-outils, les agents, leurs traces et les concepts qui les
relient vivent dans la même base que les documents. Conséquence directe : un
agent **cherche** ce qui existe avec les mêmes moyens qu'il cherche un document
— BM25, vecteur, sparse, rerank — et ce qu'il compose enrichit ce que le suivant
trouvera.

L'inversion par rapport à l'usage courant : ailleurs, la liste d'outils est
figée et le modèle choisit dedans. Ici, **le catalogue est une base de
connaissances** ; le modèle y cherche, et ne compose que si rien ne convient.

## 2. Ce qui est déjà acquis (et qui contraint le reste, en bien)

| acquis | ce qu'il donne gratuitement |
|---|---|
| `GraphDefinition::hash()` — BLAKE3 déterministe, nœuds et arêtes triés | **l'adressage par contenu** : deux graphes identiques *sont* la même version. La duplication exacte est mécaniquement impossible |
| `Scope { org, project }` et le stamp `_org`/`_project` (doc 37) | le catalogue est **cloisonné par cellule** sans une ligne de plus. L'ontologie d'un locataire est la sienne |
| `GraphNode` — un nœud *est* un sous-graphe | la **composition est structurelle**, pas ajoutée. Un outil qui en contient un autre, c'est le même objet un niveau plus haut |
| Le **reranker** (doc 47) et les trois signaux de recherche | de quoi **choisir**, pas seulement retrouver — voir §5, c'est là que ça devient intéressant |
| `RelGroup` dans le binder kuzu | une table de relation à **plusieurs paires** `FROM…TO…` : « lier à n'importe quoi » se dit en une table |
| `to_mermaid` / `parse_mermaid_template` | un outil est **lisible, éditable et rejouable** par un humain comme par un modèle |

## 3. Le modèle : les outils

**Entités simples, pas des KB.** Une KB agrège des documents ; un outil n'est pas
un document — sa surface textuelle est minuscule (nom, description, paramètres)
et sa richesse est **relationnelle**. Or c'est ce qu'un graphe sait faire. Les
entités simples de rag3weaver ont déjà chunks et embeddings, donc la description
reste cherchable sémantiquement.

| entité | rôle | champs |
|---|---|---|
| `NodeType` | le **vocabulaire** | `name`, `description`, `ports` (JSON), `config_params` (JSON), `origin` |
| `GraphTool` | l'**identité** | `name`, `description`, `params_schema`, `status` (`draft`/`promoted`/`deprecated`) |
| `GraphVersion` | le **contenu**, immuable | `hash`, `definition`, `created_at` |
| `Invocation` | la **trace** | `tool_call_id`, `arguments`, `ok`, `ms`, `error` |

```cypher
CREATE REL TABLE HAS_VERSION(FROM GraphTool TO GraphVersion);
CREATE REL TABLE CURRENT(FROM GraphTool TO GraphVersion);
CREATE REL TABLE USES_NODE_TYPE(FROM GraphVersion TO NodeType);
CREATE REL TABLE CONTAINS(FROM GraphVersion TO GraphTool);   -- la composition
CREATE REL TABLE INVOKED(FROM Invocation TO GraphVersion);
CREATE REL TABLE COMPOSED_BY(FROM GraphTool TO Session);     -- provenance
```

**Séparer identité et contenu n'est pas de la coquetterie** : une trace doit
pointer **la version exacte qui a tourné**, pas un nom dont le contenu a changé
depuis. Sans ça, « cet outil marche » ne veut rien dire.

**Et ça rend mesurable la question laissée ouverte** — quand promouvoir un
graphe composé ? Une fois les `Invocation` stockées, ce n'est plus une opinion :
*N* succès, aucune erreur récente. Un graphe reste **brouillon** par défaut, ne
pollue pas le catalogue, et se promeut **sur preuve**.

```cypher
-- les outils qui ont fait leurs preuves
MATCH (t:GraphTool)-[:CURRENT]->(v:GraphVersion)<-[:INVOKED]-(i:Invocation)
WHERE t.status = 'draft' AND t._org = $org AND t._project = $project
WITH t, count(i) AS n, sum(CASE WHEN i.ok THEN 0 ELSE 1 END) AS echecs
WHERE n >= 5 AND echecs = 0
RETURN t.name, n;

-- ce qui casserait si on retirait un type de nœud
MATCH (v:GraphVersion)-[:USES_NODE_TYPE]->(n:NodeType {name: $type})
MATCH (t:GraphTool)-[:CURRENT]->(v)
RETURN t.name;
```

**Le piège à ne pas se faire** : projeter les `NodeType` en base crée **deux
sources de vérité** — le registre compilé et les lignes. Ce doit être une
**projection à sens unique**, idempotente, rejouée au démarrage, versionnée par
une empreinte du registre. Jamais l'inverse ; jamais un nœud qui n'existe qu'en
base.

## 4. Le `Tag` : un concept, pas une étiquette

Idée de Lucie, et c'est la pièce qui rend le reste navigable. Un `Tag` n'est pas
une chaîne accrochée à une ligne : c'est un **nœud de concept** qui a

- une **identité** et une description — donc cherchable sémantiquement,
- ses **propres mémoires** (chunks, embeddings) : on peut *écrire dans* un tag,
- des liens vers **n'importe quoi** — outils, versions, entités, chunks,
  sessions, invocations,
- et des liens vers **d'autres tags** — donc un graphe de concepts.

```cypher
CREATE NODE TABLE Tag(name STRING, description STRING, _org STRING,
                      _project STRING, PRIMARY KEY(name));
-- « lier à n'importe quoi » : une seule table, plusieurs paires (RelGroup)
CREATE REL TABLE TAGGED(FROM Tag TO GraphTool, FROM Tag TO GraphVersion,
                        FROM Tag TO NodeType,  FROM Tag TO Session,
                        FROM Tag TO Invocation);
CREATE REL TABLE RELATES_TO(FROM Tag TO Tag, kind STRING, score DOUBLE);
```

Ce que ça débloque, concrètement : un agent qui échoue trois fois sur le même
sujet peut **écrire ce qu'il a appris dans le tag**, et le prochain le retrouve
en cherchant le sujet — pas en relisant les traces. Le tag devient la mémoire
**thématique**, là où l'`Invocation` est la mémoire **épisodique**.

L'ontologie n'est jamais décrétée : elle **émerge de l'usage**.

## 5. Le reranker comme mainteneur d'ontologie

C'est le meilleur de l'idée, et ce n'est pas l'usage pour lequel on l'a mis.

Le problème dur de tout étiquetage automatique est la **prolifération** :
« multi-tenant », « multitenancy », « multi tenant », « cloisonnement par
locataire » — quatre tags pour un concept, et le graphe se dissout. Un seuil de
similarité cosinus ne suffit pas : il est aveugle au contexte et il faut le
régler à la main pour chaque corpus.

Or « ce tag candidat est-il **le même concept** que celui-ci ? » est exactement
une question **(requête, passage)** — c'est-à-dire un **cross-encoder**, pas un
embedder. Le pipeline devient :

```
tag candidat  →  BM25 + vecteur sur les tags existants  →  pool de candidats
              →  RERANK (paire candidat / existant)
              →  au-dessus du seuil : réutiliser · en dessous : créer
```

L'embedding **propose**, le reranker **décide**. Et le même mécanisme sert deux
autres questions que Lucie a posées :

- **quel tag lier à quel tag** — score de la paire, et `RELATES_TO.score` garde
  la trace de la décision ;
- **quel tag choisir pour un contenu** — les tags candidats sont les passages,
  le contenu est la requête.

**Et c'est récursif** : ce pipeline est lui-même un **graphe-outil**
(`resolve_tag`). Le système entretient son ontologie avec ses propres outils.

Le même geste vaut contre les **quasi-doublons de graphes** : avant
d'enregistrer un graphe composé, on cherche dans le catalogue et on rerank ;
au-dessus du seuil, on propose la réutilisation plutôt que l'enregistrement.
**Le RAG se contrôle lui-même.**

## 6. La boucle complète

```
        ┌──────────────── l'agent cherche un outil ────────────────┐
        │  BM25 + vecteur + rerank sur GraphTool.description       │
        └───────────────┬──────────────────────┬───────────────────┘
              trouvé    │                      │  rien de convenable
                        ▼                      ▼
                  appeler l'outil        composer un graphe
                        │                (grammaire llguidance,
                        │                 validation structurelle)
                        ▼                      │
                  Invocation ◄─────────────────┘
                        │
                        ▼
          preuves suffisantes → promotion → cherchable par le suivant
                        │
                        ▼
                  resolve_tag → le concept rejoint le graphe
```

Chercher coûte moins cher que composer : la politique tombe toute seule —
**chercher, composer si besoin, garder ce qui a marché**. Le système devient
moins cher à mesure qu'il sert.

## 7. Décisions de Lucie (25 août)

**1. Un agent est un outil.** Une seule table : un agent *est* un sous-graphe
(doc 36), donc une ligne de `GraphTool` avec un statut différent. C'est ce qui
permet qu'un agent en appelle un autre comme un outil ordinaire, sans mécanisme
particulier.

**2. Deux espaces de tags, dont un temporel.** Le vocabulaire du produit (les
`NodeType`, les concepts stables) est partagé ; l'ontologie du locataire est par
cellule. Et l'un des deux porte une **dimension temporelle** — un concept a une
période de pertinence, une mémoire vieillit, un tag peut avoir été vrai à une
époque. Reste à concevoir : est-ce une propriété du tag, du lien `TAGGED`, ou
une entité `Epoch` ? (Le lien porte déjà `score` ; y ajouter `from`/`to` est la
piste la plus légère.)

**3. Aucun accès Cypher pour les agents — et ce n'est pas une question de
sûreté.** C'est une question d'**agnosticité de backend**. Tout doit passer par
l'abstraction `Catalog` et le graphe. Exposer `CypherNode` à un agent rendrait
**chaque outil composé spécifique à kuzu** : le dialecte Postgres, ou tout
backend futur, ne saurait pas les exécuter. Or c'est précisément le catalogue —
le savoir accumulé — qui doit rester portable ; c'est ce qu'on aurait de plus
coûteux à perdre.

> **Si une capacité manque par rapport à Cypher, on l'ajoute à l'abstraction —
> on ne régresse pas vers du Cypher qui nous enchaînerait à cette base pour
> toujours.**

Corollaire utile : ce qu'un agent **n'arrive pas à faire** sans Cypher devient
la **feuille de route** du dataflow. Ses échecs sont un signal, pas une gêne.
`NodeTypePolicy` (livré, `76a566b58`) est le crochet ; pour les graphes composés
par un modèle, la politique exclut `CypherNode` **par nature**, pas par
prudence.

## 8. Questions encore ouvertes

1. **Qui a le droit de promouvoir ?** L'agent seul sur preuve, ou validation
   humaine ? Décision de produit : elle décide si le catalogue reste propre.
2. **Le seuil de rerank** pour fusionner deux tags : réglé à la main, appris sur
   les décisions humaines, ou par accord de plusieurs modèles ?
3. **La forme du temporel** (§7.2) : propriété, lien daté, ou entité `Epoch`.
4. **Que fait-on d'un `NodeType` non portable ?** Certains nœuds sont
   intrinsèquement liés à un backend. Faut-il un drapeau `portable` sur
   `NodeType`, dont la politique de composition se sert automatiquement ?

## 9. Risques nommés

- **Dérive sémantique d'un tag** : son sens glisse à mesure que des choses s'y
  accrochent. Il faudra une notion de *santé* — un tag à mille membres est trop
  vague, un tag à un membre est inutile — et une opération de **fusion**.
- **Le coût du rerank à l'écriture** : reranker à chaque étiquetage n'est pas
  gratuit. À faire par lots, en tâche de fond (`Priority::Idle` sur luciole),
  jamais dans le chemin critique d'une réponse.
- **La projection des `NodeType`** qui dériverait du registre (§3).
- **L'explosion du catalogue** si la promotion est trop facile — d'où §7.3.
- **Une base qui contient ses propres outils est une base qui peut se
  saborder** : la frontière de capacités (§7.3) n'est pas une option — et sa
  raison première est la portabilité, la sûreté venant par-dessus.
- **La fuite de l'abstraction** : chaque capacité ajoutée au dataflow plutôt
  qu'empruntée à Cypher coûte du travail. C'est le prix de l'agnosticité, et il
  se paie à chaque manque. Le nommer évite de le redécouvrir comme une surprise.
