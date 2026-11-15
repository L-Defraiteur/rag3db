# Le Decide de l'archiviste tronque sa réponse au plafond de 2048 jetons

- **État** : corrigé le 15 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96).
- **Famille** : produit
- **Gravité** : refus. La section est dite en erreur dans le rapport et reste dans le L0 ; rien de faux n'est écrit.
- **Atteignable en service** : oui. Toute passe de l'archiviste avec `gemini-3.8-flash` : 1 section sur 432, et 1 sur 152, cette nuit.
- **Touche rag3weaver** : oui, le gabarit `memory`, `archive_decide.mmd` (le `Decide` sans `max_tokens`, donc 2048 par défaut).
- **Pour** : session mémoire (96).
- **Signalé par** : les rejeux avec trace des deux corpus pour le jeu de banc, la nuit du 14 au 15 novembre 2026.

## Ce qu'on voyait

« Decide (google/gemini-3.8-flash) : réponse tronquée au plafond de 2048 jetons (max_tokens) : « { "label": "create", "name": … ». Le `Decide` le dit bien comme une troncature, pas comme une étiquette hors liste.

Depuis le 14 novembre, la réponse porte le nom **et** une phrase (`about_as`). Gemini 3.x compte sa réflexion dans `max_tokens` : sur une section longue qui crée, la réponse ne tient plus dans 2048 jetons.

Les sections touchées :

- `07-novembre-2026-arbre/04-knowledge-dump.md` (la passe) ;
- une section de cœur C++ du journal des chantiers (« Duplication de clé primaire apr… »).

## Le correctif

`archive_decide.mmd` déclare `max_tokens=4096` sur son `Decide`. C'est une valeur déclarée dans le
graphe, pas en dur dans le Rust. Le jour où `models.llm` portera `max_output_tokens` (le ticket d'archi
`docs/rag3weaver-archi/tickets/2026-11-14-le-plafond-du-juge-du-rappel-suit-le-modele-declare.md`),
le `Decide` le prendra par défaut, et la valeur déclarée ici restera prioritaire.

## Le témoin

`e2e_archiviste::le_decide_de_l_archiviste_demande_le_plafond_declare` : la requête envoyée au modèle
porte `max_tokens: 4096`. Rouge sur master e95a1b8c6 (2048), vert après. `e2e_archiviste` : 16 sur 16.
