# Un numéro d'émission commun pour l'ordre des événements

- **État** : ouvert, le 23 novembre 2026
- **Famille** : produit
- **Gravité** : moyenne : sous charge, la vue reçoit des jetons d'un run après sa fin (le témoin e2e de tty en mesure 132 à 162 sur 300) ; le texte de l'agent peut s'afficher dans le désordre ou après « fini »
- **Atteignable en service** : oui, dès que la machine est chargée
- **Touche rag3weaver** : oui (`src/events.rs`, le bus ; `src/ide.rs`, le relais)
- **Pour** : archi
- **Signalé par** : tty, en mesurant l'ordre des sujets `agent` et `tokens` du flux de l'IDE, le 23 novembre 2026

## Ce qui se passe

Les événements d'un run partent sur plusieurs sujets du bus : `RunStarted`, `LlmCall`, `RunFinished` sur `agent`,
les `Token` sur `tokens` (émis par l'`EventHandler` du chat, `chat.rs`, avec un `seq` propre au run). Chaque sujet
garde son ordre, mais rien ne dit l'ordre entre sujets : le relais de l'IDE (`Flux::relayer`) les entrelace comme ils
arrivent. tty corrige l'ordre par run entre `agent` et `tokens` dans le relais, en s'appuyant sur `seq` et
`RunFinished` ; c'est une réparation locale à ces deux sujets.

## La réparation complète

Le bus pose sur chaque événement, à l'émission, un numéro croissant commun à tous les sujets (un compteur atomique du
bus), sérialisé avec l'événement. Le relais trie sur ce numéro, sur une fenêtre courte, quels que soient les sujets ;
un client qui se reconnecte reprend depuis le dernier numéro vu.

Témoin : sous charge (plusieurs runs qui émettent en même temps), aucun événement d'un run n'est relayé après son
`RunFinished`, et l'ordre relayé est celui des numéros.
