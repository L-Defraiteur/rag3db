# Le juge Gemini omet required dans la preuve du schéma

- **État** : ouvert (11 novembre 2026), observé pendant R4 du lot d'arrêt ; cause et contrat à départager avec everything declarative, pas qualifié de panne d'environnement.
- **Famille** : produit
- **Gravité** : validation rouge malgré un défaut nommé et une correction proposée ; preuve textuelle insuffisante selon le contrat courant du témoin.
- **Atteignable en service** : à départager ; le vrai juge produit le rapport, l'échec bloquant observé est celui de son témoin.
- **Touche rag3weaver** : oui (`tests/e2e_juge.rs`, contrat de preuve du juge).

## Observation

Sur `0a2d2dbd9`, à luciepc, sous lib 29a7bc52a et Gemini exigé, `le_juge_des_outils_sur_gemini_nomme_les_defauts_de_schema_et_de_retour` échoue à la ligne 443 après 438 599 ms et six itérations. Journal `~/.cache/rag3weaver-build/journaux-cx-recherche-arret-11nov-r4/e2e_juge.log`, lignes 316–320.

Le rapport nomme `parametre_devrait_etre_optionnel`, cite un `call_id` et propose un schéma avec `required: ["motif"]` (le champ `fichier` devient optionnel). Mais sa `preuve` cite seulement les arguments `{"fichier":"main.rs","ligne":"main"}` ; la preuve du défaut classé `description_trompeuse` cite seulement la description de l'outil. Aucune de ces preuves ne contient `required`, ce que le témoin exige explicitement. Le test est donc rouge pour sa condition réelle ; ce n'est ni un saut ni un timeout.

Le ticket fermé `2026-11-08-le-temoin-du-juge-des-outils-exige-un-genre-que-gemini-ne-tient-pas.md` avait élargi les deux genres admis tout en conservant cette exigence de preuve. Ici, le genre est déjà admis : c'est un cas distinct. Les R3 précédentes avaient réellement passé les cinq tests ; cela ne rend pas cette R4 verte.

## Pour départager

Rejouer le témoin inchangé et lire sa preuve complète, sans relâcher l'assertion. Si le contrat doit évoluer, décider explicitement avec ED comment relier la preuve au schéma déclaré et au call_id, et valider la correction proposée sur ses effets. Ne pas confondre une variation de formulation, une preuve manquante et un défaut du runtime. Ne glisser aucune modification du juge dans le lot d'arrêt sans revoir son périmètre et sa validation.
