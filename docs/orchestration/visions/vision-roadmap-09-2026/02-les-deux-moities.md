# 02 — Les deux moitiés du moteur

Le moteur a **deux moitiés, et elles ne servent pas les mêmes gens.** C'est la
distinction la plus facile à perdre de vue — elle a été perdue une fois en une
seule journée, et retrouvée par un rappel oral.

## 1. La distinction

| | mode **simple** (`Entity` + `Entity_Chunk`) | mode **KB** (index agrégé) |
|---|---|---|
| la matière | documents, code, pages | **catalogues d'objets** |
| la structure | **découverte** (parsing, chunking) | **déclarée**, et agrégée depuis plusieurs entités |
| le « document » | il existe, on le découpe | il est **synthétisé depuis la structure** |
| la recherche | sémantique d'abord | sémantique **et** filtres structurels |
| les cas | agent de code, base documentaire | produits, véhicules, médicaments, textes de loi, rendez-vous |

Le mode KB **n'a pas été inventé pour des documents**. Il a été bâti pour
*ingérer des catalogues, génériquement* — et la liste des domaines n'a pas
bougé d'un mot depuis février ([05](05-ce-qui-a-tenu-depuis-fevrier.md) §2).

Un article de catalogue, ce sont **beaucoup d'objets semblables à champs
structurés**, dont les champs viennent de **plusieurs entités** (une voiture a
un modèle, une marque, des options, un prix, une disponibilité). D'où
l'architecture qu'il ne faut pas prendre pour de la complexité gratuite :
`{KB}_Index` + `{KB}_Index_Chunk`, et surtout **`KBUpdateNode` qui agrège les
champs de plusieurs entités** dans une ligne d'index, ensuite découpée et
vectorisée.

Corollaire qui éclaire le [04](04-le-catalogue-comme-graphe.md) : les outils y
sont des **entités simples** et non des KB — non par commodité, mais parce
qu'**un outil n'est pas un article de catalogue**.

## 2. Ce que le moteur a déjà

La **moitié déclarée** : `EntityConfig`, `register_entity`, `register_kb`,
`FieldType::{Choice, Tags, …}`, `FilterOp::{HasAny, HasAll, HasNone, Between}`,
la fusion RRF, le rerank, le cloisonnement org × project. Tout ce qu'il faut
**quand on connaît le schéma**.

**Correction du 5 septembre 2026 :** cette liste comptait aussi les poids
`title_boost` / `content_boost`. Ils sont **acceptés et jamais appliqués** —
copiés dans `KBMetadata` et jamais relus, revérifié. Les compter parmi ce que le
moteur « a » était l'erreur que ce document reproche ailleurs : confondre ce qui
est déclaré et ce qui est fait. Leur remplacement est une **topologie** — une
branche BM25 par champ, pesée à la fusion — parce que lucivy n'a aucune
pondération par champ. Voir [06](06-la-feuille-de-route.md) §4.

## 3. Ce qui manque : l'entrée

Il manque précisément la moitié qui rend le produit utilisable **par quelqu'un
qui arrive avec un tableur**. Une spécification complète existe depuis février
(`architecture/universal-catalog-schema.md`), avec :

- **un identifiant unique**, auto-détecté ou déclaré, pour les mises à jour
  incrémentales — protocole en quatre étapes, cité dans
  [05](05-ce-qui-a-tenu-depuis-fevrier.md) §4.1 ;
- **des champs flexibles** portant type + filtre + rôle dans les KB, avec ce
  principe fondateur : *« aucun champ n'est obligatoire — l'agent s'adapte à ce
  qui est fourni. La seule contrainte est de pouvoir identifier les items de
  manière unique. »* ;
- **plusieurs KB** avec des stratégies de recherche différentes, créées
  implicitement quand un champ les référence ;
- **un mode zéro configuration** avec une table d'inférence :

