# Canonique FTS arrêtée au premier rouge TX blobs

SourceAPP **9576079d62c3f46bdf241cd19c9ab30e9218333e**, pilote **9a82d4c8d331096bc681aefdfc53c918e0f995d2**, allocation C7-FTS-CANON-9a82-16nov. Deux RO finales et vrai préflightcachepostposte/publi avantassert requis réellement franchis. Aucun retry ni exclusion.

| Étape officielle | Résultat réel |
|---|---|
| Témoin initial lecture FTS | 1/0/0 |
| e2e_code_sync | 21/0/0 |
| e2e_lectures | 27/0/0 |
| e2e_code | 36/0/0 |
| e2e_contrat_transactions | 2/0/0 |
| e2e_tx_ligne_a_ligne | 3/0/0 |
| e2e_tx_merge_puis_copy | 1/0/0 |
| e2e_tx_par_paquet_arret | interrompue au premierFAILED, non qualifiée |
| TXvecteurs, F8, preuve, lib entière, navigateur | NON JOUÉS |

Les filets auxiliaires2/0/0 par suite ne sont pas ajoutés aux compteurs. Les dix cibles vertes antérieures ne sont pas recomposées en canonique verte. Le résumé officiel est conservé tel quel : il s’arrête à TXparpaquet, aucun VERT final fabriqué.

PremierFAILED : `e2e_tx_par_paquet_arret::la_garde_des_comptes_retire_un_document_sans_ligne_en_blobs`, assertion1018 : **302documents /301lignes**, après deuxième synchronisation d’une fixture avec fantôme9000000 commité. ModeBlobs et handlecourant, pas qualification du fallbackFiles. L’écart seul n’attribue pas une régression nouvelle et ne fournit pas de bijection IDsFTS/graphe. [Ticket avant correctif](../tickets/2026-11-16-la-garde-des-comptes-en-blobs-reste-a-302-documents-pour-301-lignes.md)6a41ad5bf2, ROHotfixs/ED demandées ; aucune exécution après rouge pour innocenter, aucune DB native ouverte par le diagnostic.

## Temps, arrêt et ressources

INITIAL1180601.419729332/ENDoriginal1186601.419729332 (6000neufs après poste), reçuEND1186597.458971519<=original. Firstred1181259.031107475, soit657.611378s depuisINITIAL ; contrôle indépendant après nettoyage à684.867024s. Watcher premiermatch1181259.028946841, inode165196878/dev55,19231octets, SHA7c4752e610869b1cdd51b7a897a3e2aed6fcaf126c22b45c78354c44c3194c48. Préfixe exact des octets reconstruits depuis le log brut vérifié contre ceSHA et archivé séparément, sans réécrire le journal.

Scopevalidation-ajgm4kao **ROUGE1**, erreur «descendants laissés après la commande» lors du stop ; le confineur les arrête ensuite : **restes[]/nettoye=true/autoritéTERMINE/busabsent**. Scan indépendant983identités PID/start : alive[]/unknown[]. ComposéROUGE/rc1, aucun verdict vert partiel.

Health avant/après GET/sante :7878granite278 et7879bge-m3 HTTP200, baux0/en_cours0 à la restitution ; PID3906704/3904390 inchangés. PGfixture5433 répond au protocoleSSLRequestN, sans SQLshared de diagnostic (pg_isready absentPATH). Le premier GET/health404 était une route inexistante, corrigée par lecture du contrat /sante avant entrée, pas une preuve de panne. Aucun restart/déploiement/API7970/baseLucie. Créneau rendu directement Embar+C7 dèsquiescence avant archives, accuséG reçu puis sa canonique reprend sous son autorité originale ; plus aucun appelBGE/PG de Recherche ensuite.

Poste Recherche libéré, autreslots conservés. Target de session gardé7631837937octets. Sources et caches de préparationv1/v2 conservés. Aucun retrait autorisé ou effectué.

## Archives fidèles

PC/local : `~/.cache/rag3weaver-build/cx-archive-recherche-fts-existing-canon-v1`, **65fichiers réhachés sans écart**, manifesteSHA256 **1e5abe3d4b3830be24a9b09bccc9a73533db21127951d1017349e033bf2bb12d**. Contient client/complet, contrôleurs réellement copiés, snapshotsavant/aprèsposte, reçus/verdicts/health/scans, tous logs canoniques atteints, résumé arrêté et octets premièrelecture. [Manifeste](fts-existing-canon-red-manifest.json).

DeuxELF copiés et hachés **APRÈSrouge**, pas d’attestation avant premiers tests : libunitaire484666592octets SHA5c45191d9d3cf22c46cd9cb827ea4108bbe061baf4335d75f6cc77cc16c750a3 ; e2e471964952octets SHAc29f82e1d904a710d489fea9eb909594796bd0b3500792db26d42e51008ca073. Copies conservées hors target sur les deux postes. Les rouges/cibles précédents restent intacts.

La fixture privée de l’échec a été retrouvée sous e2e-gardes ; son PID1254532 appartient aux983owned terminés. Conservation par copie filesystem seule en cours, sans ouverture DB/FTS. Une éventuelle analyse nativeRO exige une recetteCOPIE600 et un GO distinct C7 ; aucun warmup/purge/writer/replay. Parentmaintenance et filslecture restent ouverts, aucun master.

Conservation fixture terminée : archive filesystem3fichiers/40255669octets, sourceavant/sourceaprès/copieSHA identiques, manifeste69b1734910cace8d0b05ff4d2a5b128d2594c9e2834eb136167aa462dd433192, sans ouvertureDBnative. La RO Hotfixs est rattachée au ticket : bypass statique des garde/rebâti dans code_sync::synchroniser_par_generations, hypothèse forte non tracée à l’exécution ; aucun correctif ni attribution au fallbackFiles.
