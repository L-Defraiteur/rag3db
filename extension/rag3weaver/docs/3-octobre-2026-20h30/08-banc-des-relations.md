# Le banc des relations — la référence avant tout poids de proximité

3 octobre 2026. Le banc de recherche n'avait aucune question dont la bonne
réponse dépend du graphe (vision des relations, `extension/rag3weaver/visions/2026-10-03-20h16-explorer-les-relations.md`,
§5). `tests/e2e_banc_relations.rs` en pose vingt, sur le corpus du banc de
l'étage (`src/` de rag3weaver), et note les outils qui y répondent
aujourd'hui. Fichier frère, autonome, comme les autres bancs : rien n'est
ajouté aux fichiers de la session recherche.

## Les questions, et comment leurs réponses ont été relevées

| Type | Nombre | Forme de la réponse |
|---|---|---|
| Qui appelle X | 8 | ensemble de (nom, fichier) |
| De quoi dépend X | 4 | ensemble de (nom, fichier), plus des tolérés (types) |
| Qu'est-ce qui relie X et Y | 4 | le plus court chemin, de l'appelant à l'appelé |
| Quels tests traversent X | 4 | ensemble de tests, à deux sauts au plus |

Relevées **sans l'outil mesuré** : un script cherche les appels `nom(` dans
`src/` hors commentaires et remonte à la fonction englobante ; chaque site
a été relu à la main ; les dépendances, à la lecture du corps. Seuls des
noms définis une fois ; la réponse se compare par (nom, fichier), le nom
seul confondant les homonymes (`create`, `execute`). Corpus vivant : une
réponse qui bouge avec le code se relève à nouveau.

**La notation.** Une liste : **rappel** (la part de la référence trouvée) et
**précision** (la part du rendu dans la référence ou ses tolérés) ; l'ordre
ne compte pas. Un chemin : 1 si le rendu est la référence, **ou un chemin de
même longueur dont chaque saut est un appel relevé** ; faute d'outil de
chemin aujourd'hui, `impact` est noté sur la **distance** (le départ est-il
trouvé au niveau égal à la longueur de référence).

## La note de référence (codeparsers `a569417`, pointé sur master)

| Type | Outil | Rappel | Précision |
|---|---|---|---|
| Qui appelle X | `usages` | 1,00 | 0,88 |
| De quoi dépend X | voisinage sortant (`NeighborhoodNode`) | 0,94 | 0,61 |
| Qu'est-ce qui relie X et Y | `impact` (distance) | 4 / 4 | — |
| Quels tests traversent X | `impact` | 0,96 | 0,92 |

## Ce que disent les écarts

Tous relus à la source ; aucun n'est une erreur de la référence.

- **Un accès de champ pris pour un appel.** `r.indexed_hash` (un champ)
  relié à la fonction `indexed_hash`, `self.config` à un scope `config`,
  `text.len()` à un `len` du projet, `r.chunk` (champ de `UnifiedResult`) à
  la méthode `Chunker::chunk` — ce dernier fait entrer sept tests de rendu
  parmi ceux qui « traversent » `chunk_lines` (précision 0,68). La source :
  la voie des rendez-vous (`MENTIONS`, `code.rs`) relie un nom à son
  définisseur unique sans regarder le qualificatif, alors que le résolveur
  de codeparsers écarte une référence qualifiée par une variable de type
  inconnu. Correctif à proposer à la session de l'arbre principal : sur
  cette voie aussi, une référence qualifiée par une variable sans type ne
  prend pas de rendez-vous.
- **Le type par le retour déclaré n'est pas encore pointé.** Le test smoke
  qui appelle `catalog.ingest_entities(…)` manque à `ingest_entities_jusqu_a`
  (rappel 0,83) : `catalog` vient d'un appel, son type se lit par le retour
  déclaré — codeparsers `56d039f`, pointeur 8, en attente.
- **Un conteneur à côté de son membre.** `Chunker`, `GraphTool` (des `impl`)
  apparaissent avec leurs méthodes quand le site de l'arête du conteneur
  n'est pas celui du membre ; la règle « au plus étroit » ne les reconnaît
  qu'à site égal.
- **Un appel par une méthode d'un objet de trait** (`self.dialect
  .drop_vector_index`) manque à `bulk_vector_index` : le type d'un champ
  `Box<dyn Dialect>` se lit, mais `drop_vector_index` est déclaré par le
  trait et défini par chaque dialecte.

## Les plus courts chemins du moteur, lus avant de s'y fier

`MATCH p = (a:Scope {_uuid: $a})-[:CONSUMES* SHORTEST 1..4]->(b:Scope
{_uuid: $b}) RETURN nodes(p)` : 11 à 18 ms par paire sur ce corpus (5 000
scopes environ), aucun produit cartésien. Le plan : `PRIMARY_KEY_SCAN` des
deux extrémités, `RECURSIVE_EXTEND`, jointures par hachage — **et un
`SCAN_NODE_TABLE`** (la lecture des propriétés des nœuds du chemin), qui
croîtra avec la table : à mesurer sur un gros index avant d'en faire une
section de chaque recherche. Trois chemins sur quatre sont la référence ; le
quatrième (`chunk → chunk_fixed → build_line_index`) est un autre chemin de
même longueur, dont les deux sauts sont réels.

Régénérer : `./run_e2e.sh --test e2e_banc_relations` (≈ 70 s par test,
HashEmbedder : les notes ne dépendent pas des vecteurs).
