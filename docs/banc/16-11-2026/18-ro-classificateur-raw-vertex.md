# Relecture pure du classificateur RAW Vertex

Référence exacte `197b60ebc83ec0d4b6329dc977c85572a65a7312`, branche cx-ed-vertex-raw-classifier-r1. Revue code/Git/fichiers uniquement, aucun classificateur ou test rejoué, aucun ELF ouvert/exécuté, Cargo, DB, service, HTTP/cloud ou autorité lancé.

**Avis favorable borné à la classification des RAW figés ; aucune requalification composite.** Le résultat reste RAW_CANONICAL_CLASSIFIED_ONLY, contrôleur historiqueRED/portéeROUGE, composite_requalified=false/new_execution=false, anciennes bornes conservées et expirées. La portée de cette conclusion est le run figé, pas une qualification universelle d’un parseur de logs.

Paquet local cx-ed-vertex-raw-review-v1 :22 fichiers/485696octets, toutes tailles/SHA recalculées conformes au manifeste `000e53dd15a047a34b2fef8805740ef3f8b3fc243e1c304f415df010d5824934`. Prepared `c653aa17271c82892d92faa3ff43c52db49714583287aa530da50299e9fd189b`. Code classificateur et deux sources de tests identiques aux blobs Git197, AST seulement. Le log green-v3 porte11tests/OK en0.259s (0.297479s enveloppe), pas douze fonctions. Les mutations manquant/doublon/nom/statut/panic/types/filtres/filets/résumés/F8/final figurent dans ces onze fonctions ; aucun de ces tests n’est rejoué par Banc. Noms green-v1/v2 ne changent pas leurs refus historiques ; red-v2 est distinct des erreurs de harnais initiales.

