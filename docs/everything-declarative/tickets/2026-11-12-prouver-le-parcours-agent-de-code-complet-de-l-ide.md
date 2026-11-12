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
