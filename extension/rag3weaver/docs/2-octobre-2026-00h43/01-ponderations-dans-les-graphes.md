# Pas C — les pondérations se règlent dans les graphes (proposition de conception)

2 octobre 2026, par lecture seule, sur `a2358c8f2`. Lucie a dit oui à la
forme (fusion pesée par entité, pondération par valeur de champ, gabarits de
dérivées) avec une exigence : **ces pondérations se règlent dans les graphes
de recherche**, comme les poids de fusion le sont déjà
(`FuseResultsNode(weights='bm25:0.6,vector:0.4')`). Ce document propose le
nœud, l'échelle de préséance (le vrai choix à lui rendre), la déclaration
d'entité, les gabarits de dérivées, le sort du sparse et les preuves — sans
dépendre du banc, muet tant que les poids granite-278m se régénèrent.

## 1. Le nœud : `FieldWeightNode`

```
boost["FieldWeightNode(field='scope_type', weights='file:0.6,namespace:0.7', default=1.0)"]
```

- **`field`** : le champ lu dans `result.data` (les résultats sont enrichis
  par les nœuds de signaux, la donnée est là dès la fusion).
- **`weights`** : poids par valeur, même double forme que `FuseResultsNode`
  (chaîne `'valeur:poids,…'` en `.mmd`, objet JSON en config posée).
- **`default`** : le poids d'une valeur hors table (défaut 1,0 — neutre).

Sémantique : `score ×= poids` — **un poids, pas un filtre** : rien ne sort
de la liste, elle se réordonne. Ports sur le modèle des autres nœuds :
`results` entrant/sortant, `query` entrant (la cible et les options),
`meta` sortant (ses avertissements).

Trois absences, trois conduites — jamais de rétrécissement silencieux :
- la **valeur** n'est pas dans la table → `default` ;
- le **champ** n'existe pas dans le schéma de la cible → neutre (×1) et un
  avertissement dans la méta (le motif du domaine de travail : dire ce
  qu'on ne fait pas) ;
- la **donnée** manque (résultat non enrichi) → neutre et un avertissement,
  une fois par exécution.

**Placement dans `search_base.mmd` : entre `fuse` et `rerank`.** Le pool du
rerank doit déjà refléter la pondération — sinon un genre dévalué squatte
les candidats du cross-encoder ; et quand le rerank est demandé, il reste le
juge final : c'est son contrat. Pas avant la fusion non plus : trois copies
du nœud, et le RRF par rangs rendrait l'effet illisible. Un graphe qui veut
peser *après* le cross-encoder le compose ainsi — c'est un nœud, pas une
politique. Je propose de le poser **d'office** dans `search_base.mmd`, sans
`weights` : neutre à coût nul tant que personne ne déclare, et c'est ce qui
rend la déclaration d'entité (§3) effective pour l'outil des agents sans
toucher aucun graphe.

Générique : aucun mot de `Scope` ni d'aucun schéma dans le nœud — tout
vient de sa config, de l'appelant ou de la cible résolue.

## 2. La préséance — le choix à rendre à Lucie

Aujourd'hui (`base_de_fusion`) : **l'appelant, puis l'entité déclarante,
puis le gabarit, puis le moteur** — une entité qui déclare sa fusion fait
ignorer les poids du graphe. Trois ordres possibles, les mêmes pour la
fusion et pour la pondération par champ :

**A. Statu quo étendu** — appelant > entité > gabarit > moteur.
L'auteur d'entité est maître chez lui ; l'auteur de graphe ne peut rien
contre une entité déclarante (sauf à faire passer ses poids par l'appelant,
c'est-à-dire par chaque requête). C'est précisément ce que l'exigence de
Lucie refuse.

**B. Le graphe prime** — appelant > gabarit > entité > moteur.
L'auteur de graphe obtient toujours ce qu'il écrit. Mais `search_base.mmd`
porte `weights='bm25:0.6,vector:0.4'` en dur : par l'outil des agents —
le seul chemin de recherche en production — **plus aucune entité déclarante
ne serait entendue**. Un pouvoir rendu, l'autre perdu.

**C. Distinguer le choix du défaut** — *recommandé*.
Deux paramètres sur les nœuds de fusion et de pondération : `weights` (un
**choix**, qui prime sur l'entité) et `default_weights` (un **défaut**, qui
ne vaut que si personne ne déclare). L'échelle, du plus fort au plus faible :

```
appelant (options) > weights du graphe > entité > default_weights du gabarit > moteur
```

`search_base.mmd` migre d'une ligne : ses `0,6/0,4` deviennent des
`default_weights` — ce qu'ils ont toujours été en intention. L'auteur de
graphe qui choisit obtient toujours (l'exigence de Lucie) ; l'auteur
d'entité continue de battre le défaut générique (le pouvoir existant
survit) ; `base_de_fusion` gagne un étage, et la leçon du 18 septembre
tient : rien ne s'aplatit, chaque étage est une `Option` qui dit « déclaré »
ou « muet ».

## 3. Où l'entité déclare son défaut

Dans `EntityConfig`, à côté de `fusion` :

```
field_weights : Vec<FieldWeight>          — vide = rien de déclaré
FieldWeight { field, weights: BTreeMap<valeur, f64>, default: 1.0 }
```

`Scope` dirait « les fichiers entiers et les espaces de noms pèsent moins »
par `{ field: scope_type, weights: { file: 0.6, namespace: 0.7 } }` — et
aucun graphe n'a à le savoir. Transport par `TargetInfo`, **sans
aplatissement** (le `Vec` vide reste « muet », comme `default_fusion` est
restée `Option` depuis la fusion aplatie du 18 septembre). Un
`FieldWeightNode` sans config applique la déclaration de la cible ; une
liste de plusieurs champs s'applique en chaîne (produit des poids) ; un
nœud configuré sur **un** champ ne surcharge que celui-là, les autres
déclarations de l'entité restent.

Côté requête, le pendant de `options.fusion` : `options.field_weights`,
même forme, étage « appelant ».

## 4. Les gabarits de dérivées au catalogue

Le catalogue de gabarits d'entités existe (`templates/entities/*.json`,
posés par `place`, famille `entity`, adoptés par `adopt`). Un gabarit de
dérivée est la même chose, avec le bloc `derived` (`from`, `gather`,
`render`) et ses gabarits Jinja dans `templates/render/`. Trois gabarits
utiles :

1. **`fil-et-racine.json`** — une racine et ses contributions par une
   relation, rendues dans l'ordre du temps : la forme ticket + commentaires
   (celle d'`e2e_entites_derivees`), fil + messages (le gabarit
   `conversation` existant y gagne sa moitié dérivée).
2. **`document-et-sections.json`** — un parent et ses morceaux ordonnés par
   un champ de rang : document + sections, page + blocs.
3. **`entite-et-etiquettes.json`** — une racine et ses voisines courtes
   (étiquettes, auteurs, catégories) injectées dans le `title` rendu :
   c'est le gabarit qui donne à la pondération par champ ses valeurs à
   peser sur les dérivées.

Chacun paramètre sa relation, sa direction et son tri dans `gather`, et
rend `title`/`content` par Jinja. Rien de neuf à inventer : la famille,
l'outil et l'emplacement existent.

## 5. Le sparse

Le gabarit continue de **ne pas nommer** le sparse : son poids reste le
0,2 du moteur, comme aujourd'hui. Le pas C livre le *mécanisme*, pas les
valeurs : quand le banc aura mesuré (ligne sparse comprise), le poids
choisi se posera au bon étage — `default_weights` du gabarit ou défaut du
moteur — sans retoucher le code. Aucune partie de cette proposition ne
dépend d'une mesure.

## 6. Les preuves, sans le banc

- **Unitaires du nœud** : la table multiplie et réordonne ; valeur hors
  table → `default` ; champ hors schéma → neutre + avertissement ; donnée
  absente → neutre + avertissement ; parse `.mmd` des deux formes.
- **Unitaires de préséance** : les cinq étages, pour la fusion et pour le
  champ, chacun prouvé en isolant l'étage au-dessus (l'appelant bat le
  graphe, le graphe bat l'entité, l'entité bat le `default_weights`, le
  `default_weights` bat le moteur).
