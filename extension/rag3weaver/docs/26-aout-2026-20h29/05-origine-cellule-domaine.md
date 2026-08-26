# 05 — Origine, cellule, domaine : trois choses qu'on confond encore

26 août 2026, très tard. Suite du [04](04-une-racine-est-un-point-de-vue.md).
Lucie : « un projet, ça ne veut rien dire en soi ; un git dans le graphe
s'appellerait un git, sinon on met des barrières là où il n'y en a pas ».
Et : « tel autre projet a déjà référencé ce truc, je ne m'en rends même pas
compte et je réindexe ». Et : « domaine d'agent — ce que tel agent a dans sa
vision, indépendamment du projet ».

Trois objections, trois concepts. Ils ne se recouvrent pas.

## 1. Appeler un git un git

Le doc 04 proposait une entité `Root`. Le mot est déjà pris quatre fois, et
surtout il ment : il suggère qu'un projet **a** une racine, donc qu'un
projet **est** un dépôt. C'est une barrière inventée.

Donc : **`Origin`**, une entité à part entière, qui dit ce qu'elle est.

```
Origin { id, kind, revision? }
   kind = git      id = "github.com/L-Defraiteur/rag3db"
   kind = package  id = "npm:left-pad@1.0.0"
   kind = source   id = "snapshot:9f2c…"  |  "worktree:<uuid de poste>"
```

Ce qui en découle, et qui est le point de Lucie :

- un **projet contient des origines**, il n'en *est* pas une. Un projet peut
  en avoir zéro, une, ou dix-sept ;
- une **origine peut appartenir à plusieurs projets** — la même bibliothèque
  lue par deux équipes n'est pas deux bibliothèques ;
- un **git n'est pas un projet**. Un monorepo, c'est une origine et
  plusieurs projets ; un projet à trois dépôts, c'est l'inverse. Les deux
  doivent marcher, et avec ce dessin ils marchent tous les deux sans rien
  déclarer de spécial.

`local_path` n'est pas dans l'identité : c'est une **carte par poste**,
`Origin` ↔ chemin local. Le même dépôt cloné ailleurs retrouve son graphe.

## 2. « Un autre projet l'a déjà indexé, et je ne le sais même pas »

C'est vrai, et ça le restera en partie — mais il faut savoir **quelle
partie**, parce que la réponse dépend entièrement de la frontière franchie.

Le principe qui tranche tout le reste :

> **L'identité est globale, le contenu ne l'est pas.**

Une fois `Origin.id` portable et les chemins ancrés (doc 04), *le nom d'une
chose est le même partout*, y compris dans des cellules qui ne se voient
pas. Demander « est-ce que `github.com/x/y@abc123` est déjà indexé ? » est
une question sur un **nom**, pas sur des données. Elle ne fuit rien.

Trois frontières, trois réponses, et elles ne sont pas négociables entre
elles :

| Frontière | Ce qui se passe | Pourquoi |
|---|---|---|
| **même cellule** | rien à faire, c'est déjà là | — |
| **autre projet, même org** | on **peut savoir** que l'origine est indexée, et **demander** à l'attacher | l'org est la frontière de confiance ; c'est `across_scopes(by)`, déjà audité |
| **autre org** | on ne sait pas, on réindexe | c'est ce qu'isolation veut dire. Le coût est du calcul dupliqué, pas de la vérité dupliquée |

Le mécanisme minimal : un **catalogue d'origines au niveau de l'org** — des
noms et des révisions, jamais de contenu. « Cette org connaît
`github.com/x/y@abc123`, indexé dans le projet P le 12 août. » Puis une
action **délibérée et annoncée** pour l'attacher. Jamais automatique : un
index qui se met à contenir des choses qu'on n'a pas demandées est un index
dont on ne peut plus rien affirmer.

Et une limite qu'il faut dire tout de suite, parce qu'elle est structurelle :
**partager l'index n'est pas partager le fait**. Les index FTS et sparse sont
par cellule *exprès* — l'IDF de BM25 fuirait entre locataires (doc 37). Ce
qui se partage sans douleur : le fait qu'une origine soit indexée, sa
révision, son analyse (scopes et relations sont déterministes à partir du
contenu). Ce qui ne se partage pas : les statistiques d'index. Donc
« attacher » veut dire *recopier l'analyse et réindexer localement*, ce qui
reste dix fois moins cher que réanalyser — et surtout, ça reste **explicite**.

