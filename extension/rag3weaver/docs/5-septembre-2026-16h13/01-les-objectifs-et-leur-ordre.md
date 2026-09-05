# Les objectifs, et leur ordre

**5 septembre 2026.** Écrit après la relecture de la série vision
([`vision_roadmap_09_2026`](../vision_roadmap_09_2026/00-index.md)), qui a dit
où on en est. Celui-ci dit ce qu'on fait ensuite, et **pourquoi dans cet
ordre** — c'est l'ordre qui a été discuté, pas la liste.

## L'ordre, et le renversement qui le décide

J'avais proposé la lecture des documents en premier. Lucie a inversé :

> les écrivains multi-processus d'abord — fiabiliser la faisabilité d'une
> solution cloud avant de faire cinquante passes de features.

**Elle a raison, et la raison est structurelle.** La lecture des documents est
**additive** : elle ajoute de la matière à un moteur qui marche. Les écrivains
multi-processus sont **porteurs** : c'est une hypothèse d'architecture. Si elle
ne tient pas, tout ce qu'on empile au-dessus est à reprendre — et cinquante
passes de features, c'est exactement l'empilement qui rend une reprise
impossible.

C'est le même principe qui a fait passer le second backend devant le reste :
*prouver ce qui porte avant d'ajouter ce qui pèse*. Je l'avais appliqué aux
backends et sous-appliqué ici.

## 1. Les écrivains multi-processus — **la question à poser avant la tâche**

### Elle est peut-être déjà résolue, et il faut le savoir d'abord

**PostgreSQL accepte plusieurs écrivains, nativement.** C'est ce que fait toute
base serveur. Si le produit cloud tourne sur PostgreSQL — ce qui est
l'hypothèse la plus probable pour du multi-locataire élastique — alors la
faisabilité est **déjà acquise**, et ce qui reste à faire est de le *mesurer*
plutôt que de le construire.

Le mur du verrou unique est celui de **rag3db**, une base *embarquée*. C'est une
propriété de cette famille-là, pas un défaut : SQLite et DuckDB font pareil.

**Donc la première question n'est pas « comment faire plusieurs écrivains »,
c'est « sur quel backend tourne le produit cloud ».** Selon la réponse :

| si le cloud tourne sur… | ce qu'il reste à faire |
|---|---|
| **PostgreSQL** | mesurer plusieurs catalogues écrivains sur la même base : cloisonnement, files d'ingestion concurrentes, marque d'eau à plusieurs, cohérence des index lucivy/sparse partagés |
| **rag3db** | un gestionnaire de verrous inter-processus, précédé de la relecture du MVCC de Vela |
| **les deux** | les deux, et le second est de loin le plus cher |

Une demi-journée de mesure sur PostgreSQL peut donc rendre la moitié du sujet
sans écrire de moteur. C'est par là qu'il faut commencer.

### Ce qu'on sait déjà, et qui ne suffit pas

Trois configurations, et l'état au 5 septembre :

| | état |
|---|---|
| plusieurs fils écrivains, un processus | oui, par le report d'un fork — **non adopté** |
| plusieurs processus **lecteurs** | **oui, mesuré** — 80 ouvertures pendant qu'on écrit, zéro refus |
| plusieurs processus **écrivains** | **non**, sur rag3db |

Ce qui a été bâti pour les lecteurs sert déjà les écrivains le jour venu : la
**marque d'eau d'ingestion** publie ce qu'un processus n'a pas encore vidé, et
elle est écrite pour plusieurs écrivains — une marque par écrivain, avec
péremption. Elle n'a été **éprouvée qu'à un seul**.

### Le préalable qui n'est pas à nous

Sur rag3db, le plancher est le **MVCC de Vela** : rotation de WAL, points de
reprise non bloquants. **Personne ne l'a relu.** On ne pose pas un gestionnaire
de verrous inter-processus sur une couche de stockage qui suppose un écrivain
unique — et c'est la session du cœur C++ qui est chez elle sur ce terrain.

## 2. La lecture des documents

