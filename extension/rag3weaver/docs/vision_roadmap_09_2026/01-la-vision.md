# 01 — La vision : un chaos contrôlé

**Écrit sous la dictée de Lucie, 25 août 2026.** Ses mots :

> Un chaos contrôlé, un truc qui peut servir à tout. C'est un agent de code, mais
> il peut gérer sa base de données dans laquelle il vit, il peut aussi construire
> des backends se servant de la même techno… il fait tout et n'importe quoi, un
> gloubiboulga du futur.

Ce document existe pour que cette phrase ne se perde pas, et pour dire **pourquoi
ce n'est pas un fourre-tout**. Il ouvre la série : les cinq autres en déplient
les conséquences — [02](02-les-deux-moities.md) les deux moitiés du moteur,
[03](03-normaliser-des-tableurs.md) l'entrée des catalogues,
[04](04-le-catalogue-comme-graphe.md) le savoir et les capacités dans la même
matière, [05](05-ce-qui-a-tenu-depuis-fevrier.md) l'archéologie,
[06](06-la-feuille-de-route.md) l'ordre du travail.

## 1. La boucle étrange

Ce qui est décrit là n'est pas « un agent qui a beaucoup d'outils ». C'est un
agent dont **le substrat est aussi le sujet** :

- il **vit dans** une base de données,
- il **gère** cette base,
- il **construit** avec la technologie dont il est fait,
- et ce qu'il construit **retourne** dans la base où il vit.

Un système qui se contient lui-même. Ce n'est pas une métaphore ni une ambition
lointaine : c'est à trois ou quatre fils de ce qui existe aujourd'hui.

## 2. Pourquoi ça tient debout (l'axe)

Une seule idée porte tout : **tout est un graphe, et un graphe est une donnée**.

| ce que c'est | ce que c'en fait |
|---|---|
| un pipeline d'ingestion | un graphe |
| un outil | un graphe + un spécificateur (livré, `76a566b58`) |
| un agent | un sous-graphe compilé en workflow (doc 36, `../23-aout-2026-20h33/`) — **donc un outil** |
| un backend | un ensemble de graphes plus une surface |
| le catalogue de tout ça | des **entités dans la base** ([04](04-le-catalogue-comme-graphe.md)) |

D'où la conséquence qui n'est pas décorative : **l'agent cherche ses propres
capacités avec les mêmes moyens qu'il cherche un document** — BM25, vecteur,
sparse, rerank. Il n'a pas une liste d'outils : il a **un RAG sur ses outils**.
Et ce qu'il compose enrichit ce que le suivant trouvera.

C'est là que « il fait tout et n'importe quoi » cesse d'être un défaut : le
système n'a pas besoin qu'on prévoie ses usages, parce que **les usages sont des
données qu'il fabrique et retrouve**.

## 3. La réflexivité est déjà là, en pièces détachées

Rien de ce qui suit n'est spéculatif — c'est dans le dépôt aujourd'hui :

- ~~**`codeparsers`** (orphelin)~~ — **branché, puis sorti en dépôt séparé**
  (3 sept. 2026). L'agent lit **son propre code source** comme un graphe
  navigable : c'est ce que fait `e2e_code`, sur `src/dataflow/`. La phrase
  « le plus gros actif dormant du dépôt » n'a plus d'objet.
- **Les graphes-outils sont sérialisables** (`to_mermaid`, `GraphDefinition`,
  empreinte BLAKE3 déterministe) : ils **se stockent dans la base**, se
  versionnent, se cherchent.
- **Le schéma de la base est généré** par `SchemaDialect` : l'agent peut donc
  raisonner sur sa propre structure, pas seulement sur son contenu. Et depuis le
  3 septembre, **dans deux langues** — le dialecte est le lieu où l'intention se
  traduit, et il y en a deux ([15](15-le-moteur-cesse-d-etre-mono-backend.md)).
- **La boucle d'agent existe** (`c77a2c809`) et ne sait pas qui la pilote —
  Gemini, un modèle local, ou un autre agent.
- **`ToolBox` coupe dans les deux sens** : un agent est un outil pour un autre
  agent.

Il manque **l'ingestion du réel** — documents *et* catalogues — et c'est pour ça
qu'elle passe devant tout le reste (§5 bis, §6).

## 4. Ce qui rend le chaos *contrôlé*

