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

## Revue du correctif de lecture Git b0090e2be

Les preuves36e restent partielles : vrai chat Vertex, exploration, test nommé
rouge puis édition et test vert ; diff refusé128, reprise pas encore atteinte.
Le helper proposé réduit les chemins communs et conserve les écritures.

Contre-exemple concret à qualifier avant nouvel avis favorable : créer une
racine faux avec son entrée `.git` **lien symbolique vers le gitfile d'un vrai
worktree**, puis appeler avec_lecture_git sur faux et le principal du dépôt.
La comparaison actuelle canonicalise les deux gitfiles : retour enregistré et
gitfile faux deviennent le même chemin réel. Le contrôle de retour passe alors
pour une racine qui n'est pas le worktree enregistré ; son administration
(éventuellement celle d'un voisin) est ajoutée aux lectures.

Revue statique, aucun scénario de ce type exécuté par orchestration. Ajouter le
témoin négatif avec la même exigence que le gitfile copié déjà refusé ; valider
la racine de worktree enregistrée, pas seulement l'égalité après résolution du
lien final, ou refuser explicitement ce gitfile symbolique non qualifié. Préserver
le dépôt ordinaire, le worktree lié réel et ses espaces ; aucune écriture du
principal/voisin ne devient nécessaire. ED garde le correctif et sa qualification
kernel puis chat sous source figée ; cette revue ne remplace pas ces résultats.
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
