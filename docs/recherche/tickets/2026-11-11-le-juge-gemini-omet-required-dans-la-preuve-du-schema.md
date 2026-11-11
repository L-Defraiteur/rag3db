# Le juge Gemini omet required dans la preuve du schéma

- **État** : ouvert (11 novembre 2026), observé pendant R4 du lot d'arrêt ; cause et contrat à départager avec everything declarative, pas qualifié de panne d'environnement.
- **Famille** : produit
- **Gravité** : validation rouge malgré un défaut nommé et une correction proposée ; preuve textuelle insuffisante selon le contrat courant du témoin.
- **Atteignable en service** : à départager ; le vrai juge produit le rapport, l'échec bloquant observé est celui de son témoin.
- **Touche rag3weaver** : oui (`tests/e2e_juge.rs`, contrat de preuve du juge).
- **Pour** : recherche (suivi et lot ; toute réattribution passe par l’orchestration).
- **Signalé par** : recherche ; sources et observations dans ce ticket et le [rapport de session](../11-11-2026/01-rapport-de-session.md).

## Observation

Sur `0a2d2dbd9`, à luciepc, sous lib 29a7bc52a et Gemini exigé, `le_juge_des_outils_sur_gemini_nomme_les_defauts_de_schema_et_de_retour` échoue à la ligne 443 après 438 599 ms et six itérations. Journal `~/.cache/rag3weaver-build/journaux-cx-recherche-arret-11nov-r4/e2e_juge.log`, lignes 316–320.

Le rapport nomme `parametre_devrait_etre_optionnel`, cite un `call_id` et propose un schéma avec `required: ["motif"]` (le champ `fichier` devient optionnel). Mais sa `preuve` cite seulement les arguments `{"fichier":"main.rs","ligne":"main"}` ; la preuve du défaut classé `description_trompeuse` cite seulement la description de l'outil. Aucune de ces preuves ne contient `required`, ce que le témoin exige explicitement. Le test est donc rouge pour sa condition réelle ; ce n'est ni un saut ni un timeout.

Le ticket fermé `2026-11-08-le-temoin-du-juge-des-outils-exige-un-genre-que-gemini-ne-tient-pas.md` avait élargi les deux genres admis tout en conservant cette exigence de preuve. Ici, le genre est déjà admis : c'est un cas distinct. Les R3 précédentes avaient réellement passé les cinq tests ; cela ne rend pas cette R4 verte.

## Pour départager

Rejouer le témoin inchangé et lire sa preuve complète, sans relâcher l'assertion. Si le contrat doit évoluer, décider explicitement avec ED comment relier la preuve au schéma déclaré et au call_id, et valider la correction proposée sur ses effets. Ne pas confondre une variation de formulation, une preuve manquante et un défaut du runtime. Ne glisser aucune modification du juge dans le lot d'arrêt sans revoir son périmètre et sa validation.

## Provenance et limites

Modèle annoncé dans le journal : `google/gemini-3.8-flash`, Vertex, région `global`, projet `lr-hub-472010`, `RAG3WEAVER_EXIGER_GEMINI=1`, jeu large de features avec `openai-llm,anthropic-llm`. Identifiants transmis au crate par le chemin prévu ; aucun contenu de fichier d'identifiants consulté ou copié. La sortie complète est conservée dans le journal rouge, pas remplacée par un rejeu.

La réponse exacte de `lire_le_run` vue par le modèle n'est pas conservée dans ce journal : sa teneur reste inconnue. Le code courant reconstruit les définitions via `BackendTools::tool_defs()` ; la fixture inchangée déclare `required`, mais cette relecture n'est pas une trace de ce que le modèle a reçu. Aucune absence de schéma ni dérive temporelle du backend n'est démontrée. La seule omission directement observée est celle du champ `preuve` dans le bilan final.

## Départage unique inchangé

La suite R4 complète a fini naturellement : quatre réussis, un échoué, zéro ignoré, 999,52 s, code 101. Le garde d'arrêt a trouvé son lanceur déjà absent ; aucun signal n'a été envoyé. Les PID du lanceur et du binaire étaient absents et la recherche de restes par exécutable target et cwd était vide.

Le seul ciblé autorisé ensuite, borné extérieurement à 900 s pour le diagnostic, passe **sans changement** : 1 réussi/0 échec/0 ignoré, 50,72 s, code 0. Le succès des deux filets auxiliaires n'est pas compté pour ce témoin. Source `0a2d2dbd9ce107d8671013a729b8d61fd1167621` ; SHA256 de `tests/e2e_juge.rs` : `0babc0b606e089b49104c7dd5f0348b848fb14f3dc1aeb299d26fc548dfcc093`.

Cette fois la preuve du défaut est `"required":["fichier","ligne"]`, et la forme proposée ne requiert plus que `mot`. Même modèle et configuration, assertion inchangée : la variabilité du résultat est observée, sa cause n'est pas établie. Le ticket reste ouvert ; le ciblé ne transforme pas la R4 en passe globale verte.

Preuves à luciepc, sous `journaux-cx-recherche-arret-11nov-r4/` :

- `e2e_juge.log`, SHA256 `cbfda8a38996b1b76c77e5bb23ae99edaad020e4542acf9d4d504c8cd1a486df` ;
- `juge-cible-unique.log`, SHA256 `2cb61d15078e4cd95811a098d7fa267bdc7649750a3108d68b7e66e964ce1cd3` ;
- `juge-cible-unique.meta`, `.verdict`, `.exit` et `etat-rearbitrage.log`.

Aucun autre retry ni nouvelle instrumentation n'est ajouté. La suite du lot attend l'atterrissage ED et la base convenue avec l'orchestration ; quatre suites transactionnelles, features, lib entière et navigateur restent non joués pour R4.
