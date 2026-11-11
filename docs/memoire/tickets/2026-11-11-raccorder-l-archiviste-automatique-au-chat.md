# Raccorder l’archiviste automatique au chat et à la compression

- **État** : ouvert (11 novembre 2026), conception écrite ; implémentation et activation non engagées par ce ticket.
- **Famille** : produit
- **Gravité** : planification ; le classement et le rappel automatiques décrits ne sont pas encore raccordés au chat.
- **Atteignable en service** : non, parcours attendu non livré ; l’outil explicite archive existe séparément.
- **Touche rag3weaver** : oui, conversation de l’atelier, mémoire et traitements déclarés.
- **Pour** : mémoire, avec les sessions déclaratif et architecture IDE pour les contrats de conversation.
- **Signalé par** : demandes de Lucie du 11 novembre 2026 ; conception initiale 3ff8d9b79, précisions de rappel et de hiérarchie 418e47a2e.

## Source et périmètre

La référence courante est la [clarification de M9](../11-11-2026/05-archiviste-automatique-du-chat.md),
reliée aux conceptions du 4 octobre, du 18 octobre et au découpage M4/M9 du 25 octobre.

L’archiviste passe aux frontières de message utilisateur, de tour complet de l’agent et de
compression. Il reçoit le contexte disponible, les fiches ouvertes et ses reçus précédents.
Le rangement et le rappel sont des décisions indépendantes ; `terminer(raison)` peut clore
un passage sans écriture mémoire ni injection. Un contexte déjà suffisant n’est pas surchargé.

Conserver les sources L0, apports L1, consolidations L1bis et états L2 avec leur provenance,
ainsi que les idées historiques à réconcilier. Ce chantier n’introduit pas un orchestrateur
multiagents général et ne vaut pas autorisation d’activer la mémoire réelle.

## Ce qui reste à préciser et réaliser

Les contrats de frontière et d’instantané, le repère par conversation, les reçus et la reprise
des effets partiels ; les deux portes du rappel avant le tour agent, leur budget et la
coordination avec le classement de fond ; les outils et la politique de création applicables.
La synthèse L2 et la consolidation restent des travaux séparables, à relier à ce ticket si
des lots distincts sont ouverts. La conception seule ne clôt pas ce raccordement.

## Pour fermer

Les critères de la section 7 de la conception sont vérifiés sur le parcours réel : abstention,
classement sourcé, rappel utile avant le tour visé, absence de doublons, conservation à la
compression et reprise après interruption. La clôture précise les commits, les preuves et
le périmètre effectivement livré, ou explique l’abandon décidé et son éventuel remplacement.

## Preuves du socle et précision de la reprise

Le socle M4 et son correctif sont livrés sous `41526b29a` ; la [note de preuve](../11-11-2026/04-alias-candidats.md) conserve les passes et le premier document traité sur copie, sept sections sans refus. Ce résultat ne prouve pas le raccord automatique, ni la synthèse L2.

Le repère examiné doit progresser aussi après une abstention, sans devenir un repère de rangement fictif. Une erreur ou des effets partiels ne constituent pas une réussite complète. Fiches/version et génération de contexte bornent le rappel avant le tour visé ; un nouveau message en cours de passage doit conserver sa propre frontière. Ces points sont des critères de la conception à vérifier, pas des garanties livrées. Le suivi est porté par mémoire, sans lot d’implémentation ouvert.