C'est le mot important de la phrase, et c'est ce qu'on a construit toute la
journée sans le dire :

| invariant | ce qu'il empêche |
|---|---|
| **Adressage par contenu** (BLAKE3 sur le graphe trié) | la duplication ; deux graphes identiques *sont* la même version |
| **Identité ≠ contenu** (`GraphTool` / `GraphVersion`) | qu'« un outil qui marche » perde son sens quand le nom change de contenu |
| **Traces d'invocation** | qu'on promeuve à l'opinion ; la promotion se fait **sur preuve** |
| **Frontière de capacités** (`NodeTypePolicy`) | qu'un graphe composé par un modèle instancie ce qui détruit |
| **Aucun Cypher pour les agents** (décision Lucie) | que le savoir accumulé devienne prisonnier d'une base |
| **Cloisonnement org × project** | que le chaos d'un locataire déborde chez un autre |
| **Erreurs lisibles par un agent, jamais des paniques** | qu'une boucle meure au lieu de se corriger |
| **Abandon sur erreur répétée** | que l'auto-correction devienne une boucle infinie facturée |
| **Une absence se nomme** (3 sept. 2026) | qu'un repli, un saut ou une garantie dégradée passent pour un succès |
| **Une recherche dit quand rien n'est probant** (3 sept. 2026) | qu'une coïncidence lexicale se présente comme une réponse |

> **Le chaos est dans ce que le système peut devenir. Le contrôle est dans ce
> qu'il ne peut pas perdre.**

**Les deux dernières lignes ne viennent pas d'une décision.** Elles viennent
d'une répétition : on les a trouvées violées dix fois en une journée, sur du
code qui compilait, ne levait rien, et mentait — voir
[15](15-le-moteur-cesse-d-etre-mono-backend.md) §3. Elles ont leur corollaire :
**une pièce écrite mais jamais appelée se dégrade sans bruit**, ce qui rend une
option non empruntée aussi dangereuse qu'une option absente.

## 5. Le danger, nommé

Un système qui peut modifier son propre substrat peut **se casser lui-même**.
C'est la contrepartie exacte de la réflexivité, et il n'y a pas de version de
cette vision où le risque disparaît. Ce qui existe, ce sont des filets :

- **le versionnage par contenu** — on peut toujours revenir à une version qui
  marchait, parce qu'elle est immuable et adressée par son empreinte ;
- **la frontière de capacités** — par défaut, un graphe composé par un modèle
  est en lecture seule ;
- **la provenance** — on sait quelle session a fabriqué quoi, donc on peut
  élaguer par origine ;
- **et les checkpoints du dataflow**, qui savent déjà défaire une ingestion.

Le risque résiduel assumé : un agent qui compose un outil *valide* mais *faux*,
que les traces finiront par condamner — mais après coup. C'est le prix de
l'ouverture, et il vaut mieux le nommer que le découvrir.

## 5 bis. Les deux moitiés du moteur — voir [02](02-les-deux-moities.md)

Le mode **KB** n'a pas été inventé pour des documents. Il a été bâti pour
**ingérer des catalogues, génériquement** :

> voitures · produits · services · rendez-vous · listes de médicaments ·
> projets de loi…

C'est une famille de problèmes très différente du document. Un article de
catalogue, ce sont **beaucoup d'objets semblables à champs structurés**, dont les
champs viennent de **plusieurs entités** (une voiture a un modèle, une marque,
des options, un prix, une disponibilité), qu'on cherche **sémantiquement** *et*
qu'on filtre **structurellement**. Le « document » y est **synthétisé depuis la
structure**, pas l'inverse.

D'où l'architecture qui existe déjà et qu'il ne faut pas prendre pour de la
complexité gratuite : `{KB}_Index` + `_Index_Chunk`, et surtout `KBUpdateNode`
qui **agrège les champs de plusieurs entités** dans une ligne d'index, laquelle
est ensuite découpée et vectorisée.

Le moteur a donc **deux moitiés, et elles ne servent pas les mêmes gens** :

| | mode **simple** (`Entity` + `Entity_Chunk`) | mode **KB** (index agrégé) |
|---|---|---|
| la matière | documents, code, pages | **catalogues d'objets** |
| la structure | découverte (parsing, chunking) | **déclarée**, et agrégée depuis plusieurs entités |
| la recherche | sémantique d'abord | sémantique **et** filtres structurels |
| les cas | agent de code, base documentaire | produits, médicaments, textes de loi, rendez-vous |

