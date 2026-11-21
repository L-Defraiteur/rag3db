# Un gros dépôt se rouvre dix-huit minutes après une pose, et l'IDE ne dit pas où il en est

- **État** : ouvert (21 novembre 2026, session recherche, scénario 1 de la nuit).
- **Gravité** : bloquant pendant la réouverture : chat et outils MCP de code refusés
- **Atteignable en service** : oui (hôte de service de la tablette, fed132c964 rebâti, posé à 01 h 55).
- **Touche rag3weaver** : oui (l'ouverture d'un backend, `/ide/chat`, les outils).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Le pas exact

Hôte rebâti posé à 01 h 55. `/ide/chat?backend=12389e156742` (`rag3db`) répond « … s'ouvre depuis N s (son schéma et
ses index se chargent) : cet appel n'a rien fait, recommencez dans un moment » de 01 h 57 à 02 h 13, N passant de 83 à
plus de 1 000. Le chat reste sur « Connexion… », `search_code` refuse pour toutes les sessions. La tablette est à 15 de
charge (des passes d'autres sessions tournent). Ni l'IDE ni le refus ne disent l'étape, ni une estimation.

## Le contournement

Attendre, et relire `/ide/chat` (ou `daemon_status`) de temps en temps.

## Pour le fermer

- Mesurer ce que la réouverture fait pendant ces minutes (schéma, index, une repasse due à des règles d'analyse
  neuves ?), sur une copie de la base, hors charge, puis sous charge.
- Le refus et l'IDE disent l'étape en cours et son avancement.
