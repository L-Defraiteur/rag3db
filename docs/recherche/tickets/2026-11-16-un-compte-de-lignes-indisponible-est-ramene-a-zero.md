# Un compte de lignes indisponible est ramené à zéro

- **État** : ouvert, constat statique NONQUALIFIÉ runtime.
- **Famille** : produit
- **Pour** : Recherche/C7, arbitrage séparé du bypassgénérations.
- **Parent** : [garde302/301](2026-11-16-la-garde-des-comptes-en-blobs-reste-a-302-documents-pour-301-lignes.md).

`catalog/progress.rs::count_rows_of` utilise execute(...).ok()/unwrap_or(0) ; `check_fts_counts` le lit pour décider une reconstruction. Une erreurCOUNT ou un résultat absent/maltypé devient donc0 et peut provoquer une maintenance injustifiée. Constat source3ee/957 lors de l'extraction du helpercommun, pas cause du rougeDOUBLE et aucun rougeSQL qualifié. Ne pas prétendre que le correctif de dispatch strictifie cette primitive historique. Aucun test/SQL natif lancé pour ce ticket ; témoin erreurCOUNT àécrire/voirrouge avantcorrectif séparé, sousGOcoordonné.
