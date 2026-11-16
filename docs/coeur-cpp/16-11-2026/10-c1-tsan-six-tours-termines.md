# C1 : la passe TSan historique a fini ses six tours

Lecture finale le16novembre, aucune relance ni arrêt forcé. Parent C1 reste [ouvert](../tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md) ; ce n’est pas une qualification du correctif033/888.

## Fin effectivement observée

Contrôleur ~/.cache/rag3db-moteur-notes/dict-tsan/passe.out sur luciepc : FIN après tour6 à12:38:29. Au relevé final, aucun argument de processus ne désigne dict-tsan.sh ni le binaireconcurrence_test.tsan ; place rendue par la chaîne terminée. Source annoncée64eee2e1f6c3599f5e4a0bcd3369f6202e33eaca ; gitlog enginePaths sur ce commit donne59ac9c86bc2b3529b590a7462ab7051e3ad26b8a, conforme au BUILD_ID réellement émis dans les six stdout, profilRelWithDebInfo/tsanTRUE/rcOFF. Pool du scénario200Mio selon passation et stdout199Mio, aucun nouveau réglage appliqué ici.

| Tour | Fin | Code processus | Avertissements TSan | GTest fonctionnel |
|---|---|---:|---:|---|
| 1 | 08:55:52 | 66 | 15 | 1passed |
| 2 | 09:40:36 | 66 | 18 | 1failed,catalog-as-expected |
| 3 | 10:23:49 | 66 | 16 | 1passed |
| 4 | 11:08:04 | 66 | 19 | 1passed |
| 5 | 11:53:58 | 66 | 23 | 1passed |
| 6 | 12:38:29 | 66 | 20 | 1passed |

Total111avertissements, pas111défauts distincts. Les six rapports complets lus ne nomment ni DictionaryChunk ni StringChunkData ; le contrôleur compte0segv à chaque tour. Cela ne prouve pas absence de course, ne blanchit pas les autres familles, et ne rend pas TSanvert : **les six processus sont rouges66**. Aucun résultat ancien repeint.

Les rapports nomment entre autres RelTable::getNumTotalRows/reserveRelOffsets, CatalogEntry::setTimestamp et NodeTable::getColumn/addColumn ; origine/appelants/tickets et paires de piles restent à confronter avant attribution. La liste tsan_signatures.txtC5/C6 ne valide pas automatiquement ces rapportsDdlEveryMinute.

## Rouge du tour2 conservé séparément

CREATE_VECTOR_INDEX refusé pour poolplein, puis présence doc_vec après réouverture alors que scénario attendfalse. Comptages de lignes corrects dans ce relevé ; pas de régression de perteNote déduite. [Ticket de départage oracle/état](../tickets/2026-11-16-le-scenario-tsan-revoit-un-index-dont-la-creation-a-ete-refusee.md) créé avant signalement : ne pas supposer qu’un nom présent est un indexREADY, ni affirmer qu’un étatNOTED a été observé aprèscoup. Aucun nouveau juge ni runtime lancé.

## Préservation et prochaine lecture C1

[Lecture finale, SHA et résumés](preuves/c1/tsan-final/lecture-finale.json), [archive brute des six rapports/sorties/contrôleur](preuves/c1/tsan-final/rapports-et-sorties.tar.gz), [empreintearchive](preuves/c1/tsan-final/archive-sha256.json). Tous les fichiers bruts restent sur luciepc dans dict-tsan et copie locale ~/.cache/rag3db-moteur-notes/dict-tsan-final ; aucun core/binaire/proof ancien supprimé. Le binaireTSan original reste à sa place, aucun rebuild.

Conformément à la passation, six tours sans nom dictionnaire invitent à poursuivre **la lecture** des accès/concurrences et durées de vie de StringChunkData pendant NodeGroup::checkpointInMemAndOnDisk, et non à pousser256nonprouvé. Cette suite de lecture ne se confond pas avec la garantie d’exception des offsets déjà isolée/5ciblésverts. La passe globale888 reste en attente de revue/admission ; aucun cap/GO ancien réactivé et aucun appel7970 pendant la fenêtre froide.
