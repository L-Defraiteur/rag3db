# Le reçu d'un index annonce un modèle que l'index n'emploie pas

- **État** : ouvert (12 octobre 2026, codeparsers)
- **Gravité** : réponse fausse. Le reçu de l'outil `index` (et l'estimation) nomme un modèle et sa
  raison, alors que les vecteurs se calculent avec un autre. Rien n'est perdu ; c'est ce que lit
  l'humain ou l'agent pour savoir quel modèle sert sa base qui est faux, et la durée prévue avec
- **Atteignable en service** : oui, dès qu'un manifeste déclare son modèle d'embarquement
  (`embeddings.model`, avec ou sans `address`) et que l'heuristique du premier index en
  choisirait un autre
- **Touche rag3weaver** : oui (`src/estimate.rs`, `estimate_here`)

## Ce que c'est

Le 12 octobre à 10 h 43, réindexation de `code-rag3db` sur `lucie-tablette` (binaire
`22e156eb9a57`, en autonome). Le manifeste déclare
`"embeddings": {"model": "granite-278m", "dimensions": 768, "address": "127.0.0.1:7979"}`. Le
reçu de `index` dit :

> modèle : granite-107m — la seule carte du poste porte l'affichage : granite-107m pour le premier
> index, ses rafales libèrent l'écran plus vite

Le journal de la même indexation dit, ligne après ligne :
`plein texte prêt · vecteurs 109302 sur 139956 (78 %, granite-278m)`. Les vecteurs sont bien ceux
du modèle déclaré ; le reçu annonce l'autre.

## La cause

`estimate_here` (`src/estimate.rs:417-433`) choisit le modèle par
`embedding_choice::choose(files, bytes, card, sole, explicit)`, où `explicit` ne lit que la variable
`RAG3WEAVER_EMBED_MODEL`, et `service_attached()` que la variable du service. **Le modèle déclaré
au manifeste n'y entre pas.** Sans variable posée, l'heuristique du premier index décide (ici « la
seule carte porte l'affichage » → 107m) et le reçu l'écrit comme le choix. L'indexation, elle,
prend le modèle du manifeste, qui l'emporte (la précédence du module : le code avant la variable,
la variable avant l'heuristique).

## La recette

Un manifeste qui déclare `granite-278m`, aucune variable `RAG3WEAVER_EMBED_MODEL`, un poste dont
la seule carte porte l'affichage (ou un dépôt de plus de ~50 000 fichiers) : `estimate`, puis
`index` ; le reçu dit `granite-107m`, le journal `granite-278m`.

## Le témoin

Absent. À écrire : `estimate_here` (ou ce qui le remplace) appelé avec un modèle déclaré au
manifeste et des déclencheurs de l'heuristique vrais (`sole_card` forcé, ou un compte de fichiers
au-dessus du seuil) — le choix rendu est le modèle déclaré, et sa raison dit « déclaré au
manifeste » ; sans déclaration, l'heuristique reprend la main (le cas d'aujourd'hui, inchangé).

## Ce qu'il faut pour le fermer

Passer le modèle déclaré au manifeste à l'estimation, au rang de la précédence que le module
`embedding_choice` décrit déjà (le code, puis la variable, puis l'heuristique, puis le défaut) :
le reçu et l'estimation nomment alors le modèle qu'emploie l'index, avec la raison « déclaré au
manifeste », et la durée prévue suit son débit. Signalé à l'arbre principal (`estimate.rs` est à
lui).
