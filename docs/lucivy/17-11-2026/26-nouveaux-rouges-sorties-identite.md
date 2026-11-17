# Nouveaux témoins RED des sorties et de l’identité — préparation seule

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Source **bdf83ad76cf73a74cdde8358affb364b857e27e2**, lucivy, branche `cx-lucivy-fs-output-red-17nov`, parent0668432.
[Pins](preuves/fs-output-red-bdf83ad/source-pins.json). Aucun correctif des
nouveaux défauts, aucune compilation/test/rustfmt/ELF/proc/corpus/PC/target/
service ni import des sources préparées. Lecture/diff/check/SHA seulement.
Attendus ci-dessous **NON OBSERVÉS** ; cinq nouveaux rouges restent à acquérir.

## Revues de source066 réservées

Recherche : refus/coverage/snapshot/buildplan favorables à lecture, réserves
sorties aliasées, collision reçu/rapport et rapport dans input, puis identité
manquante→0. Banc3c62cb73ec confirme notamment la collision sans alias/course.
Les quatre SHA de066 sont conformes dans les deux revues. Le ticket a été
complété AVANT cette extraction baseline, aucun correctif anticipé des réserves.
Les six rouges0f restent acquis et figés, ils ne sont pas refabriqués.

## Raccord baseline utilisé par les témoins ET le natif

`ManifestSink.report_paths` déclare les sorties du vrai consommateur mais
reste **ignoré par la gate de cette baseline** : ce manque doit être observé.
Le natif donne le même chemin pur CARGO_TARGET_TMPDIR/lucivy-reports/
v3_ground_truth_report.txt que l’ancien helper, sans créer le parent à la
construction du plan. Sa callback utilise désormais `with_v3_report` :
appel builder, trace Report, create_dir_all parent, File::create qui tronque,
puis panel. C’est l’extraction de l’effet réel, avec le défaut préservé,
pas un writer parallèle uniquement testé. Le builder des petits témoins est
un compteur ; aucun moteur ni index n’est construit.

`document_ordinal` est utilisé par la vraie boucle de recherche : même
unwrap_or(0), même conversion usize et même assert borne que066, enveloppés
dans Result. La variante Missing est posée mais pas renvoyée par cette
baseline. Son témoin doit donc observer Ok(0), pas un échec de setup.

## Cinq cas et un contrôle proposés

| Nom exact | Attendu statique sur bdf83ad, avant fix |
|---|---|
| report_receipt_collision_is_refused_before_consumer | Ok(Ok(1)), builder1/report réel ; reçu ADMITTED tronqué en index-report |
| report_inside_input_is_refused_before_consumer | Ok(Ok(1)), builder1 ; rapport réellement créé sous input |
| aliased_output_parent_is_refused_before_consumer | Ok(Ok(1)), builder1 ; reçu créé sous input via parent lié extérieur |
| missing_document_ordinal_is_not_document_zero | Ok(0) pour None et taille1, assertion Missing manquante |
| invalid_flags_have_no_receipt_or_corpus_effects | Err Flags précis mais Event::Manifest et reçu REFUSED créé, assertion zéro effet échoue |
| distinct_outputs_and_present_ordinal_reach_consumer_once | PASS, sorties distinctes, reçu ADMITTED intact, builder1/report, Some(0) accepté |

Les sorties sont toutes dans les petits TempDir possédés ; aucun vrai corpus,
chemin d’index ou service. Le lien du témoin alias est créé dans son TempDir
et pointe seulement sur l’autre TempDir possédé. La plateforme admise est
Unix/Linux ; pas de skip ou substitution sur autre plateforme.

Les observations result/compteur/events et octets bruts manifest/report sont
imprimées AVANT la première assertion discriminante. Le reçu tronqué est lu
comme octets, pas parsé en JSON avant le FAIL attendu ; pas d’ERROR de harnais
transformée en rouge sémantique. Les cas sorties exigent un refus Plan,
builder0 et aucune entrée en capture Enumerate/Read ni création reçu/rapport.
Le contrôle positif vérifie séparément le hash interne et imprime le reçu.

