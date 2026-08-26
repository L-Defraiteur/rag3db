# 04 — Une racine est un point de vue, pas une identité

26 août 2026, tard, révisé dans la nuit du 27. Lucie : « une racine devrait
rester qu'un point de vue même dans le graphe non ? »

Oui. Et il a fallu trois versions pour l'écrire vraiment — la trace des trois
vaut mieux que la conclusion seule.

| Version | Identité d'un fichier | Ce qui l'a tuée |
|---|---|---|
| v1 (avant) | chemin relatif à la **racine d'analyse** | le point de vue *était* l'identité : deux racines, deux fichiers |
| v2 (première nuit) | `(origine découverte, chemin dans l'ancre)` | l'origine reste dans la clé : elle n'est donc toujours pas un point de vue |
| **v3 (retenue)** | `(source, chemin absolu dans cette source)` | — |

## 1. Le constat qui a lancé tout ça

`the_same_file_seen_from_two_roots…` :

```
ingest(root=/projet,     ["src/core.rs"])  → 1 File, 1 Scope
ingest(root=/projet/src, ["core.rs"])      → 2 File, 2 Scope
```

Le même fichier, le même contenu, la même machine. Deux identités, parce que
la clé portait le chemin **relatif à la racine passée à l'analyse**. La
racine était un argument d'appel ; elle était devenue une identité
permanente.

## 2. Quatre choses portaient le même nom

C'est le vrai défaut, et le nommer est la plus grande partie du travail :

| Notion | Question | Où elle vit |
|---|---|---|
| **La cellule** | *quel index ?* | `Scope { org, project }` — fait (doc 37) |
| **La source** | *d'où viennent ces octets ?* | `FileSource::cursor` — existait déjà |
| **La permission** | *ai-je le droit de lire là ?* | `RootPolicy` — fait (doc 16) |
| **La lentille** | *par rapport à quoi je te l'écris ?* | nulle part — confondue avec l'identité |

## 3. v2, et pourquoi elle ne tenait pas

La deuxième version faisait **découvrir** l'ancre (le dépôt git, sinon le
manifeste, sinon le système de fichiers) et mettait son identité dans la
clé : `git:github.com/org/dépôt` + `src/x.rs`.

C'était mieux que v1 — les deux racines convergeaient — mais Lucie a mis le
doigt sur ce qui n'allait pas : *« moi j'aurais aimé que File ait un path
absolu toujours, et que qui veut un relatif le calcule depuis une origine »*.
Elle avait raison, et pour une raison qu'on peut énoncer sèchement :

> **Si l'origine est dans la clé, l'origine n'est pas un point de vue.**

Trois conséquences concrètes de v2, toutes mauvaises :

1. **Changer ce qui compte comme origine réécrit toutes les clés.** Le jour
   où un dépôt gagne un remote, où un sous-module apparaît, où on décide
   qu'un monorepo s'ancre au paquet plutôt qu'au dépôt : réindexation
   complète. Une décision de *vue* ne doit pas coûter une réindexation.
2. **Les origines imbriquées forcent un choix arbitraire.** Un sous-module
   dans un dépôt, un paquet dans un monorepo : v2 doit élire une ancre. Il
   n'y a pourtant aucune raison de choisir — on veut pouvoir calculer le
   relatif depuis l'une *ou* l'autre selon la question posée.
