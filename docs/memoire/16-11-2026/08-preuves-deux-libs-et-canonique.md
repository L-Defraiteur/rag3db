# Deux libs complètes et canonique qualifiées, sans repeindre les rouges

APP **734087a4e99d54331aac2f6321df9ef6da3b6d9c**, source fixturesr4/base mastertiesd395.
Canonique PILOTE **f7ef1b0153dc5c337a9a9e3cb2e234c96a84e789** ; correction
SECONDELIBSEULE PILOTE **f812f1fd8024aac0ab1a70e2633d75e326c5263b**.
GO exact C7 distincts, créneaux modèles coordonnés. [Ticket parent ouvert](../tickets/2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md),
[recette07](07-recette-seconde-lib-seule.md), [rougesv1/v2 conservés](06-preuves-canonique-verte-seconde-lib-rouge.md).

## Résultats réellement jugés

| Origine | Étape | Réussis/échecs/ignorés |
|---|---|---|
| Canonique v2 | codex_threads ciblé | 6/0/0,0,31s |
| Canonique v2 | F8 + preuve | 8/8 et rag3weaver revérifié |
| Canonique v2 | Première lib entière large | 2596/0/72,0filtré,81,60s |
| Canonique v2 | NAV | 45/0/0,79,45s |
| Correctif v3 | Seconde lib entière DIRECTE | 2596/0/72,0filtré,82,75s |

Comparaison des NOMS dans les deux vrais journaux :2596réussis=2596,0absent/0enplus ;
72ignorés=72,0écart. Ce sont deux passages des mêmes2596fonctions, pas5192juges
nouveaux ; les72ignorés restent NON JOUÉS. Le nouveau helper Rust fait partie des
6ciblés et des deux libs. Les trois sockets rougesv1 et le cwd rougev2 passent.

Le résumé canonique v2 demeure byteidentique, SHA
**0dddc68c959407fc97c88f024b1911fa9dcd1c718a738175b8f9f3d2e99076ca**,
produit par la vraie passe, jamais reconstruit. V3 ne lance aucun Cargo/rustc/F8/NAV,
ni nouveau target. Le résultat composé v2 demeure RED (son secondappel mal monté
a échoué), tout comme v1. V3 publie le composé explicite
`prior_canonical_plus_corrected_second_library`, avec les deux qualifications et
leurs reçus distincts ; aucun historique rouge effacé ou réécrit.

## Identité et profil avant exec

ELF original et copies identiques aux deux étapes et aux archives rouges :
**6f12273abeb4a957dd5abdcd0f518ee673607146eefbe2c2938769d9bfdf8e74**,
484292712octets. Ledger/copie publiés AVANT seconde lib v3, source734 et
canon0ddd/copies réattestés après et au final. CARGO_MANIFEST_DIR vaut exactement
`/home/lucied/git_workspaces/cx-memoire-script-fixtures-r4/extension/rag3weaver`,
existant et canonique. Profil complet whitelist archivé avantassert ; SDKexporté
UNEfois puis Pythonpréexec PID1982130 archive actual/expected/diff{}, vérifie
manifest/SDK/ELF et appelle execve[ELF] sansfiltre. C'est une capture Python
préexec, pas une attestation GDB/maps au main inventée.

SDK59ac complet, magasin réel explicite, SO b7e9...2101 et vector1669...7561
inchangés, pool1GiB/rôlesCPU. TMPprivé nouveau
`/home/lucied/.cache/rag3weaver-build/m-8e82928990`, btrfs/dev55/inode166295844/
UID1000/0700, gardeUTF8pirechemin89octets ; identité vérifiée postscope. ÉtatsXDG
et RAG frères sousP3. Capturecarry source/targetmarker/reçuv2TERMINE/manifestc804/
canonVERT0ddd/ELF/FP+buildrsRO avant/aprèsposte, pas de purge ni d'adoption.

## Borne unique et terminaison

INITIAL **1179575.678574093**, ENDoriginal **1186775.678574093** inchangés depuisv1.
Admissionv3 APRÈSvraiposte1184089.97568641, reste2682s, nouveau reçu
validation-ekmirbg1 bornéEND **1186772.020708913**<=original. Aucunreset7200.
Durée driver **4599,390194492s** depuisINITIAL ; final composé **4599,834155455s
<7200**, incluant première compilation, deux anciensrouges, diagnostics et pauses.
Le final relit SHA/canon/profil/scans, autoritéterminée et busabsent, contrôle END
APRÈSopen immédiatementavantdump et utilise le verdict réellement écrit.

Scopev3 **VERTcode0**, retour extérieur0, restes[],nettoyétrue, autoritéTERMINE.
32GiB effectifs/HIGHmax/swap0/cpu.max500000/100000. Pas de pic final inventé,
aucune absence GPU inférée d'un rôleCPU ou d'un compteur absent. Les statistiques
globales de charge des anciens journaux ne sont pas attribuées à cette portée.

Scan final pilote1184175.511705576 puis indépendant1184320.392592407 :
**426observations PID/start**, owned_alive[],owned_unknown[],source/T/TMPexe-cwd-FD
found[],11UNKNOWNglobaux nonattribués consignés. Bus et cgroupabsents. Target
5658176215octets et sources/TMP/preuves conservés, aucune suppression/retrait.
GET/sante frais aprèsquiescence :7878granite768dense et7879bge1024dual+sparse,
HTTP200/exe1794771361/Flex32/nonfactice inchangés, baux0/en_cours0. Poste etusage
BGE/PG rendus explicitement C7 ; aucun appel futur de cette qualification, aucun
restart/API7970/base de Lucie/sondeSQL/reset. Aucun crédit de créneau futur déduit.

## Preuves et suite

**40fichiers PC/local**, SHA+tailles réhashés0écart :
[manifeste](script-fixtures-green-v3-manifest.json) SHA
**da809b576a3b5e53dde0255c8c0f50f8f62c0fd196fec374a6814caafebeb618**.
Racines ~/.cache/rag3weaver-build/cx-proofs-memoire-script-fixtures-green-v3 et
suffixe-local, {client,controller,supervisor,carried-canonical}. Contiennent les
journaux complets des deux libs/canonique et leurs preuves, ELF copié, profils
sans jeton, snapshots et provenance explicite ; autorité privée exclue.

Qualification du correctif cfg(test) acquise en branche : la preuve système
établit le mécanisme du writer hérité, pas l'unicité de la cause historique de
tout ETXTBSY. Parent/fils restent ouverts en attente ROpreuves/intégration C7,
pas de GOmaster/retrait déduit. Futur candidat frais doit préserver les nouveaux
apports master, vérifier modes/OID des tests, et attendre l'arbitrage C7 de toute
recette d'intégration nécessaire. Aucun pushmaster ni pose d'hôte effectués ici.
