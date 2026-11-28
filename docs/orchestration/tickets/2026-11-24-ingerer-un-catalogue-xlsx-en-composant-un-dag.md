# Ingérer un catalogue xlsx en composant un DAG

- **État** : ouvert (24 novembre 2026, orchestration).
- **Gravité** : fonction neuve — le socle de la quête des véhicules.
- **Atteignable en service** : non (rien n'existe encore).
- **Touche rag3weaver** : oui (docparsers, nœuds, gabarit catalog).
- **Famille** : produit
- **Pour** : à attribuer par l'orchestration.
- **Signalé par** : Lucie.

Ouvert le 24 novembre 2026, 17 h. Pour la quête des véhicules : une page de vente de véhicules d'occasion, avec une
recherche filtrable, à partir d'un fichier xlsx (caractéristiques, prix, liens d'images), et rag3weaver en backend.

## Le principe

rag3weaver ne livre ni un « importeur de catalogue » tout fait, ni le DAG qui le fait. **L'agent assemble lui-même ce
DAG**, à partir de la consigne ci-dessous (partie 1). Ce ticket demande à rag3weaver **les briques** qui rendent cet
assemblage possible (partie 2). Un utilisateur ou un agent adapte ensuite le DAG à son fichier, sans recompiler.

## Partie 1 : la consigne donnée à l'agent

C'est ce texte, tel quel, qu'on donne à l'agent comme spécification de son outil d'ingestion de catalogue (Lucie,
24 novembre 2026).

### Les étapes

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
6. **Normaliser les valeurs**, selon le détail ci-dessous.
7. **Ingérer** : l'entité et ses champs déclarés depuis le schéma (`EntityConfig`), puis les nœuds d'ingestion existants.
8. **Chercher** : `search_structured` et `select_structured`, déjà livrés à tous les backends, donnent la recherche en
   langage naturel et les filtres.

### La normalisation des valeurs, en détail

Partout la même règle : **le modèle propose, une table déclarée décide, du code applique**, et **la valeur d'origine
est toujours gardée à côté de la valeur normalisée**. Une valeur qui ne se lit pas n'est jamais inventée : elle reste
brute, marquée « non lue », et le compte rendu du champ la compte.

**Les vides.** `-`, `—`, `N/A`, `NC`, `n.c.`, `?`, `inconnu`, une cellule d'espaces : valeur absente. La liste est dans
la table, extensible.

**Les nombres.**
- Séparateurs : `12 500`, `12.500`, `12,500`, `12 500,50`, `12500.5` ; la règle se déduit de la colonne entière (si une
  valeur a deux séparateurs, le dernier est le décimal ; une colonne de prix sans décimales lit `12.500` comme douze
  mille cinq cents), et le choix est dit dans le compte rendu.
- Symboles et mots : `€`, `EUR`, `$`, `HT`, `TTC`, `env.`, `~`, `≈`, `à partir de` sont retirés et notés (`TTC` va dans
  un attribut du champ, `env.` marque la valeur « approchée »).
- Intervalles : `10-15`, `10 à 15`, `10/15` deviennent un minimum et un maximum ; le filtre prend l'intervalle.
- Les dates d'Excel stockées en nombre de jours sont reconnues par le type du champ, pas confondues avec un nombre.

**Les unités.**
- Chaque champ numérique a **une unité canonique**, déclarée dans le schéma (le modèle la propose d'après l'en-tête et
  les exemples : « Kilométrage » → `km`).
- Une table de conversion déclarée lit les écritures et convertit : `km`, `kms`, `km.`, `Km` ; `12k km`, `12 000 km`,
  `12 Mm` ; `ch`, `cv`, `CV DIN` et `kW` (1 kW = 1,36 ch, le facteur est dans la table) ; `L/100`, `l/100km` ; `g/km` ;
  `mm`, `cm`, `m` ; `kg`, `t`.
- Une valeur sans unité prend l'unité canonique du champ seulement si toute la colonne est cohérente (même ordre de
  grandeur) ; sinon elle est marquée « unité douteuse ».
- L'unité d'origine est gardée pour l'affichage (« 98 kW (133 ch) »).

**Les dates.** `24/11/2026`, `2026-11-24`, `11/2026`, `nov. 2026`, `2026` seule : une date avec sa précision (jour, mois,
année). L'ordre jour/mois se déduit de la colonne (une valeur au-delà de 12 tranche), sinon de la langue du fichier.

**Les booléens.** `oui`, `non`, `x`, `✓`, `✔`, `1`, `0`, `vrai`, `faux`, `yes`, `no`, une case vide dans une colonne
de croix : la table les lit.

