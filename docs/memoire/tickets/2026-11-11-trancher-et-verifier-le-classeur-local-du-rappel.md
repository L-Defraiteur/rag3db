# Trancher et vérifier le classeur local du rappel

- **État** : ouvert ; quatre arbitrages de Lucie restent attendus. Préparation documentaire actualisée le 12 novembre, aucune activation.
- **Famille** : produit
- **Gravité** : planification ; évaluer un classement local des candidates du rappel avec un bénéfice mesuré.
- **Atteignable en service** : non pour ce classeur local proposé ; le rappel existant est distinct.
- **Touche rag3weaver** : oui, client LlamaServerDecider, politique de rappel et configuration si le lot est autorisé.
- **Pour** : session mémoire ; banc et ressources à coordonner selon les capacités réelles après décision.
- **Signalé par** : [page de conception existante, §8 actualisé](../../../extension/rag3weaver/docs/09-novembre-2026-memoire-longue/03-le-classeur-local-dans-le-rappel.md#8-questions-pour-lucie), reprise mémoire du 11 novembre et consigne orchestration du 12.

## Preuves acquises et limites

Le [banc du 12 novembre](../../banc/12-11-2026/01-le-juge-sur-card2.md), publié sur master `ee40dbcd6`, a qualifié Q8_0 avec préfixe partagé sur card2 : k5, première utile 67/88, dans les deux premières 84/88, médiane 539,9 ms ; k10, 58/88 et 78/88, médiane 786,6 ms. Sur les 34 cas de test réservés : premières 26/34 et 24/34. Les 440 scores communs sont identiques. Un positif est construit par cas ; les distracteurs ajoutés ne prouvent pas la couverture d'un vrai rappel à dix candidates. Aucun gain d'utilité réelle ni capacité d'abstention n'est acquis par ces scores.

Les temps du banc excluent rappel, préparation du contexte, file et démarrage du serveur. Le « début 2 k » est estimé en caractères, pas une borne exacte de jetons. Le placement et la consommation ont été mesurés (hausse de 6,655 Gio sur card2 pour cette recette, petit contexte seat1 stable 40 KiB), avec les limites du rapport. Pas de comparaison de vitesse à protocole identique avec l'ancien banc card0.

La [mesure historique du rappel](../../../extension/rag3weaver/docs/09-novembre-2026-memoire-longue/04-la-precision-du-rappel-d-aujourd-hui.md) relève quatre candidates pertinentes hors fiche propre sur 104 rappels ; un classeur ne crée pas une candidate utile manquante. Ce relevé d'une session du 9 novembre n'établit pas que toute mémoire actuelle est identique.

## Décisions déjà acquises : aucun faux blocage à reconduire

La [consigne durable de ressources](../../orchestration/11-11-2026/02-knowledge-dump.md) autorise card2 et les modèles pertinents au travail autorisé, sans nouvel accord GPU à chaque usage ; préserver seat1 et coordonner mémoire, charge et consommateurs. Les mesures comparatives gardent leur fenêtre `poste mesure`. Aucun plafond arbitraire de processus, choix card0 ou nouveau banc Q8 identique n'est attendu de Lucie pour préparer cette page.

Les résidents ont été audités : 7882 privé et granite107/7880 sont arrêtés, pas à réserver fictivement pour le classeur. La résidence et le préchauffage automatiques ont leur [suivi commun](../../embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md) ; leur mise en œuvre n'est pas livrée par cette proposition. Aucun service n'est lancé ici.

M8 a livré une **API de plan** (`8b3fab422`), sans adopter ou monter la mémoire réelle. [M9](../11-11-2026/05-archiviste-automatique-du-chat.md) reste la conception de l'archiviste aux frontières du chat, avec décisions séparées de chercher et d'injecter. Le classement Qwen de cette page vise les crochets externes : ni M8 API ni M9 ne l'activent ou ne lui imposent un modèle.

## Arbitrages encore attendus et proposition recommandée

Les quatre questions numérotées du §8 portent sur :

1. Remplacer le juge différé Gemini quand Qwen répond, ou lui ajouter un second avis systématique ; décider aussi du repli.
2. Garder une ou deux fiches au maximum, avec zéro permis et aucune injection déduite du seul rang.
3. Commencer par le différé ou autoriser un essai synchrone dès la première mesure.
4. Référence humaine sur 50 tours ou référence Gemini provisoire explicitement biaisée.

Recommandation mémoire, **non adoptée** : Q8/k5, différé avec échéance de 3 s proposée pour la voie locale bout en bout, deux fiches au maximum, Qwen à la place du juge pour un passage réussi et Gemini comme comparaison hors ligne/repli explicite ; référence humaine sur 50 tours comprenant des cas sans fiche utile. L'abstention et les critères d'utilité doivent être qualifiés dans le lot après ces décisions ; aucun seuil universel de score n'est validé. Le repli Gemini, s'il est retenu, demande son budget explicite et une mesure distincte ; ces trois secondes ne couvrent pas silencieusement une attente distante supplémentaire.

## Reste à faire après décision seulement

Consigner le mot de Lucie selon le [§2 bis](../../orchestration/11-11-2026/03-reprise-par-codex.md#2-bis-comment-se-prennent-les-décisions), puis proposer le périmètre du lot et ses critères. Le client actuel n'est pas le client du banc : cache désactivé, délai 60 s, 40 probabilités contre 50 au banc ; `recall_classifier` est une clé proposée, refusée par le parseur actuel. Il faut raccorder ces contrats, prouver budget/cache/repli/redite/zéro fiche sur faux serveur et préparer un rejeu réellement isolé avant mesure d'utilité/latence. Le banc GGUF et le banc card2 déjà acquis ne sont pas à répéter par habitude.

Cette préparation autorise uniquement les documents. Aucun code, service/GPU, config utilisateur ou mémoire réelle n'est activé ; aucune mesure nouvelle n'est lancée. Une activation chez Lucie reste une décision distincte après validation. Pas de second papier de conception concurrent : la page existante porte la proposition et les questions.

## Critères de fin

Décisions consignées et, si retenu, lot autorisé livré avec témoins et mesure d'utilité/latence ; activation explicitement décidée. Si abandonné, garder la raison et le remplacement éventuel dans la clôture. La mise à jour de la page, une transmission ou une attente ne ferme pas le ticket.