3. **La découverte est une heuristique**, avec ses règles (« le dépôt
   l'emporte sur le manifeste »), son repli, et ses pièges. J'en ai écrit
   deux versions fausses en une soirée. Une identité ne devrait pas dépendre
   d'une heuristique.

## 4. v3 — l'identité est ce d'où viennent les octets

> **`(source, chemin absolu dans cette source)`.**

- Fichier local : `source = "file"`, `path = /home/lucied/…/x.rs`. C'est le
  chemin absolu, toujours, exactement ce que Lucie demandait.
- Instantané : `source = "snapshot:abc123"`, `path = foo.rs`.
- Dépôt distant à une révision : `source = "git:github.com/o/r@rev"`,
  `path = src/x.rs`.

Aucune découverte, aucune heuristique, aucune règle de priorité : la source
est **connue** au moment de l'ingestion, c'est `FileSource::cursor` qu'on
avait déjà. Deux racines d'analyse donnent le même chemin absolu, donc la
même identité — la propriété est acquise **par construction**, pas par
mécanisme.

Pourquoi pas le chemin absolu tout seul, littéralement ? Parce qu'un
instantané et un dépôt distant n'en ont pas. Dès qu'on accepte ça, on a
besoin de dire *dans quoi* le chemin est absolu — et c'est exactement la
source. C'est la seule objection technique à la version littérale, et elle
suffit.

## 5. L'origine devient une lentille

Elle ne fabrique plus rien. Elle se pose — découverte (le dépôt qui contient
ce fichier) ou déclarée (« à partir d'ici ») — et elle **calcule** :

```
Origin { id, kind, anchor }        // anchor : un préfixe, sur ce poste
origin.relative("/home/…/rag3db/src/x.rs") → "src/x.rs"
```

Trois propriétés qui découlent de « ça ne fabrique rien » :

- **On peut en poser autant qu'on veut**, imbriquées, et en changer d'avis :
  aucune clé ne bouge.
- **Pas de mémoïsation.** Garder les relatifs dans l'origine ajouterait une
  invalidation à tenir (un fichier bouge, une ancre bouge) pour économiser un
  `strip_prefix` — des nanosecondes. Le calcul à la volée est plus simple
  *et* toujours juste.
- **C'est un paramètre de rendu**, pas de stockage : « montre-moi les chemins
  depuis ici » ne réindexe rien et peut changer à chaque tour de boucle.

## 6. La portabilité est une **arête**, pas une convention de nommage

Le seul vrai coût de l'absolu : `/home/lucied/…` ne veut rien dire ailleurs.
La réponse de v2 était de le cacher dans le nom. La réponse de v3 est
d'utiliser ce qu'on est — une base de graphe :

```
(file:/home/lucied/git_workspaces/rag3db/src/x.rs) -[:SAME_AS]-> (git:github.com/L-Defraiteur/rag3db@a1b2c3#src/x.rs)
```

Deux clones, deux arbres de travail, le poste de Lucie et un conteneur CI
pointent alors vers **la même identité portable, par une arête**, sans
qu'aucune clé ne soit réécrite. Et on choisit de chercher à travers, ou pas.

Ça règle du même coup un cas que v2 traitait mal dans l'autre sens : **deux
arbres de travail du même dépôt sur deux branches** — il y en a un sur ce
poste — sont deux sources distinctes, donc deux graphes, et on les relie *si
on le veut*. v2 les fusionnait d'office.

Coût honnête : un saut de plus quand on veut la portabilité. Mais c'est le
chemin rare (partage, cloud) ; le chemin courant — travailler en local —
devient maximalement simple.

## 7. Ce que ça donne, cas par cas

| Scénario | v1 | v3 |
|---|---|---|
| j'indexe un dossier, puis un sous-dossier | deux identités | une seule, sans mécanisme |
| j'indexe un dossier, puis son parent | deux identités | une seule |
| deux dépôts avec `src/main.rs` | collision possible | distincts (chemins absolus) |
| deux arbres de travail, deux branches | confondus si même racine | distincts, reliables par `SAME_AS` |
| le même dépôt sur une autre machine | inutilisable | relié par `SAME_AS`, sans réécriture |
| un fichier hors de tout projet | dépend de l'appel | son chemin absolu, point |
| « montre-moi les chemins d'ici » | réindexation | option de rendu |
| un dépôt gagne un remote | — | rien ne bouge (v2 : tout à réindexer) |

## 8. Ce que je n'ai pas retenu, et pourquoi

- **Le hash du contenu comme identité.** Deux fichiers identiques (une
  licence, un `mod.rs` vide) deviendraient le même nœud. C'est l'erreur
  qu'on venait justement de retirer sur les scopes (doc 17 §10.1).
- **Des alias : un fichier, plusieurs chemins connus, fusionnés d'office.**
  L'identité par union ne converge pas. `SAME_AS` fait la même chose *en le
  disant*, et laisse le choix de traverser ou non.
- **Laisser l'agent choisir sa racine d'identité.** C'était v1, et c'était la
  cause. Un agent choisit ce qu'il *regarde* ; il ne choisit pas comment le
  monde s'appelle.
- **Mémoïser les relatifs dans l'origine** (§5).

## 9. L'ordre

1. **`(source, chemin absolu)` comme identité** — remplace v2, déjà câblée.
   La traduction dans la couche outils *disparaît* : l'absolu est canonique.
2. **`relative_to` au rendu** — devient nécessaire, et plus seulement
   souhaitable : les chemins stockés sont longs.
3. **`SAME_AS`** quand le partage arrivera pour de vrai — pas avant, c'est
   une arête qu'on peut ajouter à tout moment sans rien réécrire. C'est
   d'ailleurs la meilleure preuve que v3 tient : *ce qui reste à faire ne
   demande aucune migration.*

---

## 10. v4 — les deux en parallèle, et la politique d'identité

Ajouté dans la foulée, sur une remarque de Lucie : *« pourrait y avoir les
deux en parallèle, le chemin git quand dispo et le chemin absolu quoi qu'il
arrive — comme ça tu reviens, hop, ce git a déjà été cloné ailleurs sur ton
PC, tu le reclones, ça fait le merge avec idempotence ? »*

Oui, et ça change la nature de la chose.

### 10.1 Une identité canonique, des coordonnées dérivées

Pas « une identité **ou** l'autre » : **une clé + des faits calculés**.

- la clé : `(source, chemin absolu)` — toujours là, aucune heuristique ;
- les **coordonnées** : d'autres façons de nommer le même fichier, produites
  par les systèmes qui savent. Git sait dire « c'est `src/x.rs` dans
  `github.com/org/dépôt` ».

Deux clones du même dépôt ont donc **deux clés** (ce sont deux fichiers sur
le disque, c'est vrai) et **la même coordonnée** — le rapprochement se fait
tout seul, sans que personne ne déclare quoi que ce soit. Le `SAME_AS` du §6
devient **dérivable au lieu d'être déclaré**, ce qui est strictement mieux.

Et ça répare l'objection à v2 : là-bas la coordonnée git *était* la clé, donc
le jour où un dépôt gagne un remote, tout était à réindexer. En coordonnée,
ce jour-là, **un champ se remplit**.

Deux précisions qui décident :

- **La coordonnée ne porte pas la révision** : `git:org/dépôt#src/x.rs`. Ce
  qu'on identifie ainsi, c'est « ce fichier dans ce dépôt », pas « cet
  état » — sinon deux clones sur deux commits ne se reconnaîtraient jamais.
  La révision est une propriété, comme `content_hash` et `cursor`.
- **Le rapprochement se fait à la lecture, pas au stockage.** On garde deux
  nœuds et on regroupe par coordonnée en préférant le plus frais. Fusionner
  physiquement referait l'erreur de l'identité par union, et écraserait le
  cas « deux arbres de travail sur deux branches », qu'on veut distincts.

### 10.2 Le vrai objet variable : la **politique d'identité**

Lucie, encore : *« ouais mais un gloubiboulga sous une configuration
différente veut être un gestionnaire de commits non ? »* — et c'est le point
qui va plus loin que la coordonnée.

Si la révision n'est qu'une propriété, ingérer le même dépôt à deux
révisions produit **un seul nœud** qui s'écrase. Un gestionnaire de commits
en veut un **par révision**. Ce qu'il veut n'est donc pas une coordonnée de
plus : c'est **la révision dans la clé**.

| Politique | `hashsafe` | Un nœud par |
|---|---|---|
| copie de travail (défaut) | `["source", "path"]` | fichier |
| gestionnaire de commits | `["repo", "revision", "repo_path"]` | fichier × révision |

Et voilà pourquoi il n'y a pas de nouveau moteur à écrire : **`hashsafe`
*est* la politique d'identité**, et elle est déjà configurable par entité.
Il ne manquait que les coordonnées comme **champs**.

Deux choses à dire franchement plutôt que de les laisser découvrir :

1. les deux politiques ne cohabitent pas dans une même cellule pour une même
   entité — un schéma par entité et par catalogue. La contrainte tombe du
   bon côté : une archive de commits et un index de travail sont deux
   usages, ils méritent deux index (doc 05 : « la cellule dit dans quel
   index ») ;
2. le mode gestionnaire de commits est **multiplicatif** — chunks,
   embeddings et index par révision. C'est pourquoi ce doit être une
   configuration délibérée, jamais un défaut.

### 10.3 Souscrire plutôt qu'énumérer — et quand *ne pas* le faire

Lucie encore : *« peut-être c'est un pattern générique qu'on rate déjà, à
séparer en un gros `if` alors que c'est une politique avec des systèmes qui
y souscrivent ? »* Oui, et pas seulement ici : `Origin::discover` (git /
manifeste / dossier / source), `detect_relationship_type_by_name` (rust /
ts / py / go / cpp), `language_name` par extension. Chaque fois, un système
qui *sait quelque chose* est enfermé dans une branche d'un `match`.

Le critère pour y aller — et il n'est pas « c'est plus propre » :

> **Un système extérieur au crate a-t-il besoin d'ajouter un cas sans éditer
> le `match` ?**

Pour les coordonnées : évidemment oui (git aujourd'hui, un registre de
paquets ou un stockage adressé par contenu demain). Pour le genre de
relation par langage : non — le parseur *possède* les langages, et l'en
sortir n'achèterait rien. Et ce n'est même pas un patron nouveau chez nous :
`NodeRegistry` / `NodeFactory` font déjà exactement ça pour les nœuds.

### 10.4 Ce qui reste ouvert, et qu'on assume

Lucie : *« on trouvera une solution plus tard, peut-être que ça ne fait
jamais partie du même project id mais qu'on pourra trouver des ponts quand
même — des entités ou relations multi-projets, ou même un projet qui sert de
pont, ou pire une entité aurait un `policyDomain` »*.

Trois pistes, notées telles quelles, sans en choisir une :

- des **relations** qui traversent les projets, sans que les entités changent
  de cellule ;
- un **projet-pont**, dont le seul rôle est de tenir ces relations ;
- une **politique par entité** (`policyDomain`), qui serait le cas le plus
  puissant et le plus dangereux — c'est-à-dire à n'ouvrir qu'avec un besoin
  mesuré devant.

Ce qui rend cette indécision confortable : **aucune de ces trois voies ne
demande de migration**. Ce sont des arêtes et des configurations sur une
identité qui, elle, ne bouge plus.
