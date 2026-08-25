# 02 — Les fichiers en temps réel, les deux modes, git et l'histoire

25 août 2026, 19h30. Décisions de conception prises à l'oral avec Lucie
**avant** d'intégrer `codeparsers`, parce qu'elles changent le schéma de
`File` et la forme du graphe d'ingestion de code. À lire avec le repérage du
crate (résumé dans le [01](01-rapport-de-progression.md) §4) et la feuille de
route (`../vision_roadmap_08_2026/06`).

## 1. `grep` et `read` lisent le réel, pas la base

La question de Lucie : *« grep/read devrait prendre les fichiers en temps
réel, pas la KB, à moins que la KB assure avoir pris les derniers
changements. »*

Réponse : **les deux, dans cet ordre.** `File` dans la base n'est pas le
fichier ; c'est son **index** — chemin, `content_hash` (blake3, que
`codeparsers` calcule déjà : `utils/hash.rs`), un **curseur de source** (§3),
et le `project`. `read` va au disque, recalcule le hash, compare :

- identique → l'index est bon, la réponse porte les offsets de la base ;
- différent → avertissement *« index périmé pour `path` »* dans
  `meta.warnings`, et **ré-ingestion de ce fichier** déclenchée (un fichier,
  pas le projet — §4).

Chaque lecture de l'agent **valide l'index gratuitement**. La base n'a pas à
garantir d'être à jour : elle sait quand elle ne l'est pas, et le dit.
C'est la culture `meta.warnings` appliquée à la fraîcheur.

`grep` suit la même règle : il tourne sur les fichiers (lucivy sait faire du
`contains` / regex sur du texte brut, et un `grep` disque reste possible),
et ses résultats sont **rapprochés** de `File`/`Scope` par chemin + offset
pour rendre des entités, pas des lignes nues. Là aussi, un hash qui diffère
est un signal.

## 2. Deux modes, une seule chose

| | **mode recherche** | **mode code** |
|---|---|---|
| exemple | portfolio en ligne, agent de démo qui « rappelle » le code des projets | agent de code local |
| la vérité | **une référence git** (un commit) | **l'arbre de travail**, changements non commités compris |
| `read` | `git show <ref>:<path>` — déterministe, sans checkout, sans arbre de travail | le disque |
| le changement | `git diff --name-status A B` | surveillance (`notify` / inotify), débounce ; à défaut, balayage des `mtime` |
| le curseur | le commit | un flux d'événements, ou un instant de balayage |

Ce qui les distingue, c'est **d'où viennent les fichiers et comment on
apprend qu'ils ont changé** — rien d'autre. D'où :

```rust
trait FileSource {
    fn list(&self) -> Vec<FileRef>;                      // chemin + hash + taille
    fn read(&self, path: &str) -> Result<Vec<u8>>;
    fn changes_since(&self, cursor: &Cursor) -> Changes; // inchangés / modifiés / nouveaux / disparus
    fn cursor(&self) -> Cursor;
}
```

Deux implémentations, `GitRef` et `WorkingTree`, et **un seul graphe
d'ingestion** qui ne sait pas laquelle il a devant lui. Un mode code peut
porter les deux : l'arbre pour le présent, git pour l'histoire (§5).

Le mode recherche « basé sur un git » qu'évoque Lucie est exactement `GitRef`.
Bonus décisif : **le problème de l'identifiant unique de février
n'existe pas ici.** Chemin + hash de blob, c'est l'identité ; `git diff`
donne le protocole de synchronisation (*93 inchangés, 2 modifiés, 5
nouveaux, 3 disparus*) sans qu'un humain désigne quoi que ce soit.

## 3. Ce que ça impose à `File` dès le premier jour

```
File {
  path          — relatif au projet (titre, indexé en BM25 — voir 01 §4, le titre
                  d'une entité simple n'est pas indexé aujourd'hui : à corriger ici)
  absolute_path — optionnel, absent en GitRef
  content_hash  — blake3 du contenu
  cursor        — commit (GitRef) ou instant/mtime (WorkingTree)
  language, lines_of_code, size_bytes
  project       — cloisonnement dès le premier jour
}
```

`File` **n'est jamais chunké** (idée de février qui a tenu) : c'est le
conteneur physique, la source de vérité des offsets. `Scope` porte
`start_byte` / `end_byte` **en plus** des lignes — l'information est
disponible gratuitement au point d'extraction (`node.start_byte()`,
`base_scope_extraction_parser.rs:2052`) et jetée aujourd'hui. Sans elle,
`read` d'un scope est une recherche de ligne fragile ; avec elle, c'est une
tranche.

