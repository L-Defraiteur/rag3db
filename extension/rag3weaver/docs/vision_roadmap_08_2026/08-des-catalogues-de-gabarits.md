# 08 — Des catalogues de gabarits

**Écrit sous la dictée de Lucie, 29 août 2026.** Ses mots :

> Pour la boucle étrange je pense qu'il faut des catalogues de templates aussi
> à chaque fois. Un backend, il faut justement pas qu'ils aient à tout coder,
> donc on leur prépare d'avance des `user`, `conversation`, `product`… Ils
> peuvent modifier les schémas mais ont une librairie de templates. Pareil
> pour le front : on leur pré-fait des templates de composants React qu'ils
> peuvent insérer où ils veulent, ils peuvent en enregistrer aussi, et bien sûr
> modifier le code une fois le template posé.
>
> Je pense que c'est le meilleur but car ça touche tout d'un coup, notre
> meilleure cible immédiate.

## 1. Pourquoi c'est la bonne cible

Le doc [01](01-la-vision.md) décrit une boucle : l'agent vit dans la base, la
gère, construit avec la technologie dont il est fait, et ce qu'il construit y
retourne. Jusqu'ici on en avait des morceaux — les outils sont des graphes
sérialisables, les schémas sont des données, la recherche est hybride.

**Un catalogue de gabarits ferme la boucle** au lieu d'y ajouter un morceau. Il
demande, en même temps :

| ce qu'il faut | ce qu'on a déjà |
|---|---|
| ranger des gabarits dans la base | les entités, `to_mermaid`, l'empreinte BLAKE3 |
| les **retrouver** | BM25 + vecteur + rerank, réparés le 28 août |
| les **adopter sous un nom** | `GraphToolRegistry::attach` — le nom vit sur l'arête |
| un schéma modifiable après coup | `EntityConfig` déclaratif, `lifecycle`, `SchemaCost` |
| produire quelque chose de visible | — c'est ce qui manque, et c'est le but |

Rien de cette colonne de droite n'a été construit pour ça. C'est le signe qu'on
tire sur le bon fil.

## 2. Ce que c'est, concrètement

**Un gabarit n'est pas un générateur.** On ne demande pas à un agent d'écrire
un modèle `User` depuis rien, et on ne lui donne pas non plus un `User` figé.
On lui donne un **point de départ nommé**, qu'il pose, puis qu'il modifie.

Trois familles, la même mécanique :

- **Gabarits d'entités** — `user`, `conversation`, `product`, `document`,
  `event`… Chacun est un `EntityConfig` : champs, types, signaux de recherche,
  découpage, cycle de vie. C'est **déjà de la donnée déclarative** ; en faire
  un catalogue ne demande pas un moteur de plus.
- **Gabarits de graphes** — ils existent : `templates/tools/*.mmd`. Le 28 août
  on a séparé ce qu'un gabarit *est* de ce qu'un agent en *voit*, précisément
  pour qu'un catalogue soit possible.
- **Gabarits de composants** — React, côté front. Territoire neuf, même forme :
  on pose, on branche, on modifie.

**Quatrième famille, et elle est transversale : les motifs.** (Lucie, même
jour :) *« la version c'est aussi un template intéressant de pattern, c'est
utilisé souvent, exemple version d'un agent »*.

Un motif ne remplace pas une entité, il **s'applique** à une entité :

| motif | ce qu'il ajoute |
|---|---|
| **versionné** | une entité par révision, l'identité qui porte la révision, « la courante » comme une vue |
| **effacé en douceur** | un état plutôt qu'une suppression — et [`Lifecycle`](../27-aout-2026-13h01/12-architecture.md) sait déjà l'exprimer |
| **audité** | qui a fait quoi, quand ; le `Run` et le `Participant` existent |
| **possédé** | un propriétaire, et donc un filtre par défaut |
| **étiqueté** | des tags, cherchables comme le reste |

C'est la famille la plus rentable, parce qu'un motif sert **à chaque fois** là
où un gabarit d'entité sert une fois. Et « versionné » est déjà à portée : le
doc [04](04-le-catalogue-comme-graphe.md) montre qu'`hashsafe` **est** la
politique d'identité — un `File` versionné, c'est
`hashsafe: ["repo", "revision", "repo_path"]` au lieu de `["source", "path"]`.
Même moteur, même schéma, autre configuration. Le motif consiste à nommer ce
choix et à le rendre posable, pas à écrire un mécanisme.

Et l'exemple de Lucie referme la boucle du doc [01](01-la-vision.md) d'un cran
de plus : **un agent versionné**. L'agent est un sous-graphe, donc une donnée,
donc versionnable dans la base où il vit — et on peut demander « qu'est-ce que
tu faisais avant-hier » à quelque chose qui est à la fois le sujet et le
substrat.

**Et l'agent en enregistre.** Un gabarit qu'il a écrit rejoint le catalogue, et
le suivant le trouvera. C'est la phrase du doc 01 — *les usages sont des
données qu'il fabrique et retrouve* — sous sa forme la plus terre à terre.

## 2 bis. Deux axes pour ranger, et une palette pour travailler

