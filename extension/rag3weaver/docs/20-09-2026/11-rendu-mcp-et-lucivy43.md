# Rendu MCP intelligible et index Lucivy sans positions

## Rendu : changer la présentation, conserver les résultats

`RenderResultsNode` produisait déjà deux ports : `results` pour composer des recherches, `text` pour lire leur résultat. Le backend déclaratif ne renvoyait que le port déclaré et les métadonnées explicitement demandées. Le MCP affichait donc du JSON, même quand un rendu existait.

Le backend expose désormais aussi `presentation` lorsque le port déclaré est `results` et que le même nœud a produit `text`. Le transport MCP propose `response_format: text | json` :

- `text` transmet le rendu et les métadonnées, sans `structuredContent` dupliquant tout le JSON ;
- `json` conserve les résultats structurés et retire seulement la présentation redondante ;
- si le graphe n'a pas de rendu, le résultat structuré reste disponible comme auparavant.

Le serveur générique conserve son défaut JSON. Le lanceur Magic choisit le texte par défaut ; les scripts de vérification et de planification demandent explicitement JSON. Aucun résultat intermédiaire n'est tronqué par ce choix.

La vue des templates reçoit `payload`, le résultat structuré complet, en plus de la vue historique avec extraits bornés. Le nouveau template `tree` et le filtre Jinja du même nom montrent les objets sous forme de branches, regroupent les champs scalaires sur une ligne et affichent les listes scalaires en ligne. Les textes longs et les tableaux d'objets sont conservés. Un retour à la ligne du texte est représenté par `↵`.

Le filtre `without_fields` permet une projection explicite dans un template produit. Le template Magic retire les identifiants internes redondants, les hashes et le champ de recherche `text` quand `text_en` et `text_fr` sont déjà présents. Les UUID réutilisables, noms, scores, provenance, règles, quantités, couleurs et capacités restent affichés. Il ne synthétise pas de nouvelles relations et ne fusionne pas arbitrairement deux objets de même nom.

Les graphes Magic reçoivent ce template via `prepare_engine_render.py`, y compris les sélections et les traversées qui ne possédaient pas encore de nœud de rendu. Le modèle et la recherche restent indépendants du template produit.

## Lucivy : l'option manquante

Le README principal consulté est `/home/lucied/git_workspaces/lucivy/README.md`, version 4.3.0. La dépendance rag3weaver était verrouillée en 4.0.1, avec dictionnaire partagé, sans `derived_in_ram`, et sans l'option `positions: false` introduite en 4.1.

Le README mesure environ une division par deux de l'index sur son corpus noyau quand les positions ne sont pas stockées. Les correspondances sont alors vérifiées sur le texte stocké ; cela modifie les coûts de requête, pas le contrat des correspondances. Ce facteur ne constitue pas une mesure du corpus Magic.

La mise à jour porte sur `lucivy-core` et `ld-lucivy` 4.3.0 et leurs dépendances associées. `fts_positions` devient une option déclarative du backend : omise, elle conserve le comportement précédent ; `false` configure les nouveaux index sans positions. Tous les champs texte de ces index sont stockés. L'ouverture d'un index existant conserve sa configuration enregistrée : aucune conversion implicite.

Magic utilise `fts_positions: false`. La reconstruction se fait dans `data/engine-search-lucivy43.rag3db`, en conservant le fichier antérieur. `derived_in_ram` reste désactivé : cette optimisation disque rend des structures résidentes en RAM, ce qui n'est pas l'objectif ici.

## Mémoire et incident d'ingestion

L'ingestion initiale, avec Lucivy 4.0.1, a accusé réception de 18 560 fiches catalogue avant une erreur native `No more frame groups can be added to the allocator`. L'erreur d'indexation précisait que les écritures pouvaient être partielles. La fermeture a ensuite dépassé 120 secondes et le lanceur a terminé le processus. Ce n'est pas une ingestion complète ni une preuve de persistance de tous ces enregistrements.

Le pool rag3db passe à 15 Gio conformément à la demande. La limite de région virtuelle `RAG3DB_MAX_DB_SIZE` passe de 16 à 64 Gio ; elle est distincte du budget de pages RAM et est réservée avec `MAP_NORESERVE` sur Linux. L'exception d'origine peut provenir des régions de pages ordinaires ou temporaires : le message natif ne les distingue pas. On ne prétend donc pas avoir démontré une simple saturation de RAM physique.

Les lots d'ingestion passent de 128 à 512 enregistrements pour réduire les frontières de commit. La collection, les decks, le catalogue et les relations sont reconstruits depuis le même snapshot local ; aucun achat de carte n'est exécuté.

## Validation

- Suite complète après mise à jour : 987 tests réussis sur 988 au premier passage, dont les 19 tests de rendu. Le seul échec vient du test d’origine qui supposait `/tmp` hors de tout dépôt, alors que `/tmp/.git` existe dans cet environnement ; il passe en relance isolée avec `TMPDIR=/var/tmp`, sans modification du code testé.
- Le test du transport MCP confirme l'absence de JSON caché en mode texte, le retour intégral en mode JSON et la récupération après un format invalide.
- Le test supplémentaire qui construit puis rouvre un index sans positions passe : résultats, surlignage et restrictions par identifiants sont conservés.
- Résultats de reconstruction et de tests d'intégration à compléter après exécution ; ne pas confondre compilation et indexation terminée.

