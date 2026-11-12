# Prouver le parcours agent de code complet de l'IDE

- **État** : ouvert ; lot isolé autorisé pour la nuit après les correctifs des accords.
- **Famille** : produit
- **Gravité** : capacité principale ; l'atelier doit réellement explorer, modifier, tester et reprendre.
- **Atteignable en service** : les outils existent ; leur parcours complet dans une session réelle reste à qualifier.
- **Touche rag3weaver** : oui, chat/agent de l'atelier, outils accessibles, exécution et reprise.
- **Pour** : ED ; archi relit les contrats d'interface, sans édition concurrente.
- **Signalé par** : objectif explicite de Lucie pour l'IDE et demande de lots indépendants cette nuit.
- **Ouvert le** : 12 novembre 2026

[Répartition](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md).
Créer un hôte, un dossier d'état et un petit projet Git de fixture indépendants
de l'IDE personnel. Passer par le vrai chat et le catalogue d'outils du framework :
demande synthétique, recherche/lecture, modification, vrai test nommé, résultat,
diff et reprise. Utiliser Gemini via Vertex configuré quand le modèle est requis,
avec une recette et des appels bornés ; aucun secret ou chat réel envoyé.

Vérifier fichiers et effets réels, nombre/tests effectivement exécutés, travail
isolé et continuité des identités de conversation/outils. Une réussite verbale,
une fonction Rust présente sans binding ou un appel direct contournant le chat
ne satisfait pas le contrat. Les refus et accords suivent le mécanisme normal,
sans mode permissif de test masquant un défaut d'accessibilité.

Publier le premier constat/source/recette avant ciblé. Corriger les lacunes dans
les primitives et gabarits communs ; respecter propriété navigation/code_tools
et modèle/serveur des autres sessions. Pas de nouvel orchestrateur multiagent
builtin, ni de déploiement/restart personnel dans ce lot initial.

Fin : parcours réel reproductible avec outils accessibles, tests/diff/reprise
observés, corrections éventuelles relues et intégrées, limites restantes dites.
Le ticket reste ouvert si seule une partie du parcours est démontrée.

## Reprise ED et premier constat

Worktree isolé `cx-ed-parcours-agent-ide`, branche homonyme, base `a783013f1`.
Aucun appel modèle, build ou service lancé à ce stade. Les fichiers réservés
navigation et résidence des modèles restent à leurs propriétaires.

Le témoin existant `le_chat_d_une_tache_code_sur_gemini` utilise bien le vrai
chat et Vertex mais ne qualifie que l’édition. Le gabarit code déclare déjà
la chaîne Rust en lecture et Cargo hors réseau dans son bac à sable ; les
méthodes normales relisent tâches et conversations persistées.

Préparation : fixture Git Cargo sans dépendances, test nommé immuable,
exploration par outils exposés, échec initial attendu du test, correction et
réussite effective, diff par commande autorisée puis recréation de l’hôte et
reprise. L’attestation indépendante vérifiera les octets du principal, les
identités et les appels/résultats liés, sans remplacer les actions de l’agent.
La source et la recette bornée seront publiées avant compilation/ciblé.

## Première source et recette bornée

Nouveau témoin ignoré par défaut :
`daemon_host::chat_qui_code::le_chat_code_teste_et_reprend_sur_vertex`.
Il emploie le modèle réel par défaut de l’hôte, sans callback substitué.
Trois tours : recherche/lecture et test initial attendu rouge ; édition du
seul fichier source puis test vert et diff ; nouvel hôte, registres relus,
historique identique (identifiants outils inclus), test et diff rejoués.
Le test d’intégration immuable `salutation_nettoie_le_nom` doit effectivement
s’exécuter (1 failed attendu puis 1 passed), jamais zéro test. Les preuves JSON
et la fixture sont conservées sous un parent privé, y compris en cas de refus.

Sur luciepc uniquement, target neuf propre à ce lot, lib native29 épinglée,
j8/Max32G/HIGH0, aucune charge GPU nécessaire et aucun usage 7879 prévu.
Primitive commune extérieure, budget global1900s incluant nettoyage :
compilation lib de tests1500s maximum, puis témoin exact360s maximum.
Features initiales `rag3db-native,code,daemon,openai-llm` ; pas de navigateur
à ce préfixe (preuve du contrat hôte/chat/outils, pas encore preuve visuelle).
Compilation par `cargo test --lib --no-run --message-format=json` ; extraction
de l’unique ELF lib dans le flux JSON. Sonde `--list --ignored --exact` sur
le nom complet, puis ELF `--ignored --exact <nom> --nocapture --test-threads=1`.
Attendu : exactement1 témoin, 1 passed/0 failed/0 ignored. Au premier rouge
inattendu : arrêt, preuve et diagnostic avant toute nouvelle recette.

Variables runtime : `CARGO_MANIFEST_DIR` du worktree ; `LD_LIBRARY_PATH` lib29 ;
`RAG3WEAVER_EXIGER_GEMINI=1`, chemin autorisé des identifiants Vertex (jamais lu),
projet configuré `lr-hub-472010`. États, journaux, targets de fixture et preuves
sont isolés sous le dossier privé du lot. Le target Cargo de la fixture est
celui du contrat sandbox normal, sous `RAG3WEAVER_TARGETS_DIR` privé. Aucune
politique permissive ou réponse à un accord humain n’est injectée.

Préflight statique avant tout lourd : le gabarit livré active le rattrapage
vectoriel à l’ouverture. La déclaration de cette fixture non indexée le désactive
explicitement ; `search_code` garde son balayage déclaré. Aucun modèle substitué,
ni charge d’indexation nécessaire au parcours. Cette source complète remplace la
première pour le ciblé, encore non lancé.