**Les énumérations et leur dédoublonnage.**
1. **Forme** (code, sans modèle) : casse, accents, espaces, ponctuation et pluriels simples rapprochés : `Diesel`,
   `diesel `, `DIESEL` sont une seule valeur.
2. **Synonymes et fautes** (le modèle propose) : un appel structuré reçoit toutes les valeurs distinctes de la colonne
   avec leurs effectifs, et rend des **groupes** : une valeur canonique, ses variantes, et une raison (`Gazole`, `GO`,
   `diésel`, `Diesl` → `Diesel` ; `Electrique`, `EV`, `100 % électrique` → `Électrique`). Un rapprochement par distance
   d'édition prépare les candidats et borne ce qu'on lui montre.
3. **Relecture** : les groupes sont montrés avant d'être appliqués, comme le schéma ; ce qui est accepté entre dans la
   table des synonymes du catalogue, et s'applique ensuite **sans modèle**, à chaque nouvelle ingestion.
4. **Multi-valeurs** : `essence, hybride`, `GPS / caméra / sièges chauffants` sont découpés par les séparateurs déclarés,
   chaque morceau dédoublonné comme une valeur.
5. **Ce qui ne se range pas** reste tel quel, marqué « non regroupé », et apparaît quand même dans les filtres.

**Les identifiants.** Espaces et tirets retirés, en capitales, pour l'égalité (`AB-123-CD` = `ab 123 cd`) ; la forme
d'affichage est gardée. Un doublon d'identifiant dans le fichier est signalé, jamais fusionné en silence.

**Les liens d'images.** Une cellule peut en porter plusieurs (séparés par `;`, `,`, retour à la ligne) : découpés, le
schéma vérifié (`http`, `https`), les doublons retirés, l'ordre gardé (le premier est la photo principale).

**Les textes.** Espaces en trop, retours à la ligne doublés et formes Unicode ramenés à une seule écriture ; rien n'est
reformulé.

**La forme du fichier.**
- Une ligne de titre de section au milieu des données (une seule cellule remplie, souvent en gras) devient la catégorie
  des lignes qui la suivent.
- Les cellules fusionnées sont dépliées sur leur zone.
- Un en-tête sur deux lignes est recollé (`Moteur` / `Puissance` → « Moteur · Puissance »).
- Chaque onglet peut être une catégorie (« Citadines », « SUV »), proposée par le modèle, confirmée à la relecture.

**Le compte rendu de normalisation**, par champ : combien de valeurs lues, converties, regroupées, laissées brutes, et
pourquoi. C'est ce compte rendu que l'agent joint à sa preuve de mise en place.

**Où cela vit.** La table du catalogue (`catalog_rules.json` : vides, séparateurs, conversions d'unités, booléens,
synonymes acceptés) et les consignes des deux appels structurés (le classement des champs, les groupes d'une
énumération) sont des gabarits, modifiables sans recompiler ; l'application est un nœud de script.

## Partie 2 : les briques que rag3weaver doit offrir

Ce qui existe déjà et sert tel quel : `LlmNode` à `response_schema`, `RhaiNode`, `ForEachNode`, `ConditionNode` et
`RouteNode`, `CollectNode` et `AggregateNode`, `LireFichierNode` et `EcrireFichierNode`, `IngestEntitiesNode`,
`search_structured` et `select_structured`. Ce qui manque, ou est à vérifier :