Manifeste canonique54 SHA `2754c91e37815873fbdfd4bbeff2800ca2b3f259f1f24397b2038c88a5a9ddde` conforme. **51 fichiers non ELF** réhachés sans écart ; trois entrées elfs/* volontairement non ouvertes. Cette revue ne prétend donc pas refaire54rehashes physiques. Les pins ELF restent attestés par la conservation du porteur. Le rc0 réel de official-pass, composed-result.RED et supervisor.ROUGE sont relus sans modifier leurs preuves.

Inventaire `02455373cba66f02d8aba186c5a7de7c7f5ef2eb5bfc8fa985381c41e00cbec3`, kind POST_EXECUTION_RAW_EVENT_INVENTORY_NOT_ELF_LIST. Il est observé après exécution, figé avant implémentation ; indépendance limitée clairement conservée. Aucun test absent des RAW ne peut être découvert par ces seules listes. Cardinalités et filtres sont des constantes du contrat, pas ajustés en fonction des listes observées.

Les phases publiées concordent avec les résumés bruts relus :

| Phase | Réussis | Ignorés | Filtrés |
| --- | ---: | ---: | ---: |
| témoins Vertex | 7 | 0 | 2677 |
| lib | 2612 | 72 | 0 |
| navigateur | 45 | 0 | 2641 |
| e2e_code/filets | 2 | 0 | 942 |
| e2e_code/suite | 36 | 0 | 908 |
| e2e_lectures/filets | 2 | 0 | 942 |
| e2e_lectures/suite | 27 | 0 | 917 |

Chaque liste d’inventaire a des noms distincts et les statuts aux comptes attendus. Lecture du code : schémas JSON et clés dupliquées/nonfinis refusés ; noms comparés exactement par segments Unicode (sans réécriture), bool should_panic réellement typé, statut exact ok/ignored. running/result, compte sélectionné, tous champs du résumé et événements nommés/statuts/panic sont recoupés par phase ; les diagnostics running-over ne comptent pas comme résultats. Les filets portent leurs deux noms fixes, suite bornée par son préfixe, headers de phases ordonnés et binaire e2e unique contrôlés. Les72ignored ne sont jamais convertis en réussites.

F8 exige l’unique frontière ETAT_SUITE_E2E VERT puis exactement les huit jeux constants et verdict8/8 ; les précontrôles qui impriment aussi VERT ne sont pas pris pour les jeux. Preuve build dev présente une fois. Résumé canonique exige ordre des six sections, sept résumés de phases et fin suites jouées/`VERT` correspondant au contrat. Ces contrôles ne changent ni le statut de l’ancien composé ni les effets non observés.

Aucun écart bloquant repéré pour ces RAW épinglés. L’ancienne allocation n’est pas renouvelée ; pas de composite, OAuth/F300, nouveau cloud ou master qualifié. Le ticket Vertex reste ouvert pour ses autres limites. L’inventaire post-exécution et les rehashes ELF non refaits restent des limites explicites de cette revue.

## Réserves totales Mémoire reçues après cette relecture

C7 transmet troisP2 sur197b et retire la candidature du classificateur : headers running0/malformés/supplémentaires non tous rejetés ; sept résumés canoniques non liés à leurs blocs (peuvent être déplacés dans NAV) ; preuve acceptant Finisheddev seul sans Checking/Compiling rag3weaver. Même ticket porteur consigné avant correction groupée et nouveaux mutants prépubliés. Aucun nouveau natif requis.

L’avis ci-dessus reste strictement une concordance des RAW figés/pins et cardinalités, **pas une suffisance totale du parseur**. Les réserves reçues empêchent d’utiliser197b comme candidature suffisante jusqu’au nouveau SHA/correctifs et RO totale. Elles n’invalident pas les observations natives déjà conservées ; aucun CPU/mutant rejoué dans cette mise à jour.

## Revue totale nouveau86a1313366 : trois P2 traités, classification pure seulement

Référence exacte `86a13133667c89b7954445f7dc38eef6173ea6c5`, code SHA256 `9fc037896bd3a470585d5d54f7f602505e1c28f256e0bb5bf6a6d632fed66a7c`. Lecture du delta complet197→86, des trois mutants réels et des pins nouveaux, aucun classificateur/test exécuté par Banc.

- phase consomme maintenant toutes les lignes de header running (y compris zéro, malformé ou indenté), exige un seul header valide et refuse les événements avant les phases e2e ; le mutant couvre avant/pendant/après.
- Le résumé est segmenté en six blocs de contrat : nombres de résumés1/2/2/0/1/1, champs comparés dans le bloc attendu, aucun événement avant la première phase. F8 et preuve sont exigés dans le seul bloc check_features, dans leur ordre exact ; déplacer les sept résumés dans NAV ou déplacer les marqueurs ne passe plus ces conditions.
- La preuve exige un Checking/Compiling rag3weaver v0.1.0 au chemin union exact, avant l’unique Finisheddev au profil unoptimized+debuginfo. Le seul Finisheddev ne suffit plus ; autres crate/chemin/profil sont couverts par les mutants.

Paquet local cx-ed-vertex-raw-review-v2 :34 tailles/SHA recalculées sans écart, manifeste `5593b7b7b497cd194d63306209c9f66d20f52d7dca98e631e253f294c09fd60d`, prepared `d625f927f9e0d717c65bad55f98770d9af38645432788f648699faacb89ee794`. Classificateur et deux sources de juges correspondants sont identiques aux blobs86, AST seulement. Ancien paquet22/197 conservé.

Les trois stderr rouges prépub180dfd montrent chacun un seul test, une failure « ValueError not raised », sans ERROR ; leur résultat conserve trois rc1 distincts. Après correctif prépubc886, strict-green-v1 rapporte14tests/OK en0.376s, rc0 et0.417417s processus sous cap60. Ce sont des preuves CPU du porteur relues/hachées, aucun rejeu ici et aucun nouveau natif.

Manifeste canonique2754 et inventaire024553 inchangés. Les huit logs RAW utiles ont été réhachés contre ce manifeste, sans écart ; aucun ELF ouvert ni54rehashes physiques revendiqués ici. Lecture indépendante du résumé réel : six blocs1/2/2/0/1/1, F8+preuve dans check_features. Le log preuve contient effectivement Compiling rag3weaver au chemin union exact puis Finisheddev unoptimized+debuginfo. Les comptes/lib72ignored/NAV2641filtered/filets et noms typés/uniques de la revue initiale restent inchangés.

**Avis total favorable à la classification pure de ces RAW épinglés par ce nouveau code ; les trois P2 reçus sont traités dans cette lecture.** Limite inchangée : inventaire observé APRÈS exécution, figé AVANTimpl, pas ELF-list indépendant prétest. RAW_CANONICAL_CLASSIFIED_ONLY, composé historiqueRED/portéeROUGE et échéance expirée sont conservés ; aucune requalification composite, ancien GO réactivé, candidat runtime/master/OAuthF300 qualifié ou retrait de HOLD C7 automatique. Décision de candidature reste à C7 après les autres RO. Aucun CPU/ELF/proc/DB/service/cloud lancé par Banc.
