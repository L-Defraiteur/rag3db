# Avaler une base existante

**30 août 2026.** Lucie, en la donnant pour naïve : *« un genre d'ingesteur qui
convertit une base étrangère en compat rag3weaver, parmi les backends
supportés, détecte les tables de liaison etc., et tente de reconstruire un
graphe »*.

Elle l'est moins qu'elle n'en a l'air — à condition de ne jamais confondre ce
qui est **déclaré** et ce qui est **deviné**.

## 1. Pourquoi c'est la suite logique, et pas une idée de côté

La boucle étrange, telle qu'elle est aujourd'hui, part d'une page blanche : un
agent pose des gabarits, écrit du code, lance un backend. C'est utile pour un
projet neuf et **inutile pour une entreprise qui en a déjà un**.

Lucie l'a dit dans la même conversation : *« une boucle étrange qui ne sert que
kuzu, ça ne marche pas pour les projets en prod réels »*. Le même argument vaut
un cran plus loin : une boucle qui ne sait pas lire ce qui existe ne sert
qu'aux projets qui n'existent pas encore.

**C'est la rampe d'accès.** « Pointe-le sur ta base, il te dit ce qu'il a
trouvé, tu corriges, et l'agent connaît ton domaine. »

## 2. La partie qui n'est pas devinée

Trois choses se **lisent**, elles ne s'infèrent pas :

| ce qu'on lit | d'où |
|---|---|
| les tables, les colonnes, leurs types | `information_schema` |
| les clés primaires | idem |
| les clés étrangères, avec leurs deux bouts | idem |

Une clé étrangère déclarée **est** une relation. Il n'y a rien à deviner : le
schéma le dit. Sur une base soignée, l'essentiel du graphe se lit.

**Et la table de liaison est une règle, pas une intuition** : une table dont
les colonnes sont exactement deux clés étrangères — plus éventuellement une clé
technique et des horodatages — est une **arête**, pas un nœud. C'est vérifiable,
et les faux positifs sont rares.

C'est la moitié solide, et elle est plus grosse qu'on ne croit.

## 3. La partie devinée, et il faut la nommer

**Quelles colonnes portent du texte cherchable ?** C'est là que ça devient de
l'inférence, et c'est *la* question qui décide si le résultat sert à quelque
chose : un graphe sans contenu est un graphe que personne n'interroge.

Les indices sont faibles et se combinent mal : le nom (`title`, `name`,
`description`, `body`), le type (`TEXT` plutôt que `VARCHAR(50)`), la longueur
moyenne réelle des valeurs. Aucun ne suffit.

**Les schémas sans clés étrangères.** Très courants — des ORM qui ne les
déclarent pas, des conceptions partitionnées. Il reste le nommage
(`user_id` → `users.id`), et c'est franchement de la devinette.

## 4. La règle qui rend l'ensemble utilisable

> **L'ingesteur propose un schéma, il ne l'impose pas.** Il rend un brouillon
> où **chaque décision porte comment elle a été prise**.

C'est exactement la distinction de la porte des commandes — `Fondement` sépare
« l'utilisateur l'a dit » de « ça semble inoffensif » — appliquée ici :

```rust
enum Provenance {
    /// Le schéma le déclare. On ne l'a pas inventé.
    Declaree,
    /// Déduit d'une règle vérifiable (table de liaison, deux clés étrangères).
    Deduite { regle: String },
    /// Deviné, sur des indices faibles. **À relire.**
    Devinee { indice: String },
}
```

Sans ce champ, un brouillon est un tas d'affirmations d'autorité égale, et
personne ne sait quoi relire. Avec, la relecture devient courte : on lit les
`Devinee`, on fait confiance aux `Declaree`.

## 5. Ce que ça rend, et pourquoi ce n'est pas un mécanisme de plus

La sortie n'est pas un format nouveau : ce sont des **gabarits d'entités** —
la famille `entity` du catalogue (doc 08). Donc :

- on les **cherche** comme le reste ;
- on les **pose** avec `place`, un par un, en choisissant ;
- on les **modifie** avant de poser, puisque c'est du texte ;
- et ce qui a bien marché sur une base se **réutilise** avec `adopt`.

Rien à construire pour l'aval. L'ingesteur remplit un catalogue existant.

## 6. Ce qui casse, et qu'il faut décider d'avance

**Le volume.** Une base de production n'est pas un dépôt. Tout embarquer, c'est
des millions de lignes et un coût sans rapport avec le service rendu. L'ingestion
doit donc être **sélective par construction** — on choisit des tables, pas une
base — et incrémentale. Ce n'est pas un réglage à ajouter après : c'est la
forme de l'outil.

**La lecture seule, et sans discussion.** Pointer sur la base de quelqu'un et y
écrire serait impardonnable. La connexion doit être en lecture seule *au niveau
du compte*, pas de notre bonne volonté — c'est ce que
`Rag3dbConnection::read_only` fait déjà chez nous, et l'équivalent existe
partout.

**Les identifiants.** Une base de prod se joint avec des secrets, et on vient
de passer une nuit à faire attention à ceux-là. Ils ne doivent pas transiter par
un agent, ni finir dans une trace.

**Ce qui n'est pas un graphe.** Une table de journal de 400 millions de lignes
n'est pas une entité : c'est un flux. La proposer comme un nœud produirait un
catalogue absurde. Un seuil, ou une catégorie « ignoré, dis-moi si j'ai tort ».

## 7. Où ça peut aller — la réponse à la question de Lucie

Trois étages, du plus sûr au plus ambitieux :

1. **Lire et proposer.** Le brouillon commenté du §4. Utile tout de suite,
   faisable sur ce qui est déclaré, et sans risque puisque rien n'est appliqué.
2. **Chercher dans la base d'origine.** Une fois le schéma posé, la recherche
   hybride sur les données réelles — sans copier la base, en interrogeant à
   travers `DbConnection`. C'est ce que le backend Postgres permettrait s'il
   était éprouvé.
3. **La boucle étrange sur un domaine existant.** Un agent qui connaît le
   schéma d'une entreprise, et qui construit *dans* ce domaine plutôt qu'à côté.
   C'est là que ça cesse d'être un jouet de page blanche.

Le premier étage se tient tout seul et vaut déjà le travail. Les deux autres en
dépendent, et aucun n'est possible sans que le chemin Postgres soit **éprouvé**
— aujourd'hui il compile, il a 944 lignes de dialecte, et **zéro test E2E**.

C'est donc l'ordre : d'abord prouver le backend, ensuite lire une base
étrangère, ensuite seulement y faire travailler un agent.