1. **`ReadDocumentNode`, sur une crate `docparsers`** (sœur de `codeparsers`, décidé avec Lucie le 24 novembre) : chaque
   format est lu dans **une représentation typée commune**, jamais en markdown (le markdown n'est qu'une vue dérivée,
   produite pour le modèle quand il en a besoin). Un classeur : des onglets, des cellules (position, valeur brute, type
   d'origine, formule et sa valeur calculée, style, lien), les zones fusionnées. Un document : des blocs (titre,
   paragraphe, tableau avec ses fusions, image), avec leur page et leur position. Le lecteur dit ce que contient le
   fichier, il n'interprète rien : les sections, les en-têtes doubles et les onglets-catégories sont l'affaire du DAG.
   - **Tableurs** (xlsx, xlsm, xlsb, xls, ods) et csv d'abord, pour la quête : `calamine` (types, formules, fusions à
     confirmer) et un petit complément qui lit les XML internes du xlsx pour ce que calamine ne donne pas (les liens
     hypertextes, souvent la photo d'un catalogue derrière un texte, et le gras).
   - **Ensuite, quand une quête en aura besoin** : docx et pptx par `zip` et `quick-xml` (titres, styles, tableaux et
     fusions) ; les PDF par pdfium (texte avec sa position), notre OCR pour les scans ; html, courriels et csv par des
     crates éprouvées.
   - **Pas de dépendance à un extracteur généraliste.** Kreuzberg sert de référence : sa dernière version sous MIT
     (licence lue à l'étiquette exacte) permet de reprendre un extracteur précis avec sa notice ; ses versions récentes
     sont sous Elastic-2.0, qui interdit de le servir comme service hébergé, et sont donc exclues. Le parseur XML de
     Lucie en TypeScript (npm, luciformresearch) est à relire pour ce que `quick-xml` ne ferait pas.
2. **Un profil de colonnes en Rust** (`ProfileColumnsNode`) : sur toutes les lignes, le remplissage, les valeurs
   distinctes **avec leurs effectifs**, la part de valeurs uniques, des exemples, la forme des nombres (séparateurs vus,
   ordres de grandeur). Il parcourt tout le fichier : c'est de la performance, donc une primitive. Il sert au classement
   des champs comme au dédoublonnage des énumérations.
3. **Des fonctions de texte pour les scripts**, offertes à tous les gabarits : forme Unicode et retrait des accents,
   casse, expressions régulières, découpe sur plusieurs séparateurs, distance d'édition et rapprochement de valeurs
   voisines, lecture d'un nombre avec des séparateurs donnés, lecture d'une date avec sa précision, vérification d'un
   lien. Aujourd'hui les scripts n'en ont presque aucune.
4. **Une table déclarée lue par un script** : un gabarit de données (`catalog_rules.json`) chargé par son nom et donné
   au script en entrée, modifiable sans recompiler. À vérifier : peut-être déjà possible par `LireFichierNode`.
5. **Une pause de relecture** : le DAG s'arrête sur le schéma inféré ou sur les groupes d'une énumération, attend une
   réponse (de l'utilisatrice ou de l'agent), avec un délai et une valeur par défaut, puis reprend avec les corrections.
   C'est le nœud d'attente du ticket `2026-11-24-l-hote-ne-repond-plus-pendant-un-travail-lourd.md`, à livrer avant.
6. **Un DAG qui écrit dans les déclarations de son backend** : les synonymes acceptés entrent dans la table du catalogue,
   versionnée avec le backend, et servent à l'ingestion suivante sans modèle.
7. **Une entité déclarée à l'exécution** : le DAG produit le schéma (champs, types, drapeaux plein texte, vecteur,
   filtre) et le backend crée ou fait évoluer l'entité d'après lui (`EntityConfig` aujourd'hui déclaré à la main dans
   `backend.json`).
8. **Des champs riches** : un intervalle (minimum, maximum) filtrable par recouvrement ; une date avec sa précision ; une
   énumération à plusieurs valeurs filtrable par appartenance ; un lien d'image gardé hors des index ; la valeur
   d'origine stockée à côté de la valeur normalisée. À vérifier dans `select_structured`.
9. **Une ingestion par identité** : réingérer le fichier met à jour par identifiant, et un doublon d'identifiant est
   signalé, pas fusionné.
10. **Créer un backend depuis un gabarit, et lancer un graphe** par un outil MCP et un outil d'agent : l'agent écrit
    son `.mmd`, le fait valider (ports, types, erreurs lisibles), le joue à vide (`DryRunNode`), puis pour de bon.

Chaque brique a son témoin, par un vrai hôte, rouge d'abord ; le témoin de bout en bout est celui de la fin.

## La preuve : l'agent qui sait le monter lui-même

1. **D'abord nous, par MCP**, avec les seules briques : assembler le DAG, créer le backend, donner le fichier, relire le schéma, ingérer, puis
   trois recherches de contrôle (un filtre de prix, une énumération, une phrase en langage naturel). Si cela ne tient pas
   en quelques appels, l'agent n'y arrivera pas non plus.
2. **Puis l'agent**, qui reçoit la partie 1 et assemble son propre DAG avec les mêmes briques, avec la même preuve dans son compte rendu :
   le schéma retenu et les trois recherches avec leurs résultats.

## Le témoin

Un petit xlsx de dix véhicules sur deux onglets, une colonne de prix à virgule, une colonne d'images : le schéma classé
par un faux modèle, les règles appliquées par la table, l'ingestion, puis la recherche « diesel, moins de 15 000 € » qui
rend les bonnes lignes. Par un vrai hôte, rouge d'abord.
