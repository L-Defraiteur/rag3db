# MCP de recherche Mermaid et capacités Magic — 19 septembre 2026

## Contrat implémenté

Un manifeste peut activer `search_graphs: true`. Le backend expose alors trois outils supplémentaires via l’adaptateur MCP existant :

- `describe_backend({})` : mappings des payloads, entités, identités `hashsafe`, relations orientées, ports et paramètres des nœuds autorisés, exemples Mermaid des outils de recherche attachés.
- `validate_search({mermaid, arguments, metadata?})` : parse, lie et instancie le graphe ; vérifie paramètres, filtres structurés, entités et relations déclarées, types/branchements des ports et absence de cycle. Rend le plan et son hash sans lancer la recherche.
- `run_search(...)` : même validation, puis exécution dans le runtime existant sur le catalogue persistant ; résultat JSON, métadonnées terminales demandées et hash du plan.

Les graphes libres sont limités à une liste explicite de nœuds de recherche. Aucun nœud d’écriture, SQL arbitraire, script ou graphe imbriqué n’est autorisé. Les outils nommés déclarés par le manifeste conservent leurs capacités, y compris l’ingestion. Cette restriction ne transforme donc pas le serveur entier en serveur de lecture uniquement.

Limites de structure : 64 Kio de Mermaid, 64 nœuds, 256 arêtes, 16 ports de métadonnées. Ce ne sont **pas** des budgets de temps, mémoire ou nombre de résultats. Les sélections peuvent rester exhaustives. Une recherche dense reste une sélection top-k ; le template `search_dense_related.mmd` expose explicitement cette limite dans les options de la recherche initiale.

La validation n’est pas un estimateur de coût ni une preuve de disponibilité des services. Les erreurs du moteur restent possibles pendant l’exécution. Un résultat consommé par une arête n’est pas un port terminal : la validation refuse de le demander comme sortie finale.

## Modèle Magic

La source des cartes possédées reste toute la collection du snapshot `e44fdd1e50b8916e9d7e` : **5 380 références Arena et 10 352 exemplaires**. Les decks ne servent pas à choisir quelles cartes sont ingérées.

Le modèle complémentaire préparé comporte :

| Entité | Nombre préparé | Fonction |
|---|---:|---|
| Ability | 6 063 | Blocs natifs de capacité, identité `(ability_id, text_id)` ; texte anglais et français, FTS et dense BGE-M3 |
| Mechanic | 430 | Registre des termes et définitions disponibles dans le client, avec statut de définition ; FTS et dense |
| Deck | 188 | Decks capturés dans le snapshot ; identité stable indépendante du nom |
| DeckEntry | 8 083 | Quantité, pile, carte, deck, présence dans la collection possédée |

Relations préparées : `CardAbility` (9 811), `AbilityMechanic` (358), `DeckEntries` (8 083), `EntryOwnedCard` (5 620). Les 2 463 entrées non possédées restent présentes avec `in_owned_collection: false`, sans lien vers `OwnedCard`. Ces nombres décrivent les données préparées ; consulter le rapport de validation MCP pour l’état effectivement vérifié en base.

Les capacités ne sont pas toutes des keywords : beaucoup sont des blocs de règles propres aux cartes. L’identité native n’effectue pas de fusion approximative par casse ou sous-chaîne. Deux identités natives peuvent porter un texte identique. Les liens au glossaire proviennent de `glossary_ids` ; une simple mention textuelle d’un mot-clé n’est pas transformée en capacité intrinsèque.

Trois entrées du snapshot répétaient le même triplet deck/pile/carte. Leur quantité est additionnée pour former une entrée stable ; les nombres d’exemplaires restent ceux de la source.

Le nœud générique `RelationBatchNode` relie des objets existants par leurs identités déclarées. Il valide tous les endpoints avant d’écrire, dédouble les paires dans le lot et utilise l’insertion idempotente du moteur. Il accepte uniquement des relations sans propriétés et refuse les endpoints à politique d’écriture gérée. Quantités et piles vivent dans `DeckEntry`. Une erreur de stockage après validation n’offre pas de rollback transactionnel du lot.

## Utilisation locale

```bash
bash experiments/mtga/scripts/serve_engine_mcp.sh
```

Ce processus possède la base : ne pas démarrer simultanément un autre hôte sur le même fichier. La configuration MCP d’un client peut utiliser `command: "bash"` et le chemin absolu du script en argument. Le lancement du serveur ou son test SDK ne l’enregistre pas automatiquement dans une session Codex déjà ouverte.

Le client commence par `describe_backend`, reprend ou compose un template, fournit les valeurs dans `arguments`, valide puis exécute. Il n’a pas besoin de générer un script Python par recherche. Le script de test SDK est une vérification du transport, pas le chemin normal imposé au client.

## Validation après reconstruction à 8 Gio

Le transport MCP officiel a été testé après fermeture et réouverture de la base : 5 380 cartes possédées, 6 063 capacités, 430 définitions, 188 decks, 8 083 entrées. La sélection imbriquée retrouve les 71 cartes Trésor attendues. Une recherche dense sur les capacités retourne 10 capacités et son parcours retrouve 11 cartes possédées dédoublonnées. Le graphe exhaustif cimetière, limité au deck Mycotyrant capturé, retrouve exactement les 17 cartes de l’oracle indépendant, tant comme outil nommé que comme Mermaid libre. Les erreurs de validation restent récupérables sans désynchroniser le transport.

Preuves : `experiments/mtga/data/engine-mcp-validation.json`, `engine-mcp-description.json`, `engine-ability-dense-cards.json`, `engine-scoped-graveyard.json`. La base active est `engine-search-8g.rag3db`. Les sources du snapshot restent la référence de périmètre ; cette vérification ne signifie pas capture Arena en temps réel.

## Incident pendant la mise en service

L’ingestion étendue a rencontré un SIGSEGV natif après 5 504 capacités acquittées. La réouverture de la base échoue ensuite dans la lecture du WAL (`wal_record.cpp:79`). L’ancien fichier, son WAL et son shadow sont conservés pour diagnostic ; ne pas supprimer le WAL en prétendant avoir réparé la base.

Les sources de collection, de capacités et de decks restent disponibles pour reconstruire l’index. La reproduction sous GDB et la correction de la finalisation des dictionnaires sont décrites dans [le rapport mémoire](12-memoire-index-et-erreurs-de-persistance.md). La nouvelle base a passé les tests MCP ci-dessus ; les fichiers accidentés restent conservés.
