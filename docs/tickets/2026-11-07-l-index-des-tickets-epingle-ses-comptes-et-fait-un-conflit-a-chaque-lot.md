# L'index des tickets épingle ses comptes, et chaque lot qui ouvre ou ferme un ticket fait un conflit

- **État** : ouvert, le 7 novembre 2026, à la demande de l'orchestration (hotfixs). Aucun correctif écrit.
- **Gravité** : coût mesuré, pas de réponse fausse : le 7 novembre au soir, quatre lots sur cinq (hotfixs,
  G, tty, archi) se sont arrêtés sur un conflit de `docs/tickets/README.md`, chacun suivi d'un rebase et
  d'un rejeu.
- **Atteignable en service** : non (les sessions de code).
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/check_tickets.py`).
- **Pour** : hotfixs, dans sa file après (B).

## Ce que c'est

Chaque ticket a une ligne dans les tables de `docs/tickets/README.md`, et chaque table épingle son compte
(« Ouverts — moteur (41) »). `check_tickets.py` refuse un compte faux : c'est une bonne garde, qui
force à relire. Mais deux lots qui ajoutent chacun un ticket écrivent le même en-tête avec deux
nombres différents, et leurs lignes au même endroit : le second rebase s'arrête.

## La forme proposée

1. **La garde passe dans chaque ticket** : un en-tête obligatoire (État, Famille `moteur` ou `produit`,
   Gravité, Atteignable en service, Touche rag3weaver, Pour, Ouvert le), validé par `check_tickets.py`,
   qui refuse un champ manquant, ou un État qui contredit le dossier (« ouvert » hors de `closed/`,
   « fermé » dedans).
2. **L'index est généré et n'est plus suivi par git** : `check_tickets.py --index` l'écrit depuis les
   en-têtes, une ligne par fichier, trié par nom, les comptes calculés. Un fichier hors de git ne fait
   plus de conflit.
3. **Le README garde la prose** (ouvrir, fermer, les champs, la famille), la commande pour lire l'index,
   et plus aucune table ni aucun compte.
4. **La relecture que forçait le compte** devient une ligne de la passe : `check_tickets.py --lot <base>`
   imprime les tickets que le lot ouvre, ferme ou change d'état, à relire avant le push.
5. **La migration est un lot** : reporter dans l'en-tête de chaque ticket les champs qui ne vivent que
   dans sa ligne d'index, vider les tables, valider sur les 220 tickets.

## Pour le fermer

Le lot ci-dessus, avec ses témoins : un ticket sans un champ, un « fermé » hors de `closed/`, deux lots
qui ajoutent chacun un ticket et se rebasent l'un sur l'autre sans conflit.