pdf, docx, pptx, html, csv. Sans bibliothèque lourde : les formats Office sont
du ZIP + XML, et l'OCR livré couvre déjà les PDF scannés.

C'est le plus vieux point non tenu de la feuille de route, et le seul dont
l'absence rend inutile une bonne part du reste : on a rendu la recherche juste,
mesurée, cloisonnée et portable **sur du contenu qu'on ne sait pas ingérer**.

Additif, donc second — mais premier parmi les additifs.

## 3. `Catalog::search` devient un gabarit

Pas pour l'élégance. **La dette a un prix mesuré** : 409 lignes dans un
`catalog.rs` de 6 373, et le chemin composable existe en parallèle sans être
emprunté. Cette semaine, chaque correction de recherche a dû être portée aux
**deux** chemins — le filtre utilisateur, la marque de confiance, le dialecte
pour la résolution des chunks. L'une l'a été à moitié, et ça s'est trouvé par
accident.

Le bon moment n'est pas « maintenant » dans l'absolu : c'est **la prochaine fois
qu'on rouvre la recherche**. Le noter pour que ce moment ne passe pas inaperçu.

## 4. Le graphe de normalisation des tableurs

La porte d'entrée de toute la moitié KB. Un tableur est la forme sous laquelle
un catalogue arrive, neuf fois sur dix.

La spécification de février existe, et la pratique la corrige sur quatre points
([03](../vision_roadmap_09_2026/03-normaliser-des-tableurs.md) §9) : phases
persistées, deux représentations, renversement d'échelle, et une décision
explicite sur l'identité des lignes. C'est du travail **conçu**, pas à
concevoir.

## 5. Avaler une base étrangère

Le [13](../vision_roadmap_09_2026/13-avaler-une-base-existante.md) attendait
qu'un backend SQL existe pour cesser d'être une idée. Il existe.

**Après les tableurs**, et c'est une décision de Lucie déjà prise : la question
« quelles colonnes portent du texte cherchable » se rencontre d'abord là, sur un
terrain plus simple.

## Les petits, à faire au passage

Trois choses qui ne méritent pas leur journée mais qui mordront :

- **La troncature d'embedding.** Le dernier silence pur qui reste après une
  semaine passée à les supprimer : un chunker qui ignore la fenêtre du modèle
  tronque sans un mot. La cause est nommée dans le code — le trait `Embedder`
  n'expose ni fenêtre ni compte de jetons, le budget est en caractères parce que
  le tokenizer vit ailleurs. **Un avertissement avant une correction.**
- **`crate::acces` n'a aucun appelant en production.** Écrit hier, éprouvé par
  des tests, et rien d'autre ne s'en sert : c'est le « construit et jamais
  appelé » qu'on vient d'inscrire comme corollaire d'invariant dans le
  [01](../vision_roadmap_09_2026/01-la-vision.md) §4. Son appelant naturel
  manque aussi — rien ne permet d'ouvrir un catalogue **en lecture**.
- **Le miroir Rust de `search_base`.** Dernière duplication tenue à la main :
  70 lignes de test qui recopient `search_base.mmd` nœud par nœud. Le
  **générer serait pire** — il prouverait que l'analyseur est d'accord avec
  l'analyseur. Le bon geste est un fixture minimal découplé du graphe de
  production.

## Ce qui n'est pas dans cette liste, et pourquoi

- **La parole (TTS/STT/G2P)** — reportée avec l'**interface** du produit « agent
  de code ». Décision de Lucie, pas un oubli.
- **Neo4j** — en dernier, pas en deuxième. Il parle Cypher, donc il masquerait
  les fuites de représentation qu'un backend SQL révèle ; il restera bon marché
  précisément pour ça
  ([15](../vision_roadmap_09_2026/15-le-moteur-cesse-d-etre-mono-backend.md) §2).
- **Le renommage `kuzu` → `rag3db`** — 2 899 fichiers à chaque reprise de
  l'amont, pour un bénéfice que personne n'a énoncé. Tant qu'il ne l'est pas, ça
  reste une dépense récurrente sans contrepartie.