## 3. Le domaine d'agent : une **sélection**, pas un contenant

« J'ai indexé tout mon disque, je lance un agent, il est perdu. » Le
problème n'est pas la taille de l'index, c'est que la **vision** de l'agent
n'existe pas comme objet.

Deux façons de la faire exister, et une seule tient :

- **Un contenant** — encore une partition, dans laquelle on *met* des
  choses. Alors on a le même problème un cran plus haut : il faut y penser,
  et ça périme.
- **Une sélection** — un sélecteur nommé, sauvegardé, évalué à chaque usage.
  Ça compose avec tout ce qu'on a déjà : les filtres de recherche, la
  politique de lecture, les sélecteurs d'écoute du doc 14.

Donc un domaine, c'est :

```
Domain {
  origins:   [ "github.com/L-Defraiteur/rag3db" ],
  under:     [ "extension/rag3weaver/src" ],     // préfixes, dans ces origines
  languages: [ "rust" ],                          // facultatif
  since:     "7d",                                // facultatif
  plus:      derived                              // « ce que j'ai touché »
}
```

Trois règles pour qu'il ne devienne pas une cinquième « racine » :

1. **Un domaine ne donne jamais un droit.** Il *rétrécit* la vision à
   l'intérieur de ce que la politique de lecture autorise déjà. Vision et
   permission sont deux axes ; les confondre, c'est refaire l'erreur du 04.
2. **Le défaut n'est pas « tout ».** Un agent sans domaine déclaré voit
   **l'origine du fichier sur lequel il travaille**, pas le disque. C'est la
   réponse directe à « il est perdu » : le défaut est étroit et *dérivé*,
   pas large et déclaré.
3. **Le domaine se dit dans le rendu.** « 2 origines en vision ; 41 autres
   dans cette cellule hors du champ. » Sans ça, l'absence est invisible —
   et une absence invisible, c'est la famille de défauts qu'on passe nos
   journées à débusquer.

## 4. Les quatre axes, enfin distincts

| Axe | Question | Nature | Change quand ? |
|---|---|---|---|
| **Org** | à qui ça appartient ? | frontière de **confiance** | jamais, ou presque |
| **Cellule (projet)** | dans quel index ? | **partition** de données | à la création d'un projet |
| **Origine** | quel est ton nom ? | **identité** | jamais (c'est le point) |
| **Domaine** | qu'est-ce que je regarde ? | **sélection** | à chaque tâche |
| *(vue)* | comment je te l'écris ? | **rendu** | à chaque tour de boucle |

Rangés par vitesse de changement, du plus lent au plus rapide. C'est une
bonne façon de vérifier qu'on n'a pas confondu deux axes : **deux choses qui
changent à des rythmes différents ne sont pas la même chose.** L'ancre dans
la clé était une identité qui changeait à la vitesse d'un argument de ligne
de commande — voilà pourquoi ça cassait.

## 5. Ce que je ferais dans l'ordre

1. **`Origin` découvert par l'analyse** (doc 04, étape 1). Sans lui, rien
   au-dessus n'a de sens : ni le partage, ni le domaine.
2. **Le domaine comme sélecteur**, avec son défaut dérivé et son rendu qui
   dit ce qu'il ne montre pas. C'est ce qui rend un gros index utilisable —
   donc c'est ce qui a de la valeur tout de suite, même seul.
3. **Le catalogue d'origines par org**, et l'attachement explicite. Ça ne
   sert qu'à plusieurs, donc ça peut attendre d'être plusieurs.

Et une question ouverte que je ne tranche pas : est-ce qu'un domaine est
**un objet du graphe** (donc partageable, versionnable, attaché à un agent)
ou **un paramètre de session** ? Je penche pour le graphe — parce que « le
domaine dans lequel tel agent travaille » est exactement le genre de chose
qu'on veut pouvoir écouter, tracer et rejouer, et qu'on a déjà tout ce
qu'il faut pour ça.