## Denses : stockage et cache

Le chemin effectif est `Rag3dbSearchBackend` → `QUERY_VECTOR_INDEX` → `HNSWSearchState`. Ce dernier construit des `OnDiskEmbeddings` et des `OnDiskGraph`, et lit les vecteurs des candidats par lookup dans les tables rag3db. La recherche n'impose pas de charger préalablement toute la matrice des embeddings en RAM. Les pages consultées peuvent rester dans les caches ; un état de visite proportionnel au nombre de nœuds existe aussi.

Le modèle BGE-M3 produit ici 1 024 dimensions stockées en FLOAT32 : 4 Kio par vecteur brut, soit environ 195 Mio pour 50 000 vecteurs, avant le graphe et les autres structures. Le calcul des embeddings utilise le service GPU ; le parcours HNSW natif s'exécute côté CPU. Les entrées non encore checkpointées peuvent demander un parcours complémentaire : HNSW n'est pas une garantie de coût strictement logarithmique pour toute requête.

`experiments/mtga/scripts/measure_search_memory.py` mesure le RSS du backend natif via `/proc` toutes les 50 ms : démarrage, première recherche hybride, répétition, capacités denses, puis traversée capacité → cartes. Le service d'embeddings est exclu, et le cache de pages du système n'est pas vidé. Le rapport est écrit dans `data/engine-search-memory.json`.

## Validation des liens par lots

`RelationBatchNode` lisait individuellement chaque extrémité avant de mettre les liens en file. Sur le catalogue, un lot de 512 liens demandait environ 16–18 secondes. La validation utilise maintenant `Catalog::get_many`, par entité et par groupes de 512 UUID : même contrôle de toutes les extrémités avant toute mise en file, en moins d'une seconde par lot dans cette exécution. Les 48 654 liens carte → capacité et 897 liens capacité → mécanique ont été traités ; le rejeu des liens est idempotent. Le test MCP existant de liens absents et du rejeu passe.

## Incident WAL et arrêt MCP

Après la première mesure de recherches, une nouvelle ouverture a échoué avec `Corrupted wal file. Read out invalid WAL record type`. Le SDK MCP accorde **2 secondes** à un serveur stdio après fermeture de son entrée, puis termine son groupe de processus. Le backend natif partageait ce groupe, alors que sa fermeture complète mesurée ensuite demande **3,13 secondes**. Ce délai insuffisant permettait d'interrompre une sauvegarde en cours.

Le processus natif dispose maintenant de son propre groupe. Le protocole hôte possède un message `shutdown` : il ferme les index, détruit le backend et sa connexion, puis accuse réception. L'outil MCP `close_backend` expose cette fermeture ; tous les clients batch de l'expérience l'attendent avant la déconnexion. Un test de transport simule une fermeture de 2,2 secondes et vérifie qu'elle est attendue. Cela ne prouve pas une tolérance générale du moteur C++ à toute coupure brutale : c'est la correction du cycle de vie MCP observé ici.

La base de l'incident et son WAL restent conservés. Une copie du dernier checkpoint, sans appliquer ce WAL de la session de recherche, devient `data/engine-search-lucivy43-recovered.rag3db`. Les **47 399 payloads catalogue** y ont été vérifiés intégralement contre la source ; les contrôles collection, capacités possédées, mécaniques, decks, entrées de decks, filtres et recherches composées passent également. Les tests de rendu, de planification et de mémoire ont ensuite rouvert et refermé cette copie successivement avec succès.

Un second problème d'intégration du rendu a été corrigé : la liste fermée de choix du paramètre `template` refusait les templates nommés externes pourtant acceptés par son résolveur. Cette restriction est retirée ; le résolveur continue de refuser les chemins arbitraires et les traversées de répertoire.

## Mesures finales

Base active : **environ 2,4 Gio sur disque**. Le dossier expérimental entier fait environ 8,5 Gio car les anciennes bases, le fichier de l'incident, les exports et les preuves sont conservés. Ce n'est pas le poids requis pour servir la seule base active.

| Phase | RAM résidente du backend | Temps MCP mesuré |
|---|---:|---:|
| Démarrage | environ 367 Mio | environ 0,5 s |
| Première recherche hybride, 20 résultats | environ 0,8 Gio | environ 0,46 s |
| Répétition de la recherche hybride | environ 0,8 Gio | environ 0,19 s |
| Capacités denses, 20 résultats | environ 0,87 Gio | environ 0,09 s |
| Capacité → cartes | environ 1,23 Gio | environ 0,22 s |
| Sauvegarde et fermeture | pic natif environ 3,8 Gio | 3,13 s |

Le pool est configuré à 15 Gio : ce n'est ni une allocation résidente constante ni un plafond global du processus. Les chiffres excluent le service d'embeddings CPU/GPU et concernent des requêtes bornées. Le cache de pages système n'est pas vidé. Le fichier `data/engine-search-memory.json` contient la mesure exacte et son périmètre.