- **E2e sur corpus synthétique** — jamais les sources vivantes (`port.rs`
  est le corpus d'`e2e_code`, `src/` celui du banc) : deux genres, le
  document du genre dévalué a le meilleur score brut, la déclaration
  d'entité le fait passer deuxième ; `weights` du graphe bat l'entité,
  `default_weights` ne la bat pas ; l'appelant bat tout.
- **Non-régression** : le nœud posé d'office dans `search_base` est neutre
  sans déclaration — la passe complète des suites existantes doit rendre
  exactement ce qu'elle rend aujourd'hui, et c'est elle qui le prouve.
- **Au banc, quand les poids seront là** : la ligne G (la pondération par
  genre vaut le changement de texte, sans toucher l'embarquement), les
  défauts de fusion 0,6/0,4 contre 0,3/0,7, et le poids du sparse.


## 7. Ajout du 3 octobre — ce que les familles de gabarits ne savent pas faire

L'étape 3 est livrée avec des **familles à noms fixes** (fil, document,
fiche étiquetée : la racine, la voisine, la dérivée — rendu en ligne), parce
que le repérage a contredit la conception du 2 : `place` ne substitue rien
dans un gabarit, et aucun gabarit ne sait poser une relation. La limite est
écrite dans chaque fiche. Voici ce qui manque, en proposition — c'est un
choix à porter à Lucie, dont le principe est « le plus générique et
personnalisable sans être pénible » :

**Une famille posée d'un coup.** Un gabarit de famille serait un JSON de
plus (`templates/families/…`) : les entités membres (par référence à leurs
gabarits), les **relations** entre elles (ce que le catalogue de gabarits ne
sait pas dire aujourd'hui), et des **noms en paramètres** — `place
family=fil as=Support` poserait `Support`, `SupportMessage`,
`SupportFil`, la relation `Support_HAS_MESSAGE`, et réécrirait le `from`,
les `relation` des `gather` et les préfixes dans les rendus. Ce que ça
demande : une famille `families` dans `template.rs` (scan, read, header) ;
une substitution de noms dans `prepare_entity` (un simple remplacement
d'identifiants déclarés, pas un moteur de gabarits) ; `place` qui enregistre
plusieurs entités et leurs relations dans l'ordre (racine, voisines,
relations, dérivées) et qui dise ce qu'il a posé. Ce que ça coûte : la
validation croisée (un membre manquant, un `as` qui collisionne), l'adoption
inverse (`adopt` d'une famille vivante), et la question du démontage (on ne
sait pas retirer une entité posée).

**`GatherRule` qui trie et borne.** Aujourd'hui les voisines arrivent
triées par uuid et sans limite ; l'ordre du temps et le rang vivent dans le
Jinja (`sort(attribute=…)`), et une racine à mille voisines rend un texte de
mille lignes. `order_by: <champ>` et `limit: <n>` dans la règle diraient
l'intention dans la config — lisible par un outil, un schéma, une doc — au
lieu de l'enfouir dans la source d'un gabarit ; le Jinja resterait libre de
sur-trier. Coût faible (deux champs optionnels, le tri avant le rendu dans
`derive_nodes`), bénéfice réel : les gabarits de famille n'auraient plus
besoin d'inclure le champ de tri dans `fields` juste pour trier.
