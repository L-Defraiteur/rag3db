# Le client mince ne revient pas seul quand l'hôte revient

- **état** : corrigé (11 octobre 2026, au soir, branche `coupure-en-vol`)
- **gravité** : blocage (les outils absents jusqu'à un `/mcp` à la main) ; et un appel coupé qui aurait pu être rejoué
- **atteignable en service** : oui — vu par le cœur C++ sur l'hôte de luciepc, à 18 h et à 20 h 11
- **touche rag3weaver** : oui (et seulement)

## Ce que c'était

Trois faces d'un même défaut, relevées par le cœur C++ :

1. **Sans hôte au démarrage**, le client n'exposait que `daemon_status` et ne retentait rien
   de lui-même ; Claude Code ne relit pas la liste d'outils sans qu'on le lui dise : les
   outils restaient absents jusqu'à un `/mcp`.
2. **Un démarrage lent** (la commande qui lit le jeton d'en face par ssh) bloquait
   `initialize` : Claude Code abandonne un serveur muet au bout de 30 s.
3. **Une coupure en plein appel** (l'hôte qui redémarre sous un client vivant, « connection
   reset by peer ») passait pour « l'hôte ne répond pas », et en mode découverte l'appel
   était **rejoué** après relance — alors qu'il avait pu être joué.

## Ce qui le corrige

- le client démarre sans jamais attendre l'hôte plus de 4 s ; il se branche dans un fil,
  retente toutes les 10 s tant que l'hôte manque, et annonce
  `notifications/tools/list_changed` quand il le rejoint (capacité `listChanged: true`,
  un message interne glissé dans l'entrée de la boucle MCP) ;
- une connexion refusée (jamais reçue) se relance et se rejoue ; une coupure en plein appel
  se dit (« il a pu être joué ou non, il n'est pas rejoué ») et ne se rejoue pas.

Témoins : `e2e_mcp_cycle_de_vie::les_outils_reviennent_seuls_quand_l_hote_devient_joignable`
et `e2e_mcp_hote_designe::une_source_lente_ne_bloque_pas_le_demarrage` (rouges sur l'ancien
code : `listChanged: false`, `initialize` après 12,2 s), et les tests unitaires
`une_coupure_en_plein_appel_se_dit_et_ne_se_rejoue_pas` et
`le_message_interne_annonce_la_liste_changee`.
