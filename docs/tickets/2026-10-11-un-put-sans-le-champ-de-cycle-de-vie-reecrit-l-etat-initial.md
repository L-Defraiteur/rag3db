# Un `put` qui omet le champ de cycle de vie réécrit l'état initial, et défait une transition sans le dire

- **état** : ouvert (mesuré le 11 octobre 2026 en écrivant `tests/e2e_reaction_montee.rs`)
- **gravité** : réponse fausse — une transition acquise est perdue, silencieusement
- **atteignable en service** : seulement si le schéma d'une entité rend son champ
  d'état **facultatif**. Les trois gabarits livrés le rendent requis, donc aucun
  d'eux n'est touché aujourd'hui.
- **touche rag3weaver** : oui, et seulement rag3weaver

## Ce que c'est

`EntityRecordNode(action='put')` écrit le record donné. La vue `editable`
(`backend.rs`, `payload_schema`) retire du payload les seuls champs **générés** —
`created_at`, `updated_at`, `revision` et les dates de transition — et **pas** le
champ de cycle de vie. Si le schéma de l'entité ne le liste pas dans `required`,
un appelant peut l'omettre : la ligne est alors réécrite avec l'état **initial**
du cycle, quel que fût son état.

Une transition acquise (par `ReactTransitionNode`, par un verbe de cycle de vie,
par un agent) est donc annulée par la première édition ordinaire qui ne pense pas
à renvoyer l'état. Rien ne le dit : la réponse du `put` est acceptée, et
`warnings` est vide.

## La recette minimale

Pas de Cypher : c'est un chemin rag3weaver.

1. une entité à `lifecycle` (`field: "etat"`, `initial: "courant"`, transition
   `revoir: courant → a_revoir`) dont le schéma JSON **ne met pas** `etat` dans
   `required` ;
2. un outil `put_x` sur `graphs/put.mmd`, `input_payloads` en vue `editable` ;
3. `put_x` crée la ligne (état `courant`), une transition la passe `a_revoir` ;
4. `put_x` à nouveau, sans `etat` dans le record → relire : `courant`.

C'est exactement la séquence jouée : en laissant `etat` facultatif, l'étape 4 du
témoin rendait `Some("courant")` là où la transition venait d'écrire
`a_revoir`.

## Le témoin

Pas de témoin dédié, et c'est volontaire pour l'instant : le témoin du montage
(`tests/e2e_reaction_montee.rs`) **contourne** le défaut en rendant `etat` requis,
comme les gabarits livrés, et son schéma dit pourquoi en renvoyant ici. Un témoin
du défaut lui-même tiendrait en quinze lignes le jour où on décide du
comportement voulu.

## La cause

Deux décisions raisonnables qui, ensemble, perdent une information :

1. `payload_schema` ne retire que les champs générés — le champ d'état est tenu
   pour éditable, ce qui est vrai : un agent a parfois le droit de l'écrire ;
2. le `put` est un **remplacement** du record, pas une fusion : un champ absent
   du payload n'est pas « inchangé », il est absent, et la machine à états
   applique alors son initial.

## Ce qu'il faut pour le fermer

Trancher entre trois conduites, et la déclarer :

1. **Refuser** : le champ de cycle de vie est obligatoire dans la vue `editable`
   dès qu'une entité déclare un `lifecycle` — un schéma qui l'omet est refusé au
   chargement, en disant de l'ajouter à `required`. C'est la conduite la plus
   proche du reste du dépôt (un manifeste faux se refuse au chargement).
2. **Conserver** : un `put` sans le champ d'état garde l'état courant de la ligne
   au lieu d'appliquer l'initial. Plus doux, mais c'est une fusion déguisée dans
   un verbe qui promet un remplacement.
3. **Le dire** : appliquer l'initial, et rendre un `warning` nommé quand cela
   change l'état d'une ligne existante. Le moins coûteux, le plus facile à ignorer.

Préférence de qui l'a relevé : la 1. Un état n'est pas une valeur par défaut.