Le cinquième cas explicite la future exception **configuration invalide sans
reçu** : retourner Flags avant metadata/enumerate/read/output. Recherche juge
cette orientation cohérente, à condition de la documenter et de prouver ce
zéro effet. Les refus de données IO/UTF8/panel garderaient leur reçu REFUSED
sur sorties prévalidées sûres. Ce changement n’est PAS encore fait : l’ancien
témoin flag_presence de066 attend encore un reçu et reste inchangé dans cette
baseline. Après rouge acquis et arbitrage, aligner ce témoin sur le nouveau
contrat dans le correctif groupé ; aucune contradiction cachée en bilan vert.

## Exécution future et fin

Demande RO TOTALE de cette extraction/source/tests à Recherche et Banc sous
le même ticket. Nouveau pilote RED/pins/nonce/Q/préflight et RO couple à
préparer ensuite, puis GO C7 distinct avant tout Cargo ou test. Les anciens
pilotes sixFAIL0f et coupons consommés ne conviennent pas.

Proposition ciblée : compiler/inventorier les deux ELF depuis JSON, ledger
copié/fsync/rehash/ACK avant tout ELF, puis seulement les six nouveaux noms
exacts ci-dessus. ELF FS inventaire16 ; chaque négatif0PASS/1FAIL/0ignored/
15filtered/rc101, positif1PASS/0FAIL/0ignored/15filtered/rc0. Attribution nom,
assertion, cause et trace obligatoires ; erreur compilation/infra/mauvais
motif/positif rouge = STOP, sans retry. Le corps du second ELF et les dix
anciens tests ne sont pas proposés à l’exécution dans ce premier paquet.
Aucun résultat actuel ne leur est attribué. Même T/lock DBDB et contraintes
physiques/32G/Common, fermeture réelle et archives deux hôtes avant correction.

Après ces cinq vrais rouges et leur conservation seulement : fix GROUPÉ
outputs complets/distincts/hors input et non-troncature, traitement alias,
identité manquante, flags sans effet ; nouvelle source/pins/RO puis GO vert
séparé. Le lecteur replay reste manquant ; contrat global FS, corpus natif,
core/neufHOLD/A-B/ULP7.2/PR42 restent ouverts/non qualifiés.


RO TOTALE SOURCE RED Recherche bdf83ad/eab297 favorable PRÉPA sixcas reçue :
trois SHA Git conformes, tous initialiseurs ManifestSink recoupés, vrais
raccords natifs with_v3_report/build/File::create et document_ordinal/unwrap0
restés défectueux ; report_paths toujours non validés. Causalité des cinq
négatifs et contrôle positif relue, octets avant assertion sans parse du
reçu tronqué. Aucun autre blocage statique identifié pour cette extraction.
Inventaire16/attendu5FAIL+1PASS/15filtered NONOBSERVÉS ; dix anciens non
sélectionnés. Ancienne attente flags/reçu historique explicitement à aligner
après rouge acquis. Aucun Cargo/rustfmt/import/parse/test/ELF/proc/corpus/PC
par Recherche. Banc encore attendu ; paquet/pilote et RO couple puis GO
sixcas distinct requis. Pas de correctif groupé avant rouges conservés ;
replay/contrat FS global toujours ouverts.


RO TOTALE SOURCE RED Banc favorable PRÉPA reçue, publiée d1ecbb37e3 :
parent066/3SHA/16nomsordre exacts, raccords réels et défauts conservés, cinq
causes/positif/octets avantassert relus. Aucun autre blocage statique pour
RED ; résultats NONOBSERVÉS. Aucun exec/import/build/test/PC/DB/proc/corpus/
ELF/target/service/GO par Banc. Deux RO source favorables désormais reçues.
Arbitrage C7 30561f8229 : flags invalides doivent refuser AVANT toute metadata/
enumerate/read/réservation/output, donc zéro reçu fichier ; refus données
IO/UTF8/panel gardent REFUSED sur sorties prévalidées. Ancien témoin flags
à aligner seulement après rouge causal conservé dans correctif groupé.
Préparation du nouveau paquet/pilote/Qmetadata autorisée, aucun Cargo/test/
natif/mutationtarget ; RO couple puis GO exact distinct avant cinqFAIL+positif.
