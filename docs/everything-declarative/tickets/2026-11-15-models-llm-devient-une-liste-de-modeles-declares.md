# `models.llm` devient une liste de modèles déclarés (pour `/model`)

- **État** : ouvert (15 novembre 2026). Pas pressé : après le fork du pont
- **Famille** : produit
- **Gravité** : planification. Capacité attendue par les commandes « / » du terminal (`/model`)
- **Atteignable en service** : non, c'est une capacité neuve
- **Touche rag3weaver** : oui (`src/model_source.rs`, la lecture de `models` dans un manifeste de
  backend et dans `ChatConfig`)
- **Pour** : everything declarative (ED) pour la déclaration ; archi branche la surcharge par tâche
- **Signalé par** : archi (15 novembre 2026, midi), sur la forme tranchée par c7 avec Lucie

## La demande

`models.llm` déclare plusieurs modèles de langage, chacun avec :
- un **nom** stable, la clé que la tâche retient et que `/model` affiche ;
- son fournisseur et son modèle ;
- `context_tokens` (la jauge le lit déjà en maximum) ;
- `max_output_tokens`, la forme (a) du plafond du juge ;
- `default: true`, sur un seul d'entre eux.
Un manifeste d'aujourd'hui, avec un seul objet `models.llm`, se lit comme une liste à un élément par
défaut : aucun manifeste de Lucie ne change.

## La forme proposée (à confirmer avec archi avant de la coder)

- `models.llm` accepte un **objet** (la forme d'aujourd'hui, `ModelSource`) ou une **liste**
  d'objets. Chaque élément est un `ModelSource` plus trois champs : `name` (obligatoire dans une
  liste), `max_output_tokens` (facultatif) et `default` (booléen, faux par défaut).
- **Le repli de l'objet unique** : une liste d'un élément, `default: true`. Son nom est son `name` s'il
  en porte un, sinon son `model`.
- Le reste des capacités (`embed`, `rerank`…) garde l'objet seul.
- **Refus nommés** à la lecture du manifeste :
  - deux noms égaux ;
  - aucun `default` dans une liste de plus d'un élément ;
  - plus d'un `default`.
  Une liste d'un seul élément sans `default` le prend.
- Une API de lecture : la liste résolue, `(nom, ModelSource, max_output_tokens, défaut)`, et une
  recherche par nom qui refuse un nom inconnu en listant les autres (archi s'en sert pour la
  surcharge de tâche).

## Le témoin

Rouge d'abord, il doit montrer :
- un manifeste d'aujourd'hui se lit comme une liste à un élément par défaut, au nom de son modèle ;
- une liste à deux éléments, `default` sur le second, rend le second par défaut ;
- deux `default` dans une même liste sont refusés, en les nommant ;
- un nom inconnu est refusé, en listant les noms déclarés.

## Ce qu'il faut pour le fermer

La forme confirmée par archi, le témoin rouge puis vert, la lecture par `ChatConfig` et par le
manifeste de backend, et les nouveautés.
