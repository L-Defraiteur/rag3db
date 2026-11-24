# Ingérer un catalogue xlsx en composant un DAG

Ouvert le 24 novembre 2026, 17 h. Pour la quête des véhicules : une page de vente de véhicules d'occasion, avec une
recherche filtrable, à partir d'un fichier xlsx (caractéristiques, prix, liens d'images), et rag3weaver en backend.

## Le principe

rag3weaver ne livre pas un « importeur de catalogue » tout fait. Il livre de quoi **composer** l'ingestion en DAG : une
primitive qui lit un xlsx, les nœuds qui existent déjà (appel structuré au modèle, script, boucle, ingestion, recherche),
et un **exemple de DAG** prêt à copier. Un utilisateur ou un agent l'adapte à son fichier sans recompiler.

## Ce qui manque : une primitive

`XlsxReadNode` : un fichier xlsx en onglets, en-têtes et lignes. C'est la seule pièce neuve en Rust (une crate de lecture
de tableurs). Tout le reste est gabarit.

## Un exemple de DAG : `catalog_ingest.mmd`

1. **Lire** le fichier (`XlsxReadNode`).
2. **Profiler chaque colonne** sur toutes les lignes (un nœud de script) : remplissage, valeurs distinctes, part de
   valeurs uniques, quelques exemples.
3. **Classer les champs** par un appel structuré au modèle (`LlmNode` à `response_schema`) : type (identifiant, texte,
   énumération, nombre, date, booléen, lien d'image), nom normalisé, libellé, et le champ qui sert de titre. La consigne
   est un gabarit (`prompts/catalog_schema.md`).
4. **Décider l'indexation par des règles déclarées** (`catalog_rules.json`, lue par un nœud de script), pas par le
   modèle : quel type va au plein texte, au vecteur, aux filtres ; à partir de combien de valeurs distinctes un champ
   est une énumération.
5. **Montrer le schéma** avant d'ingérer : l'utilisatrice ou l'agent le relit et le corrige.
6. **Normaliser les valeurs** (script) : nombres à virgule, unités, liens.
7. **Ingérer** : l'entité et ses champs déclarés depuis le schéma (`EntityConfig`), puis les nœuds d'ingestion existants.
8. **Chercher** : `search_structured` et `select_structured`, déjà livrés à tous les backends, donnent la recherche en
   langage naturel et les filtres.

Le tout se livre comme un gabarit de backend `catalog`, à côté de `code`, `memory` et `notebook`.

## L'agent qui sait le monter lui-même

1. **D'abord nous, par MCP** : créer le backend depuis le gabarit, donner le fichier, relire le schéma, ingérer, puis
   trois recherches de contrôle (un filtre de prix, une énumération, une phrase en langage naturel). Si cela ne tient pas
   en quelques appels, l'agent n'y arrivera pas non plus.
2. **Puis l'agent**, par un graphe de mise en place (`setup_catalog.mmd`), avec la même preuve dans son compte rendu :
   le schéma retenu et les trois recherches avec leurs résultats.

## Le témoin

Un petit xlsx de dix véhicules sur deux onglets, une colonne de prix à virgule, une colonne d'images : le schéma classé
par un faux modèle, les règles appliquées par la table, l'ingestion, puis la recherche « diesel, moins de 15 000 € » qui
rend les bonnes lignes. Par un vrai hôte, rouge d'abord.
