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
