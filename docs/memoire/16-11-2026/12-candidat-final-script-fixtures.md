# Candidat final : fixtures de scripts qualifiées, deux tickets résolus

Branche NEUVE **cx-memoire-script-fixtures-integration-r1**, W
`~/.cache/rag3weaver-build/cx-wt-memoire-script-fixtures-integration-r1`.
Base master fraîchement récupérée **57fbd9aa56f4b7caeacb891388f3f10cbbb39d71**.
Qualification source **e2722d521192b3c25f9dd13e0e9aed47a7d7487f** surbef,
pilote **01f3cf6ea9ffbf418508e8dd07b25e8233afaab6**, preuves
**465d68a67862e264d65df63c35e83e188606e804**. [Rapport11](11-preuves-union-deux-libs.md).
Anciennequalification734/2a068 et sesrouges restent séparés et inchangés.

## Problème et résultat du lot

Les deux fixtures codex écrivaient leur script dans le parent multi-thread ;
un autre fork pouvait hériter d'un writer et bloquer exec par ETXTBSY. Le témoin
Linux privé a reproduit ce mécanisme et réfuté rename seul, avant correction.
Le helper cfg(test) fait écrire l'exécutable par un enfant attendu au plus5s :
parent sanswriter, chemin/corps passés en arguments littéraux, pas d'interpolation.
Un test de chemin/corps littéraux est ajouté, les deux fixtures historiques
l'utilisent. **Code production livrer inchangé**, pasretry, pasrelaxation de délai.
La preuve ne prétend pas établir l'unique cause de tous les anciensrouges.

Le profil de qualification long débordait ensuite sun_path ; un fils a tracé
la correction TMPprivé court surdisque0700/UID/identity/byteguard, étatsfrères.
Sonde AF_UNIXdistincte107OK/108-115refus, sans inventer l'ancienerrno Rust.
Le cas CARGO_MANIFEST_DIR manquant de l'ELFdirect a repris le contrat du ticket
ancres, sanscorriger ni fermer ce ticket étranger au lot.

Qualification UNION réellement verte :6ciblés/F8 8/8/preuve/lib2605/0/72/NAV45,
puis seconde mêmeELFc3972605/0/72 ; mêmesCounters/noms sansdoublon/absents,
72ignorésNONJUGÉS. Nouveauledger sourcee272/copie avantseconde, SDK/C_MANIFEST
préexecdiff{}, pasmainmaps inventés. Canonf7ea...3a3c intact, fenêtreDISTINCTE3600
postevrai terminée560,038s, scopeVERT0/TERMINE/restes[]/busabsent/ownedquiet.
47preuvesPC/localmanifest054388...59a9 immuables, pasrejeu/artefact créé ici.

## Clôtures bornées et histoire préservée

Seuls le [parent ETXTBSY](../../tickets/closed/2026-11-15-un-temoin-de-codex-rougit-sur-text-file-busy-sous-la-charge.md)
et le [fils profil TMP](../../tickets/closed/2026-11-16-le-profil-tmp-prive-depasse-la-longueur-des-sockets-des-temoins-du-pont.md)
sont proposésclosed/Étatcorrigé. Historique parent fusionné append-only depuis
2a068 et465, fils repris du même ticket2a068 jamais dupliqué. Aucun parent M8,
ticket ancres ou autre famille fermé par proximité. Liens actifs réparés,
passation2.1 actualisée, READMEproduit et API inchangés.

Les manifests JSON/scripts ajoutés APRÈSsource testée restent exclusivement sur
les branches de preuves/pilotes distantes. Seul ce rapport et rapport11 sont
portés enMarkdown, liens immuables vers scripts/JSON exacts, aucune conversion
ou renommage d'un JSON enMD. Les JSONaudit déjà présents AVANTgel e272 restent
inchangés et historiques. Source734physique/admin/gitlink/fullSHA, anciensELF6f12,
rougesv1/v2, cible/TMP/preuves conservés ; aucun retrait autorisé.

## Audit du candidat et garde en lecture seule

Sept patches sourcee272 reposés sur57f sansconflit, range-diff sept`=`. Le seul
Rust apporté au master par le lot est codex_threads.rs : mode/type/OID strict
sourcee272 ET734 ; préfixe avantcfg(test) byteidentique au master. Tous les
fichiers horsdocs (tests/templates/locks/gitlinks/C++/Rustoutils inclus) identiques
à e272. Tous les apports master57f horslot conservés, suppressions aussi.

Depuis basebef jusqu'à57f, seuls docs/ops/procédures/SHELL-CPU ont changé. Le
vrai code_apporte de la garde doit être vide ; la passe sourcee272 reste celle
qualifiée, jamais un résumé renommé sur le candidat. Pile à juger :
`--pile bef0b27cb33b9143dabee019fccdf7453da553a3 e2722d521192b3c25f9dd13e0e9aed47a7d7487f 57fbd9aa56f4b7caeacb891388f3f10cbbb39d71 HEAD`.
Les contrôles réels aux deux postes sont à consigner après publication du candidat.
Aucun pushmaster ici : RO exacte puis ÀTOIexact C7, postePush, dernierfetch/garde
et trois contrôles restent requis. Hôtes927 conservés, aucun redéploiement.

Audit index effectué avantcommit : **8991entrées/absences masterprotégées,0écart** ;
**6916entrées horsdocs identiques sourcee272,0écart**.
Le postsource ne porte que8cheminsMarkdown (dont suppressions desanciens
chemins de tickets et créationsclosed) ; aucun JSON/script nouveau à transférer.
Codex mode/type/OID : `100644 blob 8ad670824e4700cd5535cee16bb732704df07fbc`.