**La famille et la catégorie ne disent pas la même chose**, et les confondre
ferait un champ qui répond à deux questions — la faute qu'on passe nos journées
à défaire.

- La **famille** est structurelle : entité, graphe, composant, motif. Elle dit
  *ce qu'on peut en faire* — on ne pose pas un motif comme on pose une entité.
  Elle est fermée, le moteur la connaît.
- La **catégorie** est thématique : `auth`, `commerce`, `messagerie`, `contenu`,
  `observabilité`. Elle dit *de quoi ça parle*. Elle est ouverte, elle vient de
  qui écrit le gabarit, et elle sert à filtrer une recherche.

Un `user` est de famille « entité » et de catégorie `auth` ; un formulaire de
connexion est de famille « composant » et de la même catégorie. C'est
exactement ce qui permet de demander « tout ce qui touche à l'authentification »
et d'obtenir le schéma **et** l'écran.

**Et la palette.** (Lucie, même jour :) *« workpalette aussi, les templates que
t'as sous la main »*.

Le catalogue est la boutique ; la palette est ce qui est posé sur la table.
C'est une quatrième notion, à côté des trois qu'on a déjà séparées — et la
séparation est la même discipline :

| notion | la question à laquelle elle répond |
|---|---|
| `RootPolicy` | qu'est-ce que j'ai le droit de **toucher** |
| `WorkDomain` | qu'est-ce que je **vois** dans l'index |
| `Cwd` | **où** je me tiens |
| **`WorkPalette`** | qu'est-ce que j'ai **sous la main** |

La palette n'est pas une permission : le catalogue reste cherchable, et y
prendre quelque chose est un acte. Elle est ce qui est **chargé**, donc ce qui
occupe l'invite et ce qu'un modèle peut employer sans rien demander. C'est la
différence entre « je peux trouver un marteau » et « j'ai un marteau ».

Deux conséquences immédiates :

- **`attach` est déjà l'acte de poser sur la palette.** On l'a écrit hier pour
  les outils sans le nommer ainsi : adopter un gabarit sous un nom, c'est
  exactement ça.
- **Une palette a une taille.** C'est elle qui décide du coût en jetons de
  chaque tour, et donc le vrai argument pour ne pas tout charger : un agent
  avec trente outils n'en choisit aucun — mesuré le 28 août, deux outils
  proches et le second jamais pris.

## 3. Les quatre verbes

Un catalogue ne vaut que par ce qu'on peut en faire. Quatre actes, et ils sont
distincts :

1. **Chercher** — avec les mêmes moyens que pour un document. Un gabarit a un
   nom, une description, des champs : c'est cherchable, et il n'y a aucune
   raison d'inventer un second mécanisme.
2. **Poser** — instancier dans le projet en cours. Pour une entité, c'est un
   `register_entity` ; pour un composant, un fichier écrit.
3. **Adopter sous un nom** — le nom appartient à celui qui adopte, pas au
   gabarit. Déjà vrai pour les outils depuis le 28 août.
4. **Modifier** — après. Un gabarit posé est du code et du schéma ordinaires,
   pas une instance liée à son moule. **Rien ne doit se casser si on le change**,
   et c'est la propriété la plus importante des quatre.

## 4. Ce que ça oblige à décider

Trois questions ouvertes, notées comme telles :

**Le lien après la pose.** Un gabarit modifié se souvient-il d'où il vient ?
Une arête `POSÉ_DEPUIS` permettrait de dire « trois projets utilisent une
variante de `user` » — utile. Mais elle ne doit **jamais** devenir une
contrainte : le doc [03](03-normaliser-des-tableurs.md) a déjà montré ce que
coûte un lien qu'on croyait informatif et qui devient normatif.

**La version du gabarit lui-même.** À ne pas confondre avec le motif
« versionné » ci-dessus : ici il s'agit du gabarit qu'on corrige, et de savoir
si les projets qui l'ont posé héritent. La réponse par défaut devrait être
**non** — poser est un acte daté, pas un abonnement. Mais alors il faut pouvoir
dire « ce gabarit a bougé depuis que tu l'as pris », et c'est une notification,
pas une migration.

Amusant, et pas anodin : le catalogue voudra probablement s'appliquer à
lui-même le motif qu'il propose.

**La frontière.** Un catalogue partagé est une surface de lecture, donc elle
relève des mêmes règles que le disque : `RootPolicy`, `WorkDomain`, et
l'approbation humaine pour ce qui sort du cadre. Un agent qui parcourt un
catalogue lit quelque chose ; ce n'est pas neutre.

## 5. Comment on saura

Le critère est le même que partout : quelque chose qu'un humain regarde.

**Un backend debout, produit par un agent, à partir de gabarits qu'il a
choisis** — avec ses entités, ses relations, sa recherche, et un front qui
l'affiche. Pas un test qui compile : un artefact qu'on ouvre.

Et la mesure qui compte, celle qui a manqué à tout ce qu'on a construit
jusqu'ici : **combien a-t-il eu à écrire lui-même ?** Un catalogue réussi est
celui où la réponse est « peu, et exactement ce qui était propre à ce
projet-là ».
