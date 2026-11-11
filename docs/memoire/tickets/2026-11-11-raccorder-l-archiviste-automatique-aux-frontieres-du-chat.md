# Raccorder l’archiviste automatique aux frontières du chat

- **État** : ouvert, conception précisée ; aucun lot d’implémentation autorisé à ce stade.
- **Famille** : produit
- **Gravité** : planification ; entretenir la mémoire à partir des échanges significatifs, sans inventer d’écritures utiles.
- **Atteignable en service** : non pour le parcours automatique décrit ; l’outil archive explicite est livré.
- **Touche rag3weaver** : oui, contrats de conversation, traitement déclaré, outils mémoire, reçus et reprise.
- **Pour** : session mémoire pour le suivi ; relecture des contrats avec everything declarative et rag3weaver archi, coordination par l’orchestration.
- **Signalé par** : demande de Lucie consignée dans [la conception M9](../11-11-2026/05-archiviste-automatique-du-chat.md), commits `3ff8d9b79` et `418e47a2e` ; régularisation du suivi le 11 novembre.

## Preuves et périmètre

`archive(document)` est livré ; son correctif et ses passes sont documentés dans [la note du lot](../11-11-2026/04-alias-candidats.md), livraison `41526b29a`. Cela ne livre ni les déclencheurs automatiques du chat, ni un journal de passages, ni le recalcul L2.

La conception courante fixe les frontières message utilisateur envoyé, tour entier de l’agent outils compris, et compression. Le passage reçoit un instantané identifié du contexte visible, les fiches ouvertes/versionnées et les reçus antérieurs. Il peut terminer sans fiche : abstention normale, raison dans le journal seul. Repère examiné distinct de rangé ; une erreur ou des effets partiels ne valent pas une abstention réussie.

Le rappel à la frontière utilisateur suit ses deux portes chercher/injecter, avec abstention possible après recherche et insertion avant le tour visé, contrôlée par versions et génération de contexte. Les idées anciennes L1/L2, L1bis, sommaire, références et à_revoir restent à réconcilier selon la page de conception ; elles ne deviennent pas des capacités acquises.

## Reste à faire

Arrêter les contrats de frontière, de terminaison, d’instantané, de budget et de reprise/effets partiels ; confirmer l’ordre et les limites du premier lot avant implémentation. Conserver la source avant compression et l’ordre par conversation ; définir idempotence, reçus et autorisation d’écriture. Composer les primitives existantes, sans ouvrir un nouvel orchestrateur multiagents général.

L’activation dans une mémoire réelle reste une décision distincte. Ce ticket n’autorise ni code, ni service, ni lancement d’archivage.

## Critères de fin

Le parcours autorisé est livré avec témoins : abstention sans écriture et sans retraitement ; provenance et portée des décisions ; recherche d’une fiche non ouverte ; isolation/ordre des conversations ; source conservée à la compression ; reprise après redémarrage et effets partiels sans doublon ni perte ; rappel utile avant le tour visé, sans injection tardive dans une autre génération. Les contrats et preuves distinguent ce qui reste non livré, dont L2 et consolidation si hors lot. Une conception ou un transfert seuls ne ferment pas ce ticket.
