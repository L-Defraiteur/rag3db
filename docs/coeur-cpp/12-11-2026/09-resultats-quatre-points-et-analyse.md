# Quatre points supplémentaires : coût compatible avec les parcours répétés

Porteur cœur C++, [même ticket d’annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md).
Les quatre points autorisés sont terminés : **STOP mesures**, aucun build ou
mesure supplémentaire automatique. Le différentiel DdlEveryMinute reste ouvert.

## Résultats et dispersion

R=4 096 retraits, mêmes clés longues, même binaire, trois paires alternées par
cellule. Temps : somme des appels de retrait, hors préparation et validation.
Valeurs en millisecondes, médiane [minimum ; maximum] de trois exécutions.
La ligne N65 536/U64 est l’ancre précédente, les autres sont les quatre nouveaux points.

| Entrées N | Plages U | Clés préconnues : médiane [min ; max] | Plages : médiane [min ; max] |
|---:|---:|---:|---:|
| 16 384 | 64 | 0.282405 [0.281553 ; 0.284716] | 6.920786 [6.633396 ; 7.021285] |
| 65 536 | 64 | 0.296892 [0.295436 ; 0.303439] | 26.442622 [26.012750 ; 28.604142] |
| 262 144 | 64 | 0.324703 [0.321490 ; 0.325820] | 105.293110 [105.181885 ; 107.247036] |
| 65 536 | 8 | 0.294349 [0.260387 ; 0.294385] | 3.848231 [3.705100 ; 3.984859] |
| 65 536 | 256 | 0.300968 [0.298219 ; 0.305091] | 102.217416 [101.361114 ; 102.839854] |

À U=64 constant, N×4 donne des temps de plages ×3.82, puis
×3.98. À N=65 536 constant, U×8 donne ×6.87, puis U×4
donne ×3.87. Les clés préconnues restent vers0,28–0,325ms pour
les mêmes4 096 retraits. Les quatre points corroborent le coût proche de N×U
du parcours de toutes les partitions pour chaque plage. Il s’agit des cinq
points observés, pas d’une loi universelle ni d’une courbe complète.

Le mode clé connaît déjà les clés et omet leur relecture/matérialisation :
**ce n’est pas le moteur de base complet**. Aucun facteur de régression du
rollback complet, aucune causalité du pool plein DDL déduits de ces chiffres.
Trois répétitions seulement ; minimum/maximum et écart absolu médian (MAD)
disponibles dans les données, sans intervalle de confiance inventé. Les cellules
ont suivi l’ordre prescrit, pas un ordre intercalé/aléatoire entre N et U.

## Validations et mémoire

24processus,6parcellule,2 352plages brutes, tous codes0, sans retry.
Volumes, offsets, absence des4 096retirés, survivants et réinsertion validés
dans chaque processus. Empreintes entrées/survivants identiques dans chaque
cellule ; à N65 536 elles sont aussi identiques à l’ancre U64. Les signatures
à Ndifférent changent normalement et ne sont pas comparées comme égales.

Pools observés aux frontières, mêmes valeurs dans les deux modes et tous
les appels de la cellule :
- N16 384 :4 018 688octets ;
- N65 536 :7 647 744octets ;
- N262 144 :25 154 048octets.

Le pool configuré reste512Mio, jamais plein dans ces points. Les frontières
ne mesurent pas les allocations transitoires ; les métriques RSS incluent la
préparation, ce ne sont pas des compteurs d’allocation du seul retrait.

Une seule acquisition de poste mesure exclusif, budgets démarrés après
acquisition : cgroup effectif1Gio/swap0 vérifié ; unité890s+arrêt2 sous900s,
enfants57s+2 sous60s. Contrôleur2,831s, service2,874s, CPU2,395s, pic153,5M
sans swap. Aucun autre lourd pendant la mesure. Unité inactive/MainPID0/poste0/4
relus ; arbre et C7 informés de la libération, aucune réservation conservée.

## Provenance et bruts publiés

Même binaire SHA256
`0c868053afea43df3cb2eb5fe8e77f597701b8ab4eb2a0c403ca854ef64cf704`,
même source SHA256
`fdc3e90feb364178537c11f2a05743fbb965851e898987288d1f69cb6eca1d08`,
montés en lecture seule et vérifiés au début/à la fin. Premier refus ELF et
préparation conservés, cinq composants système acceptés, aucun rebuild/relink.

- [Données par cellule, dispersion, signatures et argv](preuves/quatre-points/preuve-execution.json).
- [Empreintes des bruts](preuves/quatre-points/SHA256SUMS).
- 24 fichiers `point-*-process-*.stdout` dans [les preuves](preuves/quatre-points/),
  avec6résumés natifs parcellule et64/64/8/256plages parprocessus.
- [Bornes réelles](preuves/quatre-points/limites.json), [journal unité](preuves/quatre-points/unite.log).
- [Ancre et premiers bruts](08-resultat-premiere-cellule-retrait.md).

Les originaux restent à luciepc dans
`~/.cache/rag3db-moteur-notes/annulation/cx-pilote-retrait-0511dece7/quatre-points-12nov-01/`.
Aucune nouvelle mesure, matrice, DDL, code produit ou lib distribuée modifié.

## Décision suivante préparée seulement

Le scan par plage a un coût matériel confirmé ; le lot ne doit pas être présenté
comme prêt sur la seule réussite fonctionnelle du témoin réduit. La
[proposition minimale à relire](10-proposition-regroupement-undo.md) examine
un regroupement strict de plages adjacentes d’undo, ses limites et les preuves
qui permettraient de choisir. Aucun changement moteur appliqué.
