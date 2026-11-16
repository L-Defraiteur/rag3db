# Relecture indépendante des preuves FIN-RO

Avis favorable à la qualification **FIN_RO_OBSERVED sur copie privée**, bornée à l’état observable après cleanup. Référence exacte : af2eb41ecb1d53b4fcdaf401748a8d3399cb5a51, branche immutable `cx-arbre-fin-ro-preuves-20261116-r1`, [rapport porteur 11](../../arbre-principal/16-11-2026/11-preuves-fin-ro-etat-observable.md). Le ticket469 et le [parent oracle](../tickets/2026-11-16-oracle-des-copies-finales-des-trois-bases.md) restent ouverts ; cette relecture ne qualifie pas l’oracle global des trois bases.

## Intégrité et provenance

Recalcul indépendant sur `~/.cache/rag3weaver-build/bascule-live-20261116/debt-fin-ro-proofs-pc-local-01` : **76/76 tailles et SHA256 conformes**, 567439444 octets. Manifeste Git SHA256 `05661b1fddc82c1e66d21f218d0c8cdfd12b69d8ef16432a2c55c3b127a9462a`. Le poste PC n’a pas été réinspecté : cette conclusion concerne la copie locale et ses preuves archivées.

Le prepared recalculé vaut `6ee3ad73dc681600da790823075f8456aae5929f6412fd5bf9a04bfc6e02cf67`, pilote `5b7a4b00a8e006cca50022a007837c300bb60d68`. Les neuf fichiers de contrôle archivés correspondent chacun au SHA du prepared ET aux blobs Git de ce pilote. Le reader archivé est inclus dans les 76 rehashes : SHA256 `add01058f3b0d6efd28d936c865b1ba9071c0e2eb85f559743bf0c52aeea6371`. Le ledger épingle ce reader et l’outil IDS `d0dd13e6d403e614038b251f413637d5f6d02a98ba1c10509388d0be1f5e15c7` ; l’ELF IDS n’est pas une seconde observation physique effectuée ici.

Les huit statuts natifs ont rc0. Les huit stderr rapportent le même BUILD_ID59ac complet. L’environnement archivé sélectionne ce magasin SDK. Aucune capture maps du reader FIN n’est fournie ; les pins, le profil et les BUILD_ID ne deviennent pas une preuve maps. Les maps historiques du writer restent distinctes.

## Invariants recalculés sur les publications

Les deux publications graphe sont égales en JSON canonique trié, nombres non finis refusés. Chacune correspond à sa référence séparée : audit RAW normalisé pour l’observé, COPYA avant pour la référence. Elles contiennent 38 tables ; l’égalité couvre leurs champs publiés avec multiplicité, et non une nouvelle observation SQL. Le pilote FIN réutilise ces publications et ne rejoue pas le snapshot. Les lignes internes exclues et les champs vector/marker/claim autorisés sur la cohorte restent hors de l’invariant applicable.

Les cinq sorties brutes `LIVE_ID` reconstruisent exactement les listes publiées, toutes sans doublons ; leurs ensembles égalent les offsets physiques archivés. Les générations et les **2247 entrées d’empreintes FTS** sont identiques à la référence COPYA.

| Index | IDs distincts | Génération |
| --- | ---: | ---: |
| Lucivy_File | 6313 | 14 |
| Lucivy_Library | 398 | 9 |
| Lucivy_Scope | 65913 | 18 |
| Lucivy_ScopeContent | 61223 | 52 |
| Lucivy_Symbol | 73373 | 12 |

Le résumé vectoriel consigne dette0, 97247 UUID contrôlés, dimension768, toutes composantes finies, index READY, et exactement les 469 UUID de DEBT_PENDING, sans doublon. La requête de dette a zéro ligne ; la requête vectorielle a 97247 lignes/cinq colonnes ; show_indexes a quatre lignes. Lecture statique de `audit.finished` : unicité UUID, dimension et types flottants finis, marker==text_hash, Counter exact de la cohorte et READY/extension_loaded=true sont effectivement jugés avant publication. **Les composantes et lignes vectorielles ne sont pas archivées : cette revue vérifie le raccordement du juge et ses résultats publiés, sans recalculer les normes/composantes depuis des valeurs absentes.**

Les 469 claims opérationnels sont consignés séparément, SHA multiset `3cb2698397b5537c6d123e33520ee01d45e47d2a989df30138e03ccb2df76472`. Le juge vérifie leur format et au plus un writer logique sur la cohorte. Ni claims0 ni dette due aux claims ne sont déduits. `computed_count` et `reused_count` restent null ; aucun minimum de calculs n’est revendiqué.

## Conservation, fin et limites

Les inventaires source avant/après sont strictement égaux. Les 2252 entrées de la copie FIN avant DB/après sont strictement égales aussi. Le champ `readers_zero_added=false` signifie, d’après le code, **aucun fichier readers vide ajouté** ; il ne signifie pas un échec. Aucune exception readers ni nouveau WAL n’est nécessaire pour expliquer les inventaires. Les répertoires font partie de la structure source archivée ; leur comparaison effective après lecture est contrôlée par le pilote avant le résultat vert.

Les six scans avant/final des trois bundles privés portent found[] et chacun onze UNKNOWN globaux non attribués, conservés comme inconnus. Le verdict archivé est VERT/code0/erreur nulle/restes[]/nettoyé. Autorité publique terminée ; owned-final-check alive[]/bus_exists=false. Ces observations archivées ne constituent pas un scan de processus refait pendant cette revue et ne prouvent aucune absence GPU globale.

Allocation FIN distincte600, outer598+kill2, common530. END1192105.971006448 donne STARTcommun dérivé1191575.971006448. L’observation de publication transmise par le porteur à1191670.633813398 fournit une borne inférieure à95s après ce START ; le composé archivé ne contient pas d’elapsed exact, et cette revue n’invente ni durée poste exacte ni horodatage manquant. Le prepared expiré antérieur et EXPIRED_WITHOUT_LAUNCH restent une histoire distincte.

COPYA et auditEDD restent ROUGES. Le shutdown initial n’a pas été observé. Durabilité, fermeture, migration complète, oracle source matched, LIVE et bascule ne sont pas qualifiés. Aucun rerun, aucune ouverture DB, aucun modèle, service, writer ou inspection proc en direct dans cette revue. Aucun code produit modifié.