## 4. Le point dur : la résolution des relations doit devenir incrémentale

`codeparsers` extrait les scopes **fichier par fichier** (parfait pour
« un fichier a changé »), mais résout les relations **globalement** :
`build_global_scope_mapping` indexe *nom → scopes* sur tous les fichiers,
puis résout chaque référence par recherche dans cet index
(`relationship_resolver.rs:244-274`). Un fichier changé = tout re-résoudre.

**Ce mapping global existe déjà : c'est la base.** `MATCH (s:Scope {name})`
*est* le résolveur. En faisant lire la base au lieu d'une map en mémoire :

- un fichier changé → ses scopes réécrits (UUID déterministes
  `blake3(file:name:type:sig)`, stables si les lignes bougent), ses relations
  sortantes recalculées contre la base, ses relations **entrantes**
  invalidées par un `MATCH ()-[r]->(s) WHERE s.file = path` ;
- le mapping n'est plus un état en mémoire à reconstruire — il est persistant,
  interrogeable, et il grandit avec le projet.

C'est la décision de conception la plus importante du chantier. Elle passe
par l'abstraction de recherche (pas de Cypher pour les agents ; ici c'est le
moteur qui interroge, via le `SearchBackend` / les nœuds de recherche —
`BM25SearchNode(fields='name', mode='parse')` sur `Scope` est déjà un
résolveur de noms).

Les deux maps vides du crate (`files`, `external_libraries`) et les
résolveurs d'imports jamais instanciés (2 150 lignes) sont à traiter dans ce
cadre : ce qu'on garde, c'est l'extraction et la logique de rapprochement ;
la table de correspondance, c'est la base.

## 5. L'histoire git, avec un horizon

Lucie : *« pourrait être intéressant aussi les historiques git, de savoir les
embedder avec TTL »*.

Oui, et ça rejoint **l'espace de tags temporel** du doc 49. Une entité
`Commit { sha, message, author, date }` reliée `TOUCHES` aux `File` / `Scope`
qu'elle modifie — `git blame` sous forme de graphe. Pour un agent, *« pourquoi
ce code est comme ça »* vaut autant que *« ce qu'il fait »*, et c'est la
seule des deux questions qu'il ne peut pas déduire du code.

Deux garde-fous :

- **On vectorise le message et la liste des scopes touchés, pas le diff.**
  Le diff est cher et bruyant ; le message plus « touche `fuse_signals`,
  `FuseResultsNode` » est ce qu'un humain cherche.
- **Le TTL est une politique de rétention à l'ingestion**, pas un minuteur
  par ligne : un horizon (N jours ou N commits) à `CommitSyncNode`, et un
  nœud d'expiration périodique qui supprime au-delà. Le graphe d'ingestion
  sait déjà supprimer (`DeleteRecordNode`, undo).

Avec `GitRef` comme source, l'histoire vient gratuitement dans les deux
modes.

## 6. La forme du graphe d'ingestion de code

```
sync["CodeSyncNode(source=…, cursor=…)"]        ── Changes : modifiés / nouveaux / disparus
  ├─► parse["ParseCodeNode"]                      ── par fichier → File, Scope, Scope_Chunk
  ├─► resolve["ResolveCodeRelationsNode"]         ── contre la base → CONSUMES, INHERITS_FROM, …
  ├─► history["CommitSyncNode(horizon=…)"]        ── optionnel → Commit, TOUCHES
  └─► delete["DeleteRecordNode"]                  ── disparus, et relations entrantes invalidées
```

`CodeSyncNode` est **la porte d'entrée des deux modes** ; tout le reste ne
sait pas si la source est un commit ou un disque. Et c'est la même forme que
le graphe de normalisation de tableurs à venir : une source, un diff, des
entités — le mode KB et le mode code convergent sur le même geste.

## 7. Ce qu'on ne décide pas ce soir

- Le débounce et la granularité de la surveillance (par fichier ? par
  rafale ?) — à mesurer sur un vrai dépôt.
- Si `grep` passe par lucivy sur un index de texte brut de `File` (rapide,
  mais un index de plus) ou par le disque (toujours juste, plus lent). Les
  deux tiennent derrière le même outil ; on commence par le disque.
- La forme du curseur `WorkingTree` quand il n'y a pas de surveillance
  (balayage des `mtime` : suffisant pour un `npm install` sans démon ?).