| motif | type | filtre |
|---|---|---|
| < 20 valeurs uniques | `choice` | multi-select |
| < 5 valeurs uniques | `choice` | single-select |
| nombres à grande variance | `number` | range + tri |
| contient `\|` ou `,` | `tags` | has-any |
| « oui/non », « true/false » | `boolean` | toggle |
| contient € ou $ | `price` | range + tri |
| date reconnue | `date` | range |
| texte > 200 caractères | `text` | plein texte |
| chemin d'image | `images` | **analyse** |

- **une synchronisation par identifiant**, chiffrée : *93 inchangés → ignorés,
  2 modifiés → réindexés, 5 nouveaux → insérés, 3 disparus → supprimés*. Ce
  n'est pas de l'ingestion, c'est de la **synchronisation** — et un catalogue
  client se resynchronise toutes les nuits.

## 4. Mais la pratique contredit cette spécification sur quatre points

Un pipeline réel a été relu ([03](03-normaliser-des-tableurs.md)). Ce qu'il dit :

| notre plan de février | ce que la pratique impose |
|---|---|
| inférence de type **par heuristiques** | l'inférence utile vient **du modèle** ; les heuristiques servent à **le contredire**, en aval, avec des bornes dures et journalisées |
| **scores de confiance** | produits, **jamais consommés**. Soit ils pilotent une décision réelle, soit on ne les demande pas |
| **questions de validation** bloquantes | **aucune question** ne marche en pratique. Ce qui marche : des avertissements **localisés et illustrés**, non bloquants, plus un cycle « corrige ton fichier et recommence » |
| **identifiant unique** pour l'incrémental | **non atteint et délibérément abandonné** là-bas — parce qu'il suppose *une désignation humaine de la colonne clé que personne n'effectue*. Le système doit rester correct sans |

Et la règle qui décide quoi remonter à un humain, la plus mûre des deux
sources :

> **On ne demande à l'humain que ce qui est irrattrapable en aval et
> silencieusement destructeur.** Une inversion de deux colonnes ne fait pas
> échouer l'import — elle corrompt les filtres sans bruit.

## 5. Le tableur est la porte d'entrée

**Un catalogue arrive sous forme de tableur neuf fois sur dix.** Le graphe de
normalisation n'est donc pas une tâche annexe : c'est la porte d'entrée de toute
cette moitié du moteur.

Et c'est exactement la forme d'un **graphe-outil** ([04](04-le-catalogue-comme-graphe.md)) :
une table brute entre, des entités et des relations sortent. Donc composable,
versionné par empreinte, cherchable — et appelable par un agent, qui pourra lire
les avertissements et y répondre.

Ce que le [03](03-normaliser-des-tableurs.md) ajoute et qu'il faut retenir dès la
conception : **des phases persistées** (une reprise saute ce qui est fait),
**deux représentations depuis une seule ligne** (texte vectorisé depuis les
valeurs *originales*, payload filtrable depuis les *normalisées*), et le
**renversement d'échelle** — ne jamais montrer au modèle ce qu'on peut lui
résumer.

## 6. Ce qui manque aussi côté documents

L'autre moitié n'est pas complète non plus : rien ne lit un `.pdf`, un `.docx`
ou un `.pptx`. L'OCR livré couvre déjà les PDF scannés.

**Correction du 5 septembre 2026 :** cette phrase disait aussi « `codeparsers`
est orphelin, jamais référencé ». Il est branché depuis le 25 août, éprouvé sur
notre propre source, et sorti en dépôt séparé le 3 septembre. Ce qui manque ici
se réduit donc aux **documents**.

Deux idées de février à reprendre ici, détaillées dans
[05](05-ce-qui-a-tenu-depuis-fevrier.md) : **`File` n'est pas un article de
catalogue** — c'est le conteneur physique, la source de vérité des offsets, et
l'unité de `grep` et `read`, jamais chunké ; et **la taille de chunk doit être
dérivée de la limite du modèle d'embedding**, sans quoi on tronque en silence.
