# Rouge Vertex observé, avant correction

Source fe40b640eb0cc54645cbd552aa13b065ed1d2fa7, pilote eb026f0a3c7c4143b511175b2bc92bd4f65c2db4. Allocation C7 distincte CPU1200 après poste réel32GiB, memory.high=max, swap.max=0, cpu.max=500000/100000. INITIAL1185444.765394719, END1186644.765394719. Aucun accès cloud, credential ou service partagé ; serveur HTTP factice local uniquement.

La compilation gardée offline/locked, features daemon,openai-llm sans défaut, a produit un unique ELF de tests : SHA53b5a47ab371ba29e7301ba0bc3fbafe307a4ced4941e39b838eecc997dc8526,195329040octets. Inventaire cinq juges, aucun ignoré. ELF et ledger SHA4f1928b416711b54b5e707db2a00bb97ffabbb45514f9486629ec746eaaa3448 copiés et réhachés sur les deux postes AVANT tests et marqueur de publication. Pas de qualification native, ni de capture maps annoncée.

Trois positifs exacts passent chacun1/0/0 : fixture_token_source_changes_its_cached_token_only_after_invalidation, a_fixed_api_bearer_is_not_refreshed_after_unauthorized, vertex_never_refreshes_or_replays_after_a_stream_fragment, sous openai_llm::vertex_refresh_tests.

Le juge vertex_rejected_token_is_refreshed_once_before_opening_a_stream échoue0/1/0, rc101 : après un premier appel accepté, le second reçoit HTTP401 ACCESS_TOKEN_EXPIRED au lieu de renouveler. First-red immuable1185663.948065542. Arrêt immédiat : le cinquième juge401persistant reste NONJOUÉ. Le pont testé prend encore un snapshot de la source factice puis utilise le vrai constructeur Vertex ; connect_llm déclaré et conservation de TokenSource ne sont PAS qualifiés.

Le scope validation-cnfj_amp reste ROUGE/code1 malgré ce rouge attendu, nettoye=true/restes[]. Scan indépendant1185741.373051858, elapsed296.607657510 : found[],717identités observées, owned_alive[]/owned_unknown[], autorité terminée, bus et cgroup absents. Les UNKNOWN globaux non attribués ne prouvent aucune absence GPU. Target et sources conservés ; aucun correctif ni retry dans cette allocation.

[Manifeste39fichiers](vertex-token-red-manifest.json), SHA141a63c6d1c0172d01a17077de5deb341ff303d107aaf0e0c7f04e644097524b. Archives PC cx-preuves-ed-vertex-token-red-v1 et portable cx-preuves-ed-vertex-token-red-v1-local sous ~/.cache/rag3weaver-build, sans autorite.json ni jeton d’autorité. Les39tailles/SHA portables sont vérifiés ; le manifeste a été construit depuis les copies PC. Pas de contenu des requêtes refusées publié dans Git.

Le ticket reste ouvert. C7 demande désormais une source corrective et un pilote nouveaux avant GO vert : conservation de la capacité TokenSource dans le raccord déclaratif réel, renouvellement borné avant début du flux, aucune reprise SSE partielle ni changement des autres API. Ni replay F300, ni Vertex réel, ni déploiement autorisés par cette preuve.