Et ça éclaire rétroactivement le doc 49 : les outils sont des **entités simples**
et non des KB — non pas par commodité, mais parce qu'un outil n'est pas un
article de catalogue.

**Conséquence sur l'ordre des travaux** : le graphe de normalisation xlsx n'est
pas une tâche annexe. **Un tableur est la forme sous laquelle un catalogue
arrive**, neuf fois sur dix. C'est le point d'entrée de toute la moitié KB, et
c'est exactement la forme d'un graphe-outil : une table brute entre, des entités
et des relations sortent.

## 6. Ce qui manque vraiment (dans l'ordre)

Décidé le 25 août, contre l'idée de livrer une surface : **on ne livre pas, il
manque de quoi avaler le réel.**

> **Relu le 5 septembre 2026.** Le point 1 est fait. Un axe qui n'était pas dans
> cette liste est passé devant les autres — **servir une base qu'on n'a pas
> écrite** —, parce qu'une boucle qui ne tourne que sur notre propre base ne sert
> qu'à nos propres projets. Voir
> [15](15-le-moteur-cesse-d-etre-mono-backend.md) et
> [06](06-la-feuille-de-route.md) §2.

**Côté documents et code** (moitié « simple ») :

1. ~~**`codeparsers` intégré**~~ — **fait**, et sorti en dépôt séparé. Ses
   modules `import_resolution` / `relationship_resolution` / `scope_extraction`
   produisent **les arêtes** : un fichier cesse d'être des chunks pour devenir
   des entités reliées. C'est ce qui rend un graphe supérieur à un magasin de
   vecteurs pour du code, et c'est éprouvé sur notre propre source.
2. **Lecture des documents** — pdf, docx, pptx, html, csv. Sans lib lourde : les
   formats Office sont du ZIP + XML. L'OCR (livré) couvre déjà les PDF scannés.

**Côté catalogues** (moitié « KB », §5 bis) :

3. **Le graphe de normalisation xlsx** — la porte d'entrée des catalogues.
   Inférer les colonnes, les types, les clés, les répétitions ; en sortir des
   entités et des relations. C'est là que le mode KB prend son sens, et c'est
   la forme même d'un graphe-outil.

4. Puis seulement : la surface (MCP), le modèle local, la parole.

## 7. Les produits, tels que Lucie les distingue

La même technologie, des priorités opposées :

| produit | ce qui compte | ce qui ne compte pas |
|---|---|---|
| **Agent de code embarqué** (`npm install`, un binaire, MCP) | tout en processus, installation sans friction, **parole locale** | le LLM local — *le développeur a déjà Ollama ou une clé* |
| **Bibliothèque Rust** | le moteur, les traits, zéro dépendance imposée | les modèles, que l'utilisateur amène |
| **Produit cloud multi-tenant** | le cloisonnement, l'élasticité, le coût à l'échelle | la latence locale |
| **Substrat navigateur** (doc 36 §4 bis) | tout en processus, **sans choix** | — bloqué par la taille des modèles |

**L'asymétrie qui décide** : un développeur **a déjà** un LLM. Il **n'a pas** de
moteur de parole. Embarquer la parole différencie ; embarquer le LLM rattrape
péniblement ce que llama.cpp fait déjà mieux.

> **À dire clairement, au 5 septembre : la parole n'existe pas.** Aucun TTS,
> aucun STT, aucun G2P dans le crate — les seules occurrences sont des étiquettes
> de facturation et des documents de repérage. Ce sur quoi repose la
> différenciation du produit « agent de code embarqué » est donc **entièrement à
> écrire**, et deux de ses dettes bloquantes sont amont (le Zipformer faux sur
> wgpu, le lexique de prononciation français). Ce n'est pas un reproche à
> l'ordre choisi — avaler le réel passait devant — mais il ne faut pas lire ce
> tableau comme un état des lieux.

## 8. La phrase à retenir

Ce n'est pas un moteur RAG auquel on ajoute des agents. C'est **un système où le
savoir, les capacités et celui qui s'en sert sont la même matière** — des
graphes dans une base — et où la seule chose qu'on impose est ce qui l'empêche
de se perdre.

Un gloubiboulga, oui. Mais un gloubiboulga qui a une colonne vertébrale.
