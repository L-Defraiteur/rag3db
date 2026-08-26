# 04 — Une racine est un point de vue, pas une identité

26 août 2026, tard. Lucie : « une racine devrait rester qu'un point de vue
même dans le graphe non ? » Oui. Et c'est parce qu'elle ne l'est pas
aujourd'hui que le même fichier peut exister deux fois.

## 1. Le constat, mesuré

`the_same_file_seen_from_two_roots_is_two_identities_today` :

```
ingest(root=/projet,     ["src/core.rs"])  → 1 File, 1 Scope
ingest(root=/projet/src, ["core.rs"])      → 2 File, 2 Scope
```

Le même fichier, le même contenu, la même machine. Deux identités, parce
que la clé porte le chemin **relatif à la racine passée à l'analyse**. La
racine était un argument d'appel ; elle est devenue une identité
permanente.

## 2. Le diagnostic : quatre choses portent le même nom

Le mot « racine » recouvre aujourd'hui **quatre notions différentes**, et
c'est ça le vrai défaut. Les séparer est la plus grande partie du travail :

| Notion | Question à laquelle elle répond | Où elle vit aujourd'hui |
|---|---|---|
| **La cellule** | *quel index ?* | `Scope { org, project }` — fait (doc 37) |
| **L'ancre** | *par rapport à quoi ce fichier se nomme-t-il ?* | **l'argument `root` d'`analyze`** — le défaut |
| **La permission** | *ai-je le droit de lire là ?* | `RootPolicy` — fait (doc 16) |
| **La vue** | *comment je te montre ce chemin ?* | nulle part — confondue avec l'ancre |

Trois sur quatre existent et sont correctes. C'est la deuxième qui est mal
placée, et la quatrième qui manque.

## 3. Ce que je propose : l'ancre se **découvre**, elle ne se passe pas

Une racine passée en argument est un accident de la ligne de commande. Un
fichier, lui, **sait où il habite** : la racine git qui le contient, à
défaut le manifeste le plus proche (`Cargo.toml`, `package.json`,
`pyproject.toml`, `go.mod`), à défaut la source elle-même.

```
/projet/.git/
/projet/src/core.rs      ancre = /projet    →  clé « src/core.rs »
```

Ingéré depuis `/projet` ou depuis `/projet/src`, **la même clé**, parce que
la question posée au fichier est la même. Le problème disparaît par
construction plutôt que par vigilance — et c'est le test ci-dessus qui
tombera pour nous le dire.

`analyze` garde son argument, mais il redevient ce qu'il aurait dû rester :
*ce que je te demande de lire*. Il ne décide plus des noms.

## 4. L'ancre est une **entité**, pas une chaîne

Deux dépôts ont chacun un `src/main.rs`. La clé doit donc contenir l'ancre —
mais `/home/lucied/git_workspaces/rag3db` ne veut rien dire sur une autre
machine, et c'est exactement pour ça que le chemin relatif avait été choisi.

Donc l'ancre devient un nœud, avec une identité **portable** et une
localisation **qui ne l'est pas** :

```
Root {
  id:         "github.com/L-Defraiteur/rag3db"   // portable : remote git, ou uuid
  local_path: "/home/lucied/git_workspaces/rag3db"  // par machine, jamais dans la clé
  kind:       git | manifest | source
}
```

`id` entre dans la clé, `local_path` non. Le même dépôt cloné ailleurs
retrouve le même graphe ; deux projets homonymes ne se marchent pas dessus ;
et un agent cloud et un agent local parlent enfin des mêmes symboles. Sans
remote — un dossier local qui n'est nulle part — l'`id` est un uuid tiré une
fois et gardé : local, mais stable.

C'est aussi là que la « politique par agent » trouve sa place, et elle
existe déjà : **la cellule** est ce qui est commun à un agent. Une cellule
peut contenir plusieurs `Root`. Quatre niveaux, quatre réponses :

> la cellule dit *dans quel index*, l'ancre dit *quel est ton nom*, la
> politique dit *ce que tu as le droit de lire*, la vue dit *comment je te
> l'écris*.

## 5. Le point de vue est un **rendu**, et rien d'autre

« Tiens, c'est de ce répertoire que j'aimerais voir les chemins
maintenant » : une option d'affichage, `relative_to`, sur `read`, `grep`,
`list` et le rendu des résultats. Ça ne touche ni la clé, ni le graphe, ni
l'index — et ça peut changer à chaque tour de boucle sans rien réindexer.
Le nœud de rendu existe déjà ; c'est un paramètre de plus.

Un agent qui travaille dans `src/dataflow/` verra `port.rs`. Le même graphe
vu depuis la racine dira `src/dataflow/port.rs`. Aucun des deux n'est
l'identité.

## 6. Ce que ça donne, cas par cas

| Scénario | Aujourd'hui | Avec l'ancre découverte |
|---|---|---|
| j'indexe un dossier, puis un sous-dossier | deux identités | une seule |
| j'indexe un dossier, puis son parent | deux identités | une seule |
| deux dépôts avec `src/main.rs` | collision si même racine | distincts par `Root.id` |
| le même dépôt sur une autre machine | inutilisable | même graphe |
| un fichier hors de tout projet | dépend de l'appel | ancré à la source, stable |
| « montre-moi les chemins d'ici » | réindexation | option de rendu |

## 7. Ce que je ne propose pas, et pourquoi

- **Identifier un fichier par le hash de son contenu.** Deux fichiers
  identiques (une licence, un `mod.rs` vide) deviendraient le même nœud.
  L'identité n'est pas le contenu — c'est tout le sens du §10.1 du doc 17.
- **Le chemin absolu comme clé.** Simple, juste, et inutilisable dès qu'un
  autre poste ou un conteneur entre en jeu.
- **Des alias : un fichier, plusieurs chemins connus.** Séduisant, mais
  l'identité par union ne converge pas : fusionner après coup deux
  sous-arbres déjà indexés est une réécriture, pas une migration.
- **Laisser l'agent choisir sa racine.** C'est ce qu'on fait, et c'est
  précisément la cause. Un agent choisit ce qu'il *regarde* ; il ne choisit
  pas comment le monde s'appelle.

## 8. L'ordre, et ce que ça coûte

1. **`Root` découvert et porté par l'analyse** — remonter jusqu'à `.git`,
   puis manifeste, puis source ; une clé par fichier, pas par appel. C'est
   la moitié du gain, et c'est contenu dans `analyze`.
2. **`Root` comme entité**, avec `id` portable et `local_path` par machine.
   Débloque le cloud, et la promotion du doc 16 (« tu touches souvent à ce
   dépôt, je l'ingère depuis sa racine ? ») écrit dedans naturellement.
3. **`relative_to` au rendu** — une demi-journée, aucun risque, et ça enlève
   la dernière raison de vouloir bricoler la clé.

Ce qui reste ouvert et que je ne tranche pas seul : faut-il **une** cellule
par dépôt, ou une cellule qui en contient plusieurs ? Les deux marchent avec
ce dessin. Une par dépôt isole mieux (BM25 compris) ; une pour plusieurs
laisse chercher à travers un monorepo *et* ses voisins d'un coup. C'est un
choix de produit, pas de moteur — et il se prend quand on saura à quoi
ressemble `lucyCode`.
